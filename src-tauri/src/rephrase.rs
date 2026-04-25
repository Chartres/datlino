//! LLM rephrase mode (brief §5.9).
//!
//! Takes a verbatim source sentence and rewrites it to inject the user's
//! current weak bigrams, while preserving factual claims, proper nouns,
//! titles, dates, and specialist terminology. Off by default — the student
//! (or parent) has to opt in per session. The UI always exposes the source
//! alongside the rephrase so the student can toggle back, preserving the
//! trust contract.
//!
//! Safety rails implemented here:
//!
//! * Hard system prompt: "do not invent facts; preserve proper nouns and
//!   numbers verbatim; target length within ±20%".
//! * Cosine similarity gate (using the active embedding provider): drops
//!   rewrites below a configurable threshold (default 0.85).
//! * Never auto-enabled for language-class material (detection is a TODO;
//!   caller passes a hint).
//! * The generator is asked to emit JSON so we can parse without brittle
//!   regexes.
//!
//! Providers: Anthropic (Claude Haiku by default — cheap + plenty capable
//! for sentence-level paraphrase). OpenAI-compatible endpoints slot in
//! behind the same trait later.

use anyhow::{anyhow, bail, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::claude_auth;
use crate::db::now_unix;
use crate::embeddings::{self, EmbeddingProvider};

const KEYCHAIN_SERVICE: &str = "org.datlino.app";
const KEYCHAIN_ENTRY_ANTHROPIC: &str = "anthropic-api-key";
// Sonnet 4.6 trades ~2x cost for noticeably better rewrite quality on
// factual material. Subscription users don't pay per token, so this is
// a free upgrade for them; BYOK users can swap back via a future
// setting.
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const DEFAULT_SIMILARITY_FLOOR: f32 = 0.85;

enum AuthPlan {
    Subscription(String),
    ApiKey(String),
}

pub fn read_anthropic_key() -> Result<Option<String>> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY_ANTHROPIC) {
        Ok(entry) => match entry.get_password() {
            Ok(s) if !s.is_empty() => Ok(Some(s)),
            Ok(_) => Ok(None),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("keyring get: {e}")),
        },
        Err(e) => Err(anyhow!("keyring entry: {e}")),
    }
}

pub fn write_anthropic_key(value: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY_ANTHROPIC)
        .map_err(|e| anyhow!("keyring entry: {e}"))?;
    if value.is_empty() {
        let _ = entry.delete_password();
    } else {
        entry
            .set_password(value)
            .map_err(|e| anyhow!("keyring set: {e}"))?;
    }
    Ok(())
}

/// How the LLM should rewrite the sentence. The three styles compose
/// orthogonally: `Keystrokes` cares about the typist's fingers,
/// `ThingExplainer` cares about the reader's mental model (a nod to
/// Randall Munroe's *Thing Explainer* — explain with common words), and
/// `Both` layers both constraints. Selected per-session from the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RephraseStyle {
    /// Inject weak bigrams/trigrams while preserving meaning. Good for
    /// older students who already understand the material and want
    /// keyboard-level gains.
    Keystrokes,
    /// Rewrite using a restricted core vocabulary so the concept clicks.
    /// Useful for younger students, language learners, or anyone rereading
    /// something dense. Facts still locked (dates, names, numbers).
    ThingExplainer,
    /// Both — simpler words AND seeds the student's weak keys. The most
    /// demanding of the LLM but often the most pedagogically useful for
    /// Cermat-age students.
    Both,
}

impl Default for RephraseStyle {
    fn default() -> Self {
        RephraseStyle::Keystrokes
    }
}

#[derive(Debug, Clone)]
pub struct RephraseRequest<'a> {
    pub source: &'a str,
    pub weak_ngrams: &'a [String],
    pub language: &'a str, // "cs" | "sk" | "en" | "de" | ...
    pub style: RephraseStyle,
    pub similarity_floor: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RephraseOutcome {
    pub text: String,
    pub similarity: f32,
    pub generator_model: String,
    pub accepted: bool,
}

