//! Datlino library entry point.
//!
//! Exposes the Tauri commands the SvelteKit frontend invokes and bundles
//! the runtime modules. `main.rs` is a thin shim that calls `run()`.

pub mod db;
pub mod ingest;
pub mod search;
pub mod segmenter;
pub mod watcher;

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

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
    state
        .watcher
        .add_root(p)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_watched_folder(
    path: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), String> {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("resolving app_data_dir");
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("datlino.db");

            // The watcher worker owns its own connection; the UI commands
            // share a separate connection through AppState.
            let watcher_conn = db::open(&db_path)?;
            let ui_conn = db::open(&db_path)?;
            let watcher = watcher::spawn(watcher_conn);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Datlino");
}

/// Headless bootstrap helper used by integration tests and dev tools — sets
/// up a fully-migrated SQLite at the given path and returns the connection.
pub fn bootstrap_db(path: &std::path::Path) -> Result<Connection> {
    db::open(path)
}
