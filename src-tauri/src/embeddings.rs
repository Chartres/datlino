//! Embedding provider abstraction (brief §5.3).
//!
//! Pluggable backends for turning text into a fixed-size f32 vector:
//!
//! * `Cohere` — cloud, `embed-multilingual-v3.0`, 1024-dim. BYOK via the
//!   OS keychain. Best quality on CZ/SK.
//! * `Fake` — deterministic hash-based 256-dim vectors. Production-unwise
//!   but fine for tests and for the "no provider configured" fallback so
//!   the session path keeps working without network or a key.
//! * `Local` — Candle + multilingual-e5-small. Reserved; not compiled in
//!   yet (the model weights are ~120 MB). The provider enum carries the
//!   variant so the UI can show a disabled tile.
//!
//! Embeddings are stored little-endian f32 in the `chunk.embedding` BLOB
//! column and (when sqlite-vec is loaded) mirrored into `chunk_vec`. The
//! active provider's name and dimension are persisted on the user_profile
//! so we can detect a provider change and invalidate old embeddings.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingProviderKind {
    None,
    Fake,
    Cohere,
    Local,
}

impl EmbeddingProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmbeddingProviderKind::None => "none",
            EmbeddingProviderKind::Fake => "fake",
            EmbeddingProviderKind::Cohere => "cohere",
            EmbeddingProviderKind::Local => "local",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "fake" => EmbeddingProviderKind::Fake,
            "cohere" => EmbeddingProviderKind::Cohere,
            "local" => EmbeddingProviderKind::Local,
            _ => EmbeddingProviderKind::None,
        }
    }
}

pub trait EmbeddingProvider: Send + Sync {
    fn kind(&self) -> EmbeddingProviderKind;
    fn dim(&self) -> usize;
    /// Embed a batch. Returns one vector per input in the same order.
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

// ---------- Fake provider (deterministic, used by tests + offline) ----------

/// Hash-of-chars projected to 256 dims then L2-normalised. Gives stable
/// similarity shapes: identical strings land at distance 0, sentences
/// sharing many character bigrams are closer than unrelated ones.
pub struct FakeEmbedder {
    dim: usize,
}

impl FakeEmbedder {
    pub const DIM: usize = 256;
    pub fn new() -> Self {
        Self { dim: Self::DIM }
    }
}

impl EmbeddingProvider for FakeEmbedder {
    fn kind(&self) -> EmbeddingProviderKind {
        EmbeddingProviderKind::Fake
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| fake_embed(t, self.dim)).collect())
    }
}

fn fake_embed(text: &str, dim: usize) -> Vec<f32> {
    use unicode_normalization::UnicodeNormalization;
    let mut v = vec![0.0f32; dim];
    let lower: String = text.nfc().collect::<String>().to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    // Character bigrams → hashed into dims.
    for w in chars.windows(2) {
        let mut h: u64 = 0xcbf29ce484222325;
        for c in w {
            h ^= *c as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        let idx = (h as usize) % dim;
        v[idx] += 1.0;
    }
    // L2 normalise.
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    v
}

// ---------- Cohere cloud provider (BYOK) ----------

pub struct CohereEmbedder {
    api_key: String,
    model: String,
    dim: usize,
}

impl CohereEmbedder {
    /// `embed-multilingual-v3.0` is 1024-dim. Caller supplies a key —
    /// pulled from the OS keychain by the higher-level config code.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "embed-multilingual-v3.0".into(),
            dim: 1024,
        }
    }
}

#[derive(Serialize)]
struct CohereRequest<'a> {
    texts: &'a [String],
    model: &'a str,
    input_type: &'a str,
    embedding_types: &'a [&'a str],
}

#[derive(Deserialize)]
struct CohereResponse {
    embeddings: CohereEmbeddings,
}

#[derive(Deserialize)]
struct CohereEmbeddings {
    float: Option<Vec<Vec<f32>>>,
}