/// Run one rephrase round-trip. Returns `accepted = false` (with the raw
/// rewrite) when the similarity gate rejects it — the caller should fall
/// back to the verbatim source and optionally tell the user the LLM
/// drifted too far.
pub fn rephrase_sentence(
    req: &RephraseRequest<'_>,
    provider: &dyn EmbeddingProvider,
) -> Result<RephraseOutcome> {
    let sys = system_prompt(req.language, req.style);
    let user = user_prompt(req);

    let body = serde_json::json!({
        "model": DEFAULT_MODEL,
        "max_tokens": 512,
        "system": sys,
        "messages": [
            { "role": "user", "content": user }
        ],
    });

    // Prefer the student's Claude subscription (Pro/Max) if Claude Code
    // has cached OAuth credentials on this machine — Bearer auth with
    // the OAuth beta header routes usage through their plan, not our
    // bill. Fall back to BYOK API key, then explain clearly.
    let sub = claude_auth::detect();
    let auth_plan = match sub {
        Some(s) if !claude_auth::is_expired(&s) => AuthPlan::Subscription(s.access_token),
        Some(_) => {
            // Token present but expired — prefer BYOK fallback; otherwise
            // tell the student to re-login.
            match read_anthropic_key()? {
                Some(k) => AuthPlan::ApiKey(k),
                None => bail!(
                    "Tvůj Claude subscription token vypršel. \
                     Spusť `claude login` v terminálu a zkus to znovu."
                ),
            }
        }
        None => match read_anthropic_key()? {
            Some(k) => AuthPlan::ApiKey(k),
            None => bail!(
                "Rephrase mode potřebuje přihlášení — buď spusť \
                 `claude login` (doporučené, používá tvé předplatné), \
                 nebo vlož Anthropic API klíč v Nastavení."
            ),
        },
    };

    let mut request = ureq::post("https://api.anthropic.com/v1/messages")
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json");
    match &auth_plan {
        AuthPlan::Subscription(token) => {
            request = request
                .set("Authorization", &format!("Bearer {token}"))
                .set("anthropic-beta", "oauth-2025-04-20");
        }
        AuthPlan::ApiKey(key) => {
            request = request.set("x-api-key", key);
        }
    }
    let resp = request
        .send_json(body)
        .map_err(|e| anyhow!("anthropic: {e}"))?;
    let parsed: AnthropicResponse = resp.into_json()?;

    let candidate = parse_rewrite(&parsed)?;
    if candidate.trim().is_empty() {
        bail!("rephrase model returned empty string");
    }

    // Similarity gate — reject drifted rewrites.
    let vectors = provider.embed_batch(&[req.source.to_string(), candidate.clone()])?;
    let similarity = if vectors.len() == 2 {
        embeddings::cosine_similarity(&vectors[0], &vectors[1])
    } else {
        0.0
    };
    let floor = req.similarity_floor.unwrap_or(DEFAULT_SIMILARITY_FLOOR);
    let accepted = similarity >= floor;

    Ok(RephraseOutcome {
        text: candidate,
        similarity,
        generator_model: DEFAULT_MODEL.to_string(),
        accepted,
    })
}

