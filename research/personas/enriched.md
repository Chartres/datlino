# Enriched Persona Profiles

Seven students Datlino is built for. Each carries their own context,
keyboard, study rhythm, and frustrations. These are fuller profiles than
the ones baked into `src-tauri/tests/` — they inform UX decisions, not
just regression tests.

---

## 1. Pája Dravcová — 17, maturita this May

**Where she is.** Fourth year at gymnázium in Brno. Profile subjects are
český jazyk, dějepis, angličtina. Maturita oral in May, didaktický test in
April. Wants to get into journalism at FSV UK.

**Study rhythm.** Evenings 19:00–22:00, Sundays full day. Material is
mixed: teacher's PDFs from iTřída, her own Markdown notes in Obsidian,
occasional Wikipedia paste.

**Typing level.** ~45 WPM on Czech text, learned by hunting-and-pecking.
Home row sometimes, shifts to look at keyboard when typing numbers or
punctuation.

**Devices.** MacBook Air M1 (2021, her father's old one). Czech ABC
input method, sometimes switches to English for vocabulary.

**What she'd say about Datlino:**

> "Když chci znát něco z dějepisu, projedu si to přes ChatGPT nebo si
> to odříkám do telefonu. Ale pak to nikde není — a zkoušející se mě
> může zeptat na formulaci. Přijde mi dobrý nápad to psát. Hlavně mě
> zajímá, jestli se mi to budou ve fingery učit tak, jak to reálně
> řekneš u maturity."

**What would make her daily user:** reliability on Cermat-style topics,
real sentences from her own dějepis folder, a sense that ten minut denně
skutečně zlepšuje WPM.

**What frustrates her right now (based on code read):**
- The "Nastavení" page makes her hesitate — "proč mám vybírat provider?
  já nevím co to je." She'd want Datlino to just work.
- She doesn't know what α-slider does in Hybrid mode.
- She won't figure out that clicking a document name starts "Trénovat
  celý".

---

## 2. Eliška Novotná — 14, 9th grade, Cermat in 2 months

**Where she is.** Základní škola, connecting to her first-choice
gymnázium hinges on the přijímačky. Studies primarily from a scanned
přijímačkové testy book her mother bought.

**Study rhythm.** After dinner, 30–45 minutes, often with mom checking
in. Easily distracted. Procrastinates but is self-critical about it.

**Typing level.** ~20 WPM. Uses three fingers per hand on average.
Genuinely can't touch-type and knows it.

**Devices.** Shared family Dell laptop running Windows 11, Czech keyboard
layout QWERTZ with direct diacritic keys on the number row.

**What she'd say about Datlino:**

> "Chtěla bych umět psát líp. Ve škole píšeme slohovky a trvá mi to
> dýl než všem ostatním. Mamka řekla že bych to mohla zkusit. Doufám
> že to nebude jako ta nudná psací hra co jsme dělali na informatice."

**What would make her daily user:** A clear beginner track. Something
that says "začni tady". Immediate visible progress (unlocked lesson,
first badge).

**What frustrates her right now:**
- Default Warmup mode drops her into content drills that reference
  things she hasn't learned — confidence hit on day 1.
- Diacritics mode is a great idea but she doesn't know it exists.
- The 16-lesson IntroLesson ladder would save her — but it's one of six
  tiles on a cluttered page.

**Important:** Eliška is the student whose *parent* is the buyer. Her
experience matters for WTP.

---

## 3. Martin Klíma — 18, language + tech nerd

**Where he is.** Čtvrťák at gymnázium in Praha. Doing FCE English and
plans DELF B2 French next year. Self-teaches programming on the side
(Python, some Rust).

**Study rhythm.** Blocks of 2 hours, 3x/week. Chapter-by-chapter.

**Typing level.** 70+ WPM on English, a bit slower on Czech.

**Devices.** Thinkpad X1 running Arch Linux. Multiple keyboard layouts.

**What he'd say about Datlino:**

> "Vidím co se snažíte dělat. BYOK je super, ale přepínač provider by
> mě měl prosit o klíč — ne naopak. Local Candle by u mě vyhrál, jen
> mi dej vědět kolik GB to sežere a jak dlouho to bude kompilovat."

**What would make him daily user:** Chapter mode on his English grammar
notes, the Hybrid α-slider (he'll actually use it), clean
local-first setup.

**What frustrates him right now:**
- Compile time for Candle silently happens on first launch.
- No visible "what's my provider doing right now?" status.
- CZ-QWERTY dead-key handling was broken until the last commit;
  he'd have noticed immediately.

---

## 4. Tereza Horáková — 16, handwritten-note maximalist

**Where she is.** Third-year gymnázium v Hradci. Writes EVERYTHING in
GoodNotes on her iPad, exports PDF weekly to her MacBook to back up.

**Study rhythm.** Reviews week's notes on Saturday. Types summaries for
self-test.

**Typing level.** ~30 WPM, inconsistent. Strong on diacritics (she's
Czech), weak on numbers + punctuation.

**Devices.** 2020 MacBook Pro + iPad. Czech ABC input.

**What she'd say about Datlino:**

> "Hele, moje poznámky jsou PDF z GoodNotes. Jsou to obrázky mého
> rukopisu. Myslím že to nepůjde přečíst, ale zkusit to musím."

**What would make her daily user:** OCR that actually works. Even if it
misreads 10 % of her cramped handwriting, the other 90 % is real study
material. Diacritic-aware OCR language packs.

**What frustrates her right now:**
- Tesseract not installed by default — "chybí" indicator on Settings
  doesn't tell her what to do after `brew install`.
- OCR happens silently; she doesn't know which PDFs were OCR'd vs
  text-layer.

---

## 5. Jonáš Vlček — 15, studies on metro

**Where he is.** Second-year gymnázium in Ostrava. Hour-long commute
each way. Wants to reclaim that time.

**Study rhythm.** 40 min on train going to school, 40 min back. No Wi-Fi
on the line reliably.

**Typing level.** Solid 50 WPM. Not the bottleneck.

**Devices.** 2019 MacBook Pro. School-issued. Allowed personal use.

**What he'd say about Datlino:**

> "Na vlaku mi nefunguje internet půlku cesty. Potřebuju aby to jelo
> offline. Embeddings na cloudu mi v tunelu nepomůžou."

**What would make him daily user:** Offline-first. The Local Candle
provider, intro-lessons curriculum, corpus ingested before he leaves
home. Session finalisation that doesn't depend on network.

**What frustrates him right now:**
- Network failures aren't surfaced well — the rephrase flow just
  `eprintln!`s and silently falls back.
- He doesn't know Local Candle is now default-on until he opens
  Settings.

---

## 6. Lucie Marková — 19, first-year university, thesis prep

**Where she is.** First year Bc. dějepisu FF MU Brno. Writing a
semestrální práce on Habsburg succession — 30 pages of sources.

**Study rhythm.** All-nighters followed by rest days. Chaotic.

**Typing level.** 60 WPM, but sloppy. High error rate on uncommon
diacritics and long academic words.

**Devices.** Cheap Windows laptop. Wireless keyboard from home.

**What she'd say about Datlino:**

> "Mám dvě stě PDF zdrojů. Doufám že to to všechno sežere. A že mi to
> opravdu najde ty kapitoly kde se píše o stavovském povstání, ne
> jen pět vět rozházených sem a tam."

**What would make her daily user:** ExamPrep that actually works (works
best with embeddings, we flagged this honestly), chapter-mode across a
large library, the document picker for drilling specific sources.

**What frustrates her right now:**
- On large corpora the UI hangs during initial embedding (we don't
  background-embed yet).
- `list_chapters` is still responsive at 120 chapters — but her real
  library is ~2000. Untested.

---

## 7. Filip Šimek — 13, 7th grade beginner

**Where he is.** Základka v Opavě. No specific exam pressure. Just
wants to be good at stuff.

**Study rhythm.** 15 minutes after school, before gaming.

**Typing level.** True beginner. Uses index fingers only. Looks at
keyboard constantly.

**Devices.** Family iMac (Intel) + his own iPad Mini.

**What he'd say about Datlino:**

> "Mamka mi to nainstalovala. Takže fajn. Ukaž mi kde jsou prsty."

**What would make him daily user:** The on-screen keyboard is the
feature for him. Visible home-row dots, finger colors, the next-key
glow. The IntroLesson ladder gives him an obvious daily increment.

**What frustrates him right now:**
- Keyboard highlights only the *next* character, not the next sequence
  — he's looking for a roadmap, not a spotlight.
- No animated hand on top of the keyboard showing finger *stretch*.
- He doesn't know what the finger colors mean until he hovers over a
  key (the legend at the bottom is easy to miss).

---

## What these personas agree on

- **Too many top-level choices.** Every persona except Martin wants
  Datlino to just pick the right path.
- **Offline / local-first matters.** Jonáš and Tereza can't rely on
  cloud; Pája and Eliška don't understand it.
- **BYOK is opaque.** Even Martin, who's technical, hits "why do I
  need this?" on first run.

## What they disagree on

- **α-slider and provider choice.** Martin wants them. Eliška would
  break things if exposed to them.
- **Mascot.** Filip and Eliška want it immediately. Pája and Lucie
  don't care. Martin actively prefers no mascot.
