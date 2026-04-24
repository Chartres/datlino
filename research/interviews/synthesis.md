# Persona Interview Synthesis

Seven personas. Fifteen scripted Playwright runs against the real
production build. Every spec green. Signal came from the *qualitative*
observations each persona recorded at key moments. What follows is the
pattern across voices, ranked by severity.

---

## Blockers (fix before shipping)

### B1 — Settings page bounces the target user

**Voices:** Eliška (14), Filip (13), and implicitly Pája when she
admits "proč mám vybírat provider?"

**What's happening:** The Nastavení page opens with "Aktuální provider",
"Vyber provider", "Cohere API klíč", "OCR", "Rephrase mode" in a flat
list. These are all *infrastructure* concerns — a student who just wants
to study should not be parachuted into them.

**Mitigation:** Settings is currently 1-depth flat. Split into:
* **Profil** (baseline, reset, export) — visible to everyone
* **Kvalita vyhledávání** (embedding provider + OCR) — collapsed by default
* **Remix (AI přepis)** (Anthropic key + style) — collapsed by default
* **O aplikaci** — links, version

Default is "don't show the scary stuff unless the student asks."

---

### B2 — Six modes on the practice picker is too many

**Voices:** Eliška ("'Úvodní lekce' je první — good. Ale taky vidím
pět dalších"), Pája, Filip.

**What's happening:** The picker lists Úvodní lekce / Moje materiály /
Zahřívání / Diakritika / Tvá slabá místa / Mix as equal-weight tiles.
Warmup overlaps with Úvodní lekce chapters 1–3. Diacritics overlaps with
chapters 10–12. WeakKeys is a drill, not a mode. Mix is a power-user
toggle.

**Mitigation:** The IA proposal you already agreed to: collapse to two
tiles — **Učím se psát** (IntroLesson curriculum) and **Učím se obsah**
(content-driven). WeakKeys becomes a button on Pokrok ("procvič slabé
kombinace 5 min"). Mix's α slider moves behind an "Pokročilé" expander
inside Content mode.

---

## High-value confusions

### C1 — Two-click mode selection
**Voice:** Eliška.

> "Klikla jsem na Úvodní lekce a nic se nestalo. Musím kliknout ještě
> na Začít?"

Tiles act as *selectors*, not actions. A beginner expects one click to
start. Keep the duration/settings form but make the default a single
click-through. If the student wants to tune duration, they can — but the
happy path is tile → session.

**Fix:** Tiles become buttons that route directly. Tuning options move
to an "Upravit" link adjacent to the tile.

---

### C2 — "Cíl: 10 WPM · 92 % přesnost" means nothing to Eliška
**Voice:** Eliška.

> "WPM je co? Slov za minutu? Ale já ani nevím co mám jako cíl, když
> začínám."

Thresholds were designed for adults. A 14-year-old on day 1 needs
relative framing, not absolute numbers.

**Fix:** Replace `10 WPM · 92 %` with a visual "obtížnost" bar + a
human-readable phrase: "Zvládneš, když z deseti pokusů napíšeš osm
bez chyb." Keep the numbers underneath for adults who care.

---

### C3 — Candle switch is silent
**Voice:** Martin.

> "Kliknul jsem Lokální Candle a UI řeklo 'provider změněn'. Ale v
> realitě si stahuje 120 MB modelu první použití. Kde je progress bar?"

**Fix:** When switching to Local: open a modal or inline card showing
model download progress (hf-hub reports bytes). Until the model is
ready, queue re-embedding; fall back to BM25 for searches meanwhile.

---

### C4 — Filip wants a roadmap, not a spotlight
**Voice:** Filip (13).

> "Svítí jen ta další. A co bude POTOM?"

For complete beginners, one-key lookahead is overwhelming because they
can't plan ahead. Two-to-three-key lookahead (next three highlighted in
decreasing brightness) would give him a sight-line.

**Fix:** Keyboard component takes `nextChars: string[]` instead of
`nextChar: string`; render the current one in full finger-color and the
next two in 50 %/25 % opacity.

---

### C5 — Empty library has no invitation
**Voice:** Tereza.

> "Dostala jsem se na Knihovnu a není tam nic. Kde je pozvánka?"

`/` with zero documents shows the empty-state hero paragraph and two
buttons but no *example*. First-run students don't know what "Přidat
složku" means in practice.

**Fix:** Empty state shows a "představení" card:
* illustration of three folder examples (Dějepis / Biologie / Angličtina)
* the three common sources (Markdown, PDF, GoodNotes export)
* "Zkus si — přidej jednu složku a uvidíš, jak to funguje."

---

### C6 — "Po instalaci tesseract musím zavřít aplikaci?"
**Voice:** Tereza.

> "Píše 'chybí'. Dal jsem si brew install. Teď co?"

OCR status is only read on mount. There's no way to re-check without
restart.

**Fix:** "Zkontrolovat znovu" button next to the status badges. Calls
`get_ocr_status` again and re-renders.

---

### C7 — Exam-prep with a thin corpus goes nowhere
**Voices:** Pája, Lucie.

> "Dala jsem 'Habsburkové a reformy' a zůstala jsem na téže stránce
> bez feedbacku."

When ExamPrep tokenises the query and none of the tokens produces
enough BM25 hits, the frontend returns an empty plan and shows an
error message — but only if `plan.sentences.length === 0`. The generic
"Pro tento dotaz jsme nic nenašli" doesn't help the student understand
WHY.

**Fix:** Backend returns richer diagnostics: which tokens matched, how
many docs/chapters scored, and a suggestion ("zkus přeformulovat" /
"přidej víc zdrojů"). Frontend surfaces those specifically.

---

## Medium-value

### M1 — Filter/search missing from large document grids
**Voice:** Lucie.

At 15 docs the grid is fine. At 200 it's not. Add a filter input atop
the documents section — case-insensitive substring match on the
filename.

### M2 — Offline/online status is implicit
**Voice:** Jonáš.

A thin strip at the top of Settings: "Offline: Lokální Candle / Fake.
Cloud: Cohere potřebuje internet." Helps commute users trust the app.

### M3 — Shift-key glow path exists but has no trigger in the curriculum
**Voice:** Filip.

`.shift-glow` CSS renders when next char is uppercase. The current
first lessons are all lowercase. Add a shift-introduction lesson early
enough that Filip sees his first shift glow within the first ~10 min.

---

## Delights (what already works)

* **On-screen keyboard with finger colors** — Filip and Eliška both
  hit "delight" on it. Home-row dots, next-key glow, legend.
* **Document → session in one click** — Pája called this "jednoklik-to-
  flow" unprompted.
* **Hide-keyboard toggle** — Filip gets the independence-training
  arc.
* **OCR install hints with real brew/apt commands** — Tereza
  copy-pasted directly.
* **Candle as a first-class Settings tile** (not greyed out) — Martin
  called it "třídní volba", which is the highest praise a tech-y
  student gives a non-technical app.
* **15 docs render cleanly in a grid** — Lucie's thesis scenario works.

---

## Prioritised action list (next ship)

1. **B1** — Split Settings into 4 sections with collapse (default all
   closed except Profil).
2. **B2** — Reduce practice picker to 2 tiles + move WeakKeys to
   Pokrok + bury α-slider under "Pokročilé".
3. **C1** — One-click mode selection for IntroLesson.
4. **C2** — Human-readable lesson targets.
5. **C5** — Onboarding card on empty Library.
6. **C3** — Candle download progress UI.
7. **C6** — "Zkontrolovat znovu" for OCR.
8. **C4** — Two-key lookahead on the keyboard.
9. **C7** — Better ExamPrep diagnostics.
10. **M1**, **M2**, **M3** — nice-to-haves.

Each of these is directly traceable to a persona quote; no item comes
from the abstract. Ship order should follow this list.
