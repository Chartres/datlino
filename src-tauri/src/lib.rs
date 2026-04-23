//! Datlino library entry point.
//!
//! Exposes the Tauri commands the SvelteKit frontend invokes and bundles
//! the runtime modules. `main.rs` is a thin shim that calls `run()`.

pub mod db;
pub mod embeddings;
pub mod ingest;
pub mod pedagogy;
pub mod progress;
pub mod search;
pub mod segmenter;
pub mod session;
pub mod watcher;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

/// Single-user install for MVP — every command uses user_id = 1.
const DEFAULT_USER_ID: i64 = 1;

/// App-wide state. `conn` is wrapped in a Mutex because rusqlite's
/// `Connection` is single-threaded.
pub struct AppState {
    pub conn: Mutex<Connection>,
    pub watcher: watcher::WatcherHandle,
}

#[derive(Debug, Serialize)]
pub struct IndexStatus {
    pub document_count: i64,
    pub chunk_count: i64,
    pub watched_roots: Vec<String>,
}

// ---------- library / search ----------

#[tauri::command]
fn search_chunks(
    query: String,
    k: usize,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<search::SearchHit>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    search::search(&conn, &query, k.clamp(1, 100)).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_watched_folder(
    path: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    let canonical = std::fs::canonicalize(&p)
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or(path);
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO watched_folder(path, added_at) VALUES (?1, strftime('%s','now'))",
            rusqlite::params![&canonical],
        )
        .map_err(|e| e.to_string())?;
    }
    state
        .watcher
        .add_root(PathBuf::from(canonical))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_watched_folder(
    path: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
    {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM watched_folder WHERE path = ?1",
            rusqlite::params![&path],
        )
        .map_err(|e| e.to_string())?;
    }
    state
        .watcher
        .remove_root(PathBuf::from(path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn index_status(state: State<'_, AppState>) -> std::result::Result<IndexStatus, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let document_count: i64 = conn
        .query_row("SELECT count(*) FROM document", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let chunk_count: i64 = conn
        .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let watched_roots = state
        .watcher
        .roots()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    Ok(IndexStatus {
        document_count,
        chunk_count,
        watched_roots,
    })
}

// ---------- sessions ----------

#[tauri::command]
fn create_session(
    request: session::SessionRequest,
    state: State<'_, AppState>,
) -> std::result::Result<session::SessionPlan, String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    session::create_session(&mut conn, DEFAULT_USER_ID, &request).map_err(|e| e.to_string())
}

#[tauri::command]
fn finalize_session(
    session_id: i64,
    attempts: Vec<progress::AttemptRecord>,
    state: State<'_, AppState>,
) -> std::result::Result<progress::SessionSummary, String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    progress::finalize_session(&mut conn, DEFAULT_USER_ID, session_id, &attempts)
        .map_err(|e| e.to_string())
}

// ---------- profile / history ----------

#[tauri::command]
fn get_profile(
    state: State<'_, AppState>,
) -> std::result::Result<progress::UserProfileView, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    progress::user_profile_view(&conn, DEFAULT_USER_ID).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_history(
    limit: usize,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<progress::SessionHistoryRow>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    progress::session_history(&conn, DEFAULT_USER_ID, limit.clamp(1, 200))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_weak_ngrams(
    limit: usize,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<pedagogy::WeakNgram>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    pedagogy::weak_ngrams(&conn, DEFAULT_USER_ID, limit.clamp(1, 50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_chapters(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<session::ChapterInfo>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    session::list_chapters(&conn).map_err(|e| e.to_string())
}

// ---------- embedding provider + settings ----------

#[derive(Debug, Serialize)]
pub struct EmbeddingStatus {
    pub provider: String,
    pub dim: i64,
    pub embedded_chunks: i64,
    pub total_chunks: i64,
    pub cohere_key_present: bool,
}

#[tauri::command]
fn get_embedding_status(
    state: State<'_, AppState>,
) -> std::result::Result<EmbeddingStatus, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let (provider, dim): (String, i64) = conn
        .query_row(
            "SELECT provider, dim FROM embedding_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let embedded_chunks: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunk WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let total_chunks: i64 = conn
        .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let cohere_key_present = embeddings::read_cohere_key()
        .ok()
        .flatten()
        .is_some();
    Ok(EmbeddingStatus {
        provider,
        dim,
        embedded_chunks,
        total_chunks,
        cohere_key_present,
    })
}

#[tauri::command]
fn set_cohere_api_key(key: String) -> std::result::Result<(), String> {
    embeddings::write_cohere_key(key.trim()).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct EmbedProgress {
    pub embedded: usize,
    pub total_chunks: i64,
}

#[tauri::command]
fn set_embedding_provider(
    kind: String,
    state: State<'_, AppState>,
) -> std::result::Result<EmbedProgress, String> {
    let kind = embeddings::EmbeddingProviderKind::parse(&kind);
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let provider = embeddings::build(kind).map_err(|e| e.to_string())?;
    embeddings::ensure_vec_table_matches(&conn, kind, provider.dim())
        .map_err(|e| e.to_string())?;
    let embedded = embeddings::reembed_missing(&mut conn, &*provider, 96)
        .map_err(|e| e.to_string())?;
    let total_chunks: i64 = conn
        .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(EmbedProgress {
        embedded,
        total_chunks,
    })
}

#[tauri::command]
fn embed_pending(
    state: State<'_, AppState>,
) -> std::result::Result<EmbedProgress, String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let (provider_str, _): (String, i64) = conn
        .query_row(
            "SELECT provider, dim FROM embedding_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let kind = embeddings::EmbeddingProviderKind::parse(&provider_str);
    let provider = embeddings::build(kind).map_err(|e| e.to_string())?;
    let embedded =
        embeddings::reembed_missing(&mut conn, &*provider, 96).map_err(|e| e.to_string())?;
    let total_chunks: i64 = conn
        .query_row("SELECT count(*) FROM chunk", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(EmbedProgress {
        embedded,
        total_chunks,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("resolving app_data_dir");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("datlino.db");

            // The watcher worker owns its own connection; the UI commands
            // share a separate connection through AppState.
            let watcher_conn = db::open(&db_path)?;
            let ui_conn = db::open(&db_path)?;
            let watcher = watcher::spawn(watcher_conn);

            // Restore watched folders from the previous session. Migration
            // 0003 invalidated checksums, so the initial ingest inside each
            // AddRoot command will back-fill section info on every chunk.
            {
                let mut stmt =
                    ui_conn.prepare("SELECT path FROM watched_folder ORDER BY added_at ASC")?;
                let paths: Vec<String> = stmt
                    .query_map([], |r| r.get::<_, String>(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                for p in paths {
                    let pb = PathBuf::from(&p);
                    if pb.exists() {
                        let _ = watcher.add_root(pb);
                    }
                }
            }

            app.manage(AppState {
                conn: Mutex::new(ui_conn),
                watcher,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            search_chunks,
            add_watched_folder,
            remove_watched_folder,
            index_status,
            create_session,
            finalize_session,
            get_profile,
            get_history,
            get_weak_ngrams,
            list_chapters,
            get_embedding_status,
            set_cohere_api_key,
            set_embedding_provider,
            embed_pending,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Datlino");
}

/// Headless bootstrap helper used by integration tests and dev tools — sets
/// up a fully-migrated SQLite at the given path and returns the connection.
pub fn bootstrap_db(path: &std::path::Path) -> Result<Connection> {
    db::open(path)
}
