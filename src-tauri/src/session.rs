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
use crate::embeddings::{self, EmbeddingProviderKind};
use crate::pedagogy;
use crate::rephrase::{self, RephraseStyle};
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
    /// Structured progression for absolute beginners — home row → full
    /// keyboard → diacritics → real sentences.
    IntroLesson,
}

impl PracticeMode {
    fn as_str(&self) -> &'static str {
        match self {
            PracticeMode::Content => "content",
            PracticeMode::Warmup => "warmup",
            PracticeMode::Diacritics => "diacritics",
            PracticeMode::WeakKeys => "weak_keys",
            PracticeMode::Hybrid => "hybrid",
            PracticeMode::IntroLesson => "intro_lesson",
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
    /// Opt in to LLM rephrase mode (brief §5.9). Off by default. Auto-
    /// disabled for generated / non-chunk sentences and language-class
    /// material (auto-detection is a TODO — caller supplies the language).
    #[serde(default)]
    pub rephrase: bool,
    /// When `rephrase == true`, which recipe to use. Defaults to Keystrokes.
    #[serde(default)]
    pub rephrase_style: Option<RephraseStyle>,
    /// Language hint fed to the LLM system prompt. "cs" / "sk" / "en" / "de".
    #[serde(default)]
    pub language: Option<String>,
    /// When `mode == IntroLesson`, which lesson id to load (from
    /// `lessons::curriculum`). If absent, picks the first lesson the
    /// student hasn't yet passed.
    #[serde(default)]
    pub lesson_id: Option<String>,
    /// When `mode == Content` and the student picks a specific file via
    /// the document picker, we bypass BM25 and return every chunk of
    /// that document in source order.
    #[serde(default)]
    pub document_id: Option<i64>,
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
            rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionSentence {
    /// `None` for generated drills (Diacritics fallback strings).
    pub chunk_id: Option<i64>,
    /// What the student actually types — either the verbatim source or an
    /// LLM-rephrased variant.
    pub text: String,
    pub source_path: Option<String>,
    pub is_generated: bool,
    /// When the rephrase pipeline produced an accepted rewrite, the
    /// verbatim source is mirrored here so the UI can show "Originál ↔
    /// Remix" and the student can toggle back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_text: Option<String>,
    /// `rephrased_chunk.id` when a rewrite is attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rephrased_id: Option<i64>,
    /// Cosine(source, rephrase) against the active embedding provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,
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
    let mut sentences = match req.mode {
        PracticeMode::Content if req.document_id.is_some() => {
            pick_document(conn, req.document_id.unwrap(), req.target_duration_s)?
        }
        PracticeMode::Content => match req.content_strategy.unwrap_or(ContentStrategy::Across) {
            ContentStrategy::Across => pick_content(conn, user_id, req)?,
            ContentStrategy::Chapter => pick_chapter(conn, req)?,
            ContentStrategy::ExamPrep => pick_exam_prep(conn, req)?,
        },
        PracticeMode::Warmup => pick_warmup(conn, req)?,
        PracticeMode::Diacritics => pick_diacritics(conn, req)?,
        PracticeMode::WeakKeys => pick_weak_keys(conn, user_id, req)?,
        PracticeMode::Hybrid => pick_hybrid(conn, user_id, req)?,
        PracticeMode::IntroLesson => pick_intro_lesson(conn, user_id, req)?,
    };

    // Optional rephrase pass — only for Content mode with backing chunks.
    // Failures (missing key, network error, similarity gate rejection)
    // leave the verbatim sentence in place; the student never gets a
    // broken session.
    if req.rephrase && matches!(req.mode, PracticeMode::Content) {
        if let Err(e) = apply_rephrase(conn, user_id, &mut sentences, req) {
            eprintln!("[datlino] rephrase pass failed, continuing with verbatim: {e}");
        }
    }

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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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

    // Also match the query against document source_paths — a student
    // searching "chemie" should land on `chemie-periodicka-soustava.md`
    // even if the word never appears in the body. We pull EVERY chunk of
    // any document whose path matches.
    let path_doc_ids = documents_matching_path(conn, query, pinned)?;

    // If BM25 only surfaced a single chunk, but it lives in a much bigger
    // document (as "chemie" currently does — one keyword buried in a
    // 10-sentence file), fall through to showing the whole document. The
    // student clearly wants to study the chapter, not squint at one hit.
    let scarce_hit_doc = {
        let mut doc_hits: HashMap<i64, usize> = HashMap::new();
        for h in &hits {
            *doc_hits.entry(h.document_id).or_insert(0) += 1;
        }
        if doc_hits.len() == 1 && *doc_hits.values().next().unwrap_or(&0) <= 2 {
            doc_hits.keys().next().copied()
        } else {
            None
        }
    };

    let expand_doc_ids: HashSet<i64> = path_doc_ids
        .iter()
        .copied()
        .chain(scarce_hit_doc.into_iter())
        .collect();
    if !expand_doc_ids.is_empty() {
        return expand_whole_documents(conn, &expand_doc_ids, budget);
    }

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
                source_text: None,
                rephrased_id: None,
                similarity: None,
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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

    // Dedup + length filter so short stopwords don't flood the scorer.
    let mut seen = std::collections::HashSet::new();
    let tokens: Vec<String> = q
        .split_whitespace()
        .filter(|t| t.chars().count() >= 3)
        .map(|t| t.to_lowercase())
        .filter(|t| seen.insert(t.clone()))
        .collect();
    let queries: Vec<String> = if tokens.is_empty() {
        vec![q.to_string()]
    } else {
        tokens
    };

    // Rank chapters by aggregate BM25 across all tokens. One SQL per token
    // with the section joined inline — no N+1 lookups.
    let mut chapter_scores: HashMap<(i64, String), (f64, String)> = HashMap::new();
    for qq in &queries {
        let match_expr = search::build_match_expression(qq);
        if match_expr.is_empty() {
            continue;
        }
        let mut stmt = conn.prepare(
            "SELECT c.document_id, c.section, d.source_path, bm25(chunk_fts) AS bm
             FROM chunk_fts
             JOIN chunk    c ON c.id = chunk_fts.rowid
             JOIN document d ON d.id = c.document_id
             WHERE chunk_fts MATCH ?1
             ORDER BY bm ASC
             LIMIT 80",
        )?;
        let rows = stmt.query_map(params![&match_expr], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                -r.get::<_, f64>(3)?, // SQLite bm25 is lower-better; flip.
            ))
        })?;
        for row in rows {
            let (doc_id, section, source_path, score) = row?;
            if !source_allowed(&source_path, &req.pinned_source_prefixes) {
                continue;
            }
            let entry = chapter_scores
                .entry((doc_id, section))
                .or_insert_with(|| (0.0, source_path));
            entry.0 += score;
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
                source_text: None,
                rephrased_id: None,
                similarity: None,
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

/// Build a chunk_id → cosine(query, chunk) map using the active embedding
/// provider, or return `None` if no provider is configured / no chunks
/// are embedded yet. Values are scaled to [0, 1] to blend with the α mix.
fn embedding_relevance(
    conn: &Connection,
    query: &str,
) -> Result<Option<std::collections::HashMap<i64, f64>>> {
    use std::collections::HashMap;

    // Read the active provider off embedding_meta. "none" or dim 0 means
    // no embeddings to consult — bail and let caller fall back to BM25.
    let (provider, dim): (String, i64) = conn.query_row(
        "SELECT provider, dim FROM embedding_meta WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let kind = EmbeddingProviderKind::parse(&provider);
    if matches!(kind, EmbeddingProviderKind::None) || dim == 0 {
        return Ok(None);
    }

    // Do we actually have any embedded chunks? If not, don't spend a
    // provider call on the query.
    let embedded_count: i64 = conn.query_row(
        "SELECT count(*) FROM chunk WHERE embedding IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    if embedded_count == 0 {
        return Ok(None);
    }

    let provider = match embeddings::build(kind) {
        Ok(p) => p,
        Err(_) => return Ok(None), // misconfigured (e.g. Cohere key missing) → fall back
    };
    let q_vecs = provider.embed_batch(&[query.to_string()])?;
    let q_vec = q_vecs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("embedding provider returned no vector"))?;

    let mut stmt =
        conn.prepare("SELECT id, embedding FROM chunk WHERE embedding IS NOT NULL")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;

    let mut sims: Vec<(i64, f32)> = Vec::new();
    let mut min_s = f32::INFINITY;
    let mut max_s = f32::NEG_INFINITY;
    for row in rows {
        let (id, blob) = row?;
        let v = embeddings::blob_to_vec(&blob);
        let s = embeddings::cosine_similarity(&q_vec, &v);
        sims.push((id, s));
        if s < min_s {
            min_s = s;
        }
        if s > max_s {
            max_s = s;
        }
    }
    if sims.is_empty() {
        return Ok(None);
    }
    // Rescale cosine [−1, 1] → [0, 1] relative to this query's spread so
    // the blend with pedagogy_density is well-calibrated.
    let span = (max_s - min_s).max(1e-6);
    let mut out: HashMap<i64, f64> = HashMap::with_capacity(sims.len());
    for (id, s) in sims {
        out.insert(id, ((s - min_s) / span) as f64);
    }
    Ok(Some(out))
}

fn apply_rephrase(
    conn: &Connection,
    user_id: i64,
    sentences: &mut [SessionSentence],
    req: &SessionRequest,
) -> Result<()> {
    // Zone-of-proximal-development n-grams (keys pushing the student at
    // their level, not the totally-broken ones — those go in the isolated
    // weak-keys drill). Fall back to the full weak list if the user is too
    // new to have a meaningful zone yet.
    let zone = pedagogy::learning_zone_ngrams(conn, user_id, 12)?;
    let weak_list: Vec<String> = if zone.is_empty() {
        pedagogy::weak_ngrams(conn, user_id, 12)?
            .into_iter()
            .map(|w| w.ngram)
            .collect()
    } else {
        zone.into_iter().map(|w| w.ngram).collect()
    };
    let language = req.language.as_deref().unwrap_or("cs");
    let style = req.rephrase_style.unwrap_or(RephraseStyle::Keystrokes);

    // Need an embedding provider for the similarity gate. Build from
    // current profile; if none configured, fall back to the Fake provider
    // — cosine on character-bigram hashes still distinguishes "same
    // meaning" from "drifted".
    let (provider_str, _): (String, i64) = conn.query_row(
        "SELECT provider, dim FROM embedding_meta WHERE id = 1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let kind = EmbeddingProviderKind::parse(&provider_str);
    let provider = embeddings::build(match kind {
        EmbeddingProviderKind::None => EmbeddingProviderKind::Fake,
        other => other,
    })?;

    for s in sentences.iter_mut() {
        let Some(chunk_id) = s.chunk_id else { continue };
        if s.is_generated {
            continue;
        }
        let request = rephrase::RephraseRequest {
            source: &s.text,
            weak_ngrams: &weak_list,
            language,
            style,
            similarity_floor: None,
        };
        match rephrase::rephrase_sentence(&request, &*provider) {
            Ok(outcome) if outcome.accepted => {
                let id = rephrase::store_rephrase(conn, chunk_id, &outcome, &weak_list)?;
                s.source_text = Some(std::mem::replace(&mut s.text, outcome.text));
                s.rephrased_id = Some(id);
                s.similarity = Some(outcome.similarity);
            }
            Ok(outcome) => {
                // Below the similarity floor — drop the rewrite silently.
                eprintln!(
                    "[datlino] rephrase drifted (sim {:.2}); using verbatim",
                    outcome.similarity
                );
            }
            Err(e) => {
                eprintln!("[datlino] rephrase error: {e}");
            }
        }
    }
    Ok(())
}

/// Match a user's query against document `source_path`s. Splits the path
/// on non-alphanumerics so "chemie" matches "chemie-periodicka-soustava.md".
/// Returns matching document ids.
fn documents_matching_path(
    conn: &Connection,
    query: &str,
    pinned: &[String],
) -> Result<Vec<i64>> {
    let q = query.trim().to_lowercase();
    if q.len() < 3 {
        return Ok(Vec::new());
    }
    // Pull the whole document list (tens to hundreds of rows — tiny for
    // SQLite) and filter in Rust; lets us split the path on anything.
    let mut stmt = conn.prepare("SELECT id, source_path FROM document")?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, path) = row?;
        if !source_allowed(&path, pinned) {
            continue;
        }
        let slug = std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        let tokens: Vec<&str> = slug
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .collect();
        if tokens.iter().any(|t| *t == q) {
            out.push(id);
            continue;
        }
        // Fuzzy: any query word appears whole in a filename token.
        for q_word in q.split_whitespace() {
            if q_word.len() >= 3 && tokens.iter().any(|t| *t == q_word) {
                out.push(id);
                break;
            }
        }
    }
    Ok(out)
}

fn expand_whole_documents(
    conn: &Connection,
    doc_ids: &std::collections::HashSet<i64>,
    budget: usize,
) -> Result<Vec<SessionSentence>> {
    let mut out = Vec::new();
    let mut total = 0usize;
    // Take docs in a stable order so the session doesn't jump randomly.
    let mut sorted: Vec<i64> = doc_ids.iter().copied().collect();
    sorted.sort();
    for doc_id in sorted {
        let src: String = conn.query_row(
            "SELECT source_path FROM document WHERE id = ?1",
            params![doc_id],
            |r| r.get(0),
        )?;
        let mut stmt = conn.prepare(
            "SELECT id, text FROM chunk WHERE document_id = ?1 AND is_heading = 0
             ORDER BY char_offset ASC",
        )?;
        let rows = stmt.query_map(params![doc_id], |r| {
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
                source_text: None,
                rephrased_id: None,
                similarity: None,
            });
            if total >= budget * 2 {
                return Ok(out);
            }
        }
    }
    Ok(out)
}

