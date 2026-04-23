//! Typing-engine flavoured personas: dead-key diacritics recorded as
//! a single correct keystroke (not two wrong ones), and the document
//! picker that gives the student a whole-file session bypassing BM25.
//!
//! Why these live here: the actual keyboard handler is in the Svelte
//! session page, but the *contract* it implements (one Keystroke per
//! expected codepoint, `actual` == composed char) is what the pedagogy
//! model depends on. We can exercise that contract by constructing the
//! log a correctly-wired frontend would produce and making sure the
//! pedagogy model sees a single correct keystroke, not two failures.

use datlino_lib::pedagogy::{self, Keystroke};
use datlino_lib::progress::{self, AttemptRecord};
use datlino_lib::session::{self, PracticeMode, SessionRequest};
use datlino_lib::{db, ingest};

use std::fs;
use tempfile::TempDir;

// ---------- Dead-key composition: ř registered as ONE correct keystroke ----------

#[test]
fn dead_key_diacritic_registers_as_one_correct_keystroke() {
    // On CZ layouts, ř is typed via a dead key + r. The Svelte handler
    // filters `key === "Dead"` and waits for compositionend, producing a
    // single Keystroke with actual = "ř". Here we assert the pedagogy
    // model sees exactly one n-gram observation for "ř" after the
    // student types it correctly three times — no phantom "Dead"
    // keystrokes polluting the stats.
    let mut conn = db::open_in_memory().unwrap();
    for _ in 0..3 {
        let keystrokes = vec![
            Keystroke {
                t_ms: 0,
                actual: "t".into(),
                expected: "t".into(),
                correct: true,
            },
            Keystroke {
                t_ms: 150,
                actual: "ř".into(),
                expected: "ř".into(),
                correct: true,
            },
            Keystroke {
                t_ms: 300,
                actual: "i".into(),
                expected: "i".into(),
                correct: true,
            },
        ];
        pedagogy::update_stats(&mut conn, 1, &keystrokes).unwrap();
    }
    let (occurrences, error_count): (i64, i64) = conn
        .query_row(
            "SELECT occurrences, error_count FROM ngram_stat WHERE user_id = 1 AND ngram = 'ř'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(occurrences, 3, "exactly one observation per attempt");
    assert_eq!(error_count, 0, "dead-key compose didn't create fake errors");
}

#[test]
fn dead_key_diacritic_typed_wrong_counts_as_one_error_not_two() {
    // Student intended ř but pressed the wrong dead key and got é. The
    // frontend emits one Keystroke with actual = "é", expected = "ř",
    // correct = false. Pedagogy should see 1 error on ř, not 2 (one for
    // "Dead", one for the letter).
    let mut conn = db::open_in_memory().unwrap();
    let keystrokes = vec![Keystroke {
        t_ms: 0,
        actual: "é".into(),
        expected: "ř".into(),
        correct: false,
    }];
    pedagogy::update_stats(&mut conn, 1, &keystrokes).unwrap();
    let (occurrences, error_count): (i64, i64) = conn
        .query_row(
            "SELECT occurrences, error_count FROM ngram_stat WHERE user_id = 1 AND ngram = 'ř'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(occurrences, 1);
    assert_eq!(error_count, 1, "one error, not two — dead key wasn't double-counted");
}

// ---------- Document picker: "chemie" query reveals the full chapter ----------

#[test]
fn chemie_search_returns_whole_document_not_one_sentence() {
    // Reproduces the user's complaint: searching for "chemie" on a corpus
    // where "chemie" only appears in the filename — the current BM25
    // returned zero or one body sentence, making study impossible. With
    // filename-aware matching + whole-document expansion, the student
    // should now get every chunk of the matching file.
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("chemie-periodicka-soustava.md");
    fs::write(
        &p,
        "# Periodická soustava prvků\n\n\
         Periodickou soustavu sestavil Dmitrij Mendělejev v roce 1869. \
         Uspořádal prvky podle rostoucí atomové hmotnosti. \
         Dnešní soustava je řazena podle protonového čísla. \
         Prvky jedné skupiny mají obdobné chemické vlastnosti. \
         Alkalické kovy tvoří I. skupinu. Halogeny jsou velmi reaktivní.",
    )
    .unwrap();
    ingest::ingest_file(&mut conn, &p).unwrap();

    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            query: Some("chemie".into()),
            ..Default::default()
        },
    )
    .unwrap();

    assert!(
        plan.sentences.len() >= 4,
        "querying 'chemie' on a matching filename should surface the whole doc, got {}",
        plan.sentences.len()
    );
    // Order preserved — the Mendělejev intro sentence should come first.
    assert!(plan.sentences[0].text.contains("Mendělejev"));
}