impl EmbeddingProvider for CohereEmbedder {
    fn kind(&self) -> EmbeddingProviderKind {
        EmbeddingProviderKind::Cohere
    }
    fn dim(&self) -> usize {
        self.dim
    }
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        // Cohere's batch cap is 96 inputs; chunk larger batches.
        let mut out = Vec::with_capacity(texts.len());
        for batch in texts.chunks(96) {
            let body = CohereRequest {
                texts: batch,
                model: &self.model,
                input_type: "search_document",
                embedding_types: &["float"],
            };
            let resp = ureq::post("https://api.cohere.com/v2/embed")
                .set("Authorization", &format!("Bearer {}", self.api_key))
                .set("Content-Type", "application/json")
                .send_json(serde_json::to_value(&body)?)
                .map_err(|e| anyhow!("cohere: {e}"))?;
            let parsed: CohereResponse = resp.into_json()?;
            let vectors = parsed
                .embeddings
                .float
                .ok_or_else(|| anyhow!("cohere: missing float embeddings in response"))?;
            if vectors.iter().any(|v| v.len() != self.dim) {
                bail!("cohere: unexpected embedding dimension");
            }
            out.extend(vectors);
        }
        Ok(out)
    }
}

// ---------- Candle local provider (feature = "candle") ----------

