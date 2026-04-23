//! Session generator — picks sentences for a typing session.
//!
//! Practice modes (Montessori-inspired names in the UI — mapping here):
//!
//! * `Content` — real sentences from the student's corpus (prepared
//!   environment, real materials). Uses BM25 if a query is given, else
//!   recency-aware random sampling.
//! * `Warmup` — short, simple corpus sentences first; confidence before
//!   challenge.
//! * `Diacritics` — generated drills isolating Czech/Slovak diacritics
//!   (isolation of difficulty). Falls back to corpus words rich in the
//!   target glyphs when the corpus is populated.
//! * `WeakKeys` — corpus sentences chosen to maximise the student's own
//!   weak-ngram density (control of error, self-directed repair). Falls
//!   back to Diacritics for brand-new users with no pedagogy data.
//! * `Hybrid` — the full §2 scorer: α·relevance + (1-α)·pedagogy − λ·recency.
//!
//! The generator always writes a `session` row and returns an ordered list
//! of sentences. Recording per-sentence attempts is the caller's job.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::now_unix;
use crate::pedagogy;
use crate::search;

const RECENCY_LAMBDA: f64 = 0.15;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    Content,
    Warmup,
    Diacritics,
    WeakKeys,
    Hybrid,
}

impl PracticeMode {
    fn as_str(&self) -> &'static str {
        match self {
            PracticeMode::Content => "content",
            PracticeMode::Warmup => "warmup",
            PracticeMode::Diacritics => "diacritics",
            PracticeMode::WeakKeys => "weak_keys",
            PracticeMode::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionRequest {
    pub mode: PracticeMode,
    /// 0.0 = pure pedagogy, 1.0 = pure content. Ignored outside Hybrid.
    pub alpha: f64,
    /// Target duration in seconds — drives how many sentences we pick.
    pub target_duration_s: i64,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub pinned_source_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSentence {
    /// `None` for generated drills (Diacritics fallback strings).
    pub chunk_id: Option<i64>,
    pub text: String,
    pub source_path: Option<String>,
    pub is_generated: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionPlan {
    pub session_id: i64,
    pub mode: PracticeMode,
    pub alpha: f64,
    pub sentences: Vec<SessionSentence>,
}

/// Target char budget for a session = duration × expected throughput.
/// Assume ~240 chars/min (≈ 48 WPM) as the typical high-schooler target —
/// overshoots for fast kids, no harm done.
fn char_budget(duration_s: i64) -> usize {
    ((duration_s as f64 / 60.0) * 240.0).ceil() as usize
}

pub fn create_session(
    conn: &mut Connection,
    user_id: i64,
    req: &SessionRequest,
) -> Result<SessionPlan> {
    let sentences = match req.mode {
        PracticeMode::Content => pick_content(conn, user_id, req)?,
        PracticeMode::Warmup => pick_warmup(conn, req)?,
        PracticeMode::Diacritics => pick_diacritics(conn, req)?,
        PracticeMode::WeakKeys => pick_weak_keys(conn, user_id, req)?,
        PracticeMode::Hybrid => pick_hybrid(conn, user_id, req)?,
    };

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO session(user_id, created_at, mode, alpha, target_duration_s, query, pinned_sources)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            user_id,
            now_unix(),
            req.mode.as_str(),
            req.alpha,
            req.target_duration_s,
            req.query.clone().unwrap_or_default(),
            serde_json::to_string(&req.pinned_source_prefixes)?,
        ],
    )?;
    let session_id = tx.last_insert_rowid();
    tx.commit()?;

    Ok(SessionPlan {
        session_id,
        mode: req.mode,
        alpha: req.alpha,
        sentences,
    })
}

/// Pick by FTS5 relevance if query, else recency-aware sampling.
fn pick_content(
    conn: &Connection,
    _user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(req.target_duration_s);
    let mut out = Vec::new();
    let mut total = 0usize;

    let rows = if let Some(q) = req.query.as_deref().filter(|s| !s.trim().is_empty()) {
        let hits = search::search(conn, q, 200)?;
        hits.into_iter()
            .filter(|h| source_allowed(&h.source_path, &req.pinned_source_prefixes))
            .map(|h| (Some(h.chunk_id), h.text, Some(h.source_path)))
            .collect::<Vec<_>>()
    } else {
        least_recently_typed(conn, 200, &req.pinned_source_prefixes)?
    };

    for (chunk_id, text, source_path) in rows {
        if total >= budget && out.len() >= 6 {
            break;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id,
            text,
            source_path,
            is_generated: false,
        });
    }
    Ok(out)
}

fn pick_warmup(conn: &Connection, req: &SessionRequest) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(req.target_duration_s);
    // Short, simple sentences from the corpus — build confidence first.
    let rows = corpus_sentences_ranked_by(
        conn,
        &req.pinned_source_prefixes,
        |text| {
            let len = text.chars().count() as f64;
            // Prefer length around 35 chars; penalise far-from-center.
            let len_score = 1.0 / (1.0 + (len - 35.0).abs() / 20.0);
            let punct = text.chars().filter(|c| ",;:()—–\"'".contains(*c)).count() as f64;
            let punct_penalty = 1.0 / (1.0 + punct);
            len_score * punct_penalty
        },
        80,
    )?;

