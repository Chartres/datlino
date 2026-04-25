<script lang="ts">
  import { goto } from '$app/navigation';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { api } from '$lib/api';
  import CopyPasteRephraseModal from '$lib/CopyPasteRephraseModal.svelte';
  import { currentSession } from '$lib/session-store.svelte';
  import type {
    ChapterInfo,
    ContentStrategy,
    DocumentInfo,
    ExamRamp,
    IndexStatus,
    SessionPlan,
    WeakNgram
  } from '$lib/types';

  type RemixMode = 'off' | 'sub' | 'copypaste' | 'byok';

  type IngestEvent =
    | { kind: 'start'; path: string }
    | {
        kind: 'file';
        path: string;
        stats: {
          files_seen: number;
          files_ingested: number;
          files_skipped_unchanged: number;
          chunks_written: number;
        };
      }
    | {
        kind: 'done';
        stats: {
          files_seen: number;
          files_ingested: number;
          files_skipped_unchanged: number;
          chunks_written: number;
        };
      };

  let ingestProgress = $state<{
    active: boolean;
    currentPath: string;
    filesDone: number;
    filesSeen: number;
    chunks: number;
  }>({
    active: false,
    currentPath: '',
    filesDone: 0,
    filesSeen: 0,
    chunks: 0
  });

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
  let dictation = $state(false);
  let anthropicPresent = $state(false);
  let claudeSubLive = $state(false);
  let examRamps = $state<ExamRamp[]>([]);

  // Three-tier remix auth cascade. UI shows the highest-priority option
  // we can actually run; user can override.
  let remixMode = $state<RemixMode>('off');
  let copyPasteOpen = $state(false);
  let pendingPlan = $state<SessionPlan | null>(null);
  let pendingWeak = $state<WeakNgram[]>([]);
  let copyPasteToast = $state<string | null>(null);

  $effect(() => {
    reload();
    // Subscribe to Rust-side ingest progress. Returns an unlisten.
    let unlisten: UnlistenFn | null = null;
    listen<IngestEvent>('ingest-progress', async (ev) => {
      const payload = ev.payload;
      if (payload.kind === 'start') {
        ingestProgress = {
          active: true,
          currentPath: payload.path,
          filesDone: ingestProgress.filesDone,
          filesSeen: ingestProgress.filesSeen + 1,
          chunks: ingestProgress.chunks
        };
      } else if (payload.kind === 'file') {
        ingestProgress = {
          active: true,
          currentPath: payload.path,
          filesDone:
            payload.stats.files_ingested + payload.stats.files_skipped_unchanged,
          filesSeen: payload.stats.files_seen,
          chunks: payload.stats.chunks_written
        };
      } else if (payload.kind === 'done') {
        ingestProgress = { ...ingestProgress, active: false };
        await reload(); // refresh the document list
      }
    }).then((u) => (unlisten = u));
    return () => {
      unlisten?.();
    };
  });

  async function reload() {
    try {
      const [s, docs, chs, key, sub, ramps] = await Promise.all([
        api.indexStatus(),
        api.listDocuments(),
        api.listChapters(),
        api.anthropicKeyPresent(),
        api.claudeSubscriptionStatus(),
        api.listExamRamps()
      ]);
      status = s;
      documents = docs;
      chapters = chs;
      anthropicPresent = key;
      claudeSubLive = sub.detected && !sub.expired;
      examRamps = ramps;
    } catch (e) {
      error = String(e);
    }
  }

  async function startRamp(rampId: string, lessonId?: string) {
    busy = true;
    error = null;
    try {
      const plan = await api.createSession({
        mode: 'exam_ramp',
        alpha: 0,
        target_duration_s: 600,
        ramp_id: rampId,
        ramp_lesson_id: lessonId
      });
      if (!plan.sentences.length) {
        error = 'Ramp je prázdný.';
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

  // Copy-paste is always available — it just needs the student to have
  // a browser and any free LLM tab open. Claude sub & BYOK are gated on
  // credentials being present.
  const remixAvailable = $derived(true);

  // Pick the default remix mode whenever the student toggles "rephrase"
  // on. Cascade: subscription → BYOK → copy-paste (free).
  $effect(() => {
    if (!rephrase) {
      remixMode = 'off';
    } else if (remixMode === 'off') {
      if (claudeSubLive) remixMode = 'sub';
      else if (anthropicPresent) remixMode = 'byok';
      else remixMode = 'copypaste';
    }
  });

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
      // Server-side rephrase only when sub or BYOK is the chosen path.
      // Copy-paste mode runs the modal AFTER we have raw sentences so
      // the student sees what they're remixing.
      const useServerRephrase = rephrase && (remixMode === 'sub' || remixMode === 'byok');
      // Dictation is incompatible with chapter strategy (sentence
      // selection lives in pick_chapter, not pick_content).
      const useDictation = dictation && strategy !== 'chapter';
      const plan = await api.createSession({
        mode: useDictation ? 'dictation' : 'content',
        alpha: advanced ? alpha : 1.0,
        target_duration_s: duration,
        content_strategy: useDictation ? undefined : strategy,
        query: strategy !== 'chapter' ? query.trim() || undefined : undefined,
        chapter_id: strategy === 'chapter' && chapterId ? chapterId : undefined,
        rephrase: useServerRephrase,
        rephrase_style: useServerRephrase ? 'keystrokes' : undefined,
        language: 'cs'
      });
      if (!plan.sentences.length) {
        error = emptyMessage();
        return;
      }
      if (rephrase && remixMode === 'copypaste') {
        // Stash the plan and open the modal. We need weak ngrams for the
        // prompt; fetch in parallel with showing the UI.
        pendingPlan = plan;
        pendingWeak = await api.getWeakNgrams(8);
        copyPasteOpen = true;
        return;
      }
      currentSession.plan = plan;
      currentSession.summary = null;
      await goto('/practice/session');
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  function applyCopyPasteAndStart(
    rewrites: string[],
    meta: { accepted: number; total: number; warnings: string[] }
  ) {
    if (!pendingPlan) return;
    // In-place replacement: same chunk_ids, same order, just swapped
    // text. Keystroke target follows automatically.
    const sentences = pendingPlan.sentences.map((s, i) => ({
      ...s,
      text: rewrites[i] ?? s.text
    }));
    currentSession.plan = { ...pendingPlan, sentences };
    currentSession.summary = null;
    copyPasteOpen = false;
    pendingPlan = null;
    copyPasteToast = `Remix: ${meta.accepted}/${meta.total} přijato${
      meta.warnings.length > 0 ? ` (${meta.warnings.length} upozornění)` : ''
    }.`;
    void goto('/practice/session');
  }

  function cancelCopyPaste() {
    copyPasteOpen = false;
    pendingPlan = null;
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

{#if copyPasteToast}
  <p class="toast" onanimationend={() => (copyPasteToast = null)}>{copyPasteToast}</p>
{/if}

{#if copyPasteOpen && pendingPlan}
  <CopyPasteRephraseModal
    sources={pendingPlan.sentences.map((s) => s.text)}
    weakNgrams={pendingWeak.map((w) => w.ngram)}
    style="keystrokes"
    language="cs"
    onApply={applyCopyPasteAndStart}
    onCancel={cancelCopyPaste}
  />
{/if}

{#if ingestProgress.active}
  <section class="ingest-bar" aria-live="polite">
    <div class="ingest-bar-row">
      <strong>Načítám tvou knihovnu…</strong>
      <span>{ingestProgress.filesDone} / {ingestProgress.filesSeen} souborů · {ingestProgress.chunks} vět</span>
    </div>
    <div class="ingest-track">
      <div
        class="ingest-fill"
        style={`width: ${
          ingestProgress.filesSeen === 0
            ? 5
            : Math.min(100, (ingestProgress.filesDone / ingestProgress.filesSeen) * 100)
        }%`}
      ></div>
    </div>
    <p class="ingest-path">{ingestProgress.currentPath.split(/[\\/]/).pop() ?? ''}</p>
  </section>
{/if}

{#if examRamps.length > 0}
  <section class="ramps">
    <h3 class="section-head">Příprava na zkoušku — ramps</h3>
    <p class="muted small">
      Připravené sady pro Cermat a maturitu. Žádná potřeba vlastních
      materiálů — Datlino si text hlídá za tebe.
    </p>
    <div class="ramps-grid">
      {#each examRamps as ramp (ramp.id)}
        <article class="ramp-card">
          <h4>{ramp.title}</h4>
          <p class="ramp-sub">{ramp.subtitle}</p>
          <ul class="ramp-lessons">
            {#each ramp.lessons as lesson (lesson.id)}
              <li>
                <span class="ramp-lesson-title">{lesson.title}</span>
                <span class="ramp-lesson-count">
                  {lesson.passages.length} vět
                </span>
                <button
                  type="button"
                  class="ramp-go"
                  onclick={() => startRamp(ramp.id, lesson.id)}
                  disabled={busy}
                >
                  Začít
                </button>
              </li>
            {/each}
          </ul>
          <button
            type="button"
            class="ramp-all"
            onclick={() => startRamp(ramp.id)}
            disabled={busy}
          >
            Trénovat celý ramp →
          </button>
        </article>
      {/each}
    </div>
  </section>
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
          <input type="checkbox" bind:checked={dictation} disabled={strategy === 'chapter'} />
          Diktát — Datlino věty čte nahlas a automaticky pauzuje, když nestíháš
          {#if strategy === 'chapter'}
            <span class="muted small">(nedostupné pro celé kapitoly)</span>
          {/if}
        </label>
        <label class="adv-toggle">
          <input type="checkbox" bind:checked={rephrase} />
          Remix (LLM přepíše věty s cílem na tvé slabiny)
        </label>
        {#if rephrase}
          <div class="remix-modes">
            <button
              type="button"
              class="rmode"
              class:active={remixMode === 'sub'}
              disabled={!claudeSubLive}
              onclick={() => (remixMode = 'sub')}
            >
              <strong>Claude subscription</strong>
              <span>
                {#if claudeSubLive}
                  Přihlášen ✓ — bezešvé.
                {:else}
                  Není přihlášen. <a href="/settings">Přihlásit</a>
                {/if}
              </span>
            </button>
            <button
              type="button"
              class="rmode"
              class:active={remixMode === 'copypaste'}
              onclick={() => (remixMode = 'copypaste')}
            >
              <strong>Copy-paste do volného LLM <span class="badge">free</span></strong>
              <span>
                Datlino ti dá prompt, ty ho strčíš do ChatGPT / Claude.ai /
                Gemini, výsledek vrátíš zpátky.
              </span>
            </button>
            <button
              type="button"
              class="rmode"
              class:active={remixMode === 'byok'}
              disabled={!anthropicPresent}
              onclick={() => (remixMode = 'byok')}
            >
              <strong>Vlastní Anthropic klíč</strong>
              <span>
                {#if anthropicPresent}
                  Klíč uložen ✓ — automaticky, platí se za volání.
                {:else}
                  Nemáš klíč. <a href="/settings">Přidat</a>
                {/if}
              </span>
            </button>
          </div>
        {/if}
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
  .ingest-bar {
    padding: 0.75rem 1rem;
    background: rgba(179, 39, 31, 0.05);
    border: 1px solid rgba(179, 39, 31, 0.2);
    border-radius: 8px;
    margin-bottom: 1rem;
  }
  .ingest-bar-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
    color: #292524;
    margin-bottom: 0.4rem;
  }
  .ingest-track {
    height: 4px;
    background: rgba(28, 25, 23, 0.08);
    border-radius: 2px;
    overflow: hidden;
  }
  .ingest-fill {
    height: 100%;
    background: #b3271f;
    transition: width 250ms ease;
  }
  .ingest-path {
    margin: 0.3rem 0 0;
    color: #78716c;
    font-size: 0.75rem;
    font-family: ui-monospace, monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ramps { margin-bottom: 2rem; }
  .ramps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 0.6rem;
    margin-top: 0.75rem;
  }
  .ramp-card {
    padding: 0.9rem 1rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .ramp-card h4 { margin: 0; font-size: 0.98rem; color: #b3271f; }
  .ramp-sub { margin: 0; font-size: 0.82rem; color: #57534e; }
  .ramp-lessons { list-style: none; padding: 0; margin: 0.4rem 0; }
  .ramp-lessons li {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.5rem;
    align-items: center;
    padding: 0.35rem 0.1rem;
    border-top: 1px dashed rgba(28, 25, 23, 0.08);
    font-size: 0.85rem;
  }
  .ramp-lessons li:first-child { border-top: 0; }
  .ramp-lesson-title { color: #292524; }
  .ramp-lesson-count { color: #78716c; font-size: 0.75rem; }
  .ramp-go, .ramp-all {
    padding: 0.3rem 0.7rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    color: #44403c;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.78rem;
  }
  .ramp-all {
    margin-top: 0.4rem;
    align-self: flex-start;
    border-color: #b3271f;
    color: #b3271f;
    font-weight: 600;
  }

  .remix-modes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
    gap: 0.4rem;
    margin: 0.4rem 0 0.2rem;
  }
  .rmode {
    text-align: left;
    padding: 0.55rem 0.7rem;
    border: 1px solid rgba(28, 25, 23, 0.12);
    background: #fffaf2;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .rmode strong {
    color: #b3271f;
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .rmode span {
    font-size: 0.75rem;
    color: #57534e;
  }
  .rmode.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.06);
  }
  .rmode:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .rmode a { color: #b3271f; }
  .badge {
    background: rgba(45, 106, 45, 0.15);
    color: #2d6a2d;
    padding: 0.05rem 0.35rem;
    border-radius: 10px;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 700;
  }

  .toast {
    background: rgba(45, 106, 45, 0.1);
    color: #2d6a2d;
    border: 1px solid rgba(45, 106, 45, 0.25);
    border-radius: 6px;
    padding: 0.5rem 0.85rem;
    margin-bottom: 0.75rem;
    font-size: 0.88rem;
    animation: fadeOut 4.5s forwards;
  }
  @keyframes fadeOut {
    0%, 70% { opacity: 1; }
    100% { opacity: 0; }
  }
</style>
