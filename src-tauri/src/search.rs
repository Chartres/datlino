//! FTS5 BM25 keyword search over `chunk.text`.
//!
//! Week 1 only ships the keyword path. Hybrid (vector + BM25) re-ranking
//! arrives in Week 2 once `sqlite-vec` is wired up and embeddings exist.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub chunk_id: i64,
    pub document_id: i64,
    pub source_path: String,
    pub text: String,
    pub char_offset: i64,
    /// BM25 score; lower is better in SQLite's bm25() — we negate so callers
    /// can sort descending like a typical relevance score.
    pub score: f64,
}

pub fn search(conn: &Connection, query: &str, k: usize) -> Result<Vec<SearchHit>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let match_expr = build_match_expression(query);

    let mut stmt = conn.prepare(
        "SELECT
            c.id,
            c.document_id,
            d.source_path,
            c.text,
            c.char_offset,
            bm25(chunk_fts) AS bm
         FROM chunk_fts
         JOIN chunk    c ON c.id = chunk_fts.rowid
         JOIN document d ON d.id = c.document_id
         WHERE chunk_fts MATCH ?1
         ORDER BY bm ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![&match_expr, k as i64], |r| {
        Ok(SearchHit {
            chunk_id: r.get(0)?,
            document_id: r.get(1)?,
            source_path: r.get(2)?,
            text: r.get(3)?,
            char_offset: r.get(4)?,
            score: -r.get::<_, f64>(5)?,
        })
    })?;
    let mut hits = Vec::with_capacity(k);
    for row in rows {
        hits.push(row?);
    }
    Ok(hits)
}

/// Turn a free-form user query into a safe FTS5 MATCH expression. We tokenise
/// on whitespace, strip FTS5-significant characters (`"`, `*`, `(`, `)`, `:`),
/// drop empty tokens, and OR them together so a partial match still ranks.
fn build_match_expression(query: &str) -> String {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(sanitise_token)
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return String::new();
    }
    // Quote each token to disable FTS5 operators and avoid syntax errors.
    tokens
        .iter()
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn sanitise_token(t: &str) -> String {
    t.chars()
        .filter(|c| !matches!(c, '"' | '*' | '(' | ')' | ':' | '^'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::params;

    fn seed(conn: &Connection, sentences: &[(&str, &str)]) {
        conn.execute(
            "INSERT INTO document(source_path, kind, ingested_at, checksum) VALUES (?1, 'md', 0, 'x')",
            params!["/tmp/seed.md"],
        )
        .unwrap();
        let doc_id: i64 = conn
            .query_row("SELECT id FROM document WHERE source_path = '/tmp/seed.md'", [], |r| r.get(0))
            .unwrap();
        for (text, ctx) in sentences {
            conn.execute(
                "INSERT INTO chunk(document_id, text, char_offset, context, created_at) VALUES (?1, ?2, 0, ?3, 0)",
                params![doc_id, text, ctx],
            )
            .unwrap();
        }
    }

    #[test]
    fn empty_query_returns_no_hits() {
        let conn = db::open_in_memory().unwrap();
        assert!(search(&conn, "", 10).unwrap().is_empty());
        assert!(search(&conn, "   ", 10).unwrap().is_empty());
    }

    #[test]
    fn finds_match_by_keyword_with_diacritics() {
        let conn = db::open_in_memory().unwrap();
        seed(
            &conn,
            &[
                ("Habsburkové vládli v Čechách.", "Dějepis"),
                ("Fotosyntéza je proces.", "Biologie"),
            ],
        );
        let hits = search(&conn, "Habsburkové", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("Habsburkové"));
    }

    #[test]
    fn ranks_more_relevant_first() {
        let conn = db::open_in_memory().unwrap();
        seed(
            &conn,
            &[
                ("Habsburkové a Habsburkové vládli.", "Dějepis"),
                ("Habsburkové.", "Dějepis"),
                ("O něčem úplně jiném.", "Jiné"),
            ],
        );
        let hits = search(&conn, "Habsburkové", 3).unwrap();
        assert_eq!(hits.len(), 2);
        // Higher score first (we negated bm25).
        assert!(hits[0].score >= hits[1].score);
    }

    #[test]
    fn sanitises_fts_operators_in_user_query() {
        let conn = db::open_in_memory().unwrap();
        seed(&conn, &[("Něco o Habsburcích.", "Dějepis")]);
        // Should not error despite the asterisk / quote / colon / paren.
        let hits = search(&conn, "Habs*burc\"ích: (test)", 5).unwrap();
        assert!(!hits.is_empty());
    }
}