fn system_prompt(language: &str, style: RephraseStyle) -> String {
    let style_block = match style {
        RephraseStyle::Keystrokes => {
            "STYLE — KEYSTROKES: Your secondary goal (after factual fidelity) \
             is to seed the listed weak bigrams/trigrams naturally — not at \
             the expense of grammar. Keep register and vocabulary at the \
             source's level."
        }
        RephraseStyle::ThingExplainer => {
            "STYLE — THING EXPLAINER: Your secondary goal is to make the \
             concept click. Rewrite with the ~1000 most common words of \
             the target language. Prefer short sentences. Avoid technical \
             jargon EXCEPT for the protected proper nouns and terms listed \
             below (those stay verbatim). Think Randall Munroe's *Thing \
             Explainer* applied to a textbook. If the source already uses \
             plain vocabulary, keep it — don't dumb down further."
        }
        RephraseStyle::Both => {
            "STYLE — BOTH: Rewrite with the ~1000 most common words of \
             the target language AND seed the listed weak bigrams/trigrams \
             where they fit naturally. Facts and protected terms stay \
             verbatim. Short, plain sentences; no jargon beyond what's \
             protected."
        }
    };

    format!(
        r#"You rephrase a study sentence for a Czech/Slovak high-school student who is touch-typing it.

Hard rules (ALL styles):
1. Preserve every factual claim. Never invent, add, or remove facts.
2. Preserve all proper nouns, titles, dates, numbers, units, and specialist terms VERBATIM (including diacritics). Examples: people's names, place names, battle/event names, laws, chemical formulas, mathematical symbols.
3. Target length within ±20 % of the source.
4. Output the rewrite in the same language as the source (detected language: {language}).
5. Output ONLY a JSON object of the form: {{"text": "<the rephrased sentence>"}}. No commentary, no markdown, no code fences.
6. If the sentence has too little slack to rewrite safely (e.g. a formula, a list of dates, a definition clause), emit the source verbatim inside the same JSON shape.

{style_block}
"#
    )
}

fn user_prompt(req: &RephraseRequest<'_>) -> String {
    let weak = if req.weak_ngrams.is_empty() {
        "(none — optimise for natural phrasing only)".to_string()
    } else {
        req.weak_ngrams.join(", ")
    };
    format!(
        "Source: {}\n\nWeak bigrams/trigrams to seed (use where natural): {}\n\nRewrite.",
        req.source, weak
    )
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicBlock>,
}

#[derive(Deserialize)]
struct AnthropicBlock {
    #[serde(rename = "type")]
    _ty: String,
    text: Option<String>,
}

