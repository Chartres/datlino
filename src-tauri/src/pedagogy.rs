//! Pedagogy model — per-user n-gram latency and error stats.
//!
//! Each completed attempt hands us a keystroke log. We walk it to extract
//! 1-, 2-, and 3-gram samples over the *expected* text (not what the user
//! actually typed — the target is what we want them to learn). For every
//! n-gram we update an exponential moving average of:
//!
//!   * latency, measured as the time from the first-char keystroke to the
//!     last-char keystroke of the n-gram (for 1-grams this is the single
//!     inter-keystroke interval);
//!   * error rate, 0 if the entire n-gram was typed correctly and 1 if any
//!     keystroke inside it was wrong.
//!
//! The "weakness" score used for pedagogy ranking blends normalised latency
//! and error rate. Weak bigrams drive both the weak-keys practice mode and
//! the pedagogy-density term of the session scorer (brief §2).
//!
//! We explicitly ignore whitespace-only n-grams — "typing the space bar" is
//! uninteresting and would swamp the rankings.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::db::now_unix;

/// One user keystroke captured by the typing engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keystroke {
    /// Monotonic ms from the start of the attempt.
    pub t_ms: u64,
    /// What the user actually produced (could be a correction).
    pub actual: String,
    /// What we expected at this cursor position.
    pub expected: String,
    /// True if `actual == expected`.
    pub correct: bool,
}

/// EMA smoothing factor. New observations weigh 25%. Small enough to let
/// a weak-key improvement show up over several sessions rather than one
/// lucky attempt.
const EMA_ALPHA: f64 = 0.25;

/// Below this many total observations an n-gram is treated as "not yet
/// learned" and ranked lower — we want enough signal before calling
/// something weak.
const MIN_OBSERVATIONS: i64 = 3;

pub fn update_stats(
    conn: &mut Connection,
    user_id: i64,
    keystrokes: &[Keystroke],
) -> Result<()> {
    if keystrokes.is_empty() {
        return Ok(());
    }
    let now = now_unix();
    let tx = conn.transaction()?;

    for (ngram, latency_ms, had_error) in extract_ngrams(keystrokes) {
        upsert_ngram(&tx, user_id, &ngram, latency_ms, had_error, now)?;
    }
    tx.commit()?;
    Ok(())
}

/// Extract contiguous 1/2/3-gram samples along the *expected* text. Returns
/// `(ngram, latency_ms, any_error_inside)` tuples. Breaks the run at
/// whitespace so cross-word n-grams aren't formed.
fn extract_ngrams(keystrokes: &[Keystroke]) -> Vec<(String, u32, bool)> {
    let mut out = Vec::new();

    // Collapse backspaces / corrections into one entry per target position.
    // We trust the caller to pass the final correct-or-incorrect state per
    // expected char; one Keystroke per cursor position.
    let normalised: Vec<_> = keystrokes
        .iter()
        .filter(|k| !k.expected.is_empty())
        .collect();

    for window_size in [1usize, 2, 3] {
        if normalised.len() < window_size {
            continue;
        }
        for start in 0..=normalised.len() - window_size {
            let slice = &normalised[start..start + window_size];
            // Don't form n-grams that straddle whitespace or contain only
            // whitespace — "the space bar" is a separate skill we don't drill.
            if slice
                .iter()
                .any(|k| k.expected.chars().all(|c| c.is_whitespace()))
            {
                continue;
            }
            let ngram: String = slice.iter().map(|k| k.expected.as_str()).collect();
            let ngram: String = ngram.nfc().collect();
            // Duration from the first keystroke time to the last.
            let latency_ms = slice
                .last()
                .unwrap()
                .t_ms
                .saturating_sub(slice.first().unwrap().t_ms) as u32;
            let had_error = slice.iter().any(|k| !k.correct);
            out.push((ngram, latency_ms, had_error));
        }
    }
    out
}

