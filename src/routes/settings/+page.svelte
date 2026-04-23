<script lang="ts">
  import { api } from '$lib/api';
  import type { EmbeddingProviderKind, EmbeddingStatus } from '$lib/types';

  let status = $state<EmbeddingStatus | null>(null);
  let cohereKey = $state('');
  let anthropicKey = $state('');
  let anthropicPresent = $state(false);
  let anthropicEnvKey = $state<string | null>(null);
  let ocr = $state<{ tesseract: boolean; pdftoppm: boolean; available: boolean } | null>(
    null
  );
  let savingKey = $state(false);
  let savingAnth = $state(false);
  let switching = $state<EmbeddingProviderKind | null>(null);
  let embedRunning = $state(false);
  let message = $state<string | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    refresh();
  });

  async function refresh() {
    try {
      [status, ocr, anthropicPresent, anthropicEnvKey] = await Promise.all([
        api.getEmbeddingStatus(),
        api.getOcrStatus(),
        api.anthropicKeyPresent(),
        api.detectAnthropicEnvKey()
      ]);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function saveAnthropic() {
    savingAnth = true;
    try {
      await api.setAnthropicApiKey(anthropicKey);
      anthropicKey = '';
      await refresh();
      message = 'Anthropic klíč uložen.';
    } catch (e) {
      error = String(e);
    } finally {
      savingAnth = false;
    }
  }

  async function importEnvKey() {
    if (!anthropicEnvKey) return;
    savingAnth = true;
    try {
      await api.setAnthropicApiKey(anthropicEnvKey);
      await refresh();
      message =
        'Klíč z proměnné prostředí ANTHROPIC_API_KEY uložen do klíčenky.';
    } catch (e) {
      error = String(e);
    } finally {
      savingAnth = false;
    }
  }

  function openConsole() {
    // Tauri's webview supports window.open() for external URLs — no plugin
    // required for a simple "open in system browser" link.
    window.open('https://console.anthropic.com/settings/keys', '_blank');
  }

  function openCohereDashboard() {
    window.open('https://dashboard.cohere.com/api-keys', '_blank');
  }

  async function saveKey() {
    savingKey = true;
    message = null;
    error = null;
    try {
      await api.setCohereApiKey(cohereKey);
      cohereKey = '';
      await refresh();
      message = 'Klíč uložen do klíčenky OS.';
    } catch (e) {
      error = String(e);
    } finally {
      savingKey = false;
    }
  }

  async function pick(kind: EmbeddingProviderKind) {
    switching = kind;
    error = null;
    message = null;
    try {
      const res = await api.setEmbeddingProvider(kind);
      message = `Provider změněn. Naindexováno ${res.embedded} nových vět.`;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      switching = null;
    }
  }

  async function embedRemaining() {
    if (!status) return;
    embedRunning = true;
    error = null;
    message = null;
    try {
      const res = await api.embedPending();
      message = `Zpracováno ${res.embedded} vět z ${res.total_chunks}.`;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      embedRunning = false;
    }
  }

  function label(kind: string): string {
    return (
      {
        none: 'Žádný (BM25 jen)',
        fake: 'Lokální heuristika (offline test)',
        cohere: 'Cohere multilingual-v3 (cloud)',
        local: 'Lokální model (Candle) – připravujeme'
      } as Record<string, string>
    )[kind] ?? kind;
  }
</script>

<section>
  <h2>Nastavení</h2>
  <p class="muted">
    Embeddings umožňují Datlinu pochopit, co se vlastně učíš — i když přesné
    slovo v otázce nepoužiješ. Nastav si zde, jaký model se používá.
  </p>
</section>

{#if status}
  <section class="card">
    <h3>Aktuální provider</h3>
    <p class="big">{label(status.provider)}</p>
    <p class="muted">
      {status.embedded_chunks} z {status.total_chunks} vět je oembeddováno
      · dim = {status.dim || '—'}
    </p>
    {#if status.total_chunks > status.embedded_chunks && status.provider !== 'none'}
      <button class="secondary" onclick={embedRemaining} disabled={embedRunning}>
        {embedRunning ? 'Zpracovávám…' : `Doindexovat zbývajících ${status.total_chunks - status.embedded_chunks} vět`}
      </button>
    {/if}
  </section>

  <section>
    <h3>Vyber provider</h3>
    <div class="tiles">
      <button
        class="tile"
        class:active={status.provider === 'none'}
        disabled={switching !== null}
        onclick={() => pick('none')}
      >
        <strong>Žádný</strong>
        <span>Vyhledává se jen pomocí BM25 (klíčová slova). Rychlé, funguje bez sítě.</span>
      </button>

      <button
        class="tile"
        class:active={status.provider === 'fake'}
        disabled={switching !== null}
        onclick={() => pick('fake')}
      >
        <strong>Lokální heuristika</strong>
        <span>Deterministické vektory ze znaků — ne pro produkci, ale offline a rychlé.</span>
      </button>

      <button
        class="tile"
        class:active={status.provider === 'cohere'}
        disabled={switching !== null || !status.cohere_key_present}
        onclick={() => pick('cohere')}
      >
        <strong>Cohere multilingual-v3</strong>
        <span>
          Cloud, nejlepší kvalita na CZ/SK.
          {#if !status.cohere_key_present}
            Nejdříve ulož svůj API klíč níže.
          {/if}
        </span>
      </button>

      <button
        class="tile"
        class:active={status.provider === 'local'}
        disabled={switching !== null}
        onclick={() => pick('local')}
      >
        <strong>Lokální Candle</strong>
        <span>
          Multilingual-e5-small (~120 MB). Nic neodchází ze zařízení. První
          použití stáhne model; kompilace vyžaduje
          <code>cargo build --features candle</code>.
        </span>
      </button>
    </div>
  </section>

  <section class="card">
    <h3>Cohere API klíč</h3>
    <p class="muted">
      Uloží se do klíčenky operačního systému (macOS Keychain / Windows
      Credential Manager / libsecret), nikoli do souboru aplikace.
      {#if status.cohere_key_present}
        <br /><strong>Aktuálně uložen.</strong> Nový klíč ho přepíše, prázdné pole ho smaže.
      {/if}
    </p>
    <div class="login-row">
      <button class="secondary" onclick={openCohereDashboard}>
        Otevřít Cohere Dashboard →
      </button>
    </div>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        saveKey();
      }}
    >
      <input
        type="password"
        placeholder="co-XXXXXXXXXXXXXXXXX…"
        bind:value={cohereKey}
        disabled={savingKey}
      />
      <button class="primary" type="submit" disabled={savingKey}>
        {savingKey ? 'Ukládám…' : 'Uložit'}
      </button>
    </form>
  </section>
{/if}

{#if ocr}
  <section class="card">
    <h3>OCR (skenované PDF a GoodNotes)</h3>
    <p class="muted">
      Když PDF obsahuje jen obrázky (např. export z GoodNotes nebo naskenovaná
      učebnice), Datlino si přečte text pomocí
      <code>tesseract</code> + <code>pdftoppm</code>.
    </p>
    <ul class="binlist">
      <li>
        <code>tesseract</code>:
        {#if ocr.tesseract}<span class="ok">k dispozici</span>{:else}<span class="warn">chybí</span>{/if}
      </li>
      <li>
        <code>pdftoppm</code>:
        {#if ocr.pdftoppm}<span class="ok">k dispozici</span>{:else}<span class="warn">chybí</span>{/if}
      </li>
    </ul>
    {#if !ocr.available}
      <p class="muted small">
        Na macOS: <code>brew install tesseract tesseract-lang poppler</code>.
        Na Ubuntu/Debian:
        <code>apt install tesseract-ocr tesseract-ocr-ces tesseract-ocr-slk tesseract-ocr-eng poppler-utils</code>.
      </p>
    {/if}
  </section>
{/if}

<section class="card">
  <h3>Rephrase mode (volitelný)</h3>
  <p class="muted">
    Režim, který s pomocí LLM přepíše věty z tvých materiálů tak, aby
    obsahovaly tvé aktuálně slabé kombinace kláves. Implementace Claude API
    (model <code>claude-haiku-4-5</code>). Klíč se ukládá do klíčenky
    operačního systému. Před použitím každé rephrase se porovná podobnost
    s původní větou; drift &gt; 15 % se zahazuje.
    {#if anthropicPresent}
      <br /><strong>Anthropic klíč uložen.</strong> Režim lze zapnout u
      každého sezení zvlášť.
    {/if}
  </p>

  {#if anthropicEnvKey && !anthropicPresent}
    <div class="suggestion">
      <span>
        ✨ Našli jsme <code>ANTHROPIC_API_KEY</code> v proměnných prostředí.
        Můžeš ho importovat jedním klikem:
      </span>
      <button class="secondary" onclick={importEnvKey} disabled={savingAnth}>
        Importovat
      </button>
    </div>
  {/if}

  <div class="login-row">
    <button class="secondary" onclick={openConsole}>
      Otevřít Anthropic Console →
    </button>
    <span class="muted small">
      Sign-in with Claude vyžaduje registraci Datlina jako OAuth aplikace
      u Anthropic — zatím nejjednodušší cesta je přihlásit se do Console,
      vytvořit klíč a vložit ho dolů.
    </span>
  </div>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      saveAnthropic();
    }}
  >
    <input
      type="password"
      placeholder="sk-ant-XXXXXXXXXXXXXXXX…"
      bind:value={anthropicKey}
      disabled={savingAnth}
    />
    <button class="primary" type="submit" disabled={savingAnth}>
      {savingAnth ? 'Ukládám…' : 'Uložit'}
    </button>
  </form>
</section>

{#if message}
  <p class="msg ok">{message}</p>
{/if}
{#if error}
  <p class="msg err">{error}</p>
{/if}

<style>
  section {
    margin-bottom: 2rem;
  }
  .muted {
    color: #78716c;
    font-size: 0.9rem;
  }
  .card {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 8px;
    padding: 1rem 1.25rem;
  }
  h3 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
    color: #292524;
  }
  .big {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 0.75rem;
  }
  .tile {
    text-align: left;
    padding: 1rem;
    border: 1px solid rgba(28, 25, 23, 0.1);
    background: #fffaf2;
    border-radius: 8px;
    cursor: pointer;
    font: inherit;
  }
  .tile.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.06);
  }
  .tile strong {
    display: block;
    margin-bottom: 0.25rem;
    color: #b3271f;
  }
  .tile span {
    font-size: 0.85rem;
    color: #57534e;
  }
  form {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }
  input[type='password'] {
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
  button.secondary {
    margin-top: 0.5rem;
    padding: 0.4rem 0.8rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  button.secondary:hover {
    border-color: #b3271f;
    color: #b3271f;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .msg {
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    font-size: 0.9rem;
  }
  .msg.ok {
    background: rgba(34, 139, 34, 0.08);
    color: #2d6a2d;
    border: 1px solid rgba(34, 139, 34, 0.2);
  }
  .msg.err {
    background: rgba(179, 39, 31, 0.08);
    color: #b3271f;
    border: 1px solid rgba(179, 39, 31, 0.2);
  }
  .binlist {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
    display: flex;
    gap: 1.5rem;
  }
  .binlist li {
    font-size: 0.9rem;
  }
  .ok {
    color: #2d6a2d;
    font-weight: 600;
  }
  .warn {
    color: #b3271f;
    font-weight: 600;
  }
  .small {
    font-size: 0.8rem;
  }
  code {
    background: rgba(28, 25, 23, 0.05);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  .suggestion {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: rgba(179, 39, 31, 0.04);
    border: 1px solid rgba(179, 39, 31, 0.15);
    border-radius: 6px;
    margin: 0.5rem 0;
    font-size: 0.85rem;
    color: #44403c;
  }
  .suggestion code {
    font-size: 0.8rem;
  }
  .login-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin: 0.5rem 0;
    flex-wrap: wrap;
  }
  .login-row .muted {
    flex: 1;
    margin: 0;
  }
</style>
