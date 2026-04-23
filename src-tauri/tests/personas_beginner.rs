//! Persona + journey tests for the absolute-beginner arc.
//!
//! These cover the intro-lesson curriculum, lesson-unlock progression,
//! and the keyboard-layout module (indirectly — the finger/char mapping
//! is driven by TS on the frontend, but we can still sanity-check the
//! curriculum emits the glyphs each lesson claims to teach, so the
//! keyboard highlight has something to light up).

use datlino_lib::lessons::{self, curriculum, lesson_by_id, next_lesson_id};
use datlino_lib::pedagogy::Keystroke;
use datlino_lib::progress::{self, AttemptRecord};
use datlino_lib::session::{self, PracticeMode, SessionRequest};
use datlino_lib::{db, ingest};

use std::fs;
use tempfile::TempDir;

fn clean_attempt(target: &str) -> AttemptRecord {
    let chars: Vec<char> = target.chars().collect();
    let ks: Vec<Keystroke> = chars
        .iter()
        .enumerate()
        .map(|(i, c)| Keystroke {
            // 200 ms / key ≈ 60 WPM — well above the intro thresholds.
            t_ms: (i as u64) * 200,
            actual: c.to_string(),
            expected: c.to_string(),
            correct: true,
        })
        .collect();
    AttemptRecord {
        chunk_id: None,
        target_text: target.to_string(),
        started_at_ms: 0,
        finished_at_ms: (chars.len() as u64) * 200,
        keystrokes: ks,
        completed: true,
    }
}

fn slow_sloppy_attempt(target: &str) -> AttemptRecord {
    // 700 ms / key, half the keys wrong — nowhere near the mastery bar.
    let chars: Vec<char> = target.chars().collect();
    let ks: Vec<Keystroke> = chars
        .iter()
        .enumerate()
        .map(|(i, c)| Keystroke {
            t_ms: (i as u64) * 700,
            actual: if i % 2 == 0 { "x".to_string() } else { c.to_string() },
            expected: c.to_string(),
            correct: i % 2 != 0,
        })
        .collect();
    AttemptRecord {
        chunk_id: None,
        target_text: target.to_string(),
        started_at_ms: 0,
        finished_at_ms: (chars.len() as u64) * 700,
        keystrokes: ks,
        completed: false,
    }
}

// ---------- Persona: Filip, absolute beginner ----------

#[test]
fn filip_first_ever_session_is_the_home_row_left_lesson() {
    // Default call with no lesson_id should land the student on the
    // first unpassed lesson — for a brand-new profile that's the first
    // one in the curriculum.
    let mut conn = db::open_in_memory().unwrap();
    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::IntroLesson,
            alpha: 0.0,
            target_duration_s: 120,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!plan.sentences.is_empty(), "intro lesson must emit drills");
    assert!(
        plan.sentences.iter().all(|s| s.is_generated),
        "lessons are generated drills"
    );
    // The first lesson's drills should be asdf-flavoured.
    let all_text: String = plan.sentences.iter().map(|s| s.text.clone()).collect();
    assert!(
        all_text.contains("asdf") || all_text.contains("sad") || all_text.contains("dad"),
        "expected home-row-left content: {all_text}"
    );
}

