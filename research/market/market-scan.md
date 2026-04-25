# Market scan — typing tutors, study apps, where Datlino fits

Source: research agent run 2026-04-24, with explicit "verify before
citing" flags on items the agent couldn't ground from training memory.
Treat this document as a starting brief, not a citation source.

## Players

### Typing tutors

| App | Core | Loved for | Criticised for | Pricing |
| --- | --- | --- | --- | --- |
| Monkeytype | Minimalist web speed-test, deep theming + leaderboards | Clean UX, customization, competitive culture | No curriculum, beginner-hostile | Free, OSS, donations |
| TypingClub / EduTyping | K-12 structured curriculum, teacher dashboards | Lesson progression, accessibility | Childish for teens, slow to adapt | Free + ~$3-6/student/yr school |
| Keybr | Adaptive weak-key drilling via generated pseudo-words | Closest public analog to Datlino's remix | Pseudo-words boring, no zero-from-scratch onboarding | Free + ~$5/mo |
| Typing.com | Broad K-12 curriculum + ads | Free, school-friendly | Ads, dated UI | Free / school licences |
| 10FastFingers | 1-min tests, multilingual incl. CZ/SK | Quick competition, language breadth | Shallow progression | Free + small premium |
| Typing Master | Windows desktop tutor | Office + older-school staple | Dated UX, paid upfront | ~$25 one-time |
| Ratatype | Web tutor + speed certificate | "Proof of speed" appeal for students | Shallow | Free + paid certification |

### Czech / Slovak specific (verify before citing)

- **ATF / Všech deset** — long-running CZ desktop tutor, often bundled
  in school licensing. Active state + pricing **not verified**.
- **Mistr klávesnice** (mistrklavesnice.cz) — CZ web tutor and speed
  tests. Feature set + monetisation **not verified**.
- **ZAV / Písař** — dominant method for state typing certification at
  Czech business academies. Institutional, paid per seat. Adjacent,
  not directly competitive with Datlino.

### Study apps (adjacent, not competing on typing)

- **Anki** — gold-standard SRS, ugly defaults, steep learning curve.
  Free desktop / $25 iOS one-time.
- **RemNote** — notes + cloze-heavy SRS. Heavy for teens. Freemium
  (~$8/mo).
- **Quizlet** — flashcards, broad student install base, monetisation
  pressure. ~$36/yr.
- **Notion / Obsidian** — where students keep notes; targets to ingest
  *from*, not compete with.

### Language learners

- **Duolingo** — gamification benchmark; criticised for shallow
  learning + dark patterns. Freemium / ~$7/mo Super.
- **Memrise** — video-immersion, pivoting to AI tutor.
- **Babbel** — structured adult courses, ~$14/mo.

## Retention drivers

What keeps users coming back daily, observed across the cohort above:

1. **Visible daily progress in <5 minutes.** Session length matters
   more than depth. Crown system / instant-results loops.
2. **Streaks with forgiveness mechanics.** Pure streaks rage-quit;
   Duolingo's freeze is cited as retention lift.
3. **Personal-best graphs.** WPM-over-time is the #1 share-screenshot
   in Monkeytype communities.
4. **Low-friction start.** Open → typing in <10 seconds.
5. **Personally-relevant content.** Quizlet shared decks, Anki own-
   decks. Generic drills bounce users.

## Bounce drivers

1. Childish / sterile UI for teenagers.
2. Paywall before value.
3. Progression feels rigged or can't be skipped.
4. Notification spam and dark patterns.
5. Sync / account friction (desktop-only without backup; account
   required for basics).

## Validated mechanics, who does each well

- **Streaks** — Duolingo (freezes are SOTA); Monkeytype proves
  competitive drive replaces them.
- **Spaced repetition** — Anki canonical, RemNote integrated, FSRS
  is current SOTA scheduler. Datlino's FSRS is a real edge if
  surfaced clearly.
- **Leaderboards** — Duolingo Leagues (engaging + toxic);
  class-level outperforms global for teens.
- **WPM graphs / stats** — Monkeytype + Keybr execute well,
  TypingClub buries them.
- **Lesson trees** — Duolingo's 2022 path simplification boosted
  completion; TypingClub's linear progression fatigues.
- **Generative AI content** — Duolingo Max, Quizlet Q-Chat,
  Khanmigo. Quality is uneven. Datlino routing through the
  student's own Claude key is novel.
- **Pre-session calibration / metacognition** — research-backed
  (Dunlosky, Bjork) but almost no consumer app uses it. A
  differentiator that needs teaching to land.

## What kills these products

- Content goes stale (Typing Master).
- Web migration leaves desktop behind (Mavis Beacon lineage).
- School channel captures product, consumer is ignored
  (TypingClub's teen problem).
- Pedagogy without polish (Anki's adoption ceiling).
- Polish without pedagogy (Monkeytype wins on speed only).
- Single-vendor model dependency (apps that baked in GPT-3
  pre-pricing-shock).
- Dark-pattern retention (Duolingo is burning brand equity here).
- Losing to "good enough" free (Monkeytype + Keybr exist for free).

## CZ / SK market specifics

- **Student TAM** — Czech upper-secondary ≈ 420-450k; Slovak
  ≈ 210-230k (verify with current ČSÚ / ŠÚ SR). Ages 13-19
  combined plausibly 1.1-1.3M.
- **Cermat / maturita** is a distinct, high-stakes prep market.
  Players: Maturita.cz, Pepa.cz, Priprav.me. **No typing-based
  exam-prep tool found** — gap.
- **Willingness to pay** — parent-paid tutoring is normalised;
  price tolerance for exam prep meaningfully higher than for
  general apps. Student-paid is low + piracy-tolerant.
- **Distribution** — MŠMT and MŠVVaŠ don't mandate a typing tool.
  Business academies use ZAV for state cert; that's a separate
  regulated niche.

## Pricing patterns

- One-time desktop: ~$25 (Typing Master). Fading.
- Freemium web: free → ~$5/mo (Keybr).
- Consumer subscription: $7-14/mo, $84-150/yr.
- School licensing: $3-6/seat/yr, bulk discounts at 50+.
- CZ exam prep: 299-999 CZK one-time or seasonal. Parents buy.

A realistic shape for Datlino: free core + ~149-249 CZK / mo or
~990-1490 CZK / yr student tier; family ~1.3x; school
~80-150 CZK / seat / year once traction is real. BYO-Claude-key
keeps AI cost off P&L.

## What we can't verify

- Current ATF / Mistr klávesnice / ZAV product state and pricing.
- Exact CZ / SK secondary enrolment numbers for 2025-26.
- Current Cermat / maturita app market shares.
- Whether any stealth competitor has shipped AI-typing-study
  fusion in the last ~6 months. **Search before any deck claim.**