/// Metadata for the document picker — lets students drill whole files
/// without having to pick a specific chapter.
#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    pub id: i64,
    pub source_path: String,
    pub kind: String,
    pub chunk_count: i64,
}

pub fn list_documents(conn: &Connection) -> Result<Vec<DocumentInfo>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.source_path, d.kind, COUNT(c.id) AS chunks
         FROM document d
         LEFT JOIN chunk c ON c.document_id = d.id AND c.is_heading = 0
         GROUP BY d.id
         ORDER BY d.source_path ASC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(DocumentInfo {
            id: r.get(0)?,
            source_path: r.get(1)?,
            kind: r.get(2)?,
            chunk_count: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Whole-document session generator — returns every chunk of the given
/// document in source order (the single-doc analogue of Chapter mode).
pub fn pick_document(
    conn: &Connection,
    document_id: i64,
    duration_s: i64,
) -> Result<Vec<SessionSentence>> {
    let budget = char_budget(duration_s);
    let doc_ids: std::collections::HashSet<i64> = [document_id].into_iter().collect();
    expand_whole_documents(conn, &doc_ids, budget)
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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

/// Full brief §2 formula. When a real embedding provider is configured
/// AND the query is non-empty, relevance is cosine(query_emb, chunk_emb).
/// Otherwise we fall back to normalised BM25 — always works, no network.
fn pick_hybrid(
    conn: &Connection,
    user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let alpha = req.alpha.clamp(0.0, 1.0);
    let weak = pedagogy::weak_ngrams(conn, user_id, 20)?;
    let budget = char_budget(req.target_duration_s);

    // Relevance signal — prefer embeddings when available, BM25 otherwise.
    let mut relevance: std::collections::HashMap<i64, f64> = Default::default();
    if let Some(q) = req.query.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Some(by_cosine) = embedding_relevance(conn, q)? {
            relevance = by_cosine;
        } else {
            let hits = search::search(conn, q, 500)?;
            let max = hits.iter().map(|h| h.score).fold(0.0_f64, f64::max).max(1.0);
            for h in hits {
                relevance.insert(h.chunk_id, (h.score / max).max(0.0));
            }
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
            source_text: None,
            rephrased_id: None,
            similarity: None,
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

/// **IntroLesson** mode — load drills from the `lessons` curriculum.
/// Picks the requested lesson (or the first not-yet-passed one if the
/// caller didn't specify).
fn pick_intro_lesson(
    conn: &Connection,
    user_id: i64,
    req: &SessionRequest,
) -> Result<Vec<SessionSentence>> {
    let lesson_id: String = match req.lesson_id.as_deref() {
        Some(id) => id.to_string(),
        None => first_unpassed_lesson(conn, user_id)?,
    };
    let Some(lesson) = crate::lessons::lesson_by_id(&lesson_id) else {
        return Ok(Vec::new());
    };
    let drills = (lesson.drills)();
    Ok(drills
        .into_iter()
        .map(|d| SessionSentence {
            chunk_id: None,
            text: d.text,
            source_path: Some(format!("lesson://{}", lesson.meta.id)),
            is_generated: true,
            source_text: None,
            rephrased_id: None,
            similarity: None,
        })
        .collect())
}

fn first_unpassed_lesson(conn: &Connection, user_id: i64) -> Result<String> {
    let mut stmt = conn.prepare(
        "SELECT lesson_id FROM intro_lesson_progress
         WHERE user_id = ?1 AND first_passed_at IS NOT NULL",
    )?;
    let passed: std::collections::HashSet<String> = stmt
        .query_map(params![user_id], |r| r.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    for lesson in crate::lessons::curriculum() {
        if !passed.contains(lesson.meta.id) {
            return Ok(lesson.meta.id.to_string());
        }
    }
    // All passed — stay on the last lesson as a refresher.
    Ok(crate::lessons::curriculum()
        .last()
        .map(|l| l.meta.id.to_string())
        .unwrap_or_default())
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
        source_text: None,
        rephrased_id: None,
        similarity: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
    fn chapter_strategy_returns_full_section_in_source_order() {
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
            .query_row(
                "SELECT id FROM document WHERE source_path LIKE '%dejiny.md'",
                [],
                |r| r.get(0),
            )
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
        };
        let plan = create_session(&mut conn, 1, &req).unwrap();
        assert_eq!(
            plan.sentences.iter().map(|s| s.text.as_str()).collect::<Vec<_>>(),
            vec![
                "Prvni veta sekce o krize.",
                "Druha veta o dopadech.",
                "Treti veta o reakci vlad.",
            ],
            "chapter mode must preserve source order exactly"
        );
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
        rephrase: false,
            rephrase_style: None,
            language: None,
            lesson_id: None,
            document_id: None,
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