#[test]
fn filip_sloppy_run_does_not_unlock_next_lesson() {
    let mut conn = db::open_in_memory().unwrap();
    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::IntroLesson,
            alpha: 0.0,
            target_duration_s: 120,
            lesson_id: Some("home_row_left".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let attempts: Vec<_> = plan.sentences.iter().map(|s| slow_sloppy_attempt(&s.text)).collect();
    let summary = progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();
    assert!(summary.lesson_mastered.is_none(), "sloppy run must not pass the lesson");
    assert!(summary.lesson_progress.is_some(), "progress row is still created");

    let lessons_list = progress::list_intro_lessons(&conn, 1).unwrap();
    let first = lessons_list.iter().find(|l| l.id == "home_row_left").unwrap();
    let second = lessons_list.iter().find(|l| l.id == "home_row_right").unwrap();
    assert!(!first.passed, "first lesson not passed");
    assert!(!second.unlocked, "second lesson stays locked");
}

#[test]
fn filip_clean_run_unlocks_the_next_lesson() {
    let mut conn = db::open_in_memory().unwrap();
    let plan = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::IntroLesson,
            alpha: 0.0,
            target_duration_s: 120,
            lesson_id: Some("home_row_left".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let attempts: Vec<_> = plan.sentences.iter().map(|s| clean_attempt(&s.text)).collect();
    let summary = progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();
    assert_eq!(
        summary.lesson_mastered.as_deref(),
        Some("home_row_left"),
        "clean run hits the mastery bar"
    );

    let lessons_list = progress::list_intro_lessons(&conn, 1).unwrap();
    let first = lessons_list.iter().find(|l| l.id == "home_row_left").unwrap();
    let second = lessons_list.iter().find(|l| l.id == "home_row_right").unwrap();
    assert!(first.passed);
    assert!(second.unlocked, "next lesson unlocks after mastery");
    assert!(!second.passed);
}

#[test]
fn default_intro_session_advances_as_the_student_masters_each_lesson() {
    let mut conn = db::open_in_memory().unwrap();
    // Run three in a row without specifying lesson_id — each session
    // should pick the NEXT lesson in the curriculum.
    let mut seen_ids = Vec::new();
    for _ in 0..3 {
        let plan = session::create_session(
            &mut conn,
            1,
            &SessionRequest {
                mode: PracticeMode::IntroLesson,
                alpha: 0.0,
                target_duration_s: 120,
                ..Default::default()
            },
        )
        .unwrap();
        let sources: Vec<String> = plan
            .sentences
            .iter()
            .filter_map(|s| s.source_path.clone())
            .collect();
        seen_ids.push(sources[0].clone());
        let attempts: Vec<_> =
            plan.sentences.iter().map(|s| clean_attempt(&s.text)).collect();
        progress::finalize_session(&mut conn, 1, plan.session_id, &attempts).unwrap();
    }
    // Distinct lessons picked automatically.
    assert_eq!(seen_ids.len(), 3);
    assert!(
        seen_ids[0] != seen_ids[1] && seen_ids[1] != seen_ids[2],
        "auto-advance should step through lessons: {seen_ids:?}"
    );
}

// ---------- Curriculum structure checks ----------

#[test]
fn curriculum_walks_home_row_then_branches_out() {
    let ids: Vec<&str> = curriculum().iter().map(|l| l.meta.id).collect();
    // First three should be the home-row progression.
    assert_eq!(ids[0], "home_row_left");
    assert_eq!(ids[1], "home_row_right");
    assert_eq!(ids[2], "home_row_both");
    // Czech diacritics appear before the open-ended short-sentence lesson.
    let diac = ids.iter().position(|&x| x == "diacritics_hacek").unwrap();
    let short = ids.iter().position(|&x| x == "short_sentences").unwrap();
    assert!(diac < short, "háček lesson comes before real sentences");
}

#[test]
fn curriculum_next_lesson_id_chain_matches_ids_order() {
    let ids: Vec<&str> = curriculum().iter().map(|l| l.meta.id).collect();
    for pair in ids.windows(2) {
        assert_eq!(next_lesson_id(pair[0]), Some(pair[1]));
    }
    assert_eq!(next_lesson_id(*ids.last().unwrap()), None);
}

#[test]
fn diacritic_lessons_actually_include_diacritics() {
    let hacek = lesson_by_id("diacritics_hacek").unwrap();
    let drills = (hacek.drills)();
    let all: String = drills.iter().map(|d| d.text.clone()).collect();
    for c in ['č', 'š', 'ž', 'ř', 'ě'] {
        assert!(all.contains(c), "háček lesson should include {c}: {all}");
    }
    let kr = lesson_by_id("diacritics_krouzek").unwrap();
    let drills_kr = (kr.drills)();
    let all_kr: String = drills_kr.iter().map(|d| d.text.clone()).collect();
    assert!(all_kr.contains('ů'), "ů lesson should include ů: {all_kr}");
}

// ---------- Journey: intro graduate then shifts to Content mode ----------

#[test]
fn intro_graduate_transitions_to_corpus_content() {
    let mut conn = db::open_in_memory().unwrap();
    // Give the student a richer corpus so Content's dedup+diversity rules
    // leave enough to practice on.
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("dejepis-habsburkove.md");
    fs::write(
        &p,
        "# Dějepis\n\n## Habsburkové\n\nHabsburkové vládli po staletí v Čechách. \
         Ferdinand I. nastoupil v roce 1526. Marie Terezie byla významná panovnice. \
         Jozef II. zrušil nevolnictví. Poslední Habsburk opustil trůn v roce 1918.",
    )
    .unwrap();
    ingest::ingest_file(&mut conn, &p).unwrap();

    // First session: intro lesson (home_row_left) — clean run.
    let intro = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::IntroLesson,
            alpha: 0.0,
            target_duration_s: 120,
            lesson_id: Some("home_row_left".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let attempts: Vec<_> = intro.sentences.iter().map(|s| clean_attempt(&s.text)).collect();
    let summary = progress::finalize_session(&mut conn, 1, intro.session_id, &attempts).unwrap();
    assert!(summary.lesson_mastered.is_some());

    // Second session: Content mode. Real-sentences path still works.
    let content = session::create_session(
        &mut conn,
        1,
        &SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 120,
            query: Some("Habsburk".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!content.sentences.is_empty());
    let content_attempts: Vec<_> = content.sentences.iter().map(|s| clean_attempt(&s.text)).collect();
    let sum2 = progress::finalize_session(&mut conn, 1, content.session_id, &content_attempts).unwrap();
    // Content sessions don't emit lesson_mastered.
    assert!(sum2.lesson_mastered.is_none());
    assert!(sum2.lesson_progress.is_none());

    let view = progress::user_profile_view(&conn, 1).unwrap();
    assert_eq!(view.total_sessions, 2);
}
