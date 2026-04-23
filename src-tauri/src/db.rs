//! SQLite schema, migrations, and connection helpers.
//!
//! Storage is a single SQLite file kept in the platform user-data dir.
//! Week 1 wires up:
//!   * `document` and `chunk` tables (per §8 of the build brief)
//!   * `chunk_fts` FTS5 virtual table for BM25 search
//!   * triggers that keep FTS5 in sync with `chunk` writes/deletes
//!
//! Deferred to Week 2:
//!   * `chunk_vec` virtual table from the `sqlite-vec` extension
//!   * `embedding` column population
//!   * the rest of the v2 schema (`user_profile`, `ngram_stat`, `session`,
//!     `attempt`, `rephrased_chunk`)
//!
//! The schema is stored as raw SQL strings and applied by `apply_migrations`
//! inside a transaction. New migrations append to the `MIGRATIONS` slice.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

const MIGRATIONS: &[&str] = &[
    // 0001 — base schema, FTS5, sync triggers
    r#"
    CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY,
        applied_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS document (
        id INTEGER PRIMARY KEY,
        source_path TEXT UNIQUE NOT NULL,
        kind TEXT NOT NULL,
        ingested_at INTEGER NOT NULL,
        checksum TEXT NOT NULL,
        tags TEXT
    );

    CREATE TABLE IF NOT EXISTS chunk (
        id INTEGER PRIMARY KEY,
        document_id INTEGER NOT NULL REFERENCES document(id) ON DELETE CASCADE,
        text TEXT NOT NULL,
        char_offset INTEGER NOT NULL,
        context TEXT,
        embedding BLOB,
        created_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_chunk_document ON chunk(document_id);

    -- FTS5 mirrors chunk.text for BM25 ranking.
    CREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(
        text,
        content='chunk',
        content_rowid='id',
        tokenize='unicode61 remove_diacritics 0'
    );

    CREATE TRIGGER IF NOT EXISTS chunk_ai AFTER INSERT ON chunk BEGIN
        INSERT INTO chunk_fts(rowid, text) VALUES (new.id, new.text);
    END;
    CREATE TRIGGER IF NOT EXISTS chunk_ad AFTER DELETE ON chunk BEGIN
        INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES ('delete', old.id, old.text);
    END;
    CREATE TRIGGER IF NOT EXISTS chunk_au AFTER UPDATE ON chunk BEGIN
        INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES ('delete', old.id, old.text);
        INSERT INTO chunk_fts(rowid, text) VALUES (new.id, new.text);
    END;

    -- chunk_vec (sqlite-vec) is added in Week 2 once the embedding provider
    -- abstraction lands. Schema sketch from §8 of the brief, kept in a
    -- comment for reference:
    --   CREATE VIRTUAL TABLE chunk_vec USING vec0(embedding float[384]);
    "#,
    // 0002 — user profile, n-gram stats, sessions, attempts, badges
    r#"
    CREATE TABLE IF NOT EXISTS user_profile (
        id INTEGER PRIMARY KEY,
        alpha_default REAL NOT NULL DEFAULT 0.7,
        wpm_baseline REAL,
        accuracy_baseline REAL,
        embedding_provider TEXT NOT NULL DEFAULT 'none',
        total_xp INTEGER NOT NULL DEFAULT 0,
        current_streak INTEGER NOT NULL DEFAULT 0,
        longest_streak INTEGER NOT NULL DEFAULT 0,
        last_session_date TEXT,
        total_sessions INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    );

    INSERT INTO user_profile(id, created_at) VALUES (1, strftime('%s','now'))
    ON CONFLICT(id) DO NOTHING;

    CREATE TABLE IF NOT EXISTS ngram_stat (
        user_id INTEGER NOT NULL,
        ngram TEXT NOT NULL,
        occurrences INTEGER NOT NULL DEFAULT 0,
        error_count INTEGER NOT NULL DEFAULT 0,
        ema_latency_ms REAL,
        ema_error_rate REAL,
        last_seen INTEGER,
        PRIMARY KEY (user_id, ngram)
    );
    CREATE INDEX IF NOT EXISTS idx_ngram_weakness ON ngram_stat(user_id, ema_error_rate DESC, ema_latency_ms DESC);

    CREATE TABLE IF NOT EXISTS session (
        id INTEGER PRIMARY KEY,
        user_id INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        mode TEXT NOT NULL,
        alpha REAL NOT NULL,
        target_duration_s INTEGER,
        query TEXT,
        pinned_sources TEXT,
        finished_at INTEGER,
        xp_earned INTEGER NOT NULL DEFAULT 0,
        summary_json TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_session_user_time ON session(user_id, created_at DESC);

    CREATE TABLE IF NOT EXISTS attempt (
        id INTEGER PRIMARY KEY,
        session_id INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
        chunk_id INTEGER REFERENCES chunk(id) ON DELETE SET NULL,
        rephrased_chunk_id INTEGER,
        started_at INTEGER NOT NULL,
        finished_at INTEGER,
        wpm REAL,
        accuracy REAL,
        target_text TEXT NOT NULL,
        keystroke_log BLOB
    );
    CREATE INDEX IF NOT EXISTS idx_attempt_session ON attempt(session_id);

    CREATE TABLE IF NOT EXISTS badge (
        id INTEGER PRIMARY KEY,
        user_id INTEGER NOT NULL,
        code TEXT NOT NULL,
        earned_at INTEGER NOT NULL,
        UNIQUE (user_id, code)
    );
    "#,
];

pub fn open<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open(&path)
        .with_context(|| format!("opening sqlite at {}", path.as_ref().display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    apply_migrations(&conn)?;
    Ok(conn)
}

/// Open an in-memory database (used by tests).
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    apply_migrations(&conn)?;
    Ok(conn)
}

fn apply_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );",
    )?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let version = (i as i64) + 1;
        if version <= current {
            continue;
        }
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("applying migration {version}"))?;
        tx.execute(
            "INSERT INTO schema_version(version, applied_at) VALUES (?1, ?2)",
            params![version, now_unix()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_in_memory().unwrap();
        // Re-applying must not fail.
        apply_migrations(&conn).unwrap();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn fts_trigger_indexes_inserted_chunks() {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO document(source_path, kind, ingested_at, checksum) VALUES (?1, ?2, ?3, ?4)",
            params!["/tmp/x.md", "md", 0, "deadbeef"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chunk(document_id, text, char_offset, context, created_at) VALUES (1, ?1, 0, ?1, 0)",
            params!["Habsburkové vládli v Čechách."],
        )
        .unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM chunk_fts WHERE chunk_fts MATCH 'Habsburkové'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
