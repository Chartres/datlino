//! Persona and journey tests — each one walks the real Datlino APIs as a
//! particular student would, from empty state through a full session and
//! the state it leaves behind. They exist so that big refactors can't
//! silently break a user's story.
//!
//! Three personas:
//!
//! * **Paja** — maturita student with Czech history notes (markdown + PDF).
//!   Uses ExamPrep to pull whole chapters matching her exam topic, types
//!   through them, sees her level and streak update.
//!
//! * **Martin** — language learner with English + German notes. Prefers
//!   Chapter mode because he wants to study a specific grammar section
//!   end-to-end. Cares that diacritic n-gram stats reflect the language
//!   of the material he's actually typing.
//!
//! * **Eliška** — brand-new user with zero library. Can't use any content
//!   mode; needs Diacritics drill + Warmup to fall back gracefully.
//!
//! The persona harness doesn't hit the network — Cohere lives behind a key
//! and is exercised by unit tests in `embeddings.rs`. Hybrid and ExamPrep
//! here use BM25 + the deterministic Fake embedder.

use datlino_lib::embeddings::{self, EmbeddingProvider, EmbeddingProviderKind};
use datlino_lib::pedagogy::Keystroke;
use datlino_lib::progress::{self, AttemptRecord};
use datlino_lib::session::{self, ContentStrategy, PracticeMode, SessionRequest, SessionSentence};
use datlino_lib::{db, ingest, search};

use std::fs;
use tempfile::TempDir;

// ---------- shared harness ----------

/// Simulate a student typing a sentence. `error_positions` is the list of
/// codepoint indices where they mistyped (we replace with 'x').
fn simulate_typing(target: &str, error_positions: &[usize]) -> AttemptRecord {
    let chars: Vec<char> = target.chars().collect();
    let keystrokes: Vec<Keystroke> = chars
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let correct = !error_positions.contains(&i);
            Keystroke {
                t_ms: (i as u64) * 120,
                actual: if correct {
                    c.to_string()
                } else {
                    "x".to_string()
                },
                expected: c.to_string(),
                correct,
            }
        })
        .collect();
    let finish = (chars.len() as u64) * 120;
    AttemptRecord {
        chunk_id: None,
        target_text: target.to_string(),
        started_at_ms: 0,
        finished_at_ms: finish,
        keystrokes,
        completed: error_positions.is_empty(),
    }
}

fn fixtures_root() -> std::path::PathBuf {
    // tests/ is at src-tauri/tests; fixtures/cz is at repo root.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("repo root")
        .join("fixtures")
        .join("cz")
}

// ---------- Persona 1: Paja, maturita student ----------

#[test]
fn paja_prepares_for_maturita_with_exam_prep() {
    let mut conn = db::open_in_memory().unwrap();
    // She adds her dějepis folder — the fixtures already cover 20th-century
    // topics including the Great Depression PDF.
    let stats = ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();
    assert!(stats.files_ingested >= 10, "all fixtures ingested");

    // Step 1: she describes her exam topic in natural language.
    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 600,
            query: Some("Velká hospodářská krize New Deal Roosevelt".into()),
            content_strategy: Some(ContentStrategy::ExamPrep),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!plan.sentences.is_empty(), "ExamPrep returned content");
    // Expect chapters, so plenty of sentences — not just two stray matches.
    assert!(
        plan.sentences.len() >= 5,
        "ExamPrep should return whole chapters, got {}",
        plan.sentences.len()
    );
    // The Great Depression PDF fixture is the obvious top-scoring source.
    let has_depression = plan
        .sentences
        .iter()
        .any(|s| s.text.contains("hospodářská") || s.text.contains("Depression") || s.text.contains("Roosevelt"));
    assert!(has_depression, "expected the GD chapter to rank high");

    // Step 2: she types through the first few. Mixes clean runs and a
    // handful of errors so pedagogy stats accumulate.
    let attempts: Vec<_> = plan
        .sentences
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, s)| {
            // Every third sentence has a stumbled key — realistic, not perfect.
            if i % 3 == 0 {
                simulate_typing(&s.text, &[3])
            } else {
                simulate_typing(&s.text, &[])
            }
        })
        .collect();

    let summary = progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();

    // What her experience looks like right after.
    assert!(summary.xp_earned > 0, "earned XP");
    assert!(summary.wpm > 20.0, "wpm realistic for 120ms/key");
    assert!(summary.accuracy_pct > 80.0, "mostly correct");
    assert!(
        summary.badges_awarded.contains(&"first_session".to_string()),
        "first-session badge"
    );
    assert_eq!(summary.current_streak, 1);

    // Step 3: she comes back to the progress page — the profile reflects
    // the session and she has at least one recorded weak bigram to work on.
    let view = progress::user_profile_view(&conn, 1).unwrap();
    assert_eq!(view.total_sessions, 1);
    assert!(view.total_xp > 0);
    let history = progress::session_history(&conn, 1, 10).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].mode, "content");
    let weak = datlino_lib::pedagogy::weak_ngrams(&conn, 1, 20).unwrap();
    assert!(!weak.is_empty(), "some weak n-grams after errors");
}

