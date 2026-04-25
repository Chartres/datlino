<script lang="ts">
  import { api } from './api';
  import type { RephraseStyle } from './types';

  // Free-tier rephrase: hand the student a deterministic prompt to feed
  // any free LLM (ChatGPT, Gemini, Claude.ai), then accept their paste.
  // Same similarity gate as the API path runs server-side via
  // applyCopyPasteRephrase. On accept, parent receives the rewritten
  // sentence array (same length, same order as `sources`).

  type Outcome = {
    text: string;
    similarity: number;
    generator_model: string;
    accepted: boolean;
  };

  let {
    sources,
    weakNgrams = [],
    style = 'keystrokes' as RephraseStyle,
    language = 'cs',
    onApply,
    onCancel
  }: {
    sources: string[];
    weakNgrams?: string[];
    style?: RephraseStyle;
    language?: string;
    onApply: (rewrites: string[], meta: { accepted: number; total: number; warnings: string[] }) => void;
    onCancel: () => void;
  } = $props();

  let prompt = $state('');
  let expectedCount = $state(0);
  let raw = $state('');
  let busyBuild = $state(true);
  let busyApply = $state(false);
  let copied = $state(false);
  let error = $state<string | null>(null);
  let warnings = $state<string[]>([]);
  let promptArea: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    void buildPrompt();
  });

  async function buildPrompt() {
    busyBuild = true;
    error = null;
    try {
      const r = await api.buildCopyPastePrompt(sources, weakNgrams, language, style);
      prompt = r.prompt;
      expectedCount = r.expected_count;
    } catch (e) {
      error = String(e);
    } finally {
      busyBuild = false;
    }
  }

  async function copyPrompt() {
    try {
      await navigator.clipboard.writeText(prompt);
      copied = true;
      setTimeout(() => (copied = false), 1800);
    } catch {
      // Fallback: select the textarea so user can ⌘C themselves.
      promptArea?.select();
    }
  }

  function open(url: string) {
    window.open(url, '_blank', 'noopener');
  }

  async function apply() {
    if (!raw.trim()) {
      error = 'Vlož odpověď z LLM před tím, než klikneš na Použít.';
      return;
    }
    busyApply = true;
    error = null;
    warnings = [];
    try {
      const res = await api.applyCopyPasteRephrase(sources, raw);
      const outcomes: Outcome[] = res.outcomes;
      const rewrites = outcomes.map((o) => o.text);
      const accepted = outcomes.filter((o) => o.accepted).length;
      warnings = res.warnings;
      onApply(rewrites, {
        accepted,
        total: outcomes.length,
        warnings: res.warnings
      });
    } catch (e) {
      error = String(e);
    } finally {
      busyApply = false;
    }
  }

  function close() {
    onCancel();
  }
</script>

<div class="backdrop" role="presentation" onclick={close}></div>
<div
  class="modal"
  role="dialog"
  aria-modal="true"
  aria-labelledby="cp-title"
