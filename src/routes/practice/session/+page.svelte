<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import Keyboard from '$lib/Keyboard.svelte';
  import { currentSession } from '$lib/session-store.svelte';
  import { refreshProfile } from '$lib/profile.svelte';
  import type { AttemptRecord, Keystroke } from '$lib/types';

  let showKeyboard = $state(true);
  let showCalibration = $state(true);
  let predictedAccuracy = $state(85);
  let calibrationSaved = $state(false);

  const plan = $derived(currentSession.plan);

  // Redirect if someone hits this page directly.
  $effect(() => {
    if (!plan) goto('/practice');
  });

  // --- Core typing state ---
  // `typed` mirrors the target char-by-char; for each char we store the
  // actual keystroke OR null if not yet typed. Arrays of Unicode codepoints,
  // not JS strings — Czech `ě` is one character, not two UTF-16 halves.
  let sentenceIndex = $state(0);
  let typed = $state<(string | null)[]>([]);
  let cursor = $state(0);
  let sessionStart = $state<number | null>(null);
  let attemptStart = $state<number>(0);
  let allKeystrokes = $state<Keystroke[]>([]);
  let currentAttemptKeystrokes = $state<Keystroke[]>([]);
  let attempts = $state<AttemptRecord[]>([]);
  let submitting = $state(false);
  let lastKeyTime = $state(0);

  // Some chunks contain chars the student can't physically type on a CZ
  // or SK keyboard — subscripts (H₂O), superscripts (m²), smart quotes,
  // em/en dashes, non-breaking spaces. We normalise the *display* target
  // so the keystroke log is comparable with the keys the student can
  // actually press. The raw source text stays intact in the database.
  const ASCII_NORMALISE: Record<string, string> = {
    // Subscripts U+2080–U+2089
    '\u2080': '0', '\u2081': '1', '\u2082': '2', '\u2083': '3', '\u2084': '4',
    '\u2085': '5', '\u2086': '6', '\u2087': '7', '\u2088': '8', '\u2089': '9',
    // Superscripts U+00B2, U+00B3, U+00B9, U+2070, U+2074–U+2079
    '\u00b2': '2', '\u00b3': '3', '\u00b9': '1', '\u2070': '0',
    '\u2074': '4', '\u2075': '5', '\u2076': '6', '\u2077': '7', '\u2078': '8', '\u2079': '9',
    // Dashes
    '\u2013': '-', '\u2014': '-', '\u2212': '-',
    // Non-breaking / narrow / thin spaces
    '\u00a0': ' ', '\u2009': ' ', '\u202f': ' ',
    // Double quotes (CZ/SK „“, EN “”, guillemets «»)
    '\u201e': '"', '\u201c': '"', '\u201d': '"', '\u00ab': '"', '\u00bb': '"',
    // Single quotes / apostrophes
    '\u2018': "'", '\u2019': "'", '\u2039': "'", '\u203a': "'",
    // Misc
    '\u2026': '...',
    '\u00d7': 'x', '\u00b7': '*'
  };

  function normaliseForTyping(text: string): string {
    let out = '';
    for (const ch of Array.from(text)) {
      out += ASCII_NORMALISE[ch] ?? ch;
    }
    return out;
  }

  const target = $derived(
    normaliseForTyping(plan?.sentences[sentenceIndex]?.text ?? '')
  );
  const targetChars = $derived(Array.from(target)); // codepoints
  const totalSentences = $derived(plan?.sentences.length ?? 0);

  // Cloze mask — for the current sentence, which char indices should be
  // rendered as blanks (underscores) in the visible surface while still
  // being typed as the real character. Derived per-sentence from the
  // sentence's `cloze_span` [byte_offset, byte_length] in the source.
  const clozeHidden = $derived.by<Set<number>>(() => {
    const hidden = new Set<number>();
    const s = plan?.sentences[sentenceIndex];
    if (!s?.cloze_span) return hidden;
    const [byteOff, byteLen] = s.cloze_span;
    // Walk the normalised target + compute byte→char index mapping.
    const raw = s.text;
    let byteCursor = 0;
    let charIdx = 0;
    for (const ch of Array.from(raw)) {
      const chBytes = new TextEncoder().encode(ch).length;
      if (byteCursor >= byteOff && byteCursor < byteOff + byteLen) {
        // Map back to the normalised targetChars index. Since the
        // normaliser never changes alphabetic chars (only punctuation),
        // the index lines up 1:1 for cloze words.
        hidden.add(charIdx);
      }
      byteCursor += chBytes;
      charIdx += 1;
    }
    return hidden;
  });

  // Group chars into words + space markers so wrap happens at spaces and
  // long passages read like prose. Each `word` group is rendered as an
  // inline-block unit; each `space` group is a break point.
  type Group =
    | { kind: 'word'; indices: number[]; key: string }
    | { kind: 'space'; indices: number[]; key: string };
  const wordGroups = $derived.by<Group[]>(() => {
    const out: Group[] = [];
    let current: number[] = [];
    targetChars.forEach((ch, i) => {
      if (ch === ' ') {
        if (current.length) {
          out.push({ kind: 'word', indices: current, key: `w${current[0]}` });
          current = [];
        }
        out.push({ kind: 'space', indices: [i], key: `s${i}` });
      } else {
        current.push(i);
      }
    });
    if (current.length) {
      out.push({ kind: 'word', indices: current, key: `w${current[0]}` });
    }
    return out;
  });

  let surfaceEl = $state<HTMLElement | null>(null);

  // Some OS/browser combos fire both `compositionend` AND a follow-up
  // keydown carrying the final composed char (macOS/WebKit with some CZ
  // input methods does this). We swallow the duplicate by remembering
  // what we just accepted via composition.
  let lastComposedChar: string | null = null;
  let lastComposedAt = 0;

  // Initialise / reset per-sentence state whenever the target changes.
  $effect(() => {
    if (!target) return;
    typed = new Array(targetChars.length).fill(null);
    cursor = 0;
    currentAttemptKeystrokes = [];
    attemptStart = performance.now();
    if (sessionStart === null) sessionStart = attemptStart;
    lastKeyTime = attemptStart;
    // Scroll long passages back to the top when a new sentence starts.
    if (surfaceEl) surfaceEl.scrollTop = 0;
  });

  // Keep the active character visible as the cursor advances through a
  // long chapter passage that wraps past the viewport.
  $effect(() => {
    if (!surfaceEl) return;
    void cursor;
    queueMicrotask(() => {
      const el = surfaceEl?.querySelector<HTMLElement>(`[data-char-index="${cursor}"]`);
      if (el && typeof el.scrollIntoView === 'function') {
        el.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      }
    });
  });

  // --- Live stats ---
  const correctCount = $derived(
    typed.filter((ch, i) => ch !== null && ch === targetChars[i]).length
  );
  const typedCount = $derived(typed.filter((ch) => ch !== null).length);
  const accuracy = $derived(
    typedCount === 0 ? 100 : (correctCount / typedCount) * 100
  );

  let elapsedMs = $state(0);
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (!target) return;
    tickHandle = setInterval(() => {
      elapsedMs = performance.now() - (sessionStart ?? performance.now());
    }, 250);
    return () => {
      if (tickHandle) clearInterval(tickHandle);
    };
  });

  const totalTypedSoFar = $derived(
    allKeystrokes.length + currentAttemptKeystrokes.length
  );
  const wpm = $derived(
    elapsedMs < 500
      ? 0
      : (totalTypedSoFar / 5) / (elapsedMs / 60_000)
  );

  // --- Key handling ---
  //
  // Dead-key diacritics (e.g. `ř` typed as AltGr+= then r on CZ layouts)
  // fire a `keydown` with `key === "Dead"` that resolves to the composed
  // character via `compositionend`. If we treated the `Dead` keypress as
  // "wrong character", every ř/č/š would look like a two-key failure.
  // So: skip keys during composition, skip the literal "Dead" key, and
  // hand composed text to `acceptChar` via the compositionend handler.
  //
  // Not every OS/layout uses the `"Dead"` sentinel — some (macOS, a few
  // Windows Czech layouts) deliver the standalone spacing diacritic
  // character directly (ˇ, ´, ˘, ¨, ˜, `, ^, ˚). Treat the full set as
  // pass-through too so `řídí` records as one correct keystroke on each
  // character regardless of the input method.
  const DEAD_KEY_CHARS = new Set([
    // Spacing diacritics that some OS/layouts emit instead of "Dead".
    'ˇ', // caron (háček)
    '´', // acute (čárka)
    '˘', // breve
    '¨', // diaeresis / trema
    '˜', // small tilde
    '`', // grave
    '^', // circumflex
    '˚', // ring above (kroužek — critical for ů on SK/CZ layouts)
    '˝', // double acute (SK layout uses this for ő/ű)
    '¸', // cedilla
    '˙', // dot above
    '¯', // macron
    '΄', // Greek tonos — rare but the acute shares layout space
    // IME-composition sentinels that different browsers use.
    'Unidentified',
    'Process',
    'Compose'
  ]);

  // Combining diacritical marks: any U+0300–U+036F, U+1AB0–U+1AFF,
  // U+1DC0–U+1DFF standalone should never count as a real keystroke.
  function isCombiningMark(key: string): boolean {
    if (key.length !== 1) return false;
    const c = key.charCodeAt(0);
    return (
      (c >= 0x0300 && c <= 0x036f) ||
      (c >= 0x1ab0 && c <= 0x1aff) ||
      (c >= 0x1dc0 && c <= 0x1dff)
    );
  }

  function handleKey(ev: KeyboardEvent) {
    if (!target) return;
    if (submitting) return;

    // Let browser shortcuts pass through.
    if (ev.metaKey || ev.ctrlKey) return;
    if (ev.key === 'Tab') return;

    if (ev.key === 'Escape') {
      ev.preventDefault();
      finishSession();
      return;
    }

    if (ev.key === 'Backspace') {
      ev.preventDefault();
      if (cursor > 0) {
        cursor -= 1;
        typed[cursor] = null;
      }
      return;
    }

    // Dead-key pass-through — we wait for the final composed char.
    if (ev.key === 'Dead' || ev.isComposing || ev.keyCode === 229) return;
    if (DEAD_KEY_CHARS.has(ev.key)) return;
    if (isCombiningMark(ev.key)) return;
    // AltGraph is separate from `altKey` on several Windows layouts;
    // many dead-key combos come through AltGr+letter — let the IME
    // compose and wait for compositionend.
    if (ev.getModifierState && ev.getModifierState('AltGraph')) return;

    // Single-char keys only. Modifier keys like Shift alone produce `.key`
    // with multi-char names (Shift, Control, Meta, ...) and get ignored.
    const ch = ev.key;
    if (ch.length !== 1 && ch !== 'Enter') return;

    // Alt / Option held down with a letter is often used for diacritic
    // composition on macOS layouts — leave it to the IME and wait for
    // compositionend.
    if (ev.altKey) return;

    // De-dupe: some OSes fire compositionend AND a follow-up keydown
    // carrying the same character. Skip the second hit if it arrives
    // within 80 ms.
    if (
      lastComposedChar === ch &&
      performance.now() - lastComposedAt < 80
    ) {
      lastComposedChar = null;
      ev.preventDefault();
      return;
    }

    ev.preventDefault();
    acceptChar(ch === 'Enter' ? '\n' : ch);
  }

  function handleCompositionEnd(ev: CompositionEvent) {
    if (!target || submitting) return;
    const data = ev.data;
    if (!data) return;
    // compositionend can deliver one codepoint (CZ dead key → ř) or
    // several (some CJK IMEs); we iterate to be safe.
    const chars = Array.from(data);
    for (const ch of chars) {
      acceptChar(ch);
    }
    // Remember the last composed codepoint so the de-dupe in handleKey
    // can swallow a duplicate keydown that some OSes fire right after.
    if (chars.length > 0) {
      lastComposedChar = chars[chars.length - 1];
      lastComposedAt = performance.now();
    }
  }

  function acceptChar(ch: string) {
    if (cursor >= targetChars.length) {
      if (ch === '\n' || ch === ' ') {
        advanceSentence();
      }
      return;
    }
    const expected = targetChars[cursor] ?? '';
    const now = performance.now();
    const tRel = now - attemptStart;
    const correct = ch === expected;
    typed[cursor] = ch;
    currentAttemptKeystrokes.push({
      t_ms: Math.round(tRel),
      actual: ch,
      expected,
      correct
    });
    cursor += 1;
    lastKeyTime = now;

    if (cursor === targetChars.length) {
      const perfect = typed.every((c, i) => c === targetChars[i]);
      if (perfect) advanceSentence();
    }
  }

  function advanceSentence() {
    const now = performance.now();
    attempts.push({
      chunk_id: plan?.sentences[sentenceIndex]?.chunk_id ?? null,
      target_text: target,
      started_at_ms: Math.round(attemptStart - (sessionStart ?? attemptStart)),
      finished_at_ms: Math.round(now - (sessionStart ?? attemptStart)),
      keystrokes: [...currentAttemptKeystrokes],
      completed: typed.every((c, i) => c === targetChars[i])
    });
    allKeystrokes = [...allKeystrokes, ...currentAttemptKeystrokes];

    if (sentenceIndex + 1 < totalSentences) {
      sentenceIndex += 1;
    } else {
      finishSession();
    }
  }

  async function confirmCalibration() {
    if (!plan) return;
    try {
      await api.recordCalibrationPrediction(plan.session_id, predictedAccuracy);
      calibrationSaved = true;
    } catch (e) {
      // Non-blocking — calibration is advisory; if it fails we still
      // let the student practice.
    }
    showCalibration = false;
  }

  async function finishSession() {
    if (submitting || !plan) return;
    submitting = true;
    // Flush the in-progress sentence if the user pressed Escape mid-way.
    if (currentAttemptKeystrokes.length > 0 && typedCount > 0) {
      const now = performance.now();
      attempts.push({
        chunk_id: plan.sentences[sentenceIndex]?.chunk_id ?? null,
        target_text: target,
        started_at_ms: Math.round(attemptStart - (sessionStart ?? attemptStart)),
        finished_at_ms: Math.round(now - (sessionStart ?? attemptStart)),
        keystrokes: [...currentAttemptKeystrokes],
        completed: false
      });
    }
    try {
      const summary = await api.finalizeSession(plan.session_id, attempts);
      currentSession.summary = summary;
      await refreshProfile();
      await goto('/practice/summary');
    } catch (e) {
      alert('Nepodařilo se uložit sezení: ' + e);
      submitting = false;
    }
  }

  function skipSentence() {
    // Bank whatever's been typed, move on.
    advanceSentence();
  }
