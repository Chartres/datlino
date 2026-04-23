<script lang="ts">
  import { goto } from '$app/navigation';
  import { currentSession } from '$lib/session-store.svelte';
  import { badgeLabels } from '$lib/mode-meta';

  const summary = $derived(currentSession.summary);

  $effect(() => {
    if (!summary) goto('/practice');
  });
</script>

{#if summary}
  <section class="hero">
    <h2>Hotovo.</h2>
    <p class="muted">
      Zapsali jsme sezení. Slabá místa se pomalu zlepšují — každý drill
      se počítá.
    </p>
  </section>

  <section class="scores">
    <div class="big">
      <span class="n">{summary.wpm.toFixed(0)}</span>
      <span class="u">WPM</span>
    </div>
    <div class="big">
      <span class="n">{summary.accuracy_pct.toFixed(0)}%</span>
      <span class="u">přesnost</span>
    </div>
    <div class="big accent">
      <span class="n">+{summary.xp_earned}</span>
      <span class="u">XP</span>
    </div>
  </section>

  <section class="grid">
    <div class="card">
      <h3>Úroveň</h3>
      <p class="big-text">L{summary.level}</p>
      <p class="muted">{summary.total_xp} XP celkem</p>
    </div>
    <div class="card">
      <h3>Série</h3>
      <p class="big-text">🔥 {summary.current_streak}</p>
      <p class="muted">nejdelší: {summary.longest_streak}</p>
    </div>
    <div class="card">
      <h3>Napsáno</h3>
      <p class="big-text">{summary.words_typed}</p>
      <p class="muted">slov · {summary.characters_typed} znaků</p>
    </div>
    <div class="card">
      <h3>Hotové věty</h3>
      <p class="big-text">{summary.sentences_completed}/{summary.sentences_attempted}</p>
      <p class="muted">
        {summary.sentences_attempted - summary.sentences_completed} rozpracovaných
      </p>
    </div>
  </section>

  {#if summary.badges_awarded.length}
    <section>
      <h3>Nové odznaky</h3>
      <ul class="badges">
        {#each summary.badges_awarded as code}
          <li class="badge-new">✦ {badgeLabels[code] ?? code}</li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if summary.weak_preview.length}
    <section>
      <h3>Tvoje nejslabší kombinace právě teď</h3>
      <p class="muted">
        Tyhle budou mít největší váhu v módu „Tvá slabá místa."
      </p>
      <ul class="weak">
        {#each summary.weak_preview as w}
          <li>
            <code>{w.ngram.replace(/ /g, '␣')}</code>
            <span class="weak-meta">
              {w.ema_latency_ms.toFixed(0)} ms · chybovost {(w.ema_error_rate * 100).toFixed(0)}%
            </span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section class="next">
    <a href="/practice" class="primary">Další sezení →</a>
    <a href="/progress">Pokrok v čase</a>
  </section>
{/if}

<style>
  section {
    margin-bottom: 2rem;
  }
  .hero h2 {
    margin: 0 0 0.25rem;
    font-size: 1.6rem;
  }
  .muted {
    color: #78716c;
    font-size: 0.9rem;
    margin: 0;
  }

  .scores {
    display: flex;
    gap: 2rem;
    padding: 1.5rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
    align-items: baseline;
  }
  .big {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .big .n {
    font-size: 2.2rem;
    font-weight: 700;
    color: #1c1917;
    line-height: 1;
  }
  .big .u {
    font-size: 0.8rem;
    color: #78716c;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .big.accent .n {
    color: #b3271f;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.75rem;
  }
  .card {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    padding: 1rem;
  }
  .card h3 {
    margin: 0 0 0.5rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #78716c;
  }
  .big-text {
    margin: 0;
    font-size: 1.4rem;
    font-weight: 600;
    color: #1c1917;
  }

  .badges {
    list-style: none;
    padding: 0;
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .badge-new {
    padding: 0.35rem 0.75rem;
    background: rgba(179, 39, 31, 0.08);
    color: #b3271f;
    border-radius: 4px;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .weak {
    list-style: none;
    padding: 0;
  }
  .weak li {
    display: flex;
    justify-content: space-between;
    padding: 0.4rem 0.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 4px;
    margin-bottom: 0.3rem;
    align-items: center;
  }
  .weak code {
    font-family: ui-monospace, SFMono-Regular, monospace;
    background: rgba(28, 25, 23, 0.06);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-size: 0.95rem;
  }
  .weak-meta {
    color: #78716c;
    font-size: 0.85rem;
  }

  .next {
    display: flex;
    gap: 1rem;
    align-items: center;
  }
  .next a.primary {
    padding: 0.6rem 1.2rem;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 6px;
    text-decoration: none;
    font-weight: 600;
  }
  .next a:not(.primary) {
    color: #44403c;
    text-decoration: none;
    font-size: 0.95rem;
  }
  .next a:not(.primary):hover {
    color: #b3271f;
  }
</style>
