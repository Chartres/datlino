<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { modes } from '$lib/mode-meta';
  import { currentSession } from '$lib/session-store.svelte';
  import type { PracticeMode } from '$lib/types';

  let selected = $state<PracticeMode>('content');
  let alpha = $state(0.7);
  let duration = $state(300); // seconds
  let query = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  const meta = $derived(modes.find((m) => m.code === selected)!);

  async function startSession() {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: selected,
        alpha: meta.supportsAlpha ? alpha : selected === 'content' ? 1.0 : 0.0,
        target_duration_s: duration,
        query: meta.supportsQuery && query.trim() ? query.trim() : undefined,
        pinned_source_prefixes: []
      });
      if (!plan.sentences.length) {
        error =
          'Pro tento mód není dost materiálu. Zkus přidat složku s poznámkami nebo vyber Diakritiku.';
        return;
      }
      currentSession.plan = plan;
      currentSession.summary = null;
      await goto('/practice/session');
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section>
  <h2>Co dnes procvičíme?</h2>
  <p class="muted">
    Vyber mód. Každý má svůj pedagogický účel — piš si tooltip, nebo pokračuj
    v tom, co ti dneska nejvíc sedí.
  </p>

  <div class="modes">
    {#each modes as mode (mode.code)}
      <button
        type="button"
        class="mode"
        class:active={selected === mode.code}
        onclick={() => (selected = mode.code)}
      >
        <h3>{mode.title}</h3>
        <p class="sub">{mode.subtitle}</p>
        <p class="principle">{mode.principle}</p>
      </button>
    {/each}
  </div>
</section>

<section>
  <h3>Nastavení</h3>

  <div class="row">
    <label>Délka
      <select bind:value={duration}>
        <option value={120}>2 min · rychlá rozcvička</option>
        <option value={300}>5 min · krátký blok</option>
        <option value={600}>10 min · hlavní blok</option>
        <option value={1200}>20 min · dlouhý trénink</option>
      </select>
    </label>
  </div>

  {#if meta.supportsAlpha}
    <div class="alpha">
      <label for="alpha-slider">
        Poměr: <strong>{Math.round(alpha * 100)} %</strong> obsah ·
        <strong>{Math.round((1 - alpha) * 100)} %</strong> trénink
      </label>
      <input
        id="alpha-slider"
        type="range"
        min="0"
        max="1"
        step="0.05"
        bind:value={alpha}
      />
      <div class="alpha-ends">
        <span>↑ trénuj moje slabiny</span>
        <span>studuj obsah ↑</span>
      </div>
    </div>
  {/if}

  {#if meta.supportsQuery}
    <div class="row">
      <label>Co by tě zajímalo? (volitelné)
        <input
          type="text"
          placeholder="např. Habsburkové, derivace, perfektum…"
          bind:value={query}
        />
      </label>
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <button class="primary" onclick={startSession} disabled={busy}>
    {busy ? 'Chystám sezení…' : 'Začít'}
  </button>
</section>

<style>
  section {
    margin-bottom: 2.5rem;
  }
  h2 {
    margin: 0 0 0.25rem;
  }
  h3 {
    margin: 0 0 0.75rem;
    font-size: 1rem;
  }
  .muted {
    color: #78716c;
    margin-top: 0;
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .mode {
    text-align: left;
    padding: 1rem;
    border: 1px solid rgba(28, 25, 23, 0.1);
    border-radius: 8px;
    background: #fffaf2;
    cursor: pointer;
    font: inherit;
    transition:
      border-color 120ms,
      background 120ms;
  }
  .mode:hover {
    border-color: rgba(179, 39, 31, 0.4);
  }
  .mode.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.06);
  }
  .mode h3 {
    margin: 0 0 0.25rem;
    color: #b3271f;
    font-size: 1rem;
  }
  .mode .sub {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: #292524;
  }
  .mode .principle {
    margin: 0;
    font-size: 0.8rem;
    color: #78716c;
    font-style: italic;
  }

  .row {
    margin: 0.75rem 0;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: #44403c;
    font-size: 0.9rem;
  }
  select,
  input[type='text'] {
    padding: 0.45rem 0.6rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 4px;
    font: inherit;
    background: #fffaf2;
  }

  .alpha {
    margin: 1rem 0;
    padding: 1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 6px;
  }
  .alpha input[type='range'] {
    width: 100%;
  }
  .alpha-ends {
    display: flex;
    justify-content: space-between;
    color: #78716c;
    font-size: 0.8rem;
  }

  button.primary {
    margin-top: 0.75rem;
    padding: 0.6rem 1.2rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    font-weight: 600;
  }
  button.primary:disabled {
    opacity: 0.5;
  }
  .error {
    color: #b3271f;
  }
</style>
