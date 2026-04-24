<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { currentSession } from '$lib/session-store.svelte';
  import type { LessonListItem, WeakNgram } from '$lib/types';

  let lessons = $state<LessonListItem[]>([]);
  let weak = $state<WeakNgram[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    reload();
  });

  async function reload() {
    loading = true;
    try {
      [lessons, weak] = await Promise.all([
        api.listIntroLessons(),
        api.getWeakNgrams(10)
      ]);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const nextLesson = $derived(
    lessons.find((l) => !l.passed && l.unlocked) ?? lessons[0] ?? null
  );
  const passedCount = $derived(lessons.filter((l) => l.passed).length);

  async function startLesson(id?: string) {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'intro_lesson',
        alpha: 0,
        target_duration_s: 180,
        lesson_id: id
      });
      if (!plan.sentences.length) {
        error = 'Lekce je prázdná.';
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

  async function startWeakKeys() {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'weak_keys',
        alpha: 0,
        target_duration_s: 300
      });
      if (!plan.sentences.length) {
        error =
          'Zatím nemáme dost dat o tvých slabinách. Zkus pár úvodních lekcí a vrať se.';
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

  async function startDiacritics() {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'diacritics',
        alpha: 0,
        target_duration_s: 180
      });
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

<section class="hero">
  <div>
    <h2>Učím se psát</h2>
    <p class="muted">
      Prsty na klávesnici. Od domovské řady přes celou klávesnici až
      k háčkům a čárkám. Každé cvičení je krátké a cílené.
    </p>
  </div>
  {#if nextLesson}
    <button class="cta" onclick={() => startLesson(nextLesson!.id)} disabled={busy}>
      {busy ? 'Chystám…' : passedCount === 0 ? 'Začít první lekci' : `Pokračovat: ${nextLesson.title}`}
      →
    </button>
  {/if}
</section>

{#if error}
  <p class="error">{error}</p>
{/if}

<section class="drills">
  <button
    type="button"
    class="tile primary"
    onclick={() => startLesson()}
    onkeydown={(e) => e.key === 'Enter' && startLesson()}

    tabindex="0"
    aria-disabled={busy}
  >
    <span class="tile-icon">📚</span>
    <h3>Úvodní lekce</h3>
    <p>
      {passedCount}/{lessons.length} zvládnutých · doporučené pro začátek
    </p>
    <p class="subtitle">
      Strukturovaná cesta: ASDF → celá klávesnice → diakritika → věty.
      Každá lekce má cíl rychlosti a přesnosti.
    </p>
    <span class="tile-cta">Otevřít ladder →</span>
  </button>

  <button
    type="button"
    class="tile"
    onclick={startWeakKeys}
    onkeydown={(e) => e.key === 'Enter' && startWeakKeys()}

    tabindex="0"
    aria-disabled={busy}
    class:dimmed={weak.length === 0}
  >
    <span class="tile-icon">🎯</span>
    <h3>Tvá slabá místa</h3>
    {#if weak.length === 0}
      <p>Ještě nemáme data — zatrénuj pár lekcí.</p>
    {:else}
      <p>
        Nejslabší:
        {#each weak.slice(0, 3) as w, i}
          <code>{w.ngram.replace(/ /g, '␣')}</code>{#if i < Math.min(weak.length, 3) - 1}, {/if}
        {/each}
      </p>
    {/if}
    <p class="subtitle">
      Věty husté na tvé aktuálně nejslabší kombinace — izolovaný drill,
      řízený tvými daty.
    </p>
    <span class="tile-cta">Trénovat slabiny →</span>
  </button>

  <button
    type="button"
    class="tile"
    onclick={startDiacritics}
    onkeydown={(e) => e.key === 'Enter' && startDiacritics()}

    tabindex="0"
    aria-disabled={busy}
  >
    <span class="tile-icon">č</span>
    <h3>Diakritika</h3>
    <p>č š ř ě ů ý á í — český háček & čárka</p>
    <p class="subtitle">
      Generovaný drill pro české znaky. Funguje i bez tvých materiálů.
    </p>
    <span class="tile-cta">Procvičit háčky →</span>
  </button>
</section>

<section>
  <h3 class="section-head">Lekce podle pořadí</h3>
  {#if loading}
    <p class="muted">Načítám…</p>
  {:else}
    <ol class="lessons">
      {#each lessons as lesson, i (lesson.id)}
        <li class:passed={lesson.passed} class:locked={!lesson.unlocked}>
          <div class="num">{i + 1}</div>
          <div class="body">
            <h4>
              {lesson.title}
              {#if lesson.passed}<span class="mark">✓</span>{/if}
              {#if !lesson.unlocked}<span class="mark locked-mark">🔒</span>{/if}
            </h4>
            <p class="sub">{lesson.subtitle}</p>
            <p class="targets">
              Cíl: {lesson.target_wpm.toFixed(0)} WPM · {lesson.target_accuracy.toFixed(0)}% přesnost
              {#if lesson.attempts > 0}
                · tvé: {lesson.best_wpm.toFixed(0)} · {lesson.best_accuracy.toFixed(0)}%
              {/if}
            </p>
          </div>
          <button
            class="small"
            onclick={() => startLesson(lesson.id)}
            disabled={busy || !lesson.unlocked}
          >
            {lesson.passed ? 'Opakovat' : 'Začít'}
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .hero {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    justify-content: space-between;
    padding: 1.25rem 1.5rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
    margin-bottom: 1.5rem;
  }
  .hero h2 {
    margin: 0 0 0.25rem;
    font-size: 1.5rem;
  }
  .hero p {
    margin: 0;
    color: #57534e;
    max-width: 34rem;
  }
  .cta {
    padding: 0.7rem 1.1rem;
    background: #b3271f;
    color: #fffaf2;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    font-weight: 600;
    white-space: nowrap;
  }
  .cta:disabled { opacity: 0.5; cursor: not-allowed; }

  .drills {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 0.75rem;
    margin-bottom: 2rem;
  }
  .tile {
    text-align: left;
    padding: 1rem 1.1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.1);
    border-radius: 10px;
    cursor: pointer;
    transition: border-color 120ms, transform 120ms;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .tile:hover { border-color: rgba(179, 39, 31, 0.4); transform: translateY(-1px); }
  .tile.primary {
    border-color: #b3271f;
    background: linear-gradient(180deg, rgba(179, 39, 31, 0.08) 0%, #fffaf2 60%);
  }
  .tile.dimmed { opacity: 0.7; }
  .tile-icon {
    font-size: 1.6rem;
    font-family: ui-monospace, monospace;
  }
  .tile h3 {
    margin: 0;
    color: #b3271f;
    font-size: 1.05rem;
  }
  .tile p {
    margin: 0;
    font-size: 0.9rem;
    color: #1c1917;
  }
  .tile .subtitle {
    font-size: 0.82rem;
    color: #78716c;
    font-style: italic;
  }
  .tile code {
    background: rgba(28, 25, 23, 0.06);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  .tile-cta {
    margin-top: auto;
    color: #b3271f;
    font-size: 0.85rem;
    font-weight: 600;
  }

  .section-head {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #78716c;
    margin-bottom: 0.75rem;
  }
  .lessons {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .lessons li {
    display: grid;
    grid-template-columns: 2.2rem 1fr auto;
    gap: 1rem;
    align-items: center;
    padding: 0.7rem 0.9rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 7px;
  }
  .lessons li.passed { border-color: rgba(45, 106, 45, 0.25); background: rgba(45, 106, 45, 0.04); }
  .lessons li.locked { opacity: 0.55; }
  .num { font-size: 1.2rem; font-weight: 700; color: #b3271f; text-align: center; }
  h4 { margin: 0 0 0.15rem; font-size: 0.95rem; }
  .mark { margin-left: 0.4rem; color: #2d6a2d; font-size: 0.8rem; font-weight: 600; }
  .mark.locked-mark { color: #78716c; }
  .sub { margin: 0 0 0.2rem; font-size: 0.82rem; color: #57534e; }
  .targets { margin: 0; font-size: 0.75rem; color: #78716c; }
  button.small {
    padding: 0.4rem 0.8rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  button.small:disabled { opacity: 0.4; cursor: not-allowed; }
  .muted { color: #78716c; font-size: 0.9rem; }
  .error { color: #b3271f; }
</style>
