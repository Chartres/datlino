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

/// Three ways to pick content when the student chose the Content mode.
///
/// * `Across` — return the top-matching sentences spread across the whole
///   library. Best when the student wants to connect ideas ("everywhere
///   that mentions X").
/// * `Chapter` — return every sentence of a single chapter, in order.
///   Best for studying one section end-to-end.
/// * `ExamPrep` — the student describes their exam topic in natural
///   language; we return whole chapters ranked by how much they match.
///   BM25-only for now; embeddings (Week 2) will dramatically improve this.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentStrategy {
    Across,
    Chapter,
    ExamPrep,
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
    /// Only consulted when `mode == Content`. Defaults to `Across`.
    #[serde(default)]
    pub content_strategy: Option<ContentStrategy>,
    /// When `content_strategy == Chapter`, identifies which chapter to
    /// load. Format: "<document_id>::<section>".
    #[serde(default)]
    pub chapter_id: Option<String>,
}

impl Default for SessionRequest {
    fn default() -> Self {
        Self {
            mode: PracticeMode::Warmup,
            alpha: 0.7,
            target_duration_s: 300,
            query: None,
            pinned_source_prefixes: Vec::new(),
            content_strategy: None,
            chapter_id: None,
        }
    }
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
        PracticeMode::Content => match req.content_strategy.unwrap_or(ContentStrategy::Across) {
            ContentStrategy::Across => pick_content(conn, user_id, req)?,
            ContentStrategy::Chapter => pick_chapter(conn, req)?,
            ContentStrategy::ExamPrep => pick_exam_prep(conn, req)?,
        },
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

/// **Across** strategy — BM25 hits spread across the whole library.
///
/// The student wants to see every place their topic is mentioned, to build
/// connections. We return top-scoring sentences across many documents, de-
/// duped, capped per document to force diversity, and ordered so different
/// documents interleave rather than clumping.
///
/// Without a query we fall back to least-recently-typed, same diversity rules.
fn pick_content(
    conn: &Connection,
    _user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(req.target_duration_s);

    if let Some(q) = req.query.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return pick_across(conn, q, budget, &req.pinned_source_prefixes);
    }

