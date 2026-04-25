# Datlino — Positioning memo

A one-page distillation of the market scan.

## The two-sentence pitch

**Datlino is a touch-typing trainer that drills students on their own
study materials.** Type your dějepis notes, your maturita literature
chapter, your English grammar PDF — and your fingers learn the
material while they learn the keys.

## The market gap we own

Three things sit on different shelves in the AppStore today:

1. **Typing tutors** (Monkeytype, Keybr, TypingClub) — pure motor
   training on generic text. Boring once you're past beginner.
2. **Study apps** (Anki, RemNote, Quizlet) — retention without
   physical practice. Reading is passive.
3. **Language learners** (Duolingo, Memrise) — fixed curriculum, no
   integration with your real materials.

**No shipped product genuinely unifies them.** The closest analogues:
Keybr (adaptive keys, no content) and Anki's "type the answer" mode
(study, no typing technique).

Datlino plants its flag in the overlap. The student types **real
sentences from their own corpus** while:

- FSRS schedules due chunks for re-review
- Cloze drills force retrieval, not recognition
- Calibration + metacognition build self-knowledge of learning
- The on-screen keyboard teaches finger technique in parallel

That four-way fusion — content + spacing + retrieval + technique — is
the niche. We don't try to out-Monkeytype Monkeytype or out-Anki Anki.

## Why CZ / SK first

- **Diacritic-correct typing technique** — global tutors fake or skip
  CZ/SK keyboard layouts (dead keys, háčky, čárky, kroužek). Datlino
  handles all four common layouts (CZ-QWERTZ, CZ-QWERTY, SK-QWERTZ,
  macOS Czech ABC) on day one.
- **Cermat / maturita prep is a real, monetisable niche** with no
  typing-based player. Curated exam ramps (PE-012) ship in the box.
- **Czech-aware sentence segmentation** that handles `např.`, `tzv.`,
  `atd.`, monarch roman numerals, and CZ-specific abbreviations the
  English NLP pipeline mauls.
- **Parent buys, student uses** is the established pattern (Doučování,
  private prep). Datlino fits that purchase shape natively.

## What we explicitly are NOT

- Not a competitor to Monkeytype on raw speed. Their crowd doesn't
  want a study layer.
- Not a flashcard app. We don't do recall-only on detached cards.
- Not a Duolingo-shaped curriculum company. Our content is yours,
  not ours.
- Not a school-LMS plug-in (yet). Classroom features are PE-013 +
  DIS-003, not v1.

## Three risks of the overlap position

1. **Positioning confusion.** "Typing app" buyers want WPM numbers;
   "study app" buyers want grades. Pick the primary story per
   channel — Czech parents = exam outcome; international = study
   accelerator; tech crowd = pedagogy story.
2. **Feature sprawl.** Each half tempts expansion (multiplayer
   typing vs. deck sharing). Ruthless prioritisation; the persona-
   vote sheet is the gate.
3. **Speed-test benchmark.** Students will compare WPM with
   Monkeytype. We need to *at least* feel as good in pure-speed
   mode (the typing engine already does — keep it that way).

## What the market rewards (we're already doing or shipping next)

- Visible daily progress in <5 minutes — IntroLesson 3-min default,
  /learn one-click resume.
- Forgiving streaks — already in (broken streaks shrug, no guilt).
- Personal-best graphs — /progress sparkline.
- Low-friction start — /learn → "Pokračovat: Horní řada levá" →
  typing.
- Personally-relevant content — the entire `/study` flow.
- BYO-AI cost shield — Claude subscription auth (AI-001).

## What kills these products (and our hedge)

- Stale content → user-supplied corpus structurally avoids this.
- Web-only migration killed desktop tutors → we're Tauri (single
  binary), with PLT-001 iPad path and an eventual web-companion
  for sharing not yet committed.
- School-channel UX poisoning → keep the teen-facing tone primary.
- Single-vendor AI lock-in → Candle local + Cohere + Claude-sub
  multi-provider story is the explicit hedge.
- Dark patterns → don't copy Duolingo's notification tactics, do
  copy the streak freeze.

## Open questions (decide before pitching)

1. **Primary channel for first paid users.** Parents (CZ/SK exam
   prep), or English-speaking tech-pedagogy crowd?
2. **Pricing anchor.** 199 CZK/mo or 999 CZK/year all-in is
   plausible; aligning with parental tutoring price points argues
   for the higher end.
3. **Free-tier shape.** Free everything except Remix? Free for one
   active subject? Time-boxed trial? Pick before launch.
4. **Open-source position.** Tauri + Rust makes OSS easy, and the
   research-grounded pedagogy story benefits from auditability.
   Trade-off: makes the school B2B sale slightly harder ("we use
   the same code as everyone else"). Not a v1 question.