    let mut out = Vec::new();
    let mut total = 0usize;
    for (chunk_id, text, source_path, _score) in rows {
        if total >= budget && out.len() >= 4 {
            break;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: Some(chunk_id),
            text,
            source_path: Some(source_path),
            is_generated: false,
        });
    }
    if out.is_empty() {
        // Fresh install, no corpus — fall back to built-in warmups.
        out = builtin_warmup_drills();
    }
    Ok(out)
}

/// Diacritic-isolation drills. Pure generated sequences — no corpus needed,
/// which is why this mode is also a good default on day 1.
fn pick_diacritics(conn: &Connection, req: &SessionRequest) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(req.target_duration_s);
    let glyphs = ['č', 'š', 'ř', 'ě', 'ů', 'ý', 'á', 'í', 'é', 'ú', 'ď', 'ť', 'ň', 'ó', 'ľ', 'ĺ', 'ŕ'];

    let mut out = Vec::new();
    let mut total = 0usize;

    // Step 1: isolated repetitions — "čč čč čč áá áá áá".
    for g in glyphs.iter().take(10) {
        let text = format!("{g}{g} {g}{g} {g}{g} {g}{g} {g}{g}");
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: None,
            text,
            source_path: None,
            is_generated: true,
        });
        if total >= budget / 3 {
            break;
        }
    }

    // Step 2: diacritic bigrams — "áč čá šč čš řě ěř".
    for pair in [
        ('á', 'č'),
        ('š', 'č'),
        ('ř', 'ě'),
        ('ě', 'ř'),
        ('ů', 'ý'),
        ('ť', 'ň'),
        ('í', 'á'),
    ] {
        let (a, b) = pair;
        let text = format!("{a}{b} {b}{a} {a}{b} {b}{a} {a}{b}");
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: None,
            text,
            source_path: None,
            is_generated: true,
        });
        if total >= 2 * budget / 3 {
            break;
        }
    }

    // Step 3: weave in corpus words rich in diacritics (if corpus exists) —
    // moves the student from abstract glyphs to real Czech text.
    let real = corpus_sentences_ranked_by(
        conn,
        &req.pinned_source_prefixes,
        |text| {
            let dia = text
                .chars()
                .filter(|c| glyphs.contains(c))
                .count() as f64;
            let len = text.chars().count() as f64;
            dia / len.max(1.0)
        },
        40,
    )?;
    for (chunk_id, text, source_path, _) in real {
        if total >= budget {
            break;
        }
        if text.chars().count() > 110 {
            continue;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: Some(chunk_id),
            text,
            source_path: Some(source_path),
            is_generated: false,
        });
    }

    Ok(out)
}

