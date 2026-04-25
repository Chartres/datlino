# Datlino — Changelog

Human-curated user-facing changes. Technical refactors live in `git log`;
this file captures what a student or parent would notice.

Dates use the commit day. The `[ref]` handles point at backlog items in
`research/plan/BACKLOG.md` for full context.

## Unreleased

**Persona forum, free-tier copy-paste remix, dictation, human-readable
lesson targets.** `[RES-005 PE-014 PE-004 UX-001]`

- **Copy-paste remix (free tier).** Students who don't have a Claude
  subscription or an Anthropic API key now get a proper second-class
  path for AI rephrase: Datlino assembles a deterministic prompt with
  the picked sentences + the student's weak n-grams, the student
  pastes it into ChatGPT / Claude.ai / Gemini (any free chat), pastes
  the JSON back, and Datlino runs the same cosine-similarity gate
  (≥ 0.75) the API path uses. Remix on `/study` now offers three
  modes — Claude subscription, copy-paste (free), BYOK API key —
  with the cascade auto-picking the highest available. Settings page
  adds the matching second-tier card.
- **Dictation mode (PE-004).** New checkbox on `/study` Pokročilé:
  "Diktát — Datlino věty čte nahlas a automaticky pauzuje, když
  nestíháš." Uses the browser SpeechSynthesis API with `cs-CZ` voice
  when available; the practice page tracks the TTS character cursor
  via `onboundary` events and pauses playback when the student falls
  ≥ 8 chars behind, resumes when they catch up to within 3.
  Per-sentence rate slider (0.6–1.2×) and replay button on the
  in-session bar; mute toggle for kids in shared spaces.
- **UX-001: human-readable lesson targets.** Each intro lesson on
  `/learn` now shows a sentence like *"Zvládnul jsi to, když napíšeš
  9 z 10 vět skoro bez chyby a píšeš v klidu, ne na rychlost."* in
  italic, with the raw WPM/accuracy numbers de-emphasised below as
  *Číselný cíl:* — for kids who don't yet know what 25 WPM means.
- **Persona forum.** `research/forum/session-001.md` — five enriched
  personas argue across four rounds (what to build next, mode wars,
  killer feature for parents, biggest product risk) with a synthesis
  section. Used as input for prioritising shipped features.

**Top-4 voted features: ingest progress, exam ramps, installers, iPad
scaffold + market positioning.** `[ING-009 PE-012 DIS-001 PLT-001 RES-004]`

- **Ingest progress events.** Adding a folder no longer looks like a
  freeze on large libraries — Lucie's 200-doc thesis case. Rust
  emits `ingest-progress` events per file; `/study` shows a live
  progress bar with file count, chunk count, and the current path.
- **Cermat / maturita exam ramps.** Five curated content packs ship
  in the box: Cermat ČJL, Cermat M, Maturita ČJL, Maturita English,
  Maturita Math. Each ramp has 2-4 lessons of CC-BY-licensable
  paraphrased passages with citations. Pick a single lesson or
  drill the whole ramp from `/study`.
- **DIS-001 installer pipeline.** GitHub Actions workflow at
  `.github/workflows/release.yml` builds per-platform artefacts
  (macOS arm + Intel `.dmg`, Linux `.AppImage` + `.deb`, Windows
  `.msi` + `.exe`). Signing + notarization slots in once you wire
  the Apple + Windows code-signing secrets — full guide at
  `docs/SIGNING.md`. Builds run unsigned out of the box for
  internal dogfooding.
- **PLT-001 iPad scaffold.** `docs/IPAD.md` documents the
  `tauri ios init` path, what works today (Rust core, SvelteKit
  bundle, schema), what needs adapting (touch-friendly spacing,
  GoodNotes Share Sheet), and what's gated (Candle iOS path,
  Apple Vision OCR replacement for Tesseract). Real Xcode session
  on a Mac is the next-physical-thing-needed.
- **Market scan + positioning memo.** `research/market/market-scan.md`
  + `research/market/positioning.md`. Where Datlino fits between
  typing tutors and study apps; what kills these products; CZ/SK
  pricing and TAM; concrete risks of the overlap position.

**Pedagogy depth: FSRS + Cloze + Calibration + Metacognition.** `[PE-002 PE-003 PE-005 PE-006]`

