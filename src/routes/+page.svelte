<script lang="ts">
  import { goto } from '$app/navigation';
  import { api } from '$lib/api';
  import { profile } from '$lib/profile.svelte';
  import { currentSession } from '$lib/session-store.svelte';
  import type { DocumentInfo, IndexStatus, SearchHit } from '$lib/types';

  let status = $state<IndexStatus | null>(null);
  let documents = $state<DocumentInfo[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);

  let query = $state('');
  let hits = $state<SearchHit[]>([]);

  async function refreshStatus() {
    try {
      [status, documents] = await Promise.all([
        api.indexStatus(),
        api.listDocuments()
      ]);
    } catch (e) {
      error = String(e);
    }
  }

  async function pickAndAddFolder() {
    error = null;
    try {
      const picked = await api.pickFolder();
      if (!picked) return;
      busy = true;
      await api.addWatchedFolder(picked);
      await refreshStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function pickAndIngestFile() {
    error = null;
    try {
      const picked = await api.pickFile();
      if (!picked) return;
      busy = true;
      await api.ingestSingleFile(picked);
      await refreshStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function removeFolder(path: string) {
    busy = true;
    try {
      await api.removeWatchedFolder(path);
      await refreshStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function runSearch() {
    if (!query.trim()) return;
    busy = true;
    error = null;
    try {
      hits = await api.searchChunks(query.trim(), 10);
    } catch (e) {
      error = String(e);
      hits = [];
    } finally {
      busy = false;
    }
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
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function docName(path: string): string {
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1];
  }

  $effect(() => {
    refreshStatus();
  });
</script>

<section class="hero">
  <div>
    <h2>Ahoj {profile.data && profile.data.total_sessions > 0 ? 'zpět' : 'a vítej'}.</h2>
    <p>
      Datlino ti pomáhá se učit a zlepšovat psaní naslepo na tvých vlastních
      poznámkách. Vyber si složku s materiály, nebo rovnou vyzkoušej jeden
      z tréninkových módů.
    </p>
  </div>
  <a class="cta" href="/practice">Začít trénink →</a>
</section>

<section>
  <h3>Knihovna</h3>
  {#if status}
    <p class="muted">
      {status.document_count} dokumentů · {status.chunk_count} vět k tréninku.
    </p>
    {#if status.watched_roots.length}
      <ul class="paths">
        {#each status.watched_roots as p}
          <li>
            <code>{p}</code>
            <button class="link" onclick={() => removeFolder(p)}>odebrat</button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="muted">Zatím není přidána žádná složka.</p>
    {/if}
  {/if}

  <div class="actions">
    <button class="primary" onclick={pickAndAddFolder} disabled={busy}>
      {busy ? 'Pracuji…' : 'Přidat složku'}
    </button>
    <button class="secondary" onclick={pickAndIngestFile} disabled={busy}>
      Přidat jeden soubor
    </button>
  </div>
</section>

{#if documents.length > 0}
  <section>
    <h3>Dokumenty</h3>
    <p class="muted small">
      Klikni na dokument pro trénink celého souboru po sobě — bez hledání.
    </p>
    <ul class="docs">
      {#each documents as doc (doc.id)}
        <li>
          <div class="doc-info">
            <span class="doc-name">{docName(doc.source_path)}</span>
            <span class="doc-meta">
              {doc.chunk_count} vět · {doc.kind.toUpperCase()}
            </span>
          </div>
          <button class="secondary" onclick={() => drillDocument(doc)} disabled={busy}>
            Trénovat celý
          </button>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<section>
  <h3>Hledat ve svých materiálech</h3>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      runSearch();
    }}
  >
    <input
      type="text"
      placeholder="např. fotosyntéza, Habsburkové, perfektum…"
      bind:value={query}
      disabled={busy}
    />
    <button type="submit" class="primary" disabled={busy || !query.trim()}>Hledat</button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if hits.length}
    <ol class="hits">
      {#each hits as hit}
        <li>
          <p class="text">{hit.text}</p>
          <p class="meta">
            <span>{hit.source_path}</span>
            <span>·</span>
            <span>skóre {hit.score.toFixed(2)}</span>
          </p>
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
    margin-bottom: 2rem;
  }

  .hero h2 {
    margin: 0 0 0.25rem;
    font-size: 1.3rem;
  }
  .hero p {
    margin: 0;
    color: #57534e;
    font-size: 0.95rem;
    max-width: 38rem;
  }

  .cta {
    padding: 0.6rem 1.1rem;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 6px;
    text-decoration: none;
    font-weight: 600;
    white-space: nowrap;
  }

  section {
    margin-bottom: 2.5rem;
  }
  h3 {
    font-size: 1.05rem;
    margin: 0 0 0.5rem;
    color: #292524;
  }

  form {
    display: flex;
    gap: 0.5rem;
    margin: 0.75rem 0;
  }
  input[type='text'] {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 4px;
    font: inherit;
    background: #fffaf2;
  }

  button.primary {
    padding: 0.5rem 1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  button.link {
    background: none;
    border: none;
    color: #b3271f;
    cursor: pointer;
    font: inherit;
    font-size: 0.8rem;
    margin-left: 0.5rem;
  }

  .muted {
    color: #78716c;
    font-size: 0.9rem;
  }
  .paths {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
  }
  .paths li {
    padding: 0.35rem 0;
    font-size: 0.9rem;
  }
  .paths code {
    background: rgba(28, 25, 23, 0.05);
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
  }
  .hits {
    padding-left: 1.5rem;
  }
  .hits li {
    margin: 0.75rem 0;
    padding: 0.5rem 0.75rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 4px;
  }
  .text {
    margin: 0 0 0.25rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.95rem;
    line-height: 1.5;
  }
  .meta {
    margin: 0;
    color: #78716c;
    font-size: 0.8rem;
    display: flex;
    gap: 0.5rem;
  }
  .error {
    color: #b3271f;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  .small {
    font-size: 0.85rem;
  }
  button.secondary {
    padding: 0.5rem 1rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    color: #44403c;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
  }
  button.secondary:hover {
    border-color: #b3271f;
    color: #b3271f;
  }
  button.secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .docs {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.5rem;
  }
  .docs li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.6rem 0.8rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 6px;
  }
  .doc-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }
  .doc-name {
    font-weight: 600;
    color: #1c1917;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .doc-meta {
    font-size: 0.75rem;
    color: #78716c;
  }
</style>