/// Pick corpus sentences maximising pedagogy density against the user's
/// current weak-ngram profile. Empty profile → fall back to Diacritics.
fn pick_weak_keys(
    conn: &Connection,
    user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let weak = pedagogy::weak_ngrams(conn, user_id, 20)?;
    if weak.is_empty() {
        return pick_diacritics(conn, req);
    }
    let budget = char_budget(req.target_duration_s);

    let rows = corpus_sentences_ranked_by(
        conn,
        &req.pinned_source_prefixes,
        |text| pedagogy::pedagogy_density(text, &weak),
        120,
    )?;
    let mut out = Vec::new();
    let mut total = 0usize;
    for (chunk_id, text, source_path, score) in rows {
        if score <= 0.0 {
            break;
        }
        if total >= budget && out.len() >= 5 {
            break;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: Some(chunk_id),
            text,
            source_path: Some(source_path),
            is_generated: false,
        });
    }
    // Top up with Diacritic drills if the corpus didn't yield enough density.
    if total < budget / 2 {
        for s in pick_diacritics(conn, req)?.into_iter().take(5) {
            total += s.text.chars().count();
            out.push(s);
            if total >= budget {
                break;
            }
        }
    }
    Ok(out)
}

/// Full brief §2 formula. Relevance is BM25-normalised for now; embeddings
/// arrive in Week 2.
fn pick_hybrid(
    conn: &Connection,
    user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let alpha = req.alpha.clamp(0.0, 1.0);
    let weak = pedagogy::weak_ngrams(conn, user_id, 20)?;
    let budget = char_budget(req.target_duration_s);

    // Relevance signal — normalised BM25 over the query if given.
    let mut relevance: std::collections::HashMap<i64, f64> = Default::default();
    if let Some(q) = req.query.as_deref().filter(|s| !s.trim().is_empty()) {
        let hits = search::search(conn, q, 500)?;
        let max = hits.iter().map(|h| h.score).fold(0.0_f64, f64::max).max(1.0);
        for h in hits {
            relevance.insert(h.chunk_id, (h.score / max).max(0.0));
        }
    }

    let rows = corpus_sentences_ranked_by(
        conn,
        &req.pinned_source_prefixes,
        |_| 0.0,
        500,
    )?;
    let now = now_unix() as f64;
    let recency = session_recency_map(conn, user_id)?;

    let mut scored: Vec<(f64, i64, String, String)> = rows
        .into_iter()
        .map(|(id, text, src, _)| {
            let rel = *relevance.get(&id).unwrap_or(&0.0);
            let ped = pedagogy::pedagogy_density(&text, &weak);
            let days_since = recency
                .get(&id)
                .map(|t| (now - *t) / 86_400.0)
                .unwrap_or(365.0);
            // Recency penalty peaks when we typed this in the last day.
            let rec_pen = (-days_since).exp().min(1.0);
            let score = alpha * rel + (1.0 - alpha) * ped - RECENCY_LAMBDA * rec_pen;
            (score, id, text, src)
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut out = Vec::new();
    let mut total = 0usize;
    for (_, id, text, src) in scored {
        if total >= budget && out.len() >= 5 {
            break;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: Some(id),
            text,
            source_path: Some(src),
            is_generated: false,
        });
    }
    if out.is_empty() {
        return pick_warmup(conn, req);
    }
    Ok(out)
}

// ---------- helpers ----------

fn least_recently_typed(
    conn: &Connection,
    limit: usize,
    pinned: &[String],
) -> Result<Vec<(Option<i64>, String, Option<String>)>> {
    // "Recency" for Week 1: chunks that have never been an attempt target
    // first, then fall back to newest-ingested. Simple, cheap, improves once
    // we have richer history.
    let mut stmt = conn.prepare(
        "SELECT c.id, c.text, d.source_path,
                (SELECT MAX(a.started_at) FROM attempt a WHERE a.chunk_id = c.id) AS last_seen
         FROM chunk c
         JOIN document d ON d.id = c.document_id
         ORDER BY last_seen IS NOT NULL, last_seen ASC, c.id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit as i64], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, text, src) = row?;
        if !source_allowed(&src, pinned) {
            continue;
        }
        out.push((Some(id), text, Some(src)));
    }
    Ok(out)
}