>
  <header>
    <h3 id="cp-title">Remix přes copy-paste (zdarma)</h3>
    <button type="button" class="x" onclick={close} aria-label="Zavřít">×</button>
  </header>

  <p class="lede">
    Datlino ti připravilo přesný prompt s tvými větami a slabými klávesami.
    Vlož ho do libovolného volného LLM (ChatGPT, Claude.ai, Gemini),
    výsledek vrať zpátky sem. Sezení pojede s těmi přepsanými větami.
  </p>

  <ol class="steps">
    <li class="step">
      <header class="step-h">
        <span class="num">1</span>
        <strong>Zkopíruj prompt</strong>
        <button type="button" class="copy" onclick={copyPrompt} disabled={busyBuild}>
          {copied ? '✓ zkopírováno' : 'Kopírovat'}
        </button>
      </header>
      {#if busyBuild}
        <p class="muted">Stavím prompt…</p>
      {:else}
        <textarea
          bind:this={promptArea}
          readonly
          rows="8"
          value={prompt}
          aria-label="Prompt pro LLM"
        ></textarea>
        <p class="muted small">
          Obsahuje {expectedCount} {expectedCount === 1 ? 'větu' : 'vět'} a tvoje slabiny.
          Žádná data se nikam neposílají, dokud ho ručně někam nevložíš.
        </p>
      {/if}
    </li>

    <li class="step">
      <header class="step-h">
        <span class="num">2</span>
        <strong>Vlož ho do LLM</strong>
      </header>
      <div class="llm-row">
        <button type="button" class="llm" onclick={() => open('https://chatgpt.com/')}>
          ChatGPT (free) ↗
        </button>
        <button type="button" class="llm" onclick={() => open('https://claude.ai/new')}>
          Claude.ai (free) ↗
        </button>
        <button type="button" class="llm" onclick={() => open('https://gemini.google.com/')}>
          Gemini (free) ↗
        </button>
      </div>
      <p class="muted small">
        Otevře se nové okno. Vlož prompt, počkej na odpověď a zkopíruj
        celý JSON (i s kódovými ploty, jestli je tam má).
      </p>
    </li>

    <li class="step">
      <header class="step-h">
        <span class="num">3</span>
        <strong>Vrať výsledek</strong>
      </header>
      <textarea
        bind:value={raw}
        rows="8"
        placeholder="Sem vlož celou odpověď z LLM (JSON pole)…"
        disabled={busyApply}
        aria-label="Odpověď z LLM"
      ></textarea>
      <p class="muted small">
        Datlino z toho vytáhne JSON, zkontroluje podobnost (≥ 0,75 cos)
        a zachová věty, které prošly. Ostatní zůstanou ve verbatim podobě.
      </p>
    </li>
  </ol>

  {#if warnings.length > 0}
    <ul class="warnings">
      {#each warnings as w}
        <li>{w}</li>
      {/each}
    </ul>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <footer>
    <button type="button" class="secondary" onclick={close} disabled={busyApply}>
      Zrušit
    </button>
    <button type="button" class="primary" onclick={apply} disabled={busyApply || busyBuild}>
      {busyApply ? 'Zpracovávám…' : 'Použít a začít'}
    </button>
  </footer>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(28, 25, 23, 0.55);
    z-index: 30;
  }
  .modal {
    position: fixed;
    inset: 5% 50% auto auto;
    transform: translateX(50%);
    width: min(720px, 92vw);
    max-height: 90vh;
    overflow-y: auto;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.15);
    border-radius: 12px;
    box-shadow: 0 18px 48px rgba(28, 25, 23, 0.25);
    z-index: 31;
    padding: 1.25rem 1.5rem 1.5rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  h3 { margin: 0; font-size: 1.15rem; color: #1c1917; }
  .x {
    background: none;
    border: none;
    font-size: 1.6rem;
    line-height: 1;
    cursor: pointer;
    color: #78716c;
    padding: 0 0.3rem;
  }
  .x:hover { color: #b3271f; }
  .lede {
    margin: 0 0 0.85rem;
    color: #44403c;
    font-size: 0.92rem;
  }
  .steps {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .step {
    background: #fff;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    padding: 0.75rem 0.9rem;
  }
  .step-h {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin: 0 0 0.5rem;
  }
  .num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.6rem;
    height: 1.6rem;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.85rem;
  }
  .step-h strong {
    flex: 1;
    color: #1c1917;
    font-size: 0.95rem;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 0.55rem 0.7rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 5px;
    background: #fffaf2;
    font: inherit;
    font-size: 0.85rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    resize: vertical;
  }
  textarea[readonly] {
    background: rgba(28, 25, 23, 0.04);
    color: #292524;
  }
  .copy {
    padding: 0.35rem 0.75rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.82rem;
    font-weight: 600;
  }
  .copy:disabled { opacity: 0.5; cursor: not-allowed; }
  .llm-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin: 0.3rem 0;
  }
  .llm {
    padding: 0.45rem 0.85rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: #fffaf2;
    color: #44403c;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  .llm:hover { border-color: #b3271f; color: #b3271f; }
  .warnings {
    margin: 0.75rem 0 0;
    padding: 0.55rem 0.9rem 0.55rem 1.6rem;
    background: rgba(180, 143, 0, 0.08);
    border: 1px solid rgba(180, 143, 0, 0.25);
    border-radius: 6px;
    color: #6a5400;
    font-size: 0.8rem;
  }
  .warnings li { margin: 0.1rem 0; }
  .error {
    margin: 0.75rem 0 0;
    padding: 0.55rem 0.9rem;
    background: rgba(179, 39, 31, 0.08);
    border: 1px solid rgba(179, 39, 31, 0.25);
    border-radius: 6px;
    color: #b3271f;
    font-size: 0.85rem;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1rem;
    padding-top: 0.85rem;
    border-top: 1px dashed rgba(28, 25, 23, 0.1);
  }
  .primary {
    padding: 0.55rem 1.1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
    font-weight: 600;
  }
  .secondary {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    color: #44403c;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
  }
  .primary:disabled, .secondary:disabled { opacity: 0.5; cursor: not-allowed; }
  .muted { color: #78716c; }
  .small { font-size: 0.8rem; }
</style>