- **FSRS** (Free Spaced Repetition Scheduler) now runs on your content
  chunks. After every session, each typed sentence is graded 1–4 based
  on accuracy + WPM vs target, and the FSRS-4.5 update rule computes
  a new stability, difficulty, and due date for that chunk. Napříč
  materiály now surfaces *overdue* chunks first, novel chunks
  afterwards — so repeat-review material lands when it's genuinely
  time to review, not when random chance picks it.
- **Cloze drill** — new tile on **Učím se psát**. One high-information
  word per sentence is blanked in the display (you see `________`);
  you type the full sentence from context. Retrieval practice +
  keystrokes in one move. Uses the same content-across scoring so
  it respects FSRS due dates too.
- **Pre-session calibration** — before any session starts, a small
  modal asks "kolik % znaků napíšeš správně?" with a slider. Your
  prediction is stored; the post-session reflection pins it next to
  the actual accuracy so over time you see if you over- or
  under-estimate.
- **Post-session metacognition** — the summary page now asks
  difficulty 1–5 and a free-text note ("co ti šlo, co procvičíš
  příště?"). Optional. Stored per-session so the Pokrok page can
  eventually plot a difficulty-vs-WPM curve.

**Seven bug-fixes from hands-on testing.** `[UX-002 UX-009 KB-002 BR-001]`

- Settings disclosure arrows are now proper ▾ chevrons at the right
  edge that rotate 180° on open, not tiny ▸ in a confusing column.
- Remix toggle on `/study` now unlocks when a Claude subscription OR
  a BYOK key is present — previously the subscription wasn't
  consulted.
- Candle provider switch uses an explicit `ApiBuilder` so the
  "RelativeUrlWithoutBase" hf-hub error under Tauri-launched
  processes is avoided.
- Logo redrawn closer to the Scholarly Peck concept — classical
  silhouette, flowing red crest, chisel beak, keyboard-grid wing.
- Dead-key filter extended for Slovak layouts: ˝ ¸ ˙ ¯ plus full
  Unicode combining-mark ranges plus AltGraph + `Process` +
  `Compose` IME sentinels. `ů` and `ř` on SK layout now register
  as one correct keystroke.
- Subscript / superscript / dash / smart-quote normalisation on the
  typing display — so H₂O becomes typeable as H2O, em-dashes as
  regular dashes, and smart quotes as straight. Raw text stays
  intact in the DB.
- Rephrase model upgraded: Haiku → Sonnet 4.6.

**Settings regroup + Sign in with Claude.** `[IA-007 AI-001 ING-008]`

- **Settings** is now four collapsible sections — **Profil** (your
  stats and baseline), **Kvalita vyhledávání** (embedding provider
  + Cohere key), **Remix** (AI přepis), and **OCR**. Only Profil is
  open by default. Eliška never sees the infrastructure unless she
  opens it; Martin can expand everything in four clicks.
- **Sign in with Claude.** Datlino now detects Claude Code's OAuth
  credentials on your machine (from `~/.claude/.credentials.json` or
  the OS keychain) and uses them as Bearer auth against the Anthropic
  Messages API — Remix runs on your **own Pro / Max subscription
  quota**, not a separate BYOK bill. If the token is present and
  valid, an **Aktivní** pill shows under Remix; if expired, we nudge
  you to run `claude login` again. BYOK API key remains as a
  fallback for users without Claude Code.
- **OCR re-check.** After installing `tesseract` + `pdftoppm`, a
  **Zkontrolovat znovu** button refreshes the status without
  restarting the app.

## 2026-04-24

**IA reorg: two doors instead of six modes.** `[IA-001 IA-002 IA-003 IA-004 IA-005 IA-006]`

- Home screen is now a **"Co dnes?"** dashboard with a contextual
  greeting and two big doors: **Učím se psát** (keystrokes path) and
  **Učím se obsah** (content path).
- **Učím se psát** combines the 16-lesson intro curriculum, a
  **Tvá slabá místa** drill that targets your own weak key
  combinations, and a **Diakritika** drill for č/š/ř/ě/ů/ý/á/í.
  One-click "Pokračovat" on the next unpassed lesson.
- **Učím se obsah** holds the three content strategies (Napříč
  materiály / Celá kapitola / Příprava na zkoušku), your document
  list with a filter, and the empty-state invitation that mentions
  Markdown, PDF, and GoodNotes exports.
- The α-mix slider and LLM Remix toggle moved behind a **Pokročilé**
  expander so beginners never meet them by accident.

**Brand artwork.** `[BR-001]`

- The **Scholarly Peck** woodpecker logo lands in the header next to
  the wordmark.
- The **Integrated Icon** becomes the app icon + favicon. Deep-red
  field with an open book and a peeking woodpecker.

## 2026-04-23

**Dead-key diacritics fixed for all layouts.** `[KB-002]`

- `ř`, `č`, `š`, `ž`, `ě`, `ů` now register as ONE correct keystroke
  regardless of the keyboard layout (CZ-QWERTZ direct keys, CZ-QWERTY
  dead-keys, macOS Czech ABC, Linux IBus). No more phantom wrong
  characters.

**16-lesson intro curriculum.** `[KB-001 PE-001]`

- From home row (ASDF / JKL;) through the whole keyboard to háčky,
  čárky, kroužek, shift, numbers, and punctuation. Sixteen lessons,
  each with a speed/accuracy target that unlocks the next lesson.
- On-screen keyboard shows finger-zone colors, home-row dots on F/J
  A/;, and glows on the next key to press. Toggle-hide when you're
  ready to fly solo.

**Learning-zone weak-ngram targeting.** `[PE-001]`

- Remix mode (when enabled) rewrites sentences to seed the keys at
  the *edge of your fluency* — not the broken ones (those stay in
  the isolated drill). Zone-of-proximal-development signal: 5-30 %
  error rate, median-to-p90 latency.

## 2026-04-22

**Content strategies: three ways to study your materials.** `[IA-003 IA-004]`

- **Napříč materiály** — sentences from anywhere in your library
  that mention the topic. Connect ideas across sources.
- **Celá kapitola** — every sentence of one Markdown section, in
  order. End-to-end reading.
- **Příprava na zkoušku** — describe the exam topic; Datlino
  surfaces the most relevant chapters.

**Document picker.** `[ING-003 ING-004 ING-005]`

- Every ingested file is a card in the library. Click "Trénovat celý"
  and type the whole document in source order — no BM25 detour.
- Filename-aware search: searching "chemie" now finds the chemistry
  file even if the word never appears in the body.
- "Přidat jeden soubor" alongside "Přidat složku" for quick imports.

## 2026-04-21

**Week 2 foundation: embeddings + PDF + OCR + Rephrase.** `[AI-003 ING-002 ING-006]`

- **Local Candle embeddings** (multilingual-e5-small, 384-dim) ship
  on by default. No key or account — embeddings happen on your
  machine.
- **Cohere embed-multilingual-v3** as a cloud alternative. BYOK into
  the OS keychain.
- **PDF ingestion** via text-layer extraction, with dehyphenation
  and paragraph preservation.
- **OCR dispatch** (`tesseract` + `pdftoppm`) for image-only PDFs
  and GoodNotes exports. Detected heuristically when the text layer
  is too thin.
- **LLM Remix** (opt-in, per-session): Claude Haiku rewrites each
  sentence to inject your weak-key combinations while preserving
  facts and proper nouns. Cosine similarity gate rejects drift.
- **Cosine-similarity hybrid scorer**: when a provider is configured
  and chunks are embedded, `α · relevance + (1-α) · pedagogy` uses
  real embeddings for the relevance term.

## 2026-04-20

**Week 1: library, keyboard, typing engine.** `[KB-001 KB-002 ING-001]`

- Folder ingest walks subdirectories; `.md` / `.markdown` / `.txt`
  covered. SHA-256 checksums prevent re-work.
- SQLite schema + FTS5 for keyword search.
- Czech/Slovak-aware sentence segmentation (handles `např.`, `tzv.`,
  Unicode bullets, Roman numerals on kings like `Karel IV.`).
- Typing engine with per-character correctness, live WPM/accuracy,
  keystroke log, red woodpecker caret.
- Gamification: XP, levels, streaks, 10 badges.
- Progress page: WPM sparkline, weak-key bars, session history.