// ---------- Persona 2: Martin, language learner ----------

#[test]
fn martin_studies_one_chapter_end_to_end() {
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();

    // Martin has a grammar note on present perfect, nicely structured.
    let p = tmp.path().join("english-grammar.md");
    fs::write(
        &p,
        "# English Grammar\n\n## Present Perfect\n\n\
         I have lived here for five years. \
         She has finished her homework. \
         They have never been to Paris. \
         We have just arrived at the station.\n\n\
         ## Past Simple\n\n\
         Odlišná kapitola, jiné věty, na kterou se teď nechceme soustředit.",
    )
    .unwrap();
    ingest::ingest_file(&mut conn, &p).unwrap();

    let chapters = session::list_chapters(&conn).unwrap();
    let pp_chapter = chapters
        .iter()
        .find(|c| c.section.contains("Present Perfect"))
        .expect("Present Perfect chapter is listed");
    assert!(pp_chapter.sentence_count >= 4);

    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            content_strategy: Some(ContentStrategy::Chapter),
            chapter_id: Some(pp_chapter.id.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(
        plan.sentences.len(),
        4,
        "Chapter mode returned all 4 present-perfect sentences"
    );
    // Past Simple chapter should not leak.
    assert!(plan
        .sentences
        .iter()
        .all(|s: &SessionSentence| !s.text.contains("Odlišná")));
    // Source order is preserved.
    assert!(plan.sentences[0].text.starts_with("I have lived"));
    assert!(plan.sentences[3].text.starts_with("We have just"));

    // He types it, makes errors on the 'h' key (Czech speakers commonly
    // drop it when typing English). Pedagogy captures that.
    let mut attempts: Vec<AttemptRecord> = Vec::new();
    for s in &plan.sentences {
        let error_idxs: Vec<usize> = s
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, c)| if c.to_ascii_lowercase() == 'h' && i < 6 { Some(i) } else { None })
            .collect();
        attempts.push(simulate_typing(&s.text, &error_idxs));
    }
    let summary = progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();

    // 'h' should be the single most-flagged weak bigram / unigram because
    // that's where every error occurred.
    assert!(summary
        .weak_preview
        .iter()
        .any(|w| w.ngram.contains('h') || w.ngram.contains('H')));
}

// ---------- Persona 3: Eliška, new user, empty library ----------

