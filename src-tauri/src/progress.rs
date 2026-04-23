//! Progress + light gamification.
//!
//! We finalise a session by aggregating its attempts, updating the user's
//! n-gram stats, then updating the profile (XP, streak, baselines) and
//! awarding any badges that were unlocked this session.
//!
//! Design notes (aligned with the brief):
//! * Streaks measured in local days; a broken streak resets to 1 — no guilt
//!   messaging, just a quiet counter reset. The mascot shrugs.
//! * XP rewards accuracy over volume: `xp = round(words × (accuracy/100)^2)`.
//!   Typing 500 chars at 60 % accuracy scores the same as 180 chars at 100 %.
//! * Level = floor(sqrt(total_xp / 10)). Each level roughly doubles the
//!   previous one's cost — gentle progression that never taps out.
//! * Badges exist for shape, not for addiction. Each code is unique per
//!   user and never re-awarded.

use anyhow::Result;
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::now_unix;
use crate::pedagogy::{self, Keystroke};

/// One recorded attempt. The frontend sends this verbatim at session end.
#[derive(Debug, Clone, Deserialize)]
pub struct AttemptRecord {
    pub chunk_id: Option<i64>,
    pub target_text: String,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub keystrokes: Vec<Keystroke>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSummary {
    pub session_id: i64,
    pub wpm: f64,
    pub accuracy_pct: f64,
    pub xp_earned: i64,
    pub total_xp: i64,
    pub level: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub words_typed: i64,
    pub characters_typed: i64,
    pub sentences_completed: i64,
    pub sentences_attempted: i64,
    pub badges_awarded: Vec<String>,
    pub weak_preview: Vec<pedagogy::WeakNgram>,
}

pub fn finalize_session(
    conn: &mut Connection,
    user_id: i64,
    session_id: i64,
    attempts: &[AttemptRecord],
) -> Result<SessionSummary> {
    // --- 1. Persist attempts + fold keystrokes into pedagogy stats ---
    let mut characters_typed = 0i64;
    let mut correct_chars = 0i64;
    let mut sentences_completed = 0i64;

    for a in attempts {
        let correct = a.keystrokes.iter().filter(|k| k.correct).count() as i64;
        let total = a.keystrokes.len() as i64;
        characters_typed += total;
        correct_chars += correct;
        if a.completed {
            sentences_completed += 1;
        }
        let wpm = attempt_wpm(a);
        let acc = if total == 0 { 0.0 } else { correct as f64 / total as f64 };
        let blob = serde_json::to_vec(&a.keystrokes)?;

        conn.execute(
            "INSERT INTO attempt(session_id, chunk_id, started_at, finished_at, wpm, accuracy, target_text, keystroke_log)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session_id,
                a.chunk_id,
                (a.started_at_ms / 1000) as i64,
                (a.finished_at_ms / 1000) as i64,
                wpm,
                acc,
                &a.target_text,
                blob,
            ],
        )?;

        pedagogy::update_stats(conn, user_id, &a.keystrokes)?;
    }

    // --- 2. Session totals ---
    let accuracy_pct = if characters_typed == 0 {
        0.0
    } else {
        (correct_chars as f64 / characters_typed as f64) * 100.0
    };
    let total_elapsed_ms: u64 = attempts
        .iter()
        .map(|a| a.finished_at_ms.saturating_sub(a.started_at_ms))
        .sum();
    let minutes = (total_elapsed_ms as f64) / 60_000.0;
    let wpm = if minutes > 0.0 {
        (characters_typed as f64 / 5.0) / minutes
    } else {
        0.0
    };
    let words_typed = (characters_typed as f64 / 5.0).round() as i64;
    let xp_earned = xp_for_session(words_typed, accuracy_pct);

    // --- 3. Update user profile (xp, streak, baselines) ---
    let profile_before = read_profile(conn, user_id)?;
    let today = Local::now().date_naive();
    let new_streak = streak_after(&profile_before, today);
    let longest = profile_before.longest_streak.max(new_streak);
    let total_xp = profile_before.total_xp + xp_earned;
    let total_sessions = profile_before.total_sessions + 1;

    let wpm_baseline = blended_baseline(profile_before.wpm_baseline, wpm);
    let accuracy_baseline = blended_baseline(profile_before.accuracy_baseline, accuracy_pct);

    conn.execute(
        "UPDATE user_profile
         SET total_xp = ?1,
             current_streak = ?2,
             longest_streak = ?3,
             last_session_date = ?4,
             total_sessions = ?5,
             wpm_baseline = ?6,
             accuracy_baseline = ?7
         WHERE id = ?8",
        params![
            total_xp,
            new_streak,
            longest,
            today.to_string(),
            total_sessions,
            wpm_baseline,
            accuracy_baseline,
            user_id,
        ],
    )?;

    // --- 4. Badges ---
    let badges_awarded = award_badges(
        conn,
        user_id,
        total_sessions,
        new_streak,
        accuracy_pct,
        wpm,
    )?;

