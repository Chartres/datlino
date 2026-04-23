<script lang="ts">
  import { api } from '$lib/api';
  import { profile } from '$lib/profile.svelte';
  import { badgeLabels } from '$lib/mode-meta';
  import type { SessionHistoryRow, WeakNgram } from '$lib/types';

  let history = $state<SessionHistoryRow[]>([]);
  let weak = $state<WeakNgram[]>([]);
  let loading = $state(true);

  $effect(() => {
    void reload();
  });

  async function reload() {
    loading = true;
    try {
      [history, weak] = await Promise.all([api.getHistory(30), api.getWeakNgrams(20)]);
    } finally {
      loading = false;
    }
  }

  function modeLabel(m: string): string {
    const map: Record<string, string> = {
      content: 'Moje materiály',
      warmup: 'Zahřívání',
      diacritics: 'Diakritika',
      weak_keys: 'Slabá místa',
      hybrid: 'Mix'
    };
    return map[m] ?? m;
  }

  function formatDate(unix: number): string {
    const d = new Date(unix * 1000);
    return d.toLocaleDateString('cs-CZ', {
      day: 'numeric',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  // Sparkline helper — scaled to [0, 100] for WPM.
  const wpmSeries = $derived(
    history
      .filter((h) => h.summary)
      .map((h) => h.summary!.wpm)
      .reverse() // oldest → newest for left-to-right chart
  );
  const maxSeries = $derived(Math.max(60, ...wpmSeries));
</script>

<section>
  <h2>Pokrok</h2>
  <p class="muted">
    Všechny tvé scores, slabá místa a odznaky na jednom místě. Nic ti tady
    nikdo neukradne — streaks vycházejí z reálného psaní, nikoli z
    přihlášení do aplikace.
  </p>
</section>

{#if profile.data}
  <section class="profile-grid">
    <div class="card">
      <h3>Úroveň</h3>
      <p class="big">L{profile.data.level}</p>
      <p class="muted">{profile.data.total_xp} XP · {profile.data.total_sessions} sezení</p>
    </div>
    <div class="card">
      <h3>Série</h3>
      <p class="big">🔥 {profile.data.current_streak}</p>
      <p class="muted">nejdelší: {profile.data.longest_streak}</p>
    </div>
    <div class="card">
      <h3>WPM baseline</h3>
      <p class="big">
        {profile.data.wpm_baseline !== null
          ? profile.data.wpm_baseline.toFixed(0)
          : '—'}
      </p>
      <p class="muted">vážený průměr posledních sezení</p>
    </div>
    <div class="card">
      <h3>Přesnost</h3>
      <p class="big">
        {profile.data.accuracy_baseline !== null
          ? `${profile.data.accuracy_baseline.toFixed(0)} %`
          : '—'}
      </p>
      <p class="muted">vážený průměr</p>
    </div>
  </section>
{/if}

{#if wpmSeries.length > 1}
  <section>
    <h3>WPM v čase</h3>
    <svg class="spark" viewBox={`0 0 ${Math.max(wpmSeries.length * 16, 200)} 60`} preserveAspectRatio="none">
      <polyline
        fill="none"
        stroke="#b3271f"
        stroke-width="2"
        points={wpmSeries
          .map((v, i) => `${i * 16},${60 - (v / maxSeries) * 55}`)
          .join(' ')}
      />
      {#each wpmSeries as v, i}
        <circle cx={i * 16} cy={60 - (v / maxSeries) * 55} r="2.5" fill="#b3271f" />
      {/each}
    </svg>
  </section>
{/if}

<section>
  <h3>Slabá místa</h3>
  {#if loading}
    <p class="muted">Načítám…</p>
  {:else if weak.length === 0}
    <p class="muted">
      Potřebujeme pár sezení, abychom tě líp poznali. Zkus mód Diakritika —
      tam si nasbíráš dostatek dat.
    </p>
  {:else}
    <ul class="weak">
      {#each weak as w}
        <li>
          <code>{w.ngram.replace(/ /g, '␣')}</code>
          <div class="bar">
            <div class="bar-fill" style="width: {w.weakness * 100}%"></div>
          </div>
          <span class="meta">
            {w.ema_latency_ms.toFixed(0)} ms · {(w.ema_error_rate * 100).toFixed(0)} % chyb
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section>
  <h3>Odznaky</h3>
  {#if profile.data && profile.data.badges.length}
    <ul class="badges">
      {#each profile.data.badges as code}
        <li class="badge">{badgeLabels[code] ?? code}</li>
      {/each}
    </ul>
  {:else}
    <p class="muted">Zatím žádné. První sezení ti jeden přinese.</p>
  {/if}
</section>

<section>
  <h3>Historie sezení</h3>
  {#if history.length === 0}
    <p class="muted">Žádná dokončená sezení.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Kdy</th>
          <th>Mód</th>
          <th>WPM</th>
          <th>Přesnost</th>
          <th>XP</th>
        </tr>
      </thead>
      <tbody>
        {#each history as row (row.session_id)}
          <tr>
            <td>{formatDate(row.created_at)}</td>
            <td>{modeLabel(row.mode)}</td>
            <td>{row.summary ? row.summary.wpm.toFixed(0) : '—'}</td>
            <td>{row.summary ? row.summary.accuracy_pct.toFixed(0) + '%' : '—'}</td>
            <td>+{row.xp_earned}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
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
    font-size: 0.9rem;
  }

  .profile-grid {
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
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #78716c;
  }
  .big {
    margin: 0;
    font-size: 1.4rem;
    font-weight: 600;
  }

  .spark {
    width: 100%;
    height: 60px;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 6px;
    padding: 4px;
  }

  .weak {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .weak li {
    display: grid;
    grid-template-columns: 3rem 1fr auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.4rem 0.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 4px;
    margin-bottom: 0.3rem;
  }
  .weak code {
    background: rgba(28, 25, 23, 0.06);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    text-align: center;
  }
  .bar {
    height: 6px;
    background: rgba(28, 25, 23, 0.08);
    border-radius: 3px;
    overflow: hidden;
  }
  .bar-fill {
    height: 100%;
    background: #b3271f;
  }
  .meta {
    color: #78716c;
    font-size: 0.8rem;
    white-space: nowrap;
  }

  .badges {
    list-style: none;
    padding: 0;
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .badge {
    padding: 0.35rem 0.75rem;
    background: rgba(179, 39, 31, 0.08);
    color: #b3271f;
    border-radius: 4px;
    font-size: 0.85rem;
    font-weight: 600;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    background: #fffaf2;
    border-radius: 6px;
    overflow: hidden;
  }
  th,
  td {
    text-align: left;
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid rgba(28, 25, 23, 0.06);
    font-size: 0.9rem;
  }
  th {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #78716c;
    font-weight: 600;
    background: rgba(28, 25, 23, 0.03);
  }
</style>
