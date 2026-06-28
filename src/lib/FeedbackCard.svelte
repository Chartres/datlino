<script lang="ts">
  // Flywheel feedback widget. User-initiated → always sends (no consent gate).
  // Collapsed by default so it never crowds the page it sits on.
  import { sendFeedback } from '$lib/flywheel';

  let { context = 'app' }: { context?: string } = $props();

  let open = $state(false);
  let ellis = $state<'very' | 'somewhat' | 'not' | null>(null);
  let text = $state('');
  let sent = $state(false);
  let sending = $state(false);

  async function submit() {
    if (sending || (!text.trim() && !ellis)) return;
    sending = true;
    await sendFeedback({
      sean_ellis: ellis ?? undefined,
      text: text.trim() ? `[${context}] ${text.trim()}` : undefined
    });
    sending = false;
    sent = true;
  }

  const ellisLabels = {
    very: 'Hodně by mi chybělo',
    somewhat: 'Trochu by mi chybělo',
    not: 'Nechybělo by mi'
  } as const;
</script>

<section class="fb">
  {#if sent}
    <p class="muted">Díky! Zpětnou vazbu jsme dostali. 🙏</p>
  {:else if !open}
    <button type="button" class="link" onclick={() => (open = true)}>
      💬 Máš nápad nebo narazil/a jsi na chybu? Napiš nám
    </button>
  {:else}
    <h3>Tvoje zpětná vazba</h3>
    <p class="muted">Co by ti pomohlo? Co nefunguje? Píšeš přímo autorovi.</p>
    <div class="ellis">
      <span class="muted small">Kdyby Datlino zítra zmizelo…</span>
      <div class="ellis-row">
        {#each ['very', 'somewhat', 'not'] as const as key}
          <button
            type="button"
            class="chip"
            class:active={ellis === key}
            onclick={() => (ellis = ellis === key ? null : key)}
          >
            {ellisLabels[key]}
          </button>
        {/each}
      </div>
    </div>
    <textarea bind:value={text} rows="3" placeholder="Napiš cokoli (volitelné)…"></textarea>
    <div class="actions">
      <button type="button" class="ghost" onclick={() => (open = false)}>Zavřít</button>
      <button
        type="button"
        class="primary"
        disabled={sending || (!text.trim() && !ellis)}
        onclick={submit}
      >
        {sending ? 'Odesílám…' : 'Odeslat'}
      </button>
    </div>
  {/if}
</section>

<style>
  .fb {
    margin-bottom: 2rem;
    padding: 1rem 1.25rem;
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
  }
  .muted {
    color: #78716c;
    font-size: 0.9rem;
    margin: 0 0 0.5rem;
  }
  .small {
    font-size: 0.8rem;
  }
  h3 {
    margin: 0 0 0.25rem;
    font-size: 1.05rem;
  }
  .link {
    background: none;
    border: none;
    color: #b45309;
    cursor: pointer;
    font: inherit;
    padding: 0;
  }
  .ellis-row,
  .actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .ellis {
    margin: 0.5rem 0 0.75rem;
  }
  .chip {
    padding: 0.35rem 0.7rem;
    border: 1px solid rgba(28, 25, 23, 0.18);
    border-radius: 999px;
    background: #fff;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .chip.active {
    background: #b45309;
    color: #fff;
    border-color: #b45309;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid rgba(28, 25, 23, 0.18);
    border-radius: 8px;
    padding: 0.5rem;
    font: inherit;
    resize: vertical;
  }
  .actions {
    margin-top: 0.75rem;
    justify-content: flex-end;
  }
  button.primary {
    background: #b45309;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 0.45rem 1rem;
    cursor: pointer;
  }
  button.primary:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button.ghost {
    background: none;
    border: 1px solid rgba(28, 25, 23, 0.18);
    border-radius: 8px;
    padding: 0.45rem 1rem;
    cursor: pointer;
  }
</style>