    // --- 5. Write summary + finalise session row ---
    let weak_preview = pedagogy::weak_ngrams(conn, user_id, 5)?;
    let level = level_for_xp(total_xp);
    let summary = SessionSummary {
        session_id,
        wpm,
        accuracy_pct,
        xp_earned,
        total_xp,
        level,
        current_streak: new_streak,
        longest_streak: longest,
        words_typed,
        characters_typed,
        sentences_completed,
        sentences_attempted: attempts.len() as i64,
        badges_awarded,
        weak_preview,
    };
    let summary_json = serde_json::to_string(&summary)?;
    conn.execute(
        "UPDATE session SET finished_at = ?1, xp_earned = ?2, summary_json = ?3 WHERE id = ?4",
        params![now_unix(), xp_earned, summary_json, session_id],
    )?;

    Ok(summary)
}

fn attempt_wpm(a: &AttemptRecord) -> f64 {
    let ms = a.finished_at_ms.saturating_sub(a.started_at_ms);
    if ms == 0 {
        return 0.0;
    }
    let chars = a.keystrokes.len() as f64;
    (chars / 5.0) / (ms as f64 / 60_000.0)
}

fn xp_for_session(words_typed: i64, accuracy_pct: f64) -> i64 {
    let acc = (accuracy_pct / 100.0).clamp(0.0, 1.0);
    (words_typed as f64 * acc.powi(2)).round() as i64
}

pub fn level_for_xp(total_xp: i64) -> i64 {
    ((total_xp.max(0) as f64 / 10.0).sqrt()).floor() as i64
}

fn blended_baseline(previous: Option<f64>, observed: f64) -> f64 {
    match previous {
        Some(p) => 0.7 * p + 0.3 * observed,
        None => observed,
    }
}

#[derive(Debug, Default)]
struct Profile {
    total_xp: i64,
    current_streak: i64,
    longest_streak: i64,
    last_session_date: Option<String>,
    total_sessions: i64,
    wpm_baseline: Option<f64>,
    accuracy_baseline: Option<f64>,
}

fn read_profile(conn: &Connection, user_id: i64) -> Result<Profile> {
    let p = conn.query_row(
        "SELECT total_xp, current_streak, longest_streak, last_session_date,
                total_sessions, wpm_baseline, accuracy_baseline
         FROM user_profile WHERE id = ?1",
        params![user_id],
        |r| {
            Ok(Profile {
                total_xp: r.get(0)?,
                current_streak: r.get(1)?,
                longest_streak: r.get(2)?,
                last_session_date: r.get(3)?,
                total_sessions: r.get(4)?,
                wpm_baseline: r.get(5)?,
                accuracy_baseline: r.get(6)?,
            })
        },
    )?;
    Ok(p)
}

/// Proper streak math with access to the previous value (the signatureless
/// helper above was keeping control flow tidy; the real advance lives here).
fn streak_after(prev: &Profile, today: NaiveDate) -> i64 {
    let Some(prev_date) = prev
        .last_session_date
        .as_deref()
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
    else {
        return 1;
    };
    match (today - prev_date).num_days() {
        0 => prev.current_streak.max(1),
        1 => prev.current_streak + 1,
        _ => 1,
    }
}

fn award_badges(
    conn: &mut Connection,
    user_id: i64,
    total_sessions: i64,
    streak: i64,
    accuracy: f64,
    wpm: f64,
) -> Result<Vec<String>> {
    let candidates: Vec<(&'static str, bool)> = vec![
        ("first_session", total_sessions == 1),
        ("five_sessions", total_sessions == 5),
        ("twenty_sessions", total_sessions == 20),
        ("streak_3", streak >= 3),
        ("streak_7", streak >= 7),
        ("streak_30", streak >= 30),
        ("accuracy_95", accuracy >= 95.0),
        ("wpm_30", wpm >= 30.0),
        ("wpm_40", wpm >= 40.0),
        ("wpm_50", wpm >= 50.0),
    ];
    let mut awarded = Vec::new();
    for (code, earned) in candidates {
        if !earned {
            continue;
        }
        let rows = conn.execute(
            "INSERT OR IGNORE INTO badge(user_id, code, earned_at) VALUES (?1, ?2, ?3)",
            params![user_id, code, now_unix()],
        )?;
        if rows > 0 {
            awarded.push(code.to_string());
        }
    }
    Ok(awarded)
}

#[derive(Debug, Serialize)]
pub struct UserProfileView {
    pub total_xp: i64,
    pub level: i64,
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_sessions: i64,
    pub wpm_baseline: Option<f64>,
    pub accuracy_baseline: Option<f64>,
    pub badges: Vec<String>,
}

