//! End-to-end smoke test against the bundled CZ fixtures.
//!
//! Runs the full Week-1 pipeline (ingest → FTS5 → search) on the five
//! Markdown files in `/fixtures/cz`. Catches regressions where any layer
//! works in isolation but the integration breaks.

use datlino_lib::{db, ingest, search};
use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    // tests/ is at src-tauri/tests; fixtures/cz is at repo root.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("repo root")
        .join("fixtures")
        .join("cz")
}

#[test]
fn ingests_all_five_cz_fixtures() {
    let mut conn = db::open_in_memory().unwrap();
    let stats = ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    assert_eq!(stats.files_seen, 5, "expected exactly five fixture files");
    assert_eq!(stats.files_ingested, 5);
    assert!(stats.chunks_written > 30, "expected a healthy chunk count");
}

#[test]
fn finds_habsburg_content() {
    let mut conn = db::open_in_memory().unwrap();
    ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    let hits = search::search(&conn, "Habsburkové", 5).unwrap();
    assert!(!hits.is_empty(), "Habsburkové should be searchable");
    assert!(
        hits.iter().any(|h| h.source_path.contains("dejepis")),
        "top hit should be from dejepis fixture: {:?}",
        hits.iter().map(|h| &h.source_path).collect::<Vec<_>>()
    );
}

#[test]
fn finds_photosynthesis_content() {
    let mut conn = db::open_in_memory().unwrap();
    ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    let hits = search::search(&conn, "fotosyntéza", 5).unwrap();
    assert!(!hits.is_empty(), "fotosyntéza should be searchable");
    assert!(hits.iter().any(|h| h.source_path.contains("biologie")));
}

#[test]
fn cermat_phrase_query_returns_relevant_passage() {
    let mut conn = db::open_in_memory().unwrap();
    ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    // Multi-word query — exercises sanitisation + OR scoring.
    let hits = search::search(&conn, "stavovské povstání bitva", 5).unwrap();
    assert!(!hits.is_empty());
    let top = &hits[0];
    assert!(
        top.text.to_lowercase().contains("bitva")
            || top.text.to_lowercase().contains("stav"),
        "expected battle/uprising hit, got: {}",
        top.text
    );
}