/// Load N corpus sentences, score each with the caller-supplied function,
/// sort descending and return.
fn corpus_sentences_ranked_by<F>(
    conn: &Connection,
    pinned: &[String],
    scorer: F,
    limit: usize,
) -> Result<Vec<(i64, String, String, f64)>>
where
    F: Fn(&str) -> f64,
{
    let mut stmt = conn.prepare(
        "SELECT c.id, c.text, d.source_path
         FROM chunk c JOIN document d ON d.id = c.document_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
    })?;

    let mut scored: Vec<(i64, String, String, f64)> = Vec::new();
    for row in rows {
        let (id, text, src) = row?;
        if !source_allowed(&src, pinned) {
            continue;
        }
        let s = scorer(&text);
        scored.push((id, text, src, s));
    }
    scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);
    Ok(scored)
}

fn source_allowed(path: &str, pinned: &[String]) -> bool {
    if pinned.is_empty() {
        return true;
    }
    pinned.iter().any(|p| path.starts_with(p))
}

fn session_recency_map(conn: &Connection, user_id: i64) -> Result<std::collections::HashMap<i64, f64>> {
    let mut stmt = conn.prepare(
        "SELECT a.chunk_id, MAX(a.started_at) FROM attempt a
         JOIN session s ON s.id = a.session_id
         WHERE s.user_id = ?1 AND a.chunk_id IS NOT NULL
         GROUP BY a.chunk_id",
    )?;
    let rows = stmt.query_map(params![user_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)? as f64))
    })?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        let (id, t) = row?;
        map.insert(id, t);
    }
    Ok(map)
}

fn builtin_warmup_drills() -> Vec<SessionSentence> {
    [
        "asdf jkl; asdf jkl; asdf jkl;",
        "ahoj svete ahoj svete ahoj svete",
        "česká abeceda začíná áčďě šťň",
        "píšeme pomalu a přesně",
    ]
    .iter()
    .map(|t| SessionSentence {
        chunk_id: None,
        text: (*t).to_string(),
        source_path: None,
        is_generated: true,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, ingest};
    use rusqlite::params;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn seed_corpus(conn: &mut Connection) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("dejepis.md");
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(
            b"Habsburkove vladli v Cechach. Karel IV. byl vyznamny.\n\nPisi cestinou. Mam rad diakritiku: cscr ero eru.",
        )
        .unwrap();
        ingest::ingest_file(conn, &path).unwrap();
        tmp
    }

    #[test]
    fn warmup_mode_returns_short_sentences_or_builtins() {
        let mut conn = db::open_in_memory().unwrap();
        let req = SessionRequest {
            mode: PracticeMode::Warmup,
            alpha: 0.7,
            target_duration_s: 60,
            query: None,
            pinned_source_prefixes: vec![],
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
        // No corpus seeded ⇒ built-in drills.
        assert!(plan.sentences.iter().all(|s| s.is_generated));
    }

    #[test]
    fn diacritics_mode_always_produces_drills_without_corpus() {
        let mut conn = db::open_in_memory().unwrap();
        let req = SessionRequest {
            mode: PracticeMode::Diacritics,
            alpha: 0.0,
            target_duration_s: 30,
            query: None,
            pinned_source_prefixes: vec![],
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
        assert!(plan.sentences[0].text.chars().any(|c| "čšřěůýáíéúďťňóľĺŕ".contains(c)));
    }

    #[test]
    fn content_mode_uses_corpus() {
        let mut conn = db::open_in_memory().unwrap();
        let _tmp = seed_corpus(&mut conn);
        let req = SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 120,
            query: Some("Habsburkove".into()),
            pinned_source_prefixes: vec![],
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
        assert!(plan.sentences.iter().any(|s| s.text.contains("Habsburk")));
    }

    #[test]
    fn weak_keys_falls_back_to_diacritics_for_new_user() {
        let mut conn = db::open_in_memory().unwrap();
        let req = SessionRequest {
            mode: PracticeMode::WeakKeys,
            alpha: 0.0,
            target_duration_s: 60,
            query: None,
            pinned_source_prefixes: vec![],
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
    }

    #[test]
    fn session_row_is_persisted() {
        let mut conn = db::open_in_memory().unwrap();
        let req = SessionRequest {
            mode: PracticeMode::Diacritics,
            alpha: 0.0,
            target_duration_s: 30,
            query: None,
            pinned_source_prefixes: vec![],
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM session WHERE id = ?1",
                params![plan.session_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
