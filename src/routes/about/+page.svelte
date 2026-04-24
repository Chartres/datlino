<script lang="ts">
  import { api } from '$lib/api';

  let changelog = $state('');
  let version = $state('');
  let loading = $state(true);
  let error = $state<string | null>(null);

  $effect(() => {
    void load();
  });

  async function load() {
    try {
      [changelog, version] = await Promise.all([
        api.getChangelog(),
        api.getVersion()
      ]);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // Minimal markdown-to-HTML — headings, lists, bold, code, horizontal
  // rules, inline links. No third-party parser; keeps the bundle small
  // and the output auditable.
  function renderMarkdown(src: string): string {
    const escapeHtml = (s: string) =>
      s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

    const inline = (s: string) =>
      s
        .replace(/`([^`]+)`/g, (_, c) => `<code>${escapeHtml(c)}</code>`)
        .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
        .replace(/\*([^*]+)\*/g, '<em>$1</em>')
        .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>');

    const lines = src.split('\n');
    const out: string[] = [];
    let inList = false;
    const closeList = () => {
      if (inList) {
        out.push('</ul>');
        inList = false;
      }
    };

    for (const raw of lines) {
      const line = raw.trimEnd();
      if (line === '') {
        closeList();
        continue;
      }
      if (/^# /.test(line)) {
        closeList();
        out.push(`<h1>${inline(escapeHtml(line.slice(2)))}</h1>`);
      } else if (/^## /.test(line)) {
        closeList();
        out.push(`<h2>${inline(escapeHtml(line.slice(3)))}</h2>`);
      } else if (/^### /.test(line)) {
        closeList();
        out.push(`<h3>${inline(escapeHtml(line.slice(4)))}</h3>`);
      } else if (/^- /.test(line)) {
        if (!inList) {
          out.push('<ul>');
          inList = true;
        }
        out.push(`<li>${inline(escapeHtml(line.slice(2)))}</li>`);
      } else if (/^---$/.test(line)) {
        closeList();
        out.push('<hr />');
      } else {
        closeList();
        out.push(`<p>${inline(escapeHtml(line))}</p>`);
      }
    }
    closeList();
    return out.join('\n');
  }

  const html = $derived(renderMarkdown(changelog));
</script>

<section class="head">
  <h2>O aplikaci</h2>
  <p class="muted">
    Datlino v{version || '—'} · touch-typing trainer pro české a slovenské
    středoškoláky. Kód žije v git repu na tomto zařízení — žádné anonymní
    telemetry bez tvého svolení.
  </p>
</section>

{#if loading}
  <p class="muted">Načítám…</p>
{:else if error}
  <p class="error">{error}</p>
{:else}
  <article class="changelog">
    {@html html}
  </article>
{/if}

<style>
  .head { margin-bottom: 1.5rem; }
  .muted { color: #78716c; font-size: 0.9rem; }
  .error { color: #b3271f; }

  .changelog {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
    padding: 1.5rem 2rem;
    line-height: 1.55;
  }
  .changelog :global(h1) { font-size: 1.6rem; margin: 0 0 1rem; color: #1c1917; }
  .changelog :global(h2) {
    font-size: 1.15rem;
    margin: 2rem 0 0.6rem;
    padding-bottom: 0.3rem;
    border-bottom: 1px dashed rgba(28, 25, 23, 0.1);
    color: #b3271f;
  }
  .changelog :global(h2:first-child) { margin-top: 0; }
  .changelog :global(h3) { font-size: 1rem; margin: 1.2rem 0 0.4rem; color: #44403c; }
  .changelog :global(p) { margin: 0 0 0.75rem; color: #292524; }
  .changelog :global(ul) { margin: 0 0 0.75rem 0.25rem; padding-left: 1.1rem; }
  .changelog :global(li) { margin: 0.15rem 0; color: #292524; }
  .changelog :global(code) {
    background: rgba(179, 39, 31, 0.06);
    color: #b3271f;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
    font-family: ui-monospace, SFMono-Regular, monospace;
    font-size: 0.85rem;
  }
  .changelog :global(strong) { color: #1c1917; }
  .changelog :global(em) { font-style: italic; color: #57534e; }
  .changelog :global(hr) {
    border: none;
    border-top: 1px dashed rgba(28, 25, 23, 0.15);
    margin: 1.5rem 0;
  }
  .changelog :global(a) { color: #b3271f; }
</style>