#[test]
fn eliska_starts_without_a_library_and_still_gets_a_session() {
    let mut conn = db::open_in_memory().unwrap();

    // Content modes should not crash or return garbage.
    let content_plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 120,
            query: Some("Great Depression".into()),
            content_strategy: Some(ContentStrategy::Across),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        content_plan.sentences.is_empty(),
        "no library → no content; the UI shows a helpful error"
    );

    // Diacritics always works — no corpus needed.
    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Diacritics,
            alpha: 0.0,
            target_duration_s: 120,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!plan.sentences.is_empty(), "Diacritics drill always generates");
    assert!(plan.sentences.iter().all(|s| s.is_generated));

    // She completes her first drill. First-session badge + streak = 1.
    let attempts: Vec<_> = plan
        .sentences
        .iter()
        .take(3)
        .map(|s| simulate_typing(&s.text, &[]))
        .collect();
    let summary =
        progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();
    assert_eq!(summary.current_streak, 1);
    assert_eq!(summary.sentences_completed, 3);
    assert!(summary.badges_awarded.contains(&"first_session".to_string()));
    assert_eq!(summary.level, 0, "level stays 0 on a tiny first drill");
}

// ---------- Journey: embedding provider switch invalidates old vectors ----------

#[test]
fn switching_embedding_provider_resets_stale_vectors() {
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("x.md");
    fs::write(&p, "Habsburkove vladli. Druha veta o historii. Treti veta.").unwrap();
    ingest::ingest_file(&mut conn, &p).unwrap();

    // Step 1: enable the Fake provider, embed everything.
    let fake = embeddings::FakeEmbedder::new();
    embeddings::ensure_vec_table_matches(&conn, EmbeddingProviderKind::Fake, fake.dim()).unwrap();
    let embedded = embeddings::reembed_missing(&mut conn, &fake, 10).unwrap();
    assert!(embedded >= 3);
    let count_embedded: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunk WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count_embedded, 3);

    // Step 2: "switch" to a different dim (simulate provider change).
    // The helper should drop chunk_vec and clear all chunk.embedding BLOBs.
    embeddings::ensure_vec_table_matches(&conn, EmbeddingProviderKind::Cohere, 1024).unwrap();
    let after_switch: i64 = conn
        .query_row(
            "SELECT count(*) FROM chunk WHERE embedding IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        after_switch, 0,
        "stale Fake vectors are cleared when provider dim changes"
    );
}

// ---------- Journey: watched-folder persistence (simulated via DB) ----------

#[test]
fn watched_folders_persist_across_sessions_via_db() {
    // Simulates the app being closed and reopened — the watcher restart
    // logic lives in lib.rs::run; here we verify the DB contract it relies
    // on (table exists, INSERT/DELETE work, SELECT is ordered).
    let conn = db::open_in_memory().unwrap();
    conn.execute(
        "INSERT INTO watched_folder(path, added_at) VALUES (?1, strftime('%s','now'))",
        rusqlite::params!["/home/user/notes"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO watched_folder(path, added_at) VALUES (?1, strftime('%s','now'))",
        rusqlite::params!["/home/user/school"],
    )
    .unwrap();

    let paths: Vec<String> = conn
        .prepare("SELECT path FROM watched_folder ORDER BY added_at ASC")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(paths, vec!["/home/user/notes", "/home/user/school"]);

    // Remove one; the other remains.
    conn.execute(
        "DELETE FROM watched_folder WHERE path = ?1",
        rusqlite::params!["/home/user/notes"],
    )
    .unwrap();
    let remaining: i64 = conn
        .query_row("SELECT count(*) FROM watched_folder", [], |r| r.get(0))
        .unwrap();
    assert_eq!(remaining, 1);
}

// ---------- Sanity: search flow reaches both file kinds ----------

#[test]
fn search_reaches_markdown_and_pdf_fixtures_independently() {
    let mut conn = db::open_in_memory().unwrap();
    ingest::ingest_tree(&mut conn, &fixtures_root()).unwrap();

    // Topic only the PDF covers.
    let pdf_hits = search::search(&conn, "Roosevelt", 5).unwrap();
    assert!(
        pdf_hits.iter().any(|h| h.source_path.ends_with(".pdf")),
        "PDF content must be searchable: {pdf_hits:?}"
    );

    // Topic only the markdown fixtures cover.
    let md_hits = search::search(&conn, "fotosyntéza", 5).unwrap();
    assert!(
        md_hits.iter().any(|h| h.source_path.ends_with(".md")),
        "markdown content must be searchable: {md_hits:?}"
    );
}