fn upsert_ngram(
    tx: &rusqlite::Transaction,
    user_id: i64,
    ngram: &str,
    latency_ms: u32,
    had_error: bool,
    now: i64,
) -> Result<()> {
    let existing: Option<(i64, i64, Option<f64>, Option<f64>)> = tx
        .query_row(
            "SELECT occurrences, error_count, ema_latency_ms, ema_error_rate
             FROM ngram_stat WHERE user_id = ?1 AND ngram = ?2",
            params![user_id, ngram],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, Option<f64>>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                ))
            },
        )
        .ok();

    let err_sample = if had_error { 1.0 } else { 0.0 };
    let lat_sample = latency_ms as f64;

    let (occurrences, error_count, ema_lat, ema_err) = match existing {
        Some((occ, errs, prev_lat, prev_err)) => {
            let next_lat = ema(prev_lat, lat_sample);
            let next_err = ema(prev_err, err_sample);
            (occ + 1, errs + had_error as i64, next_lat, next_err)
        }
        None => (1, had_error as i64, lat_sample, err_sample),
    };

    tx.execute(
        "INSERT INTO ngram_stat(user_id, ngram, occurrences, error_count, ema_latency_ms, ema_error_rate, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(user_id, ngram) DO UPDATE SET
            occurrences = excluded.occurrences,
            error_count = excluded.error_count,
            ema_latency_ms = excluded.ema_latency_ms,
            ema_error_rate = excluded.ema_error_rate,
            last_seen = excluded.last_seen",
        params![user_id, ngram, occurrences, error_count, ema_lat, ema_err, now],
    )?;
    Ok(())
}