fn parse_rewrite(resp: &AnthropicResponse) -> Result<String> {
    let raw = resp
        .content
        .iter()
        .filter_map(|b| b.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");
    if raw.trim().is_empty() {
        bail!("rephrase model returned no content blocks");
    }
    // Permissive JSON extraction — the model sometimes wraps JSON in code
    // fences despite instructions.
    let candidate = extract_json_object(&raw).unwrap_or(raw.clone());
    #[derive(Deserialize)]
    struct Out {
        text: String,
    }
    let parsed: Out = serde_json::from_str(candidate.trim()).map_err(|e| {
        anyhow!("rephrase model produced invalid JSON ({e}); raw: {raw}")
    })?;
    Ok(parsed.text)
}

fn extract_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0i32;
    for (i, c) in s[start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

// ---------- Copy-paste path (free-tier students) ----------
//
// Many students don't have a Claude subscription and won't paste a
// BYOK key — but they DO have free ChatGPT / Claude.ai / Gemini /
// Mistral chat. This path generates a deterministic prompt the
// student copies into their chat of choice; pastes the JSON answer
// back into Datlino; we parse, gate on similarity, and run the
// session with the rewrites.
//
// Constraints same as the API path: preserve facts / proper nouns
// verbatim, ±20 % length, JSON-only output. We're explicit about the
// shape so the student gets a useful answer regardless of which LLM
// they pick.

#[derive(Debug, Clone, Serialize)]
pub struct CopyPastePrompt {
    /// The full prompt as a single string the student can copy with
    /// one click. Already includes system instructions + per-sentence
    /// numbering.
    pub prompt: String,
    /// The number of source sentences embedded — the student's paste
    /// must include the same count.
    pub expected_count: usize,
}

pub fn format_copy_paste_prompt(
    sources: &[String],
    weak_ngrams: &[String],
    language: &str,
    style: RephraseStyle,
) -> CopyPastePrompt {
    let style_block = match style {
        RephraseStyle::Keystrokes => {
            "Cíl: do každé věty zapracuj víc kombinací z 'klíčů k procvičení'. \
             Stejnou úroveň slovníku, žádné zjednodušování."
        }
        RephraseStyle::ThingExplainer => {
            "Cíl: přepiš tak, aby věta používala jen ~1000 nejběžnějších slov \
             cílového jazyka. Vlastní jména, data, čísla a odborné termíny \
             zůstávají VERBATIM."
        }
        RephraseStyle::Both => {
            "Cíl: jednodušší slovník + zapracuj víc kombinací z 'klíčů k procvičení'. \
             Vlastní jména a fakta nikdy neměň."
        }
    };

    let mut sources_block = String::new();
    for (i, s) in sources.iter().enumerate() {
        sources_block.push_str(&format!("{}. {}\n", i + 1, s));
    }

    let weak_block = if weak_ngrams.is_empty() {
        "(žádné — piš přirozeně)".to_string()
    } else {
        weak_ngrams.join(", ")
    };

    let prompt = format!(
        "Jsi pomocník studenta, který se učí psát naslepo na svých vlastních materiálech.

Hard rules pro každou větu:
1. Zachovej KAŽDÉ faktické tvrzení, vlastní jméno, datum, číslo a odborný termín VERBATIM (včetně diakritiky).
2. Délka rewrite v rozmezí ±20 % oproti zdroji.
3. Cílový jazyk: {language}.
4. {style_block}

Vstup je očíslovaný seznam vět. Odpověz POUZE jedním JSON polem stejné délky:
[{{\"i\": 1, \"text\": \"<rewrite první věty>\"}}, {{\"i\": 2, \"text\": \"<rewrite druhé>\"}}, …]

Žádné komentáře, žádný text před ani za JSON. Pokud se některá věta přepsat nedá (formule, datovaný seznam), vrať ji verbatim.

Klíče k procvičení (zapracuj kde se hodí): {weak_block}

Věty k přepisu:
{sources_block}",
        language = language,
        style_block = style_block,
        weak_block = weak_block,
        sources_block = sources_block.trim_end(),
    );

    CopyPastePrompt {
        prompt,
        expected_count: sources.len(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CopyPasteResult {
    /// Per-source-index → rewrite text, where we managed to parse
    /// something. Indices not present here mean the LLM dropped that
    /// line; the caller falls back to the verbatim source.
    pub by_index: Vec<(usize, String)>,
    pub raw_count: usize,
    pub parse_warnings: Vec<String>,
}

/// Parse what the student pasted from their LLM. Tolerant of code
/// fences, leading prose, or trailing commentary — we extract the
/// first valid JSON array of `{i, text}` objects.
pub fn parse_copy_paste_result(raw: &str) -> Result<CopyPasteResult> {
    let json_slice = extract_json_array(raw)
        .ok_or_else(|| anyhow!("Nepodařilo se najít JSON pole v odpovědi. Zkontroluj, že LLM vrátil [{{\"i\":1,\"text\":\"…\"}}, …]."))?;

    #[derive(Deserialize)]
    struct Item {
        #[serde(default)]
        i: Option<usize>,
        #[serde(default)]
        index: Option<usize>,
        text: String,
    }
    let items: Vec<Item> = serde_json::from_str(&json_slice)
        .map_err(|e| anyhow!("JSON parse: {e}"))?;

    let mut warnings = Vec::new();
    let mut out = Vec::new();
    for (pos, it) in items.iter().enumerate() {
        let idx = it.i.or(it.index).unwrap_or(pos + 1);
        let text = it.text.trim();
        if text.is_empty() {
            warnings.push(format!("Položka {idx} má prázdný text — přeskočena."));
            continue;
        }
        // The student's source numbering started at 1; we store
        // 0-based for downstream zip with the source list.
        out.push((idx.saturating_sub(1), text.to_string()));
    }
    Ok(CopyPasteResult {
        raw_count: items.len(),
        by_index: out,
        parse_warnings: warnings,
    })
}

fn extract_json_array(s: &str) -> Option<String> {
    let start = s.find('[')?;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut prev_escape = false;
    for (i, c) in s[start..].char_indices() {
        if in_string {
            if !prev_escape && c == '"' {
                in_string = false;
            }
            prev_escape = c == '\\' && !prev_escape;
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Apply parsed rewrites to a list of source sentences, returning
/// per-sentence outcomes. The similarity gate runs against the active
/// embedding provider — same contract as the API path. Rejected
/// rewrites return the verbatim source instead.
pub fn apply_copy_paste(
    sources: &[String],
    parsed: &CopyPasteResult,
    provider: &dyn EmbeddingProvider,
    similarity_floor: Option<f32>,
) -> Result<Vec<RephraseOutcome>> {
    let floor = similarity_floor.unwrap_or(DEFAULT_SIMILARITY_FLOOR);
    let mut outcomes = Vec::with_capacity(sources.len());
    for (idx, src) in sources.iter().enumerate() {
        let rewrite = parsed
            .by_index
            .iter()
            .find(|(i, _)| *i == idx)
            .map(|(_, t)| t.clone());
        let Some(text) = rewrite else {
            outcomes.push(RephraseOutcome {
                text: src.clone(),
                similarity: 1.0,
                generator_model: "copy-paste:none".into(),
                accepted: false,
            });
            continue;
        };
        let vectors = provider.embed_batch(&[src.clone(), text.clone()])?;
        let sim = if vectors.len() == 2 {
            embeddings::cosine_similarity(&vectors[0], &vectors[1])
        } else {
            0.0
        };
        let accepted = sim >= floor;
        outcomes.push(RephraseOutcome {
            text: if accepted { text } else { src.clone() },
            similarity: sim,
            generator_model: "copy-paste:user-llm".into(),
            accepted,
        });
    }
    Ok(outcomes)
}

// ---------- Storage ----------

/// Persist an accepted rephrase in `rephrased_chunk` so the session can
/// reference it later (and so the attempt log distinguishes verbatim vs
/// rephrased).
pub fn store_rephrase(
    conn: &rusqlite::Connection,
    source_chunk_id: i64,
    outcome: &RephraseOutcome,
    weak_ngrams: &[String],
) -> Result<i64> {
    let target_json = serde_json::to_string(weak_ngrams)?;
    conn.execute(
        "INSERT INTO rephrased_chunk(source_chunk_id, text, target_ngrams, generator_model, similarity_to_source, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            source_chunk_id,
            &outcome.text,
            &target_json,
            &outcome.generator_model,
            outcome.similarity as f64,
            now_unix(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_object_finds_embedded_braces() {
        let raw = "Sure! Here is the rewrite:\n```json\n{\"text\": \"Ahoj světe.\"}\n```";
        let got = extract_json_object(raw).unwrap();
        assert_eq!(got, "{\"text\": \"Ahoj světe.\"}");
    }

    #[test]
    fn parse_rewrite_handles_code_fenced_json() {
        let resp = AnthropicResponse {
            content: vec![AnthropicBlock {
                _ty: "text".into(),
                text: Some("```json\n{\"text\":\"Karel IV. byl významný.\"}\n```".into()),
            }],
        };
        let got = parse_rewrite(&resp).unwrap();
        assert_eq!(got, "Karel IV. byl významný.");
    }

    #[test]
    fn parse_rewrite_errors_on_nonsense() {
        let resp = AnthropicResponse {
            content: vec![AnthropicBlock {
                _ty: "text".into(),
                text: Some("no json here, just prose".into()),
            }],
        };
        assert!(parse_rewrite(&resp).is_err());
    }

    #[test]
    fn system_prompt_injects_language_hint() {
        let p = system_prompt("cs", RephraseStyle::Keystrokes);
        assert!(p.contains("cs"));
        assert!(p.contains("proper nouns"));
        assert!(p.contains("±20"));
    }

    #[test]
    fn thing_explainer_style_asks_for_common_words() {
        let p = system_prompt("cs", RephraseStyle::ThingExplainer);
        assert!(p.contains("Thing Explainer") || p.contains("THING EXPLAINER"));
        assert!(p.contains("1000 most common words"));
        assert!(p.contains("Munroe"));
    }

    #[test]
    fn both_style_layers_vocab_and_keystroke_constraints() {
        let p = system_prompt("cs", RephraseStyle::Both);
        assert!(p.contains("BOTH"));
        assert!(p.contains("1000 most common words"));
        assert!(p.contains("weak bigrams"));
    }

    #[test]
    fn copy_paste_prompt_embeds_sentences_and_weak_keys() {
        let p = format_copy_paste_prompt(
            &["První věta.".to_string(), "Druhá věta.".to_string()],
            &["řč".to_string(), "ěš".to_string()],
            "cs",
            RephraseStyle::Keystrokes,
        );
        assert_eq!(p.expected_count, 2);
        assert!(p.prompt.contains("První věta."));
        assert!(p.prompt.contains("Druhá věta."));
        assert!(p.prompt.contains("řč"));
        assert!(p.prompt.contains("Cílový jazyk: cs"));
        assert!(p.prompt.contains("[{") || p.prompt.contains("[{\"i\""));
    }

    #[test]
    fn parse_copy_paste_handles_code_fences_and_extra_prose() {
        let raw = r#"Jasné, tady to je:

```json
[
  {"i": 1, "text": "Rewrite první."},
  {"i": 2, "text": "Rewrite druhé."}
]
```
Doufám, že se hodí!"#;
        let r = parse_copy_paste_result(raw).unwrap();
        assert_eq!(r.raw_count, 2);
        assert_eq!(r.by_index[0].0, 0);
        assert_eq!(r.by_index[0].1, "Rewrite první.");
        assert_eq!(r.by_index[1].0, 1);
    }

    #[test]
    fn parse_copy_paste_errors_when_no_json() {
        let r = parse_copy_paste_result("Promiň, nic nemám.");
        assert!(r.is_err());
    }

    #[test]
    fn apply_copy_paste_falls_back_to_source_on_low_similarity() {
        use crate::embeddings::FakeEmbedder;
        let sources = vec!["Habsburkové vládli v Čechách.".to_string()];
        // The "rewrite" is on a totally different topic — drift gate
        // should reject and we fall back to source.
        let parsed = CopyPasteResult {
            raw_count: 1,
            by_index: vec![(0, "Fotosyntéza je biologický proces.".into())],
            parse_warnings: vec![],
        };
        let provider = FakeEmbedder::new();
        let outcomes = apply_copy_paste(&sources, &parsed, &provider, Some(0.85)).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(!outcomes[0].accepted);
        assert_eq!(outcomes[0].text, sources[0]); // verbatim fallback
    }

    #[test]
    fn store_rephrase_writes_row_with_similarity() {
        use crate::db;
        let conn = db::open_in_memory().unwrap();
        // Need a source chunk for the FK.
        conn.execute(
            "INSERT INTO document(source_path, kind, ingested_at, checksum) VALUES ('x.md', 'md', 0, 'c')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO chunk(document_id, text, char_offset, context, created_at) VALUES (1, 'foo', 0, '', 0)",
            [],
        )
        .unwrap();
        let outcome = RephraseOutcome {
            text: "bar".into(),
            similarity: 0.9,
            generator_model: "test".into(),
            accepted: true,
        };
        let id = store_rephrase(&conn, 1, &outcome, &["ab".to_string()]).unwrap();
        assert!(id > 0);
        let (src_id, sim): (i64, f64) = conn
            .query_row(
                "SELECT source_chunk_id, similarity_to_source FROM rephrased_chunk WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(src_id, 1);
        assert!((sim - 0.9).abs() < 1e-6);
    }
}