#[cfg(feature = "candle")]
pub mod candle_backend {
    //! Multilingual-e5-small via Candle + hf-hub. Cold path downloads
    //! ~120 MB of safetensors on first use into the HF cache dir; warm
    //! inference on CPU sits around 60-150 ms per sentence on modern
    //! hardware, sub-30 ms on Apple Silicon via the Metal backend.
    //!
    //! The e5 family requires "query: " / "passage: " prefixes at inference
    //! time — we use "passage: " for stored chunks and "query: " for the
    //! user's search text.
    use super::*;
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config as BertConfig, HiddenAct};
    use hf_hub::{api::sync::ApiBuilder, Repo, RepoType};
    use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams};

    const MODEL_ID: &str = "intfloat/multilingual-e5-small";
    const DIM: usize = 384;
    const MAX_LEN: usize = 512;

    pub struct CandleEmbedder {
        model: BertModel,
        tokenizer: Tokenizer,
        device: Device,
    }

    impl CandleEmbedder {
        pub fn dim_const() -> usize {
            DIM
        }

        /// Download (cached) + load the multilingual-e5-small weights. Uses
        /// the HuggingFace cache dir so repeated app launches are instant
        /// after the first download.
        pub fn load() -> Result<Self> {
            let device = pick_device();
            // Build an explicit ApiBuilder rather than Api::new() —
            // the latter silently builds with whatever HF_HOME was in
            // the shell, which on some Tauri-launched processes picks
            // up a stale value that breaks URL resolution with a
            // "RelativeUrlWithoutBase". Setting progress on doubles
            // as a sanity check that the builder succeeded.
            let api = ApiBuilder::new()
                .with_progress(true)
                .build()
                .map_err(|e| anyhow!("hf-hub builder: {e}"))?;
            let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));
            let config_path = repo.get("config.json").map_err(|e| anyhow!("config: {e}"))?;
            let tokenizer_path = repo
                .get("tokenizer.json")
                .map_err(|e| anyhow!("tokenizer: {e}"))?;
            let weights_path = repo
                .get("model.safetensors")
                .map_err(|e| anyhow!("weights: {e}"))?;

            let config: BertConfig = serde_json::from_slice(&std::fs::read(&config_path)?)
                .map_err(|e| anyhow!("config parse: {e}"))?;
            let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
                .map_err(|e| anyhow!("tokenizer load: {e}"))?;
            tokenizer
                .with_padding(Some(PaddingParams {
                    strategy: PaddingStrategy::BatchLongest,
                    ..Default::default()
                }))
                .with_truncation(Some(TruncationParams {
                    max_length: MAX_LEN,
                    ..Default::default()
                }))
                .map_err(|e| anyhow!("tokenizer config: {e}"))?;

            let vb = unsafe {
                VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)
                    .map_err(|e| anyhow!("safetensors: {e}"))?
            };
            let mut cfg = config;
            cfg.hidden_act = HiddenAct::Gelu;
            let model = BertModel::load(vb, &cfg).map_err(|e| anyhow!("bert load: {e}"))?;
            Ok(Self {
                model,
                tokenizer,
                device,
            })
        }
    }

    fn pick_device() -> Device {
        // Candle's Metal backend is compiled in on macOS when features
        // allow; default feature set uses CPU. This helper auto-picks
        // whatever's available at runtime.
        if candle_core::utils::cuda_is_available() {
            Device::new_cuda(0).unwrap_or(Device::Cpu)
        } else if candle_core::utils::metal_is_available() {
            Device::new_metal(0).unwrap_or(Device::Cpu)
        } else {
            Device::Cpu
        }
    }

    impl EmbeddingProvider for CandleEmbedder {
        fn kind(&self) -> EmbeddingProviderKind {
            EmbeddingProviderKind::Local
        }
        fn dim(&self) -> usize {
            DIM
        }
        fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            if texts.is_empty() {
                return Ok(Vec::new());
            }
            // e5 expects "passage: …" at indexing time.
            let prefixed: Vec<String> =
                texts.iter().map(|t| format!("passage: {}", t)).collect();
            let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
            let encodings = self
                .tokenizer
                .encode_batch(refs, true)
                .map_err(|e| anyhow!("tokenise: {e}"))?;

            let token_ids: Vec<Vec<u32>> = encodings.iter().map(|e| e.get_ids().to_vec()).collect();
            let attention_mask: Vec<Vec<u32>> = encodings
                .iter()
                .map(|e| e.get_attention_mask().to_vec())
                .collect();

            let n = token_ids.len();
            let seq = token_ids[0].len();
            let ids_flat: Vec<u32> = token_ids.into_iter().flatten().collect();
            let mask_flat: Vec<u32> = attention_mask.into_iter().flatten().collect();
            let ids = Tensor::from_vec(ids_flat, (n, seq), &self.device)
                .map_err(|e| anyhow!("tensor ids: {e}"))?;
            let mask = Tensor::from_vec(mask_flat, (n, seq), &self.device)
                .map_err(|e| anyhow!("tensor mask: {e}"))?;
            let token_type_ids = ids
                .zeros_like()
                .map_err(|e| anyhow!("token types: {e}"))?;

            let hidden = self
                .model
                .forward(&ids, &token_type_ids, Some(&mask))
                .map_err(|e| anyhow!("forward: {e}"))?;
            // Mean-pool with attention mask, then L2-normalise.
            let mask_f = mask
                .to_dtype(DType::F32)
                .map_err(|e| anyhow!("mask cast: {e}"))?
                .unsqueeze(2)
                .map_err(|e| anyhow!("mask dim: {e}"))?;
            let masked = (&hidden * &mask_f).map_err(|e| anyhow!("mul: {e}"))?;
            let sum = masked
                .sum(1)
                .map_err(|e| anyhow!("sum: {e}"))?;
            let mask_sum = mask_f
                .sum(1)
                .map_err(|e| anyhow!("mask sum: {e}"))?
                .clamp(1e-9, f32::INFINITY)
                .map_err(|e| anyhow!("clamp: {e}"))?;
            let mean = (sum / mask_sum).map_err(|e| anyhow!("div: {e}"))?;
            let norm = mean
                .sqr()
                .map_err(|e| anyhow!("sqr: {e}"))?
                .sum(1)
                .map_err(|e| anyhow!("norm sum: {e}"))?
                .sqrt()
                .map_err(|e| anyhow!("sqrt: {e}"))?
                .unsqueeze(1)
                .map_err(|e| anyhow!("norm dim: {e}"))?;
            let normed = (mean / norm).map_err(|e| anyhow!("norm div: {e}"))?;
            let vectors: Vec<Vec<f32>> = normed
                .to_vec2::<f32>()
                .map_err(|e| anyhow!("to_vec2: {e}"))?;
            Ok(vectors)
        }
    }

    /// Embed a user query with the required "query: " prefix — different
    /// from stored chunks which use "passage: ".
    pub fn embed_query(model: &CandleEmbedder, query: &str) -> Result<Vec<f32>> {
        let v = model.embed_batch(&[format!("query: {}", query)])?;
        v.into_iter()
            .next()
            .ok_or_else(|| anyhow!("candle returned no embedding"))
    }
}

