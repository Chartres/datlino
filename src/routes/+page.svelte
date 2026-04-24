<script lang="ts">
  import { api } from '$lib/api';
  import { profile } from '$lib/profile.svelte';
  import type { IndexStatus, LessonListItem } from '$lib/types';

  let status = $state<IndexStatus | null>(null);
  let lessons = $state<LessonListItem[]>([]);
  let loading = $state(true);

  $effect(() => {
    void reload();
  });

  async function reload() {
    loading = true;
    try {
      [status, lessons] = await Promise.all([
        api.indexStatus(),
        api.listIntroLessons()
      ]);
    } finally {
      loading = false;
    }
  }

  const passedLessons = $derived(lessons.filter((l) => l.passed).length);
  const totalLessons = $derived(lessons.length);
  const hasLibrary = $derived((status?.document_count ?? 0) > 0);
  const totalSessions = $derived(profile.data?.total_sessions ?? 0);
  const isFirstRun = $derived(totalSessions === 0 && !hasLibrary);

  function greetingKind(): string {
    const hour = new Date().getHours();
    if (hour < 9) return 'Dobré ráno';
    if (hour < 17) return 'Ahoj';
    return 'Dobrý večer';
  }
</script>

<section class="hello">
  <div>
    <h2>{greetingKind()}{profile.data && profile.data.total_sessions > 0 ? ' zpět' : ''}.</h2>
    {#if isFirstRun}
      <p class="lead">
        Datlino tě učí psát naslepo <em>a zároveň</em> se učit tvůj
        studijní obsah. Dvě cesty — pojď se podívat.
      </p>
    {:else if !hasLibrary}
      <p class="lead">
        Pokračuj v úvodním tréninku, nebo přidej pár poznámek a začni
        psát reálný obsah.
      </p>
    {:else}
      <p class="lead">
        Máš {status?.document_count} souborů připravených ke studiu.
        Kam dnes?
      </p>
    {/if}
  </div>
  {#if profile.data && profile.data.total_sessions > 0}
    <div class="strip">
      <span><strong>L{profile.data.level}</strong> · {profile.data.total_xp} XP</span>
      <span>🔥 {profile.data.current_streak} dní v řadě</span>
      {#if profile.data.wpm_baseline}
        <span>{profile.data.wpm_baseline.toFixed(0)} WPM průměr</span>
      {/if}
    </div>
  {/if}
</section>

<section class="doors">
  <a class="door learn" href="/learn">
    <span class="door-icon">⌨️</span>
    <h3>Učím se psát</h3>
    <p>
      Úvodní lekce od domovské řady po háčky, drill na tvé slabé klávesy.
      Pro začátečníky i pokročilé.
    </p>
    {#if totalLessons > 0}
      <div class="progress-bar" aria-label="Průběh úvodní kurzu">
        <div class="fill" style={`width: ${(passedLessons / totalLessons) * 100}%`}></div>
      </div>
      <span class="progress-label">
        {passedLessons} z {totalLessons} lekcí zvládnutých
      </span>
    {/if}
    <span class="door-cta">Otevřít →</span>
  </a>

  <a class="door study" href="/study">
    <span class="door-icon">📖</span>
    <h3>Učím se obsah</h3>
    <p>
      Piš věty ze svých poznámek. Napříč materiály, celé kapitoly, nebo
      příprava na konkrétní zkoušku.
    </p>
    <div class="door-meta">
      {#if hasLibrary}
        {status?.document_count} souborů · {status?.chunk_count} vět připraveno
      {:else}
        <em>Zatím prázdno — přidej složku a začni.</em>
      {/if}
    </div>
    <span class="door-cta">Otevřít →</span>
  </a>
</section>

{#if profile.data && profile.data.total_sessions > 0}
  <section class="recent">
    <h3 class="section-head">Naposledy</h3>
    <p class="muted small">
      Podrobnou historii, slabé klávesy a WPM křivku najdeš na
      <a href="/progress">Pokroku</a>.
    </p>
  </section>
{/if}

<style>
  .hello {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    justify-content: space-between;
    padding: 1.5rem 1.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 12px;
    margin-bottom: 1.5rem;
  }
  .hello h2 { margin: 0 0 0.5rem; font-size: 1.6rem; }
  .lead { margin: 0; color: #57534e; max-width: 38rem; line-height: 1.55; }
  .strip {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.9rem;
    color: #44403c;
    text-align: right;
  }
  .strip strong { color: #b3271f; }

  .doors {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 1rem;
    margin-bottom: 2rem;
  }
  .door {
    text-decoration: none;
    color: inherit;
    padding: 1.5rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.1);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    transition: transform 140ms, border-color 140ms, box-shadow 140ms;
  }
  .door:hover {
    transform: translateY(-2px);
    border-color: rgba(179, 39, 31, 0.5);
    box-shadow: 0 4px 24px rgba(179, 39, 31, 0.08);
  }
  .door.learn { border-top: 3px solid #b3271f; }
  .door.study { border-top: 3px solid #292524; }
  .door-icon { font-size: 2.2rem; }
  .door h3 { margin: 0; font-size: 1.3rem; color: #1c1917; }
  .door p { margin: 0; color: #57534e; font-size: 0.95rem; line-height: 1.45; }
  .door-meta { font-size: 0.85rem; color: #78716c; }
  .door-cta {
    margin-top: auto;
    color: #b3271f;
    font-weight: 600;
    font-size: 0.95rem;
  }
  .progress-bar {
    height: 6px;
    background: rgba(28, 25, 23, 0.08);
    border-radius: 3px;
    overflow: hidden;
    margin-top: 0.5rem;
  }
  .progress-bar .fill {
    height: 100%;
    background: #b3271f;
    transition: width 200ms;
  }
  .progress-label { font-size: 0.78rem; color: #78716c; }

  .section-head {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #78716c;
    margin: 0 0 0.5rem;
  }
  .recent a { color: #b3271f; }
  .muted { color: #78716c; }
  .small { font-size: 0.85rem; }
</style>
