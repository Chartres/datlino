<!--
  LIVING BACKLOG — Datlino

  This document is the source of truth for what might be built, what's
  actively being built, and what shipped. Updated every push.

  Vote notation: each persona scores each item on
    +3  "I'd become a daily user because of this"
    +2  "this would make me happy"
    +1  "nice to have"
     0  "indifferent"
    -1  "actively don't want this"

  Votes are advisory. Final product decisions stay with Paja Dravec
  (product). Persona votes are produced from each enriched persona's
  described context, goals, frustrations, and lived constraints —
  re-scored whenever a persona's context changes.

  Columns:
    id           stable handle for cross-reference with the changelog
    item         what we'd build
    status       backlog | planned | shipping | shipped | dropped
    shipped_in   commit SHA or version tag when status = shipped
    votes        persona scores — sum (avg) [per-persona detail below]
-->

# Datlino — Living Backlog

## Status index

| id | item | status | shipped_in |
| -- | ---- | ------ | ---------- |
| IA-001 | Two-door IA (`/learn` + `/study`) replacing six-tile picker | shipped | e6d4931 |
| IA-002 | Home "Co dnes?" dashboard | shipped | e6d4931 |
| IA-003 | WeakKeys + Diacritics as drill tiles on /learn | shipped | e6d4931 |
| IA-004 | α-slider buried under "Pokročilé" on /study | shipped | e6d4931 |
| IA-005 | Empty-library invitation card | shipped | e6d4931 |
| IA-006 | Document filter when >6 docs | shipped | e6d4931 |
| IA-007 | Settings regrouped into 4 collapsible sections (Profil / Kvalita / Remix / OCR) | shipped | 1244b02 |
| BR-001 | Brand artwork (Scholarly Peck logo + Integrated Icon) | shipped | 870a9fb |
| BR-002 | Full visual system derived from approved palette | backlog | — |
| BR-003 | Animated mascot (peck tempo tracks WPM, tilt on miss) | backlog | — |
| BR-004 | First-run "meet Datlino" onboarding moment | backlog | — |
| KB-001 | On-screen keyboard with finger-zone colors | shipped | b0fd48f |
| KB-002 | Dead-key diacritic handling (ˇ ´ ˘ ¨ ˜ ` ^ ˚) | shipped | 000c1ad |
| KB-003 | Two-key lookahead glow (not just the next one) | planned | — |
| KB-004 | Animated hand silhouette showing finger stretch | backlog | — |
| KB-005 | Shift-key introduction in early lesson (trigger .shift-glow) | backlog | — |
| PE-001 | Weak-ngram learning-zone signal (not just the broken ones) | shipped | 000c1ad |
| PE-002 | FSRS scheduler on content chunks | shipped | b2e4332 |
| PE-003 | Cloze drills (delete a word, type to fill) | shipped | b2e4332 |
| PE-004 | Dictation mode (browser TTS reads; student types) | planned | — |
| PE-005 | Pre-session calibration ("kolik % napíšeš správně?") | shipped | b2e4332 |
| PE-006 | Post-session metacognitive summary ("co se ti lepší, co horší, co dál") | shipped | b2e4332 |
| PE-007 | Desirable-difficulty toggles (hide keyboard, reduce lookahead, second-pass memory) | planned | — |
| PE-008 | Interleaving nudge when student blocks on one subject | backlog | — |
| PE-009 | Pomodoro break suggestion mid-session | backlog | — |
| PE-010 | Sleep reminder for late-night cramming (>23:00) | backlog | — |
| PE-011 | Forest progress visualization (mastered chunks grow branches) | backlog | — |
| PE-012 | Exam-mode curated ramps for Cermat / maturita | shipped | 5318b34 |
| PE-013 | Auto-regression alerts ("tenhle bigram se ti zhoršuje 2 týdny") | backlog | — |
| PE-014 | "Explain back" drill — student types one-sentence summary post-session | backlog | — |
| AI-001 | Claude subscription auth (use Pro/Max quota via Claude Code creds) | shipped | 1244b02 |
| AI-002 | Full OAuth "Sign in with Claude" (Datlino as Anthropic OAuth client) | backlog | — |
| AI-003 | Local Candle embeddings (multilingual-e5-small) | shipped | b0fd48f |
| AI-004 | Candle download progress UI on provider switch | planned | — |
| AI-005 | Background re-embedding (non-blocking) | backlog | — |
| AI-006 | Hybrid BM25 + cosine re-rank via RRF | backlog | — |
| AI-007 | Larger e5 model (multilingual-e5-large, 400 MB) option | backlog | — |
| ING-001 | Recursive folder ingest | shipped | e1bc951 |
| ING-002 | PDF text-layer ingestion | shipped | 7a8bc49 |
| ING-003 | File picker for single file | shipped | b0fd48f |
| ING-004 | Document picker bypasses BM25, drills whole file | shipped | b0fd48f |
| ING-005 | Filename-aware search | shipped | b0fd48f |
| ING-006 | Tesseract + pdftoppm OCR dispatch | shipped | 7025cc7 |
| ING-007 | Apple Vision OCR path on macOS (native) | backlog | — |
| ING-008 | "Zkontrolovat znovu" OCR button after user installs deps | shipped | 1244b02 |
| ING-009 | Ingest progress events (files, chunks, embeddings) streamed to UI | shipped | 5318b34 |
| UX-001 | Lesson target phrased in human-readable language, not raw WPM | planned | — |
| UX-002 | Candle silent-switch fixed (progress or no-op message) | planned | — |
| UX-003 | Colorblind-safe finger palette toggle | backlog | — |
| UX-004 | Reduce-motion preference respected | backlog | — |
| UX-005 | Visible-space-marker toggle (some students prefer no `␣`) | backlog | — |
| UX-006 | Audio cue on error (optional) | backlog | — |
| UX-007 | Offline / online state banner in Nastavení | backlog | — |
| UX-008 | "What's new" modal on first launch after update | backlog | — |
| DIS-001 | Notarized `.dmg` + signed `.msi` + `.AppImage` bundles | shipping | 5318b34 (CI scaffolded; signing certs pending) |
| DIS-002 | Tauri auto-updater via GitHub Releases | backlog | — |
| DIS-003 | Classroom / parent-lite aggregate progress view | backlog | — |
| DIS-004 | Licensing / pricing scaffolding | backlog | — |
| PLT-001 | iPad version (GoodNotes lives there) | shipping | 5318b34 (scaffold + docs; needs Xcode session on Mac) |
| PLT-002 | Touch-keyboard handling | backlog | — |
| PLT-003 | Larger-type "junior" mode | backlog | — |
| RES-001 | Persona vote mechanism (advisory) | shipped | this commit |
| RES-002 | CHANGELOG.md + /about in-app renderer | shipped | this commit |
| RES-003 | Living backlog kept in sync with every push | shipped | this commit |
| RES-004 | Market scan + positioning memo | shipped | 5318b34 |

## Persona votes

Each persona scores each planned/backlog item. The sum column is
advisory only — product decisions stay with you.

Columns: **E**liška (14, beginner) · **F**ilip (13, beginner) ·
**P**ája (17, maturita) · **M**artin (18, tech) · **T**ereza (16,
GoodNotes) · **J**onáš (15, offline) · **L**ucie (19, heavy corpus).

| id | E | F | P | M | T | J | L | Σ | avg |
| -- | - | - | - | - | - | - | - | - | --- |
| IA-007 Settings 4-section regroup | +3 | +2 | +2 | +1 | +2 | +1 | +1 | 12 | 1.71 |
| BR-002 Full visual system | +1 | +2 | +1 | 0 | +2 | 0 | 0 | 6 | 0.86 |
| BR-003 Animated mascot | +3 | +3 | 0 | -1 | +1 | 0 | 0 | 6 | 0.86 |
| BR-004 First-run onboarding | +3 | +2 | +1 | 0 | +1 | 0 | 0 | 7 | 1.00 |
| KB-003 Two-key lookahead | +1 | +3 | 0 | 0 | 0 | 0 | 0 | 4 | 0.57 |
| KB-004 Animated hand | +1 | +3 | 0 | -1 | 0 | 0 | 0 | 3 | 0.43 |
| KB-005 Shift intro in early lesson | +2 | +3 | 0 | 0 | 0 | 0 | 0 | 5 | 0.71 |
| PE-002 FSRS scheduler | 0 | 0 | +3 | +2 | +1 | +2 | +3 | 11 | 1.57 |
| PE-003 Cloze drills | 0 | 0 | +3 | +1 | +2 | +1 | +3 | 10 | 1.43 |
| PE-004 Dictation mode | 0 | 0 | +2 | +2 | +1 | +2 | +2 | 9 | 1.29 |
| PE-005 Pre-session calibration | 0 | 0 | +2 | +2 | +1 | +1 | +2 | 8 | 1.14 |
| PE-006 Post-session metacognition | +1 | 0 | +3 | +1 | +1 | +1 | +2 | 9 | 1.29 |
| PE-007 Desirable-difficulty toggles | -1 | -1 | +1 | +3 | +1 | +1 | +2 | 6 | 0.86 |
| PE-008 Interleaving nudge | 0 | 0 | +2 | +1 | +1 | 0 | +3 | 7 | 1.00 |
| PE-009 Pomodoro | 0 | +1 | +1 | 0 | +1 | +2 | +2 | 7 | 1.00 |
| PE-010 Sleep reminder | 0 | 0 | +1 | -1 | +1 | 0 | +1 | 2 | 0.29 |
| PE-011 Forest visualization | +3 | +3 | +1 | -1 | +2 | 0 | 0 | 8 | 1.14 |
| PE-012 Cermat/maturita exam ramps | +2 | +1 | +3 | 0 | +2 | +1 | +1 | 10 | 1.43 |
| PE-013 Regression alerts | 0 | 0 | +1 | +2 | +1 | 0 | +2 | 6 | 0.86 |
| PE-014 Explain-back drill | 0 | 0 | +2 | +1 | +1 | +1 | +2 | 7 | 1.00 |
| AI-001 Claude subscription auth | 0 | 0 | +2 | +3 | +2 | +2 | +2 | 11 | 1.57 |
| AI-002 Full OAuth flow | 0 | 0 | +1 | +2 | +1 | +1 | +1 | 6 | 0.86 |
| AI-004 Candle download progress | 0 | 0 | +1 | +3 | +1 | +2 | +1 | 8 | 1.14 |
| AI-005 Background re-embedding | 0 | 0 | +1 | +2 | +1 | +2 | +3 | 9 | 1.29 |
| AI-006 Hybrid re-rank RRF | 0 | 0 | +1 | +2 | 0 | 0 | +2 | 5 | 0.71 |
| AI-007 Larger e5 model | 0 | 0 | +1 | +2 | 0 | 0 | +2 | 5 | 0.71 |
| ING-007 Apple Vision OCR | 0 | 0 | +1 | +1 | +3 | 0 | 0 | 5 | 0.71 |
| ING-008 OCR "re-check" button | 0 | 0 | 0 | +1 | +3 | 0 | 0 | 4 | 0.57 |
| ING-009 Ingest progress events | 0 | 0 | +1 | +2 | +2 | +2 | +3 | 10 | 1.43 |
| UX-001 Human-readable lesson targets | +3 | +3 | +1 | 0 | +1 | 0 | 0 | 8 | 1.14 |
| UX-002 Candle silent-switch fix | 0 | 0 | 0 | +3 | 0 | +1 | +1 | 5 | 0.71 |
| UX-003 Colorblind-safe palette | +1 | +2 | 0 | +1 | 0 | 0 | 0 | 4 | 0.57 |
| UX-004 Reduce-motion | +1 | +1 | 0 | +1 | 0 | 0 | 0 | 3 | 0.43 |
| UX-005 Visible-space-marker toggle | 0 | 0 | 0 | +1 | 0 | 0 | 0 | 1 | 0.14 |
| UX-006 Audio cue on error | -1 | +1 | 0 | -1 | 0 | 0 | 0 | -1 | -0.14 |
| UX-007 Offline banner | 0 | 0 | 0 | +2 | 0 | +3 | 0 | 5 | 0.71 |
| UX-008 "What's new" modal | 0 | 0 | +1 | +1 | +1 | 0 | 0 | 3 | 0.43 |
| DIS-001 Notarized installers | +1 | +1 | +2 | +3 | +2 | +2 | +2 | 13 | 1.86 |
| DIS-002 Auto-updater | 0 | 0 | +1 | +3 | +1 | +1 | +1 | 7 | 1.00 |
| DIS-003 Parent-lite view | +2 | +1 | -1 | -1 | 0 | 0 | -1 | 0 | 0.00 |
| DIS-004 Licensing | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0.00 |
| PLT-001 iPad version | +2 | +2 | +2 | +1 | +3 | +2 | +1 | 13 | 1.86 |
| PLT-002 Touch keyboard | +2 | +2 | +1 | 0 | +3 | +2 | 0 | 10 | 1.43 |
| PLT-003 Junior mode | +3 | +2 | 0 | 0 | +1 | 0 | 0 | 6 | 0.86 |

## Top 10 by aggregate vote

1. **DIS-001** Notarized installers (Σ 13, avg 1.86) — everyone needs
   to actually install Datlino. Non-negotiable for shipping.
2. **PLT-001** iPad version (Σ 13, avg 1.86) — Tereza's primary
   platform. Pája and Martin would use it on the couch. Pie-in-the-sky
   until we have a stable desktop v1.
3. **IA-007** Settings 4-section regroup (Σ 12, avg 1.71) — Eliška's
   blocker; Martin wants it organised; Tereza needs the OCR section to
   stand out.
4. **PE-002** FSRS scheduler (Σ 11, avg 1.57) — the serious students
   (Pája/Lucie) score it +3. The beginners don't need it yet.
5. **AI-001** Claude subscription auth (Σ 11, avg 1.57) — you already
   pay for Pro/Max; Datlino shouldn't ask for a second BYOK. Martin
   +3, Tereza/Jonáš +2.
6. **PE-003** Cloze drills (Σ 10, avg 1.43) — strong signal from the
   study-focused personas (Pája/Lucie both +3).
7. **PE-012** Cermat/maturita exam ramps (Σ 10, avg 1.43) — Pája +3,
   Tereza +2. This IS the buyer story.
8. **ING-009** Ingest progress events (Σ 10, avg 1.43) — Lucie hits
   the wall hardest; Jonáš and Martin hate silent waits.
9. **PLT-002** Touch keyboard (Σ 10, avg 1.43) — depends on PLT-001.
10. **PE-004** Dictation mode (Σ 9, avg 1.29) — multi-sensory win.

## Top 10 by pure product-sense (your decision)

Persona votes don't always match product gravity. Here's my read of
what to build next, explained:

1. **IA-007** Settings regroup — finish Phase 1. Small.
2. **AI-001** Claude subscription auth — unblocks Rephrase mode at
   zero user-facing friction if they have a Claude sub. Big UX win.
3. **PE-002** FSRS + **PE-003** cloze + **PE-005** calibration — the
   "learning how to learn" trio. Pedagogically the highest-leverage
   thing we can add. Bundle into one Phase 3.
4. **UX-001** Human-readable lesson targets — Eliška's C2 still open.
   Two hours of work.
5. **UX-002** Candle silent-switch fix — Martin was explicit.
6. **ING-008** OCR "Zkontrolovat znovu" button — Tereza's afternoon
   saver.
7. **KB-005** Shift intro in early lesson — activates the existing
   .shift-glow path.
8. **BR-003** Animated mascot pecking tempo — brand-defining.
   Depends on final artwork delivery.
9. **DIS-001** Notarized installers — mandatory before any wider
   test cohort.
10. **PLT-001** iPad version — ambitious; wait until v1 is stable.

## How this document stays alive

* Update this file on every non-trivial commit. The
  `CHANGELOG.md` extractor validates every `shipped_in` SHA against
  `git log`.
* When a persona's context changes (e.g. Pája finishes maturita,
  Jonáš changes commute), re-score their column.
* The `/about` route in the app renders `CHANGELOG.md` so shipped
  items are discoverable to real users.
* Final product decisions are Paja's. Persona votes are signal, not
  a verdict.