pub fn user_profile_view(conn: &Connection, user_id: i64) -> Result<UserProfileView> {
    let p = read_profile(conn, user_id)?;
    let mut stmt =
        conn.prepare("SELECT code FROM badge WHERE user_id = ?1 ORDER BY earned_at ASC")?;
    let badges: Vec<String> = stmt
        .query_map(params![user_id], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(UserProfileView {
        total_xp: p.total_xp,
        level: level_for_xp(p.total_xp),
        current_streak: p.current_streak,
        longest_streak: p.longest_streak,
        total_sessions: p.total_sessions,
        wpm_baseline: p.wpm_baseline,
        accuracy_baseline: p.accuracy_baseline,
        badges,
    })
}

#[derive(Debug, Serialize)]
pub struct SessionHistoryRow {
    pub session_id: i64,
    pub created_at: i64,
    pub mode: String,
    pub alpha: f64,
    pub xp_earned: i64,
    pub summary: Option<serde_json::Value>,
}

pub fn session_history(
    conn: &Connection,
    user_id: i64,
    limit: usize,
) -> Result<Vec<SessionHistoryRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, mode, alpha, xp_earned, summary_json
         FROM session WHERE user_id = ?1 AND finished_at IS NOT NULL
         ORDER BY created_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![user_id, limit as i64], |r| {
        let summary_raw: Option<String> = r.get(5)?;
        let summary: Option<serde_json::Value> = summary_raw
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        Ok(SessionHistoryRow {
            session_id: r.get(0)?,
            created_at: r.get(1)?,
            mode: r.get(2)?,
            alpha: r.get(3)?,
            xp_earned: r.get(4)?,
            summary,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn ks(expected: &str, correct: bool, t_ms: u64) -> Keystroke {
        Keystroke {
            t_ms,
            actual: if correct {
                expected.to_string()
            } else {
                "x".to_string()
            },
            expected: expected.to_string(),
            correct,
        }
    }

    fn attempt(target: &str, wrong: &[usize]) -> AttemptRecord {
        let keystrokes: Vec<_> = target
            .chars()
            .enumerate()
            .map(|(i, c)| ks(&c.to_string(), !wrong.contains(&i), (i as u64) * 100))
            .collect();
        let end = (target.chars().count() as u64) * 100;
        AttemptRecord {
            chunk_id: None,
            target_text: target.to_string(),
            started_at_ms: 0,
            finished_at_ms: end,
            keystrokes,
            completed: true,
        }
    }

    #[test]
    fn xp_rewards_accuracy() {
        assert!(xp_for_session(100, 100.0) > xp_for_session(100, 70.0));
        assert_eq!(xp_for_session(100, 0.0), 0);
    }

    #[test]
    fn level_is_gentle() {
        assert_eq!(level_for_xp(0), 0);
        assert_eq!(level_for_xp(10), 1);
        assert_eq!(level_for_xp(40), 2);
        assert_eq!(level_for_xp(90), 3);
    }

    #[test]
    fn streak_advances_across_consecutive_days() {
        let prev = Profile {
            current_streak: 4,
            last_session_date: Some("2026-04-22".into()),
            ..Default::default()
        };
        let today = NaiveDate::parse_from_str("2026-04-23", "%Y-%m-%d").unwrap();
        assert_eq!(streak_after(&prev, today), 5);
    }

    #[test]
    fn streak_resets_after_gap() {
        let prev = Profile {
            current_streak: 10,
            last_session_date: Some("2026-04-20".into()),
            ..Default::default()
        };
        let today = NaiveDate::parse_from_str("2026-04-23", "%Y-%m-%d").unwrap();
        assert_eq!(streak_after(&prev, today), 1);
    }

    #[test]
    fn streak_unchanged_same_day() {
        let prev = Profile {
            current_streak: 7,
            last_session_date: Some("2026-04-23".into()),
            ..Default::default()
        };
        let today = NaiveDate::parse_from_str("2026-04-23", "%Y-%m-%d").unwrap();
        assert_eq!(streak_after(&prev, today), 7);
    }

    #[test]
    fn finalize_awards_first_session_badge_and_xp() {
        let mut conn = db::open_in_memory().unwrap();
        // Need a session row first.
        conn.execute(
            "INSERT INTO session(user_id, created_at, mode, alpha) VALUES (1, 0, 'warmup', 0.5)",
            [],
        )
        .unwrap();
        let session_id: i64 = conn.last_insert_rowid();

        let summary = finalize_session(&mut conn, 1, session_id, &[attempt("ahoj svete", &[])]).unwrap();
        assert!(summary.xp_earned > 0);
        assert!(summary.badges_awarded.contains(&"first_session".to_string()));
        assert_eq!(summary.current_streak, 1);
    }

    #[test]
    fn finalize_records_attempt_row_and_updates_ngram_stats() {
        let mut conn = db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO session(user_id, created_at, mode, alpha) VALUES (1, 0, 'diacritics', 0.0)",
            [],
        )
        .unwrap();
        let session_id: i64 = conn.last_insert_rowid();

        finalize_session(&mut conn, 1, session_id, &[attempt("čř", &[1])]).unwrap();
        let n: i64 = conn
            .query_row("SELECT count(*) FROM attempt", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        let m: i64 = conn
            .query_row("SELECT count(*) FROM ngram_stat", [], |r| r.get(0))
            .unwrap();
        assert!(m > 0, "ngram_stat should have rows");
    }
}