#[cfg(not(feature = "candle"))]
pub mod candle_backend {
    //! Stub so the rest of the code compiles when the feature is off.
    use super::*;
    pub struct CandleEmbedder;
    impl CandleEmbedder {
        pub fn dim_const() -> usize {
            384
        }
        pub fn load() -> Result<Self> {
            bail!(
                "Local (Candle) embeddings not compiled in. \
                 Rebuild Datlino with: cargo build --features candle"
            )
        }
    }
    impl EmbeddingProvider for CandleEmbedder {
        fn kind(&self) -> EmbeddingProviderKind {
            EmbeddingProviderKind::Local
        }
        fn dim(&self) -> usize {
            384
        }
        fn embed_batch(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
            bail!("Local embeddings not compiled in")
        }
    }
}

// ---------- Config + factory ----------

const KEYCHAIN_SERVICE: &str = "org.datlino.app";
const KEYCHAIN_ENTRY_COHERE: &str = "cohere-api-key";

pub fn read_cohere_key() -> Result<Option<String>> {
    match keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY_COHERE) {
        Ok(entry) => match entry.get_password() {
            Ok(s) if !s.is_empty() => Ok(Some(s)),
            Ok(_) => Ok(None),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("keyring get: {e}")),
        },
        Err(e) => Err(anyhow!("keyring entry: {e}")),
    }
}

pub fn write_cohere_key(value: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY_COHERE)
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

/// Build the provider currently configured on the user_profile. Returns the
/// Fake provider when nothing is configured — good enough to exercise the
/// pipeline in tests and offline use.
pub fn build(kind: EmbeddingProviderKind) -> Result<Box<dyn EmbeddingProvider>> {
    match kind {
        EmbeddingProviderKind::None | EmbeddingProviderKind::Fake => Ok(Box::new(FakeEmbedder::new())),
        EmbeddingProviderKind::Cohere => {
            let key = read_cohere_key()?
                .ok_or_else(|| anyhow!("Cohere selected but no API key in keychain"))?;
            Ok(Box::new(CohereEmbedder::new(key)))
        }
        EmbeddingProviderKind::Local => Ok(Box::new(candle_backend::CandleEmbedder::load()?)),
    }
}

// ---------- Storage ----------