#[test]
fn direct_document_id_bypasses_search_and_returns_full_document() {
    // The document-picker path on the Library page calls create_session
    // with document_id set — every chunk of that file, in source order.
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("note.md");
    fs::write(
        &p,
        "Věta jedna o historii. Věta dvě se týká ekonomiky. Třetí věta o kultuře. \
         Čtvrtá věta o jazyce. Pátá věta závěrečná.",
    )
    .unwrap();
    ingest::ingest_file(&mut conn, &p).unwrap();

    let doc_id: i64 = conn
        .query_row("SELECT id FROM document", [], |r| r.get(0))
        .unwrap();

    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            document_id: Some(doc_id),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plan.sentences.len(), 5, "all five sentences in source order");
    assert!(plan.sentences[0].text.starts_with("Věta jedna"));
    assert!(plan.sentences[4].text.starts_with("Pátá"));
}

#[test]
fn list_documents_returns_every_ingested_file_with_chunk_counts() {
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let p1 = tmp.path().join("a.md");
    fs::write(&p1, "První. Druhá. Třetí.").unwrap();
    let p2 = tmp.path().join("nested/b.md");
    fs::create_dir_all(p2.parent().unwrap()).unwrap();
    fs::write(&p2, "Další věta. A ještě jedna.").unwrap();
    ingest::ingest_file(&mut conn, &p1).unwrap();
    ingest::ingest_file(&mut conn, &p2).unwrap();

    let docs = session::list_documents(&conn).unwrap();
    assert_eq!(docs.len(), 2);
    let total_chunks: i64 = docs.iter().map(|d| d.chunk_count).sum();
    assert!(total_chunks >= 5);
}

// ---------- Journey: add folder, list documents, drill one, finalize ----------

#[test]
fn end_to_end_folder_to_document_drill_updates_profile() {
    let mut conn = db::open_in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    // Nested-subfolder layout to validate recursion once more, end-to-end.
    let deep_dir = tmp.path().join("skola/cestina");
    fs::create_dir_all(&deep_dir).unwrap();
    let deep = deep_dir.join("pravopis.md");
    fs::write(
        &deep,
        "# Pravopis\n\nPo obojetných souhláskách píšeme buď y nebo i. \
         Vyjmenovaná slova je potřeba znát. Shoda podmětu s přísudkem má svá pravidla. \
         Čárka před „ale\" se píše. Interpunkce je důležitá.",
    )
    .unwrap();

    // Simulate the "Add folder" path — recursive ingest from the top dir.
    let stats = ingest::ingest_tree(&mut conn, tmp.path()).unwrap();
    assert!(stats.files_ingested >= 1);

    // The student clicks the document in the Library.
    let docs = session::list_documents(&conn).unwrap();
    let doc = docs
        .iter()
        .find(|d| d.source_path.contains("pravopis"))
        .expect("document surfaced despite being two dirs deep");
    assert!(doc.chunk_count >= 4);

    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            document_id: Some(doc.id),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(plan.sentences.len() >= 4);

    // And they finish a clean run; profile records a session.
    let attempts: Vec<AttemptRecord> = plan
        .sentences
        .iter()
        .map(|s| {
            let chars: Vec<char> = s.text.chars().collect();
            let ks: Vec<Keystroke> = chars
                .iter()
                .enumerate()
                .map(|(i, c)| Keystroke {
                    t_ms: (i as u64) * 150,
                    actual: c.to_string(),
                    expected: c.to_string(),
                    correct: true,
                })
                .collect();
            AttemptRecord {
                chunk_id: s.chunk_id,
                target_text: s.text.clone(),
                started_at_ms: 0,
                finished_at_ms: (chars.len() as u64) * 150,
                keystrokes: ks,
                completed: true,
            }
        })
        .collect();
    let summary =
        progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();
    assert!(summary.xp_earned > 0);
    assert!(summary.wpm > 20.0);

    let view = progress::user_profile_view(&conn, 1).unwrap();
    assert_eq!(view.total_sessions, 1);
}
