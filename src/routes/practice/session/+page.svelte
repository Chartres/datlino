<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { currentSession } from '$lib/session-store.svelte';
  import { refreshProfile } from '$lib/profile.svelte';
  import type { AttemptRecord, Keystroke } from '$lib/types';

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

  const target = $derived(plan?.sentences[sentenceIndex]?.text ?? '');
  const targetChars = $derived(Array.from(target)); // codepoints
  const totalSentences = $derived(plan?.sentences.length ?? 0);

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
  function handleKey(ev: KeyboardEvent) {
    if (!target) return;
    if (submitting) return;

    // Let browser shortcuts pass through.
    if (ev.metaKey || ev.ctrlKey || ev.altKey) return;
    if (ev.key === 'Tab') return;

    if (ev.key === 'Escape') {
      ev.preventDefault();
      finishSession(); // gives up early, still records what was typed
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

    // Single-char keys only. Modifier keys like Shift alone produce `.key`
    // with names we ignore here.
    const ch = ev.key;
    if (ch.length !== 1 && ch !== 'Enter') return;
    ev.preventDefault();

    if (cursor >= targetChars.length) {
      // Enter after completing the sentence advances.
      if (ch === 'Enter' || ch === ' ') {
        advanceSentence();
      }
      return;
    }

    const expected = targetChars[cursor] ?? '';
    const actual = ch === 'Enter' ? '\n' : ch;
    const now = performance.now();
    const tRel = now - attemptStart;

    const correct = actual === expected;
    typed[cursor] = actual;

    currentAttemptKeystrokes.push({
      t_ms: Math.round(tRel),
      actual,
      expected,
      correct
    });
    cursor += 1;
    lastKeyTime = now;

    // Sentence complete?
    if (cursor === targetChars.length) {
      // Tiny pause feels nicer than instant flip; require another press
      // (Space/Enter) only if there's an error, so perfect runs flow.
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

<svelte:window onkeydown={handleKey} />

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
              data-char-index={i}
            >{targetChars[i]}</span>
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
</style>
