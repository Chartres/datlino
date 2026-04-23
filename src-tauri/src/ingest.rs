//! Folder ingestion for `.md` and `.txt` files.
//!
//! Walks a directory tree, reads each supported file, segments it via the
//! CZ/SK-aware `segmenter`, and upserts the document + its chunks into
//! SQLite. Re-ingesting the same path is cheap when the file's checksum is
//! unchanged.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::db::now_unix;
use crate::segmenter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Markdown,
    Text,
}

impl DocKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocKind::Markdown => "md",
            DocKind::Text => "txt",
        }
    }

    pub fn from_path(path: &Path) -> Option<DocKind> {
        match path.extension().and_then(|s| s.to_str()) {
            Some("md") | Some("markdown") => Some(DocKind::Markdown),
            Some("txt") => Some(DocKind::Text),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct IngestStats {
    pub files_seen: usize,
    pub files_ingested: usize,
    pub files_skipped_unchanged: usize,
    pub chunks_written: usize,
}

/// Ingest a single file. Returns true if the document was (re-)indexed.
pub fn ingest_file(conn: &mut Connection, path: &Path) -> Result<bool> {
    let kind = match DocKind::from_path(path) {
        Some(k) => k,
        None => return Ok(false),
    };
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("canonicalising {}", path.display()))?;
    let raw = fs::read_to_string(&canonical)
        .with_context(|| format!("reading {}", canonical.display()))?;
    let checksum = sha256_hex(raw.as_bytes());
    let canonical_str = canonical.to_string_lossy().to_string();

    // Skip if document is unchanged.
    let existing: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, checksum FROM document WHERE source_path = ?1",
            params![&canonical_str],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .ok();
    if let Some((_, prev)) = &existing {
        if prev == &checksum {
            return Ok(false);
        }
    }

    let tx = conn.transaction()?;
    let document_id = if let Some((id, _)) = existing {
        tx.execute("DELETE FROM chunk WHERE document_id = ?1", params![id])?;
        tx.execute(
            "UPDATE document SET kind = ?1, ingested_at = ?2, checksum = ?3 WHERE id = ?4",
            params![kind.as_str(), now_unix(), &checksum, id],
        )?;
        id
    } else {
        tx.execute(
            "INSERT INTO document(source_path, kind, ingested_at, checksum) VALUES (?1, ?2, ?3, ?4)",
            params![&canonical_str, kind.as_str(), now_unix(), &checksum],
        )?;
        tx.last_insert_rowid()
    };

    let sentences = segmenter::segment(&raw);
    {
        let mut stmt = tx.prepare(
            "INSERT INTO chunk(document_id, text, char_offset, context, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let now = now_unix();
        for s in &sentences {
            stmt.execute(params![
                document_id,
                &s.text,
                s.byte_offset as i64,
                &s.context,
                now,
            ])?;
        }
    }
    tx.commit()?;
    Ok(true)
}

/// Recursively ingest every supported file under `root`.
pub fn ingest_tree(conn: &mut Connection, root: &Path) -> Result<IngestStats> {
    let mut stats = IngestStats::default();
    walk(conn, root, &mut stats)?;
    Ok(stats)
}

fn walk(conn: &mut Connection, dir: &Path, stats: &mut IngestStats) -> Result<()> {
    if !dir.is_dir() {
        // single-file root: still try
        if DocKind::from_path(dir).is_some() {
            stats.files_seen += 1;
            ingest_one_with_stats(conn, dir, stats)?;
        }
        return Ok(());
    }
    for entry in fs::read_dir(dir)
        .with_context(|| format!("reading dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // skip hidden dirs (.git, .svelte-kit, node_modules, target)
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.') || matches!(n, "node_modules" | "target" | "build"))
                .unwrap_or(false)
            {
                continue;
            }
            walk(conn, &path, stats)?;
        } else if DocKind::from_path(&path).is_some() {
            stats.files_seen += 1;
            ingest_one_with_stats(conn, &path, stats)?;
        }
    }
    Ok(())
}

fn ingest_one_with_stats(
    conn: &mut Connection,
    path: &Path,
    stats: &mut IngestStats,
) -> Result<()> {
    let did_write = ingest_file(conn, path)?;
    if did_write {
        stats.files_ingested += 1;
        let n: i64 = conn.query_row(
            "SELECT count(*) FROM chunk WHERE document_id = (
                SELECT id FROM document WHERE source_path = ?1
             )",
            params![path
                .canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string())],
            |r| r.get(0),
        )?;
        stats.chunks_written += n as usize;
    } else {
        stats.files_skipped_unchanged += 1;
    }
    Ok(())
}

/// Remove a document (and its chunks via ON DELETE CASCADE) by path.
pub fn forget_path(conn: &Connection, path: &Path) -> Result<bool> {
    let canonical = path
        .canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string());
    let n = conn.execute(
        "DELETE FROM document WHERE source_path = ?1",
        params![canonical],
    )?;
    Ok(n > 0)
}

/// List all documents (used by the watcher to reconcile).
pub fn document_paths(conn: &Connection) -> Result<Vec<PathBuf>> {
    let mut stmt = conn.prepare("SELECT source_path FROM document")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(PathBuf::from(row?));
    }
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    let digest = h.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn ingests_markdown_file_and_skips_unchanged() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p = write_file(tmp.path(), "a.md", "První věta. Druhá věta.");

        assert!(ingest_file(&mut conn, &p).unwrap());

        let n: i64 = conn
            .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);

        // Re-ingest same file → no changes.
        assert!(!ingest_file(&mut conn, &p).unwrap());
        let n2: i64 = conn
            .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n2, 2);
    }

    #[test]
    fn re_ingest_after_edit_replaces_chunks() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p = write_file(tmp.path(), "a.md", "První.");
        ingest_file(&mut conn, &p).unwrap();

        // Edit
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(b"Nove. Vety. Tri.").unwrap();
        ingest_file(&mut conn, &p).unwrap();

        let n: i64 = conn
            .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 3);
    }

    #[test]
    fn ingest_tree_walks_recursively_and_ignores_other_extensions() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        write_file(tmp.path(), "a.md", "Věta jedna.");
        write_file(tmp.path(), "sub/b.txt", "Věta dvě.");
        write_file(tmp.path(), "sub/c.bin", "ignored");

        let stats = ingest_tree(&mut conn, tmp.path()).unwrap();
        assert_eq!(stats.files_seen, 2);
        assert_eq!(stats.files_ingested, 2);

        let n: i64 = conn
            .query_row("SELECT count(*) FROM document", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
    }
}
