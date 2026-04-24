<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { currentSession } from '$lib/session-store.svelte';
  import type {
    ChapterInfo,
    ContentStrategy,
    DocumentInfo,
    IndexStatus
  } from '$lib/types';

  type Strategy = ContentStrategy;

  let status = $state<IndexStatus | null>(null);
  let documents = $state<DocumentInfo[]>([]);
  let chapters = $state<ChapterInfo[]>([]);
  let docFilter = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  let strategy = $state<Strategy>('across');
  let chapterId = $state<string>('');
  let query = $state('');
  let duration = $state(300);
  let advanced = $state(false);
  let alpha = $state(0.7);
  let rephrase = $state(false);
  let anthropicPresent = $state(false);

  $effect(() => {
    reload();
  });

  async function reload() {
    try {
      [status, documents, chapters, anthropicPresent] = await Promise.all([
        api.indexStatus(),
        api.listDocuments(),
        api.listChapters(),
        api.anthropicKeyPresent()
      ]);
    } catch (e) {
      error = String(e);
    }
  }

  const visibleDocs = $derived.by(() => {
    const f = docFilter.toLowerCase().trim();
    if (!f) return documents;
    return documents.filter((d) => d.source_path.toLowerCase().includes(f));
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
    return path
      .split(/[\\/]/)
      .pop()!
      .replace(/\.(md|markdown|txt|pdf)$/i, '');
  }

  async function pickAndAddFolder() {
    error = null;
    try {
      const picked = await api.pickFolder();
      if (!picked) return;
      busy = true;
      await api.addWatchedFolder(picked);
      await reload();
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function pickAndIngestFile() {
    error = null;
    try {
      const picked = await api.pickFile();
      if (!picked) return;
      busy = true;
      await api.ingestSingleFile(picked);
      await reload();
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function drillDocument(doc: DocumentInfo) {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'content',
        alpha: 1.0,
        target_duration_s: 600,
        document_id: doc.id
      });
      if (!plan.sentences.length) {
        error = 'Dokument nemá žádné věty k trénování.';
        return;
      }
      currentSession.plan = plan;
      currentSession.summary = null;
      await goto('/practice/session');
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function startSession() {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'content',
        alpha: advanced ? alpha : 1.0,
        target_duration_s: duration,
        content_strategy: strategy,
        query: strategy !== 'chapter' ? query.trim() || undefined : undefined,
        chapter_id: strategy === 'chapter' && chapterId ? chapterId : undefined,
        rephrase: rephrase && anthropicPresent,
        rephrase_style: rephrase ? 'keystrokes' : undefined,
        language: 'cs'
      });
      if (!plan.sentences.length) {
        error = emptyMessage();
        return;
      }
      currentSession.plan = plan;
      currentSession.summary = null;
      await goto('/practice/session');
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  function emptyMessage(): string {
    if (documents.length === 0)
      return 'Zatím nemáš žádné materiály. Přidej složku nebo soubor níže.';
    if (strategy === 'chapter' && !chapterId)
      return 'Vyber kapitolu ze seznamu.';
    return 'Nenašli jsme nic. Zkus jiné klíčové slovo nebo jinou strategii.';
  }
</script>

<section class="hero">
  <div>
    <h2>Učím se obsah</h2>
    <p class="muted">
      Piš reálné věty ze svých poznámek. Propoj klávesnici s tím, co se
      učíš na maturitu, zkoušku nebo do školy.
    </p>
  </div>
  {#if status}
    <div class="stats">
      <strong>{status.document_count}</strong> souborů ·
      <strong>{status.chunk_count}</strong> vět
    </div>
  {/if}
</section>

{#if error}
  <p class="error">{error}</p>
{/if}

{#if documents.length === 0}
  <section class="empty-invite">
    <h3>Ještě prázdno.</h3>
    <p class="muted">
      Nejjednodušší začátek: ukaž Datlinu složku s poznámkami. Může to
      být tvoje hromada <code>.md</code> z Obsidianu, složka s PDF
      učebnicemi, export z GoodNotes, nebo cokoli textového.
    </p>
    <div class="empty-actions">
      <button class="primary" onclick={pickAndAddFolder} disabled={busy}>
        Přidat složku
      </button>
      <button class="secondary" onclick={pickAndIngestFile} disabled={busy}>
        Přidat jeden soubor
      </button>
    </div>
  </section>
{/if}

{#if documents.length > 0}
  <section class="picker">
    <h3 class="section-head">Jak chceš studovat?</h3>
    <div class="strategies">
      <button
        class="strategy"
        class:active={strategy === 'across'}
        onclick={() => (strategy = 'across')}
      >
        <strong>Napříč materiály</strong>
        <span>Hledá téma ve všech dokumentech. Propojuje souvislosti.</span>
      </button>
      <button
        class="strategy"
        class:active={strategy === 'chapter'}
        onclick={() => (strategy = 'chapter')}
      >
        <strong>Celá kapitola</strong>
        <span>Čti jednu kapitolu od začátku do konce.</span>
      </button>
      <button
        class="strategy"
        class:active={strategy === 'exam_prep'}
        onclick={() => (strategy = 'exam_prep')}
      >
        <strong>Příprava na zkoušku</strong>
        <span>Popiš téma, dostaneš relevantní kapitoly.</span>
      </button>
    </div>

    <div class="controls">
      <label>Délka
        <select bind:value={duration}>
          <option value={120}>2 min</option>
          <option value={300}>5 min</option>
          <option value={600}>10 min</option>
          <option value={1200}>20 min</option>
        </select>
      </label>

      {#if strategy === 'chapter'}
        <label>Kapitola
          <select bind:value={chapterId}>
            <option value="">— vyber —</option>
            {#each [...chaptersByDoc()] as [doc, list]}
              <optgroup label={doc}>
                {#each list as ch}
                  <option value={ch.id}>{ch.section} · {ch.sentence_count} vět</option>
                {/each}
              </optgroup>
            {/each}
          </select>
        </label>
      {:else}
        <label class="grow">
          {strategy === 'exam_prep' ? 'Co budeš zkoušený?' : 'Téma (volitelné)'}
          <input
            type="text"
            bind:value={query}
            placeholder={strategy === 'exam_prep'
              ? 'např. Velká hospodářská krize, New Deal, Roosevelt'
              : 'např. Habsburkové, derivace, perfektum'}
          />
        </label>
      {/if}
    </div>

    <details class="advanced">
      <summary>Pokročilé</summary>
      <div class="adv-grid">
        <label class="adv-toggle">
          <input type="checkbox" bind:checked={advanced} />
          α-mix: obsah × trénink slabých klávesových kombinací
        </label>
        {#if advanced}
          <div class="alpha">
            <label for="alpha">
              <strong>{Math.round(alpha * 100)}% obsah · {Math.round((1 - alpha) * 100)}% trénink</strong>
            </label>
            <input id="alpha" type="range" min="0" max="1" step="0.05" bind:value={alpha} />
          </div>
        {/if}
        <label class="adv-toggle">
          <input type="checkbox" bind:checked={rephrase} disabled={!anthropicPresent} />
          Remix (LLM přepíše věty s cílem na tvé slabiny) —
          {anthropicPresent
            ? 'zapnuto, Anthropic klíč je uložen'
            : 'nejdřív přihlásit Claude účet v Nastavení'}
        </label>
      </div>
    </details>

    <button class="primary big" onclick={startSession} disabled={busy}>
      {busy ? 'Chystám sezení…' : 'Začít'}
    </button>
  </section>

  <section>
    <div class="docs-head">
      <h3 class="section-head">Dokumenty</h3>
      {#if documents.length > 6}
        <input
          type="text"
          placeholder="Filtrovat…"
          bind:value={docFilter}
          class="doc-filter"
        />
      {/if}
    </div>
    <p class="muted small">
      Klikni na soubor pro trénink celého dokumentu po sobě — bez
      vyhledávání.
    </p>
    <ul class="docs">
      {#each visibleDocs as doc (doc.id)}
        <li>
          <div class="doc-info">
            <span class="doc-name">{shortDocName(doc.source_path)}</span>
            <span class="doc-meta">{doc.chunk_count} vět · {doc.kind.toUpperCase()}</span>
          </div>
          <button class="secondary" onclick={() => drillDocument(doc)} disabled={busy}>
            Trénovat celý
          </button>
        </li>
      {/each}
    </ul>
    <div class="add-more">
      <button class="link" onclick={pickAndAddFolder} disabled={busy}>
        + Přidat složku
      </button>
      <button class="link" onclick={pickAndIngestFile} disabled={busy}>
        + Přidat soubor
      </button>
    </div>
  </section>
{/if}

<style>
  .hero {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 1.25rem 1.5rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
    margin-bottom: 1.5rem;
  }
  .hero h2 { margin: 0 0 0.25rem; font-size: 1.5rem; }
  .hero p { margin: 0; color: #57534e; max-width: 34rem; }
  .stats { color: #78716c; font-size: 0.9rem; }
  .stats strong { color: #b3271f; }

  .empty-invite {
    padding: 1.5rem;
    background: #fffaf2;
    border: 1px dashed rgba(179, 39, 31, 0.3);
    border-radius: 10px;
    text-align: center;
  }
  .empty-invite h3 { margin: 0 0 0.4rem; color: #1c1917; }
  .empty-actions { display: flex; gap: 0.5rem; justify-content: center; margin-top: 0.75rem; }

  .section-head {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #78716c;
    margin: 0 0 0.75rem;
  }
  .picker { margin-bottom: 2rem; }
  .strategies {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .strategy {
    text-align: left;
    padding: 0.7rem 0.9rem;
    border: 1px solid rgba(28, 25, 23, 0.1);
    background: #fffaf2;
    border-radius: 7px;
    cursor: pointer;
    font: inherit;
  }
  .strategy.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.06);
  }
  .strategy strong { display: block; margin-bottom: 0.2rem; color: #b3271f; font-size: 0.95rem; }
  .strategy span { font-size: 0.8rem; color: #57534e; }

  .controls {
    display: flex;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
    flex-wrap: wrap;
  }
  .controls label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.85rem;
    color: #44403c;
  }
  .controls label.grow { flex: 1; min-width: 260px; }
  select, input[type='text'] {
    padding: 0.45rem 0.6rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 4px;
    font: inherit;
    background: #fffaf2;
  }
  .advanced {
    margin: 0.5rem 0 1rem;
    padding: 0.5rem 0.75rem;
    background: rgba(28, 25, 23, 0.03);
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .advanced summary { cursor: pointer; color: #57534e; }
  .adv-grid { display: flex; flex-direction: column; gap: 0.5rem; padding-top: 0.5rem; }
  .adv-toggle { display: flex; gap: 0.4rem; align-items: flex-start; }
  .alpha { padding: 0.5rem 0.75rem; background: #fffaf2; border-radius: 5px; }
  .alpha input[type='range'] { width: 100%; }

  button.primary {
    padding: 0.5rem 1.1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
    font-weight: 600;
  }
  button.primary.big { padding: 0.7rem 1.3rem; font-size: 1rem; }
  button.primary:disabled, button.secondary:disabled { opacity: 0.5; cursor: not-allowed; }
  button.secondary {
    padding: 0.45rem 0.9rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    color: #44403c;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  button.link {
    background: none;
    border: none;
    color: #b3271f;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    padding: 0;
    margin-right: 0.75rem;
  }

  .docs-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .doc-filter {
    max-width: 180px;
    font-size: 0.82rem;
  }
  .docs {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 0.4rem;
  }
  .docs li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.55rem 0.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 6px;
  }
  .doc-info { min-width: 0; display: flex; flex-direction: column; }
  .doc-name {
    font-weight: 600;
    color: #1c1917;
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .doc-meta { font-size: 0.72rem; color: #78716c; }
  .add-more { margin-top: 0.5rem; }
  .muted { color: #78716c; }
  .small { font-size: 0.82rem; }
  .error { color: #b3271f; }
  code {
    background: rgba(28, 25, 23, 0.05);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
  }
</style>