/// Ensure the `chunk_vec` virtual table exists at the given dim. If a
/// mismatched dim table is present (e.g. user switched providers), drop
/// and recreate. Also resets `chunk.embedding` BLOBs so we don't keep
/// vectors that no longer match the active provider.
pub fn ensure_vec_table_matches(
    conn: &rusqlite::Connection,
    provider: EmbeddingProviderKind,
    dim: usize,
) -> Result<()> {
    let (prev_provider, prev_dim): (String, i64) = conn
        .query_row(
            "SELECT provider, dim FROM embedding_meta WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((String::from("none"), 0));

    let dim_changed = (dim as i64) != prev_dim;
    let provider_changed = provider.as_str() != prev_provider;

    if dim_changed || provider_changed || prev_dim == 0 {
        // Drop the old virtual table if it existed.
        let _ = conn.execute("DROP TABLE IF EXISTS chunk_vec", []);
        // Clear stale vectors — they were at the old dim / from the old
        // provider and are no longer comparable.
        let _ = conn.execute("UPDATE chunk SET embedding = NULL", []);
        if matches!(provider, EmbeddingProviderKind::None) {
            // No vec table when no provider — saves disk, avoids confusion.
        } else {
            conn.execute(
                &format!(
                    "CREATE VIRTUAL TABLE chunk_vec USING vec0(embedding float[{}])",
                    dim
                ),
                [],
            )
            .map_err(|e| anyhow!("create chunk_vec[{dim}]: {e}"))?;
        }
        conn.execute(
            "UPDATE embedding_meta SET provider = ?1, dim = ?2 WHERE id = 1",
            rusqlite::params![provider.as_str(), dim as i64],
        )?;
    }
    Ok(())
}

/// Embed every chunk that doesn't have a stored vector yet, writing back
/// into `chunk.embedding` and `chunk_vec`. Safe to call repeatedly —
/// skips already-embedded chunks. Batches to keep API cost / latency sane.
pub fn reembed_missing(
    conn: &mut rusqlite::Connection,
    provider: &dyn EmbeddingProvider,
    batch_size: usize,
) -> Result<usize> {
    if matches!(provider.kind(), EmbeddingProviderKind::None) {
        return Ok(0);
    }
    let mut total = 0usize;
    loop {
        let rows: Vec<(i64, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, text FROM chunk WHERE embedding IS NULL LIMIT ?1",
            )?;
            let r = stmt.query_map(rusqlite::params![batch_size as i64], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut v = Vec::new();
            for row in r {
                v.push(row?);
            }
            v
        };
        if rows.is_empty() {
            break;
        }
        let texts: Vec<String> = rows.iter().map(|(_, t)| t.clone()).collect();
        let vectors = provider.embed_batch(&texts)?;
        let tx = conn.transaction()?;
        for ((id, _), vec) in rows.iter().zip(vectors.iter()) {
            let blob = vec_to_blob(vec);
            tx.execute(
                "UPDATE chunk SET embedding = ?1 WHERE id = ?2",
                rusqlite::params![&blob, id],
            )?;
            // Best-effort mirror into vec0 — may not exist when provider is
            // None. We check existence cheaply with a query that errors
            // loudly only on true schema problems.
            let _ = tx.execute(
                "INSERT OR REPLACE INTO chunk_vec(rowid, embedding) VALUES (?1, ?2)",
                rusqlite::params![id, &blob],
            );
        }
        tx.commit()?;
        total += rows.len();
        if rows.len() < batch_size {
            break;
        }
    }
    Ok(total)
}

// ---------- Vector utilities ----------

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// f32 vec ↔ little-endian BLOB. `sqlite-vec`'s `vec0` stores blobs in
/// this exact shape.
pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

pub fn blob_to_vec(b: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(b.len() / 4);
    for chunk in b.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_embedder_is_deterministic_and_normalised() {
        let e = FakeEmbedder::new();
        let a = e.embed_batch(&["Habsburkove vladli".into()]).unwrap();
        let b = e.embed_batch(&["Habsburkove vladli".into()]).unwrap();
        assert_eq!(a, b);
        let norm = a[0].iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }

    #[test]
    fn fake_embeddings_give_meaningful_similarity_shape() {
        let e = FakeEmbedder::new();
        let vecs = e
            .embed_batch(&[
                "Habsburkove vladli v Cechach.".into(),
                "Habsburkove vladli v Cechach dlouho.".into(),
                "Fotosynteza je biochemicky proces.".into(),
            ])
            .unwrap();
        let close = cosine_similarity(&vecs[0], &vecs[1]);
        let far = cosine_similarity(&vecs[0], &vecs[2]);
        assert!(close > far + 0.1, "close={close} far={far}");
    }

    #[test]
    fn blob_roundtrip_is_lossless() {
        let v: Vec<f32> = vec![0.1, -0.2, 0.3, 4.0, -999.0];
        let back = blob_to_vec(&vec_to_blob(&v));
        assert_eq!(v, back);
    }

    #[test]
    fn cosine_behaves() {
        assert!((cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-6);
        assert!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
    }
}
