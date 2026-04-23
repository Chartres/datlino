<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  type SearchHit = {
    chunk_id: number;
    document_id: number;
    source_path: string;
    text: string;
    char_offset: number;
    score: number;
  };

  type IndexStatus = {
    document_count: number;
    chunk_count: number;
    watched_roots: string[];
  };

  let query = $state('');
  let k = $state(10);
  let hits = $state<SearchHit[]>([]);
  let status = $state<IndexStatus | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let folderInput = $state('');

  async function refreshStatus() {
    try {
      status = await invoke<IndexStatus>('index_status');
    } catch (e) {
      error = String(e);
    }
  }

  async function addFolder() {
    if (!folderInput.trim()) return;
    busy = true;
    error = null;
    try {
      await invoke('add_watched_folder', { path: folderInput.trim() });
      folderInput = '';
      await refreshStatus();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function runSearch() {
    busy = true;
    error = null;
    try {
      hits = await invoke<SearchHit[]>('search_chunks', { query, k });
    } catch (e) {
      error = String(e);
      hits = [];
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    refreshStatus();
  });
</script>

<section>
  <h2>Knihovna</h2>
  {#if status}
    <p class="muted">
      {status.document_count} dokumentů, {status.chunk_count} vět.
      Sledované složky: {status.watched_roots.length || 'žádné'}
    </p>
    {#if status.watched_roots.length}
      <ul class="paths">
        {#each status.watched_roots as p}
          <li><code>{p}</code></li>
        {/each}
      </ul>
    {/if}
  {/if}

  <form
    onsubmit={(e) => {
      e.preventDefault();
      addFolder();
    }}
  >
    <input
      type="text"
      placeholder="/cesta/k/poznámkám"
      bind:value={folderInput}
      disabled={busy}
    />
    <button type="submit" disabled={busy || !folderInput.trim()}>
      Přidat složku
    </button>
  </form>
</section>

<section>
  <h2>Hledat věty</h2>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      runSearch();
    }}
  >
    <input
      type="text"
      placeholder="např. Habsburkové, fotosyntéza, derivace…"
      bind:value={query}
      disabled={busy}
    />
    <button type="submit" disabled={busy || !query.trim()}>Hledat</button>
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
            <span>BM25 {hit.score.toFixed(2)}</span>
          </p>
        </li>
      {/each}
    </ol>
  {:else if query && !busy}
    <p class="muted">Žádné shody.</p>
  {/if}
</section>

<style>
  section {
    margin-bottom: 2.5rem;
  }
  h2 {
    font-size: 1.1rem;
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
  button {
    padding: 0.5rem 1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .muted {
    color: #78716c;
    font-size: 0.9rem;
  }
  .paths {
    margin: 0.25rem 0 0.75rem 1rem;
    padding: 0;
    font-size: 0.85rem;
  }
  .hits {
    list-style: decimal;
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
</style>