</script>

<svelte:window onkeydown={handleKey} oncompositionend={handleCompositionEnd} />

{#if plan && showCalibration && !calibrationSaved}
  <div class="calibration-modal" role="dialog" aria-labelledby="cal-title">
    <div class="cal-card">
      <h3 id="cal-title">Než začneš — krátká kalibrace</h3>
      <p class="muted">
        Kolik procent znaků napíšeš správně v tomhle sezení? Je to
        malý test tvého vlastního odhadu — časem zjistíš, jestli se
        přeceňuješ nebo podceňuješ.
      </p>
      <div class="cal-slider">
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          bind:value={predictedAccuracy}
        />
        <div class="cal-value">{predictedAccuracy}%</div>
      </div>
      <div class="cal-actions">
        <button type="button" onclick={() => (showCalibration = false)}>
          Přeskočit
        </button>
        <button type="button" class="primary" onclick={confirmCalibration}>
          Začít →
        </button>
      </div>
    </div>
  </div>
{/if}

{#if plan}
  <div class="hud">
    <div class="stat">
      <span class="label">WPM</span>
      <span class="value">{wpm.toFixed(0)}</span>
    </div>
    <div class="stat">
      <span class="label">Přesnost</span>
      <span class="value">{accuracy.toFixed(0)} %</span>
    </div>
    <div class="stat">
      <span class="label">Věta</span>
      <span class="value">{sentenceIndex + 1} / {totalSentences}</span>
    </div>
    <div class="stat">
      <span class="label">Čas</span>
      <span class="value">{Math.floor(elapsedMs / 1000)} s</span>
    </div>
    <div class="actions">
      <button
        type="button"
        onclick={() => (showKeyboard = !showKeyboard)}
        disabled={submitting}
        title="Zobraz / skryj hint s prsty"
      >
        {showKeyboard ? 'Skrýt klávesnici' : 'Zobrazit klávesnici'}
      </button>
      <button type="button" onclick={skipSentence} disabled={submitting}>Přeskočit</button>
      <button type="button" onclick={finishSession} disabled={submitting}>Ukončit</button>
    </div>
  </div>

  <div class="typing-surface" role="presentation" bind:this={surfaceEl}>
    {#each wordGroups as group (group.key)}
      {#if group.kind === 'word'}
        <span class="word">
          {#each group.indices as i}
            <span
              class="char"
              class:correct={typed[i] === targetChars[i]}
              class:wrong={typed[i] !== null && typed[i] !== targetChars[i]}
              class:cursor={i === cursor}
              class:cloze={clozeHidden.has(i) && typed[i] === null}
              data-char-index={i}
            >{clozeHidden.has(i) && typed[i] === null ? '_' : targetChars[i]}</span>
          {/each}
        </span>
      {:else}
        {@const i = group.indices[0]}
        <span
          class="char space"
          class:correct={typed[i] === targetChars[i]}
          class:wrong={typed[i] !== null && typed[i] !== targetChars[i]}
          class:cursor={i === cursor}
          aria-label="mezera"
          data-char-index={i}
        >␣</span>
      {/if}
    {/each}
  </div>

  {#if plan.sentences[sentenceIndex]?.source_path}
    <p class="source">📄 {plan.sentences[sentenceIndex]?.source_path}</p>
  {:else if plan.sentences[sentenceIndex]?.is_generated}
    <p class="source">✦ Generovaný drill</p>
  {/if}

  {#if showKeyboard}
    <div class="keyboard-wrap">
      <Keyboard nextChar={targetChars[cursor] ?? null} />
    </div>
  {/if}

  {#if plan.sentences[sentenceIndex]?.source_text}
    <details class="rephrase">
      <summary>
        ✨ Remix (podobnost {((plan.sentences[sentenceIndex]!.similarity ?? 0) * 100).toFixed(0)}% s originálem) —
        rozbalit původní znění
      </summary>
      <p class="original">{plan.sentences[sentenceIndex]?.source_text}</p>
    </details>
  {/if}

  <p class="hint">
    Piš přímo — žádné klikání. <kbd>Esc</kbd> ukončí sezení. <kbd>Enter</kbd>
    nebo <kbd>mezerník</kbd> přeskočí dokončenou větu, pokud má chybu.
  </p>
{/if}

<style>
  .hud {
    display: flex;
    gap: 2rem;
    align-items: center;
    padding: 0.75rem 1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    margin-bottom: 1.5rem;
  }
  .stat {
    display: flex;
    flex-direction: column;
  }
  .stat .label {
    font-size: 0.7rem;
    color: #78716c;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .stat .value {
    font-size: 1.4rem;
    font-weight: 600;
    color: #1c1917;
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 0.5rem;
  }
  .actions button {
    padding: 0.4rem 0.8rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    color: #44403c;
  }
  .actions button:hover {
    border-color: #b3271f;
    color: #b3271f;
  }

  .typing-surface {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    padding: 2rem 2rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, 'Cascadia Mono', monospace;
    font-size: 1.5rem;
    line-height: 2.2rem;
    letter-spacing: 0.02em;
    color: #a8a29e;
    min-height: 6rem;
    /* Cap so a full chapter stays readable; cursor auto-scrolls into view. */
    max-height: 50vh;
    overflow-y: auto;
    user-select: none;
    /* Word groups are inline-block so they break between words, not
       mid-word — keeps long passages readable. */
    word-break: normal;
    overflow-wrap: break-word;
  }

  .word {
    display: inline-block;
    white-space: nowrap;
  }

  .char {
    position: relative;
    transition: color 80ms ease;
    display: inline-block;
    min-width: 0.55em;
    text-align: center;
  }
  .char.correct {
    color: #1c1917;
  }
  .char.wrong {
    color: #fffaf2;
    background: #b3271f;
    border-radius: 2px;
  }
  /* Render a muted mid-dot in place of the space so students can see where
     the gap belongs. Still correct when they press the actual space bar. */
  .char.space {
    color: rgba(28, 25, 23, 0.18);
  }
  .char.space.correct {
    color: rgba(28, 25, 23, 0.18);
  }
  .char.space.wrong {
    color: #fffaf2;
    background: rgba(179, 39, 31, 0.7);
  }
  .char.cursor {
    /* Woodpecker-red caret */
    box-shadow: inset -2px 0 0 0 #b3271f;
  }

  .source {
    margin: 1rem 0 0;
    color: #78716c;
    font-size: 0.85rem;
  }
  .hint {
    margin: 1.5rem 0 0;
    color: #78716c;
    font-size: 0.8rem;
  }
  kbd {
    background: rgba(28, 25, 23, 0.05);
    border: 1px solid rgba(28, 25, 23, 0.15);
    border-radius: 3px;
    padding: 0.05rem 0.3rem;
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
  }
  .rephrase {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: rgba(179, 39, 31, 0.04);
    border: 1px solid rgba(179, 39, 31, 0.2);
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .rephrase summary {
    cursor: pointer;
    color: #b3271f;
    font-weight: 600;
  }
  .rephrase .original {
    margin: 0.5rem 0 0;
    color: #57534e;
    font-family: ui-monospace, monospace;
  }
  .keyboard-wrap {
    margin-top: 1rem;
  }

  .char.cloze {
    color: transparent;
    background: repeating-linear-gradient(
      90deg,
      rgba(28, 25, 23, 0.15) 0,
      rgba(28, 25, 23, 0.15) 1px,
      transparent 1px,
      transparent 3px
    );
    border-bottom: 2px solid rgba(179, 39, 31, 0.4);
    border-radius: 0;
  }
  .char.cloze.cursor {
    box-shadow: inset -2px 0 0 0 #b3271f;
  }

  .calibration-modal {
    position: fixed;
    inset: 0;
    background: rgba(28, 25, 23, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .cal-card {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.15);
    border-radius: 10px;
    padding: 1.5rem 1.75rem;
    max-width: 36rem;
    box-shadow: 0 12px 40px rgba(28, 25, 23, 0.18);
  }
  .cal-card h3 {
    margin: 0 0 0.5rem;
    font-size: 1.15rem;
    color: #1c1917;
  }
  .cal-slider {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin: 1rem 0;
  }
  .cal-slider input[type='range'] { flex: 1; }
  .cal-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: #b3271f;
    min-width: 4rem;
    text-align: right;
  }
  .cal-actions {
    display: flex;
    gap: 0.6rem;
    justify-content: flex-end;
  }
  .cal-actions button {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
  }
  .cal-actions button.primary {
    border-color: #b3271f;
    background: #b3271f;
    color: #fffaf2;
    font-weight: 600;
  }
</style>
