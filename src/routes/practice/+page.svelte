<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { modes } from '$lib/mode-meta';
  import { currentSession } from '$lib/session-store.svelte';
  import type { ChapterInfo, ContentStrategy, PracticeMode } from '$lib/types';

  let selected = $state<PracticeMode>('content');
  let contentStrategy = $state<ContentStrategy>('across');
  let chapters = $state<ChapterInfo[]>([]);
  let chapterId = $state<string>('');

  let alpha = $state(0.7);
  let duration = $state(300); // seconds
  let query = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);
  let rephraseOn = $state(false);
  let rephraseStyle = $state<'keystrokes' | 'thing_explainer' | 'both'>('keystrokes');
  let anthropicPresent = $state(false);

  const meta = $derived(modes.find((m) => m.code === selected)!);

  $effect(() => {
    // Load chapters once so the Chapter strategy has something to pick.
    api
      .listChapters()
      .then((c) => (chapters = c))
      .catch(() => {});
    api
      .anthropicKeyPresent()
      .then((v) => (anthropicPresent = v))
      .catch(() => {});
  });

  const chaptersByDoc = $derived(() => {
    const groups = new Map<string, ChapterInfo[]>();
    for (const c of chapters) {
      const key = shortDocName(c.source_path);
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(c);
    }
    return groups;
  });

  function shortDocName(path: string): string {
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1].replace(/\.(md|markdown|txt)$/i, '');
  }

  async function startSession() {
    busy = true;
    error = null;
    try {
      const isContent = selected === 'content';
      const plan = await api.createSession({
        mode: selected,
        alpha: meta.supportsAlpha ? alpha : isContent ? 1.0 : 0.0,
        target_duration_s: duration,
        query:
          (isContent && contentStrategy !== 'chapter') || meta.supportsQuery
            ? query.trim() || undefined
            : undefined,
        pinned_source_prefixes: [],
        content_strategy: isContent ? contentStrategy : undefined,
        chapter_id:
          isContent && contentStrategy === 'chapter' && chapterId ? chapterId : undefined,
        rephrase: isContent && rephraseOn && anthropicPresent,
        rephrase_style: isContent && rephraseOn ? rephraseStyle : undefined,
        language: 'cs'
      });
      if (!plan.sentences.length) {
        error = explainEmpty(selected, contentStrategy);
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

  function explainEmpty(
    mode: PracticeMode,
    strat: ContentStrategy
  ): string {
    if (mode === 'content' && strat === 'chapter') {
      return 'Vyber kapitolu ze seznamu. Pokud žádná není, přidej složku s Markdown soubory obsahujícími nadpisy (#, ##).';
    }
    if (mode === 'content') {
      return 'Pro tento dotaz jsme nic nenašli. Zkus jiné klíčové slovo nebo přepni na jinou strategii.';
    }
    return 'Pro tento mód není dost materiálu. Zkus přidat složku s poznámkami.';
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

{#if selected === 'content'}
  <section class="sub-strategy">
    <h3>Jak vybíráme obsah?</h3>
    <div class="strategies">
      <button
        type="button"
        class="strategy"
        class:active={contentStrategy === 'across'}
        onclick={() => (contentStrategy = 'across')}
      >
        <strong>Napříč materiály</strong>
        <span>Věty zmiňující tvé téma ze všech dokumentů — propojuje znalosti.</span>
      </button>
      <button
        type="button"
        class="strategy"
        class:active={contentStrategy === 'chapter'}
        onclick={() => (contentStrategy = 'chapter')}
      >
        <strong>Celá kapitola</strong>
        <span>Všechny věty jedné kapitoly po sobě — pro ucelené čtení.</span>
      </button>
      <button
        type="button"
        class="strategy"
        class:active={contentStrategy === 'exam_prep'}
        onclick={() => (contentStrategy = 'exam_prep')}
      >
        <strong>Příprava na zkoušku</strong>
        <span>Popiš, z čeho budeš zkoušený, dostaneš klíčové kapitoly.</span>
      </button>
    </div>
    {#if contentStrategy === 'exam_prep'}
      <p class="note">
        Zatím používáme klíčová slova. S embeddings (Týden 2) se přesnost
        dramaticky zvýší — i skryté souvislosti najdeme.
      </p>
    {/if}
  </section>
{/if}

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

  {#if selected === 'content' && contentStrategy === 'chapter'}
    <div class="row">
      <label>Kapitola
        <select bind:value={chapterId}>
          <option value="">— vyber kapitolu —</option>
          {#each [...chaptersByDoc()] as [docName, docChapters]}
            <optgroup label={docName}>
              {#each docChapters as ch}
                <option value={ch.id}>
                  {ch.section} · {ch.sentence_count} vět
                </option>
              {/each}
            </optgroup>
          {/each}
        </select>
      </label>
      {#if chapters.length === 0}
        <p class="muted small">
          Žádné kapitoly. Markdown soubor musí mít nadpisy (<code># …</code> nebo
          <code>## …</code>).
        </p>
      {/if}
    </div>
  {:else if (selected === 'content' && contentStrategy !== 'chapter') || meta.supportsQuery}
    <div class="row">
      <label>
        {#if selected === 'content' && contentStrategy === 'exam_prep'}
          Co budeš zkoušený?
        {:else}
          Co by tě zajímalo? (volitelné)
        {/if}
        <input
          type="text"
          placeholder={selected === 'content' && contentStrategy === 'exam_prep'
            ? 'např. "Great Depression a New Deal, Roosevelt, americká politika 30. let"'
            : 'např. Habsburkové, derivace, perfektum…'}
          bind:value={query}
        />
      </label>
    </div>
  {/if}

  {#if selected === 'content'}
    <div class="rephrase-card">
      <label class="toggle">
        <input
          type="checkbox"
          bind:checked={rephraseOn}
          disabled={!anthropicPresent}
        />
        <span>
          <strong>Remix (LLM přepis)</strong> —
          {#if anthropicPresent}
            Claude Haiku si přečte větu, zachová fakta a vlastní jména, a
            přepíše ji. Původní verzi uvidíš při psaní pod tlačítkem.
          {:else}
            Nejdřív ulož Anthropic klíč v <a href="/settings">Nastavení</a>.
          {/if}
        </span>
      </label>

      {#if rephraseOn && anthropicPresent}
        <div class="style-grid">
          <button
            type="button"
            class="style"
            class:active={rephraseStyle === 'keystrokes'}
            onclick={() => (rephraseStyle = 'keystrokes')}
          >
            <strong>Klávesy na tvé úrovni</strong>
            <span>
              Claude přepíše věty tak, aby obsahovaly víc písmenových
              kombinací, které právě teď rozjíždíš — ne úplné slabiny, ale
              ty, co tě táhnou vpřed. Zóna proximálního rozvoje.
            </span>
          </button>
          <button
            type="button"
            class="style"
            class:active={rephraseStyle === 'both'}
            onclick={() => (rephraseStyle = 'both')}
          >
            <strong>Klávesy + jednodušší slovník</strong>
            <span>
              Navíc přepíše složitější slova na běžnější. Užitečné, když
              textu ještě úplně nerozumíš.
            </span>
          </button>
          <button
            type="button"
            class="style"
            class:active={rephraseStyle === 'thing_explainer'}
            onclick={() => (rephraseStyle = 'thing_explainer')}
          >
            <strong>Jen jednodušší slovník</strong>
            <span>
              Bonusový režim: přeformuluje do ~1000 nejběžnějších slov
              (jako Munroeův <em>Thing Explainer</em>), klávesy neřeší.
            </span>
          </button>
        </div>
      {/if}
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
  .small {
    font-size: 0.8rem;
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

  .sub-strategy {
    padding: 1rem 1.25rem;
    background: rgba(179, 39, 31, 0.04);
    border: 1px solid rgba(179, 39, 31, 0.15);
    border-radius: 8px;
  }
  .strategies {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.5rem;
  }
  .strategy {
    text-align: left;
    padding: 0.75rem;
    border: 1px solid rgba(28, 25, 23, 0.1);
    background: #fffaf2;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
  }
  .strategy.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.08);
  }
  .strategy strong {
    display: block;
    margin-bottom: 0.2rem;
    color: #b3271f;
    font-size: 0.92rem;
  }
  .strategy span {
    font-size: 0.82rem;
    color: #57534e;
  }
  .note {
    margin: 0.75rem 0 0;
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
  code {
    background: rgba(28, 25, 23, 0.05);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  .rephrase-card {
    margin: 0.75rem 0;
    padding: 0.75rem 1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 6px;
  }
  .toggle {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
    color: #44403c;
    font-size: 0.9rem;
  }
  .toggle strong {
    color: #b3271f;
  }
  .toggle a {
    color: #b3271f;
  }
  .style-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  .style {
    text-align: left;
    padding: 0.6rem 0.75rem;
    border: 1px solid rgba(28, 25, 23, 0.1);
    background: transparent;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
  }
  .style.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.08);
  }
  .style strong {
    display: block;
    margin-bottom: 0.2rem;
    color: #b3271f;
    font-size: 0.9rem;
  }
  .style span {
    font-size: 0.8rem;
    color: #57534e;
  }
</style>
