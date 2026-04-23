<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { currentSession } from '$lib/session-store.svelte';
  import type { LessonListItem } from '$lib/types';

  let lessons = $state<LessonListItem[]>([]);
  let loading = $state(true);
  let starting = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    reload();
  });

  async function reload() {
    loading = true;
    try {
      lessons = await api.listIntroLessons();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function startLesson(id: string) {
    starting = true;
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
      starting = false;
    }
  }
</script>

<section>
  <h2>Úvodní lekce</h2>
  <p class="muted">
    Strukturovaná cesta od domovské řady přes celou klávesnici až k českým
    znakům a interpunkci. Každou lekci můžeš opakovat, co budeš chtít; další
    se odemkne, když dosáhneš cílové rychlosti a přesnosti.
  </p>
</section>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if loading}
  <p class="muted">Načítám…</p>
{:else}
  <ol class="lessons">
    {#each lessons as lesson, i (lesson.id)}
      <li class:passed={lesson.passed} class:locked={!lesson.unlocked}>
        <div class="num">{i + 1}</div>
        <div class="body">
          <h3>
            {lesson.title}
            {#if lesson.passed}<span class="mark">✓ zvládnuto</span>{/if}
            {#if !lesson.unlocked}<span class="mark locked-mark">🔒</span>{/if}
          </h3>
          <p class="sub">{lesson.subtitle}</p>
          <p class="targets">
            Cíl: {lesson.target_wpm.toFixed(0)} WPM · {lesson.target_accuracy.toFixed(0)}% přesnost
            {#if lesson.attempts > 0}
              · tvé maximum: {lesson.best_wpm.toFixed(0)} WPM · {lesson.best_accuracy.toFixed(0)}%
              ({lesson.attempts} pokusů)
            {/if}
          </p>
        </div>
        <button
          class="primary"
          onclick={() => startLesson(lesson.id)}
          disabled={starting || !lesson.unlocked}
        >
          {lesson.passed ? 'Opakovat' : 'Začít'}
        </button>
      </li>
    {/each}
  </ol>
{/if}

<style>
  section {
    margin-bottom: 1.5rem;
  }
  .muted {
    color: #78716c;
    font-size: 0.92rem;
  }
  .error {
    color: #b3271f;
  }
  .lessons {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .lessons li {
    display: grid;
    grid-template-columns: 2.5rem 1fr auto;
    gap: 1rem;
    align-items: center;
    padding: 0.85rem 1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
  }
  .lessons li.passed {
    border-color: rgba(45, 106, 45, 0.25);
    background: rgba(45, 106, 45, 0.04);
  }
  .lessons li.locked {
    opacity: 0.55;
  }
  .num {
    font-size: 1.4rem;
    font-weight: 700;
    color: #b3271f;
    text-align: center;
  }
  h3 {
    margin: 0 0 0.2rem;
    font-size: 1rem;
    color: #1c1917;
  }
  .mark {
    margin-left: 0.5rem;
    color: #2d6a2d;
    font-size: 0.8rem;
    font-weight: 600;
  }
  .mark.locked-mark {
    color: #78716c;
  }
  .sub {
    margin: 0 0 0.25rem;
    font-size: 0.9rem;
    color: #57534e;
  }
  .targets {
    margin: 0;
    font-size: 0.8rem;
    color: #78716c;
  }
  button.primary {
    padding: 0.5rem 1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.9rem;
  }
  button.primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
