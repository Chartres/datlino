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
fn ingests_all_cz_fixtures() {
    let mut conn = db::open_in_memory().unwrap();
    let stats = ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    assert!(stats.files_seen >= 10, "expected at least 10 fixture files");
    assert_eq!(stats.files_ingested, stats.files_seen);
    assert!(stats.chunks_written > 80, "expected a healthy chunk count");
}

#[test]
fn ingests_pdf_fixture_through_pdf_extract() {
    let mut conn = db::open_in_memory().unwrap();
    ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();

    let pdf_chunks: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunk c
             JOIN document d ON d.id = c.document_id
             WHERE d.kind = 'pdf'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        pdf_chunks >= 5,
        "expected the PDF fixture to produce at least a handful of chunks, got {pdf_chunks}"
    );

    // BM25 search across the whole corpus should find PDF content.
    let hits = search::search(&conn, "Roosevelt", 5).unwrap();
    assert!(
        hits.iter().any(|h| h.source_path.ends_with(".pdf")),
        "search should surface PDF content: {:?}",
        hits.iter().map(|h| &h.source_path).collect::<Vec<_>>()
    );
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