    let mut out = Vec::new();
    let mut total = 0usize;
    for (chunk_id, text, source_path) in
        least_recently_typed(conn, 200, &req.pinned_source_prefixes)?
    {
        if total >= budget && out.len() >= 6 {
            break;
        }
        if is_low_content(&text) {
            continue;
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

fn pick_across(
    conn: &Connection,
    query: &str,
    budget: usize,
    pinned: &[String],
) -> Result<Vec<SessionSentence>> {
    use std::collections::{HashMap, HashSet};

    let hits = search::search(conn, query, 60)?;
    let hits: Vec<_> = hits
        .into_iter()
        .filter(|h| source_allowed(&h.source_path, pinned))
        .filter(|h| !is_low_content(&h.text))
        .collect();
    if hits.is_empty() {
        return Ok(Vec::new());
    }

    // Cap 3 hits per document, then round-robin interleave so the reader
    // bounces between sources — this is exactly what "connect ideas across
    // materials" means.
    let mut per_doc: HashMap<i64, Vec<search::SearchHit>> = HashMap::new();
    for h in hits {
        per_doc.entry(h.document_id).or_default().push(h);
    }
    for v in per_doc.values_mut() {
        v.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(3);
    }
    let mut queues: Vec<Vec<search::SearchHit>> = per_doc.into_values().collect();
    // Order document queues by their best hit so strongest docs go first.
    queues.sort_by(|a, b| {
        let sa = a.first().map(|h| h.score).unwrap_or(0.0);
        let sb = b.first().map(|h| h.score).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut total = 0usize;
    let mut exhausted = false;
    while !exhausted && total < budget {
        exhausted = true;
        for q in queues.iter_mut() {
            if q.is_empty() {
                continue;
            }
            let h = q.remove(0);
            exhausted = false;
            let text = h.text.trim().to_string();
            if !seen.insert(text.clone()) {
                continue;
            }
            total += text.chars().count();
            out.push(SessionSentence {
                chunk_id: Some(h.chunk_id),
                text,
                source_path: Some(h.source_path),
                is_generated: false,
            });
            if total >= budget {
                break;
            }
        }
    }
    Ok(out)
}

/// **Chapter** strategy — every sentence of a single markdown section,
/// in the order it appears in the source. Ideal for studying one chapter
/// end-to-end with full continuity.
fn pick_chapter(
    conn: &Connection,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(req.target_duration_s);
    let Some(ref id) = req.chapter_id else {
        return Ok(Vec::new());
    };
    let Some((doc_id, section)) = parse_chapter_id(id) else {
        return Ok(Vec::new());
    };

    let mut stmt = conn.prepare(
        "SELECT c.id, c.text, d.source_path
         FROM chunk c JOIN document d ON d.id = c.document_id
         WHERE c.document_id = ?1 AND c.section = ?2 AND c.is_heading = 0
         ORDER BY c.char_offset ASC",
    )?;
    let rows = stmt.query_map(params![doc_id, section], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;

    let mut out = Vec::new();
    let mut total = 0usize;
    for row in rows {
        let (id, text, src) = row?;
        if is_low_content(&text) {
            continue;
        }
        total += text.chars().count();
        out.push(SessionSentence {
            chunk_id: Some(id),
            text,
            source_path: Some(src),
            is_generated: false,
        });
        if total >= budget * 2 {
            // Chapters can exceed the budget — let them; longer passages
            // are deliberate in this mode.
            break;
        }
    }
    Ok(out)
}

/// **ExamPrep** strategy — the student describes the exam in natural
/// language; we return whole chapters ranked by relevance. BM25-only for
/// now; embeddings will massively improve topical recall (Week 2).
fn pick_exam_prep(
    conn: &Connection,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    use std::collections::HashMap;
    let budget = char_budget(req.target_duration_s);
    let Some(q) = req.query.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };

    // Each whitespace-separated token becomes its own BM25 query; aggregate
    // score per (document, section) gives us a chapter ranking.
    let tokens: Vec<&str> = q
        .split_whitespace()
        .filter(|t| t.chars().count() >= 3)
        .collect();
    let queries: Vec<&str> = if tokens.is_empty() { vec![q] } else { tokens };

    let mut chapter_scores: HashMap<(i64, String), (f64, String)> = HashMap::new();
    for qq in &queries {
        let hits = search::search(conn, qq, 80)?;
        for h in hits {
            if !source_allowed(&h.source_path, &req.pinned_source_prefixes) {
                continue;
            }
            // Look up the section of this hit chunk.
            let section: String = conn
                .query_row(
                    "SELECT section FROM chunk WHERE id = ?1",
                    params![h.chunk_id],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            let key = (h.document_id, section);
            let entry = chapter_scores
                .entry(key)
                .or_insert_with(|| (0.0, h.source_path.clone()));
            entry.0 += h.score;
        }
    }
    let mut ranked: Vec<_> = chapter_scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.partial_cmp(&a.1 .0).unwrap_or(std::cmp::Ordering::Equal));
    // Top 4 chapters — enough breadth, still bounded.
    ranked.truncate(4);

    let mut out = Vec::new();
    let mut total = 0usize;
    for ((doc_id, section), (_score, src)) in ranked {
        let mut stmt = conn.prepare(
            "SELECT id, text FROM chunk
             WHERE document_id = ?1 AND section = ?2 AND is_heading = 0
             ORDER BY char_offset ASC",
        )?;
        let rows = stmt.query_map(params![doc_id, section], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, text) = row?;
            if is_low_content(&text) {
                continue;
            }
            total += text.chars().count();
            out.push(SessionSentence {
                chunk_id: Some(id),
                text,
                source_path: Some(src.clone()),
                is_generated: false,
            });
            if total >= budget * 2 {
                break;
            }
        }
        if total >= budget * 2 {
            break;
        }
    }
    Ok(out)
}

fn parse_chapter_id(s: &str) -> Option<(i64, String)> {
    let (doc_str, section) = s.split_once("::")?;
    let doc_id: i64 = doc_str.parse().ok()?;
    Some((doc_id, section.to_string()))
}

/// Metadata for the chapter picker UI.
#[derive(Debug, Serialize)]
pub struct ChapterInfo {
    pub id: String,
    pub document_id: i64,
    pub source_path: String,
    pub section: String,
    pub sentence_count: i64,
}

pub fn list_chapters(conn: &Connection) -> Result<Vec<ChapterInfo>> {
    let mut stmt = conn.prepare(
        "SELECT c.document_id, d.source_path, c.section, COUNT(*) as n, MIN(c.char_offset) as first_off
         FROM chunk c JOIN document d ON d.id = c.document_id
         WHERE c.is_heading = 0 AND c.section != ''
         GROUP BY c.document_id, c.section
         HAVING n >= 2
         ORDER BY c.document_id ASC, first_off ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        let doc_id: i64 = r.get(0)?;
        let src: String = r.get(1)?;
        let section: String = r.get(2)?;
        let n: i64 = r.get(3)?;
        Ok(ChapterInfo {
            id: format!("{}::{}", doc_id, section),
            document_id: doc_id,
            source_path: src,
            section,
            sentence_count: n,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Sentences we never send to the typing engine — headings without content,
/// bullets that survived the segmenter, two-word fragments. We leave real
/// long sentences alone — students have explicitly asked to see them.
fn is_low_content(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() < 12 {
        return true;
    }
    // Fewer than 3 word-like tokens = probably a label.
    let tokens = trimmed
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .count();
    tokens < 3
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
            content_strategy: None,
            chapter_id: None,
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
            content_strategy: None,
            chapter_id: None,
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
            content_strategy: None,
            chapter_id: None,
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
        assert!(plan.sentences.iter().any(|s| s.text.contains("Habsburk")));
    }

    #[test]
    fn across_strategy_diversifies_across_documents() {
        // Same topic mentioned in two docs — Across mode should surface
        // hits from both, interleaved, deduped.
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("dejiny.md");
        let p2 = tmp.path().join("ekonomie.md");
        fs::write(&p1, "Hospodarska krize zasahla cely svet. Banky krachovaly. Nezamestnanost rostla rychle.").unwrap();
        fs::write(&p2, "Velka hospodarska krize byla vlnou poklesu. New Deal reagoval na krizi. Roosevelt byl prezident.").unwrap();
        ingest::ingest_file(&mut conn, &p1).unwrap();
        ingest::ingest_file(&mut conn, &p2).unwrap();

        let req = SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            query: Some("hospodarska krize".into()),
            pinned_source_prefixes: vec![],
            content_strategy: Some(ContentStrategy::Across),
            chapter_id: None,
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        let sources: std::collections::HashSet<_> = plan
            .sentences
            .iter()
            .filter_map(|s| s.source_path.clone())
            .collect();
        assert!(sources.len() >= 2, "Across should pull from multiple docs, got {sources:?}");
    }

    #[test]
    fn chapter_strategy_returns_full_section_in_order() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("dejiny.md");
        fs::write(
            &p,
            "# Dejiny 20. stoleti\n\n## Great Depression\n\nPrvni veta sekce o krize. Druha veta o dopadech. Treti veta o reakci vlad.\n\n## Studena valka\n\nOdlisne tema s jinymi vetami.",
        )
        .unwrap();
        ingest::ingest_file(&mut conn, &p).unwrap();

        let doc_id: i64 = conn
            .query_row("SELECT id FROM document WHERE source_path LIKE '%dejiny.md'", [], |r| r.get(0))
            .unwrap();
        let chapter_id = format!("{}::Dejiny 20. stoleti > Great Depression", doc_id);

        let req = SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            query: None,
            pinned_source_prefixes: vec![],
            content_strategy: Some(ContentStrategy::Chapter),
            chapter_id: Some(chapter_id),
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert_eq!(plan.sentences.len(), 3, "should return all 3 sentences of the Great Depression chapter");
        assert!(plan.sentences.iter().all(|s| !s.text.contains("Studena valka")));
    }

    #[test]
    fn exam_prep_strategy_ranks_chapters_by_aggregate_relevance() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("dejiny.md");
        fs::write(
            &p,
            "# Dejiny\n\n## Great Depression\n\nKrize zasahla USA. Roosevelt reagoval programem New Deal. Banky dostaly podporu.\n\n## Renesance\n\nItalska kultura 15. stoleti. Umeni vzkvetlo. Medici podporovali malire.",
        )
        .unwrap();
        ingest::ingest_file(&mut conn, &p).unwrap();

        let req = SessionRequest {
            mode: PracticeMode::Content,
            alpha: 1.0,
            target_duration_s: 300,
            query: Some("Great Depression Roosevelt New Deal".into()),
            pinned_source_prefixes: vec![],
            content_strategy: Some(ContentStrategy::ExamPrep),
            chapter_id: None,
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert!(!plan.sentences.is_empty());
        // Great Depression chapter should dominate; renaissance content
        // should be absent or at most trailing.
        let gd_count = plan.sentences.iter().filter(|s| s.text.contains("Roosevelt") || s.text.contains("Krize") || s.text.contains("Banky")).count();
        assert!(gd_count >= 2, "expected several GD chapter sentences, got: {:?}",
            plan.sentences.iter().map(|s| &s.text).collect::<Vec<_>>());
    }

    #[test]
    fn list_chapters_returns_sections_with_sentence_counts() {
        let mut conn = db::open_in_memory().unwrap();
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("dejiny.md");
        fs::write(&p, "# A\n\n## B\n\nVeta jedna. Veta dva. Veta tri.\n\n## C\n\nJeste veta. A dalsi.").unwrap();
        ingest::ingest_file(&mut conn, &p).unwrap();
        let chapters = list_chapters(&conn).unwrap();
        assert!(chapters.iter().any(|c| c.section.contains("B") && c.sentence_count >= 3));
        assert!(chapters.iter().any(|c| c.section.contains("C") && c.sentence_count >= 2));
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
            content_strategy: None,
            chapter_id: None,
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
            content_strategy: None,
            chapter_id: None,
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