fn ema(prev: Option<f64>, sample: f64) -> f64 {
    match prev {
        Some(p) => EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * p,
        None => sample,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeakNgram {
    pub ngram: String,
    pub occurrences: i64,
    pub ema_latency_ms: f64,
    pub ema_error_rate: f64,
    /// Combined weakness score — higher = weaker.
    pub weakness: f64,
}

/// N-grams in the **zone of proximal development** — stretching, not broken.
///
/// Why this exists separately from `weak_ngrams`:
/// * `weak_ngrams` surfaces the very worst — keys the student can barely
///   hit at all. Good for the weak-keys *drill* (isolation of difficulty).
/// * `learning_zone_ngrams` surfaces keys at the edge of fluency — some
///   errors, some lag, but clearly being learned. Good for the *rephrase*
///   pass, which injects these into sentences the student already has in
///   their materials — "push me at my level."
///
/// Filter band:
/// * occurrences ≥ `MIN_OBSERVATIONS` (real signal, not one bad attempt)
/// * 0.05 ≤ ema_error_rate ≤ 0.30 (improvable, not catastrophic)
/// * ema_latency above the user's median but below the 90th percentile
pub fn learning_zone_ngrams(
    conn: &Connection,
    user_id: i64,
    limit: usize,
) -> Result<Vec<WeakNgram>> {
    let mut stmt = conn.prepare(
        "SELECT ngram, occurrences, ema_latency_ms, ema_error_rate
         FROM ngram_stat
         WHERE user_id = ?1 AND occurrences >= ?2",
    )?;
    let rows: Vec<(String, i64, f64, f64)> = stmt
        .query_map(params![user_id, MIN_OBSERVATIONS], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    // Percentiles for this user's latency distribution.
    let mut latencies: Vec<f64> = rows.iter().map(|r| r.2).collect();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = percentile(&latencies, 0.50);
    let p90 = percentile(&latencies, 0.90);
    let max_lat = *latencies.last().unwrap_or(&1.0);

    let mut zone: Vec<WeakNgram> = rows
        .into_iter()
        .filter(|(_, _, lat, err)| {
            *err >= 0.05 && *err <= 0.30 && *lat >= p50 && *lat <= p90
        })
        .map(|(ngram, occurrences, lat, err)| {
            // "Stretch" score — prefers mid-error, mid-slow. Errors still
            // matter more than speed, but we *penalise* very low error
            // rates (those keys are basically mastered).
            let err_bell = 1.0 - (err - 0.15).abs() / 0.15; // peaks at 0.15
            let norm_lat = lat / max_lat.max(1.0);
            let stretch = 0.6 * err_bell.max(0.0) + 0.4 * norm_lat;
            WeakNgram {
                ngram,
                occurrences,
                ema_latency_ms: lat,
                ema_error_rate: err,
                weakness: stretch,
            }
        })
        .collect();
    zone.sort_by(|a, b| b.weakness.partial_cmp(&a.weakness).unwrap_or(std::cmp::Ordering::Equal));
    zone.truncate(limit);
    Ok(zone)
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Top-N weakest n-grams for the pedagogy UI and the weak-keys generator.
pub fn weak_ngrams(conn: &Connection, user_id: i64, limit: usize) -> Result<Vec<WeakNgram>> {
    let mut stmt = conn.prepare(
        "SELECT ngram, occurrences, ema_latency_ms, ema_error_rate
         FROM ngram_stat
         WHERE user_id = ?1 AND occurrences >= ?2",
    )?;
    let rows = stmt.query_map(params![user_id, MIN_OBSERVATIONS], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
            r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
        ))
    })?;

    // Gather, then normalise latency against the user's own distribution so
    // "slow" is measured relative to their average speed rather than an
    // absolute threshold.
    let mut all: Vec<(String, i64, f64, f64)> =
        rows.collect::<std::result::Result<Vec<_>, _>>()?;
    if all.is_empty() {
        return Ok(Vec::new());
    }
    let max_lat = all.iter().map(|r| r.2).fold(0.0_f64, f64::max).max(1.0);

    let mut weak: Vec<WeakNgram> = all
        .drain(..)
        .map(|(ngram, occurrences, ema_latency_ms, ema_error_rate)| {
            let norm_lat = ema_latency_ms / max_lat;
            // 60% error rate, 40% slowness — errors hurt learning more than
            // a slightly laggy finger.
            let weakness = 0.6 * ema_error_rate + 0.4 * norm_lat;
            WeakNgram {
                ngram,
                occurrences,
                ema_latency_ms,
                ema_error_rate,
                weakness,
            }
        })
        .collect();

    weak.sort_by(|a, b| b.weakness.partial_cmp(&a.weakness).unwrap_or(std::cmp::Ordering::Equal));
    weak.truncate(limit);
    Ok(weak)
}

/// Pedagogy density for a sentence given a weak-ngram table: share of the
/// sentence's length that is made up of the user's weakest n-grams, weighted
/// by their weakness score. Normalised to roughly [0, 1] for the α-blended
/// session scorer.
pub fn pedagogy_density(sentence: &str, weak: &[WeakNgram]) -> f64 {
    if sentence.is_empty() || weak.is_empty() {
        return 0.0;
    }
    let nfc: String = sentence.nfc().collect();
    let lower = nfc.to_lowercase();
    let total_chars = lower.chars().count().max(1) as f64;
    let mut score = 0.0;
    for w in weak {
        let needle = w.ngram.to_lowercase();
        if needle.is_empty() {
            continue;
        }
        let hits = count_non_overlapping(&lower, &needle);
        score += hits as f64 * w.weakness * needle.chars().count() as f64;
    }
    (score / total_chars).min(1.0)
}

fn count_non_overlapping(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count = 0;
    let mut cursor = 0;
    while let Some(pos) = haystack[cursor..].find(needle) {
        count += 1;
        cursor += pos + needle.len();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn mk_log(expected: &str, errors_at: &[usize]) -> Vec<Keystroke> {
        let mut out = Vec::new();
        for (i, c) in expected.chars().enumerate() {
            out.push(Keystroke {
                t_ms: (i as u64) * 100, // 100 ms per keystroke
                actual: if errors_at.contains(&i) {
                    "x".to_string()
                } else {
                    c.to_string()
                },
                expected: c.to_string(),
                correct: !errors_at.contains(&i),
            });
        }
        out
    }

    #[test]
    fn extract_ngrams_skips_whitespace_windows() {
        let log = mk_log("ab cd", &[]);
        let ngrams: Vec<String> =
            extract_ngrams(&log).into_iter().map(|(n, _, _)| n).collect();
        assert!(ngrams.contains(&"a".to_string()));
        assert!(ngrams.contains(&"ab".to_string()));
        assert!(!ngrams.iter().any(|n| n.contains(' ')));
    }

    #[test]
    fn errors_propagate_to_containing_ngrams() {
        let log = mk_log("abc", &[1]); // error on 'b'
        let ngrams = extract_ngrams(&log);
        let b = ngrams.iter().find(|(n, _, _)| n == "b").unwrap();
        assert!(b.2, "single-char error");
        let ab = ngrams.iter().find(|(n, _, _)| n == "ab").unwrap();
        assert!(ab.2, "bigram error propagates");
    }

    #[test]
    fn ema_updates_accumulate() {
        let mut conn = db::open_in_memory().unwrap();
        update_stats(&mut conn, 1, &mk_log("čř", &[])).unwrap();
        update_stats(&mut conn, 1, &mk_log("čř", &[0])).unwrap();
        let (occ, _errs, _lat, err_rate): (i64, i64, Option<f64>, Option<f64>) = conn
            .query_row(
                "SELECT occurrences, error_count, ema_latency_ms, ema_error_rate FROM ngram_stat WHERE ngram = 'č'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(occ, 2);
        assert!(err_rate.unwrap() > 0.0 && err_rate.unwrap() < 1.0);
    }

    #[test]
    fn learning_zone_excludes_mastered_and_broken_keys() {
        let mut conn = db::open_in_memory().unwrap();
        // 1) "aaaa" typed cleanly many times — mastered, should NOT appear.
        for _ in 0..8 {
            update_stats(&mut conn, 1, &mk_log("aaaa", &[])).unwrap();
        }
        // 2) "čšřě" typed with mid-range errors — should appear (zone).
        for i in 0..8 {
            // 1 error every 4 strokes across the loop ≈ ~15% error rate
            let errs: Vec<usize> = if i % 4 == 0 { vec![1] } else { vec![] };
            update_stats(&mut conn, 1, &mk_log("čšřě", &errs)).unwrap();
        }
        // 3) "ŵŵŵ" broken on every keystroke — too hard, isolation
        //    drill territory; should NOT be in the learning zone.
        for _ in 0..8 {
            update_stats(&mut conn, 1, &mk_log("ŵŵŵ", &[0, 1, 2])).unwrap();
        }

        let zone = learning_zone_ngrams(&conn, 1, 20).unwrap();
        assert!(!zone.is_empty(), "zone should surface mid-struggle keys");
        assert!(
            zone.iter().all(|w| !w.ngram.contains('ŵ')),
            "fully-broken keys stay OUT of the learning zone: {:?}",
            zone.iter().map(|w| &w.ngram).collect::<Vec<_>>()
        );
        assert!(
            zone.iter().any(|w| w.ngram.contains('č') || w.ngram.contains('š') || w.ngram.contains('ř') || w.ngram.contains('ě')),
            "mid-struggle diacritics should appear: {:?}",
            zone.iter().map(|w| &w.ngram).collect::<Vec<_>>()
        );
        // And the fully-mastered "a" shouldn't be in the zone either.
        assert!(
            zone.iter().all(|w| !(w.ngram == "a" || w.ngram == "aa" || w.ngram == "aaa")),
            "mastered keys stay OUT: {:?}",
            zone.iter().map(|w| &w.ngram).collect::<Vec<_>>()
        );
    }

    #[test]
    fn weak_ngrams_returns_sorted_by_weakness() {
        let mut conn = db::open_in_memory().unwrap();
        // Cleanly typed "aaa" three times
        for _ in 0..3 {
            update_stats(&mut conn, 1, &mk_log("aaa", &[])).unwrap();
        }
        // Error-prone "čč" three times
        for _ in 0..3 {
            update_stats(&mut conn, 1, &mk_log("čč", &[0, 1])).unwrap();
        }
        let weak = weak_ngrams(&conn, 1, 10).unwrap();
        assert!(!weak.is_empty());
        let top = &weak[0].ngram;
        assert!(top.contains('č'), "weak top should be the error-prone diacritic, got {top}");
    }

    #[test]
    fn pedagogy_density_favours_sentences_with_weak_ngrams() {
        let weak = vec![
            WeakNgram {
                ngram: "čř".to_string(),
                occurrences: 5,
                ema_latency_ms: 500.0,
                ema_error_rate: 0.5,
                weakness: 0.8,
            },
        ];
        let with = pedagogy_density("čřička", &weak);
        let without = pedagogy_density("jablko", &weak);
        assert!(with > without);
    }
}
