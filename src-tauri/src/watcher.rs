//! Folder watcher built on `notify`.
//!
//! For each watched root we register a recursive watcher that re-ingests
//! changed/created files and forgets removed ones. The watcher runs on a
//! background thread and dispatches events via a channel to a worker that
//! holds a SQLite connection.
//!
//! Day-1 behaviour is intentionally simple — debouncing and batching land in
//! Week 4 once we know how chatty real student folders are.

use anyhow::{Context, Result};
use notify::event::{EventKind, ModifyKind};
use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::ingest::{self, IngestStats, ProgressReporter};
use rusqlite::Connection;
use serde::Serialize;

pub enum WatchCommand {
    AddRoot(PathBuf),
    RemoveRoot(PathBuf),
    Shutdown,
}

/// Progress reporter that dispatches Tauri events. The UI listens for
/// `datlino://ingest_progress` and re-renders the library with live
/// counts.
pub struct TauriProgress {
    pub tx: Sender<IngestEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IngestEvent {
    Start { path: String },
    File { path: String, stats: IngestStats },
    Done { stats: IngestStats },
}

impl ProgressReporter for TauriProgress {
    fn on_file_start(&self, path: &Path) {
        let _ = self.tx.send(IngestEvent::Start {
            path: path.to_string_lossy().to_string(),
        });
    }
    fn on_file_done(&self, path: &Path, stats: IngestStats) {
        let _ = self.tx.send(IngestEvent::File {
            path: path.to_string_lossy().to_string(),
            stats,
        });
    }
    fn on_tree_done(&self, stats: IngestStats) {
        let _ = self.tx.send(IngestEvent::Done { stats });
    }
}

pub struct WatcherHandle {
    tx: Sender<WatchCommand>,
    roots: Arc<Mutex<Vec<PathBuf>>>,
    pub ingest_events: Arc<Mutex<Option<Receiver<IngestEvent>>>>,
    #[allow(dead_code)]
    ingest_tx: Sender<IngestEvent>,
}

impl WatcherHandle {
    pub fn add_root(&self, path: PathBuf) -> Result<()> {
        self.tx
            .send(WatchCommand::AddRoot(path))
            .context("watcher worker died")
    }

    pub fn remove_root(&self, path: PathBuf) -> Result<()> {
        self.tx
            .send(WatchCommand::RemoveRoot(path))
            .context("watcher worker died")
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        self.roots.lock().unwrap().clone()
    }
}

/// Spawn the watcher worker. The connection is moved into a dedicated
/// thread so all DB writes happen serially.
pub fn spawn(mut conn: Connection) -> WatcherHandle {
    let (cmd_tx, cmd_rx): (Sender<WatchCommand>, Receiver<WatchCommand>) = channel();
    let (event_tx, event_rx) =
        channel::<std::result::Result<notify::Event, notify::Error>>();
    let (ingest_tx, ingest_rx) = channel::<IngestEvent>();
    let progress_tx = ingest_tx.clone();
    let roots = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
    let roots_for_thread = roots.clone();

    thread::Builder::new()
        .name("datlino-watcher".into())
        .spawn(move || {
            let mut watcher: RecommendedWatcher = match recommended_watcher(move |res| {
                let _ = event_tx.send(res);
            }) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("[datlino] failed to start file watcher: {e}");
                    return;
                }
            };
            let mut watched: HashMap<PathBuf, ()> = HashMap::new();

            loop {
                // Drain commands first (non-blocking), then events with a short
                // wait. Either side waking us is fine.
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        WatchCommand::AddRoot(p) => {
                            if watched.contains_key(&p) {
                                continue;
                            }
                            if let Err(e) =
                                watcher.watch(&p, RecursiveMode::Recursive)
                            {
                                eprintln!("[datlino] watch({}) failed: {e}", p.display());
                                continue;
                            }
                            // Initial bulk ingest of this root — streams
                            // progress back to the UI via ingest events.
                            let reporter = TauriProgress {
                                tx: progress_tx.clone(),
                            };
                            if let Err(e) = ingest::ingest_tree_with_progress(&mut conn, &p, &reporter) {
                                eprintln!("[datlino] initial ingest of {} failed: {e}", p.display());
                            }
                            watched.insert(p.clone(), ());
                            roots_for_thread.lock().unwrap().push(p);
                        }
                        WatchCommand::RemoveRoot(p) => {
                            let _ = watcher.unwatch(&p);
                            watched.remove(&p);
                            roots_for_thread
                                .lock()
                                .unwrap()
                                .retain(|x| x != &p);
                        }
                        WatchCommand::Shutdown => return,
                    }
                }

                match event_rx.recv_timeout(std::time::Duration::from_millis(250)) {
                    Ok(Ok(event)) => handle_event(&mut conn, event),
                    Ok(Err(e)) => eprintln!("[datlino] watcher error: {e}"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        })
        .expect("spawn watcher thread");

    WatcherHandle {
        tx: cmd_tx,
        roots,
        ingest_events: Arc::new(Mutex::new(Some(ingest_rx))),
        ingest_tx,
    }
}

fn handle_event(conn: &mut Connection, event: notify::Event) {
    let interesting = matches!(
        event.kind,
        EventKind::Create(_)
            | EventKind::Modify(ModifyKind::Data(_))
            | EventKind::Modify(ModifyKind::Name(_))
            | EventKind::Modify(ModifyKind::Any)
            | EventKind::Remove(_)
    );
    if !interesting {
        return;
    }
    for path in event.paths {
        if let EventKind::Remove(_) = event.kind {
            let _ = ingest::forget_path(conn, &path);
            continue;
        }
        if path.is_file() && ingest::DocKind::from_path(&path).is_some() {
            if let Err(e) = ingest::ingest_file(conn, &path) {
                eprintln!("[datlino] ingest({}) failed: {e}", path.display());
            }
        }
    }
}

/// Plain unwatched bulk ingest, used by tests and the dev CLI.
pub fn ingest_only(conn: &mut Connection, root: &Path) -> Result<ingest::IngestStats> {
    ingest::ingest_tree(conn, root)
}
