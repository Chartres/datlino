<script lang="ts">
  import { api } from '$lib/api';
  import { profile, refreshProfile } from '$lib/profile.svelte';
  import type {
    ClaudeSubscriptionStatus,
    EmbeddingProviderKind,
    EmbeddingStatus
  } from '$lib/types';

  // Per Phase-1 IA reorg: four named sections, all closed by default
  // except Profil. Eliška shouldn't hit infrastructure content unless
  // she opens it deliberately; Martin can expand everything in 4 clicks.
  let openSection = $state<'profil' | 'search' | 'remix' | 'ocr' | null>('profil');

  // --- state ---
  let embStatus = $state<EmbeddingStatus | null>(null);
  let ocr = $state<{ tesseract: boolean; pdftoppm: boolean; available: boolean } | null>(null);
  let claudeSub = $state<ClaudeSubscriptionStatus | null>(null);
  let anthropicPresent = $state(false);
  let anthropicEnvKey = $state<string | null>(null);

  let cohereKey = $state('');
  let anthropicKey = $state('');
  let savingCohere = $state(false);
  let savingAnth = $state(false);
  let switching = $state<EmbeddingProviderKind | null>(null);
  let embedRunning = $state(false);
  let rechecking = $state(false);
  let message = $state<string | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void refreshAll();
  });

  async function refreshAll() {
    try {
      [embStatus, ocr, anthropicPresent, anthropicEnvKey, claudeSub] = await Promise.all([
        api.getEmbeddingStatus(),
        api.getOcrStatus(),
        api.anthropicKeyPresent(),
        api.detectAnthropicEnvKey(),
        api.claudeSubscriptionStatus()
      ]);
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  function toggle(name: 'profil' | 'search' | 'remix' | 'ocr') {
    openSection = openSection === name ? null : name;
  }

  // --- profile actions ---
  async function refreshProfileAndInfo() {
    await refreshProfile();
  }

  // --- embedding actions ---
  async function pickProvider(kind: EmbeddingProviderKind) {
    switching = kind;
    error = null;
    message = null;
    try {
      const res = await api.setEmbeddingProvider(kind);
      message = `Provider změněn. Naindexováno ${res.embedded} nových vět.`;
      await refreshAll();
    } catch (e) {
      error = String(e);
    } finally {
      switching = null;
    }
  }

  async function embedRemaining() {
    embedRunning = true;
    try {
      const res = await api.embedPending();
      message = `Zpracováno ${res.embedded} z ${res.total_chunks}.`;
      await refreshAll();
    } catch (e) {
      error = String(e);
    } finally {
      embedRunning = false;
    }
  }

  async function saveCohere() {
    savingCohere = true;
    try {
      await api.setCohereApiKey(cohereKey);
      cohereKey = '';
      await refreshAll();
      message = 'Cohere klíč uložen do klíčenky OS.';
    } catch (e) {
      error = String(e);
    } finally {
      savingCohere = false;
    }
  }

  // --- remix actions ---
  async function saveAnthropic() {
    savingAnth = true;
    try {
      await api.setAnthropicApiKey(anthropicKey);
      anthropicKey = '';
      await refreshAll();
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
      await refreshAll();
      message = 'Klíč z ANTHROPIC_API_KEY uložen do klíčenky.';
    } catch (e) {
      error = String(e);
    } finally {
      savingAnth = false;
    }
  }

  function openConsole() {
    window.open('https://console.anthropic.com/settings/keys', '_blank');
  }

  function openCohereDashboard() {
    window.open('https://dashboard.cohere.com/api-keys', '_blank');
  }

  // --- OCR actions ---
  async function recheckOcr() {
    rechecking = true;
    try {
      await refreshAll();
      message = ocr?.available
        ? 'OCR binárky nalezeny.'
        : 'Stále chybí. Zkontroluj instalaci a PATH.';
    } catch (e) {
      error = String(e);
    } finally {
      rechecking = false;
    }
  }

  function providerLabel(kind: string): string {
    return (
      {
        none: 'Žádný (BM25)',
        fake: 'Lokální heuristika (offline)',
        cohere: 'Cohere multilingual-v3',
        local: 'Lokální Candle (multilingual-e5-small)'
      } as Record<string, string>
    )[kind] ?? kind;
  }
</script>

<section class="head">
  <h2>Nastavení</h2>
  <p class="muted">
    Vše, co Datlino umí naladit. Výchozí stav je „zavřené" — otevři si
    jen to, co tě zajímá.
  </p>
</section>

{#if message}
  <p class="msg ok">{message}</p>
{/if}
{#if error}
  <p class="msg err">{error}</p>
{/if}

<!-- =================================================================
     1) Profil
     ================================================================= -->
<details class="section" open={openSection === 'profil'}>
  <summary onclick={(e) => { e.preventDefault(); toggle('profil'); }}>
    <span class="sec-num">1</span>
    <span class="sec-title">Profil</span>
    <span class="sec-sub">Tvoje statistiky a reset dat</span>
  </summary>
  <div class="section-body">
    {#if profile.data}
      <div class="grid">
        <div class="stat">
          <span class="label">Úroveň</span>
          <span class="value">L{profile.data.level}</span>
        </div>
        <div class="stat">
          <span class="label">XP</span>
          <span class="value">{profile.data.total_xp}</span>
        </div>
        <div class="stat">
          <span class="label">Série</span>
          <span class="value">🔥 {profile.data.current_streak}</span>
        </div>
        <div class="stat">
          <span class="label">Sezení</span>
          <span class="value">{profile.data.total_sessions}</span>
        </div>
        <div class="stat">
          <span class="label">WPM baseline</span>
          <span class="value">{profile.data.wpm_baseline?.toFixed(0) ?? '—'}</span>
        </div>
        <div class="stat">
          <span class="label">Přesnost</span>
          <span class="value">{profile.data.accuracy_baseline?.toFixed(0) ?? '—'} %</span>
        </div>
      </div>
    {:else}
      <p class="muted">Zatím žádné sezení.</p>
    {/if}
    <p class="muted small">
      Podrobná historie a slabé klávesy žijí na <a href="/progress">Pokroku</a>.
      Export a reset přijdou v samostatné verzi.
    </p>
  </div>
</details>

<!-- =================================================================
     2) Kvalita vyhledávání — embedding provider
     ================================================================= -->
<details class="section" open={openSection === 'search'}>
  <summary onclick={(e) => { e.preventDefault(); toggle('search'); }}>
    <span class="sec-num">2</span>
    <span class="sec-title">Kvalita vyhledávání</span>
    <span class="sec-sub">
      Embedding provider:
      <strong>{embStatus ? providerLabel(embStatus.provider) : '—'}</strong>
    </span>
  </summary>
  <div class="section-body">
    {#if embStatus}
      <p class="muted">
        {embStatus.embedded_chunks} z {embStatus.total_chunks} vět oembeddováno · dim = {embStatus.dim || '—'}
      </p>
      {#if embStatus.total_chunks > embStatus.embedded_chunks && embStatus.provider !== 'none'}
        <button class="secondary" onclick={embedRemaining} disabled={embedRunning}>
          {embedRunning ? 'Zpracovávám…' : `Doindexovat ${embStatus.total_chunks - embStatus.embedded_chunks} vět`}
        </button>
      {/if}

      <div class="tiles">
        <button
          class="tile"
          class:active={embStatus.provider === 'local'}
          disabled={switching !== null}
          onclick={() => pickProvider('local')}
        >
          <strong>Lokální Candle</strong>
          <span>~120 MB model. Nic neodchází ze zařízení. Doporučené.</span>
        </button>
        <button
          class="tile"
          class:active={embStatus.provider === 'cohere'}
          disabled={switching !== null || !embStatus.cohere_key_present}
          onclick={() => pickProvider('cohere')}
        >
          <strong>Cohere</strong>
          <span>
            Cloud, nejlepší kvalita.
            {#if !embStatus.cohere_key_present}
              Ulož klíč níže.
            {/if}
          </span>
        </button>
        <button
          class="tile"
          class:active={embStatus.provider === 'fake'}
          disabled={switching !== null}
          onclick={() => pickProvider('fake')}
        >
          <strong>Heuristika</strong>
          <span>Offline, rychlé, přiměřená kvalita. Dobré pro testy.</span>
        </button>
        <button
          class="tile"
          class:active={embStatus.provider === 'none'}
          disabled={switching !== null}
          onclick={() => pickProvider('none')}
        >
          <strong>Žádný</strong>
          <span>Jen BM25 (klíčová slova). Nejrychlejší, nejhloupější.</span>
        </button>
      </div>

      <h4>Cohere API klíč</h4>
      <div class="login-row">
        <button class="secondary" onclick={openCohereDashboard}>
          Otevřít Cohere Dashboard →
        </button>
      </div>
      <form onsubmit={(e) => { e.preventDefault(); saveCohere(); }}>
        <input
          type="password"
          placeholder="co-XXXXXXXXXX…"
          bind:value={cohereKey}
          disabled={savingCohere}
        />
        <button class="primary" type="submit" disabled={savingCohere}>
          {savingCohere ? 'Ukládám…' : 'Uložit'}
        </button>
      </form>
    {/if}
  </div>
</details>

<!-- =================================================================
     3) Remix — LLM rephrase
     ================================================================= -->
<details class="section" open={openSection === 'remix'}>
  <summary onclick={(e) => { e.preventDefault(); toggle('remix'); }}>
    <span class="sec-num">3</span>
    <span class="sec-title">Remix (AI přepis)</span>
    <span class="sec-sub">
      {#if claudeSub?.detected && !claudeSub.expired}
        Přihlášen přes Claude {claudeSub.subscription_type ?? 'subscription'}
      {:else if anthropicPresent}
        Přihlášen přes BYOK klíč
      {:else}
        Nepřihlášen
      {/if}
    </span>
  </summary>
  <div class="section-body">
    <p class="muted">
      Claude Haiku přepíše věty z tvých materiálů tak, aby obsahovaly
      víc tvých aktuálně slabých kombinací kláves. Off by default, zapínáš
      u každého sezení zvlášť.
    </p>

    <!-- Primary: Claude subscription -->
    <div class="auth-card" class:ok={claudeSub?.detected && !claudeSub.expired}>
      <h4>
        Přihlášení přes Claude subscription
        {#if claudeSub?.detected && !claudeSub.expired}
          <span class="pill ok">Aktivní</span>
        {:else if claudeSub?.detected && claudeSub.expired}
          <span class="pill warn">Vypršelo</span>
        {/if}
      </h4>
      {#if claudeSub?.detected && !claudeSub.expired}
        <p>
          Datlino našlo přihlášený Claude Code na tomhle zařízení (zdroj:
          {claudeSub.source === 'file' ? 'credentials soubor' : 'klíčenka OS'}{#if claudeSub.subscription_type}
            , plán <strong>{claudeSub.subscription_type}</strong>{/if}). Remix
          bude používat tvoje předplatné.
        </p>
      {:else if claudeSub?.detected && claudeSub.expired}
        <p>
          Token vypršel. V terminálu spusť <code>claude login</code> a vrať se.
        </p>
      {:else}
        <p>
          Používáš Claude Pro nebo Max? Pokud máš Claude Code nainstalovaný
          a přihlášený (<code>claude login</code>), Datlino si vezme token
          automaticky — nemusíš platit zvlášť za API.
        </p>
      {/if}
    </div>

    <!-- Tier 2: Copy-paste (free, always available) -->
    <div class="auth-card ok">
      <h4>
        Copy-paste do volného LLM
        <span class="pill ok">Zdarma</span>
      </h4>
      <p>
        Když nemáš Claude subscription ani API klíč, Datlino ti připraví
        deterministický prompt s tvými větami a slabinami. Vlož ho do
        <strong>ChatGPT, Claude.ai nebo Gemini</strong> (free účty stačí),
        výsledek vrátíš zpátky a Datlino ho prožene podobnostní bránou
        stejně jako přímou API.
      </p>
      <p class="muted small">
        Žádný klíč, žádné platby. Trvá pár vteřin navíc na sezení. Volíš
        u každého sezení v sekci „Pokročilé → Remix" na <a href="/study">Učím se obsah</a>.
      </p>
    </div>

    <!-- Tier 3: Fallback BYOK API key -->
    <h4>Nebo BYOK Anthropic API klíč</h4>
    <p class="muted small">
      Alternativa, když chceš plně automatický remix — platíš za
      jednotlivé volání. Klíč se ukládá do klíčenky OS, ne do souboru.
      {#if anthropicPresent}<br /><strong>Aktuálně uložen.</strong>{/if}
    </p>

    {#if anthropicEnvKey && !anthropicPresent}
      <div class="suggestion">
        <span>Našli jsme <code>ANTHROPIC_API_KEY</code> v proměnných prostředí.</span>
        <button class="secondary" onclick={importEnvKey} disabled={savingAnth}>
          Importovat
        </button>
      </div>
    {/if}

    <div class="login-row">
      <button class="secondary" onclick={openConsole}>
        Otevřít Anthropic Console →
      </button>
    </div>
    <form onsubmit={(e) => { e.preventDefault(); saveAnthropic(); }}>
      <input
        type="password"
        placeholder="sk-ant-XXXXXXXXXX…"
        bind:value={anthropicKey}
        disabled={savingAnth}
      />
      <button class="primary" type="submit" disabled={savingAnth}>
        {savingAnth ? 'Ukládám…' : 'Uložit'}
      </button>
    </form>
  </div>
</details>

<!-- =================================================================
     4) OCR — tesseract / pdftoppm status + recheck
     ================================================================= -->
<details class="section" open={openSection === 'ocr'}>
  <summary onclick={(e) => { e.preventDefault(); toggle('ocr'); }}>
    <span class="sec-num">4</span>
    <span class="sec-title">OCR (skenované PDF, GoodNotes)</span>
    <span class="sec-sub">
      {#if ocr?.available}
        Připraveno
      {:else}
        Chybí binárky
      {/if}
    </span>
  </summary>
  <div class="section-body">
    <p class="muted">
      Když PDF obsahuje jen obrázky (sken učebnice nebo export z GoodNotes),
      Datlino přečte text pomocí <code>tesseract</code> +
      <code>pdftoppm</code>.
    </p>
    {#if ocr}
      <ul class="binlist">
        <li>
          <code>tesseract</code>:
          {#if ocr.tesseract}
            <span class="ok">k dispozici</span>
          {:else}
            <span class="warn">chybí</span>
          {/if}
        </li>
        <li>
          <code>pdftoppm</code>:
          {#if ocr.pdftoppm}
            <span class="ok">k dispozici</span>
          {:else}
            <span class="warn">chybí</span>
          {/if}
        </li>
      </ul>

      {#if !ocr.available}
        <p class="muted small">
          Na <strong>macOS</strong>: <code>brew install tesseract tesseract-lang poppler</code>.
          Na <strong>Ubuntu/Debian</strong>:
          <code>apt install tesseract-ocr tesseract-ocr-ces tesseract-ocr-slk tesseract-ocr-eng poppler-utils</code>.
        </p>
      {/if}

      <button class="secondary" onclick={recheckOcr} disabled={rechecking}>
        {rechecking ? 'Hledám…' : 'Zkontrolovat znovu'}
      </button>
    {/if}
  </div>
</details>

<style>
  .head { margin-bottom: 1rem; }
  .muted { color: #78716c; font-size: 0.9rem; }
  .small { font-size: 0.85rem; }
  .msg {
    padding: 0.55rem 0.8rem;
    border-radius: 5px;
    font-size: 0.9rem;
    margin-bottom: 0.8rem;
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

  .section {
    background: #fffaf2;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 10px;
    margin-bottom: 0.6rem;
    overflow: hidden;
  }
  .section > summary {
    padding: 1rem 1.2rem 1rem 1.2rem;
    cursor: pointer;
    display: grid;
    grid-template-columns: 2rem auto 1fr 1.2rem;
    gap: 0.8rem;
    align-items: center;
    list-style: none;
  }
  .section > summary::-webkit-details-marker { display: none; }
  .section > summary::after {
    content: '▾';
    font-size: 1.15rem;
    color: #78716c;
    justify-self: end;
    transition: transform 200ms ease;
    line-height: 1;
  }
  .section[open] > summary::after {
    transform: rotate(180deg);
  }
  .section > summary:hover::after { color: #b3271f; }
  .sec-num {
    font-size: 1rem;
    font-weight: 700;
    color: #b3271f;
    text-align: center;
    border: 1px solid rgba(179, 39, 31, 0.3);
    border-radius: 50%;
    width: 2rem;
    height: 2rem;
    line-height: 1.9rem;
  }
  .sec-title { font-size: 1rem; font-weight: 600; color: #1c1917; }
  .sec-sub { color: #78716c; font-size: 0.85rem; }
  .sec-sub strong { color: #292524; }
  .section-body {
    padding: 0.25rem 1.2rem 1.2rem;
    border-top: 1px dashed rgba(28, 25, 23, 0.08);
  }
  .section-body h4 {
    margin: 1rem 0 0.5rem;
    font-size: 0.95rem;
    color: #292524;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
    gap: 0.6rem;
    margin: 0.75rem 0;
  }
  .stat {
    background: #fff;
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 7px;
    padding: 0.5rem 0.7rem;
    display: flex;
    flex-direction: column;
  }
  .stat .label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #78716c;
  }
  .stat .value {
    font-size: 1.1rem;
    font-weight: 600;
    color: #1c1917;
  }

  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.5rem;
    margin: 0.75rem 0;
  }
  .tile {
    text-align: left;
    padding: 0.7rem 0.85rem;
    background: #fff;
    border: 1px solid rgba(28, 25, 23, 0.1);
    border-radius: 7px;
    cursor: pointer;
    font: inherit;
  }
  .tile.active {
    border-color: #b3271f;
    background: rgba(179, 39, 31, 0.06);
  }
  .tile strong {
    display: block;
    margin-bottom: 0.15rem;
    color: #b3271f;
    font-size: 0.9rem;
  }
  .tile span { font-size: 0.78rem; color: #57534e; }
  .tile:disabled { opacity: 0.5; cursor: not-allowed; }

  form {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  input[type='password'] {
    flex: 1;
    padding: 0.5rem 0.7rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    border-radius: 4px;
    font: inherit;
    background: #fff;
  }

  button.primary {
    padding: 0.5rem 1rem;
    border: 1px solid #b3271f;
    background: #b3271f;
    color: #fffaf2;
    border-radius: 5px;
    cursor: pointer;
    font: inherit;
    font-weight: 500;
  }
  button.secondary {
    padding: 0.4rem 0.8rem;
    border: 1px solid rgba(28, 25, 23, 0.2);
    background: transparent;
    color: #44403c;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
  }
  button.secondary:hover {
    border-color: #b3271f;
    color: #b3271f;
  }
  button:disabled { opacity: 0.5; cursor: not-allowed; }

  .login-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin: 0.5rem 0;
    flex-wrap: wrap;
  }
  .auth-card {
    padding: 0.8rem 1rem;
    background: rgba(28, 25, 23, 0.03);
    border: 1px solid rgba(28, 25, 23, 0.08);
    border-radius: 7px;
    margin: 0.75rem 0;
  }
  .auth-card.ok {
    background: rgba(45, 106, 45, 0.04);
    border-color: rgba(45, 106, 45, 0.25);
  }
  .auth-card h4 { margin-top: 0; display: flex; gap: 0.5rem; align-items: center; }
  .auth-card p { margin: 0.3rem 0; font-size: 0.88rem; color: #292524; }
  .pill {
    font-size: 0.7rem;
    padding: 0.1rem 0.45rem;
    border-radius: 20px;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    font-weight: 600;
  }
  .pill.ok { background: rgba(45, 106, 45, 0.15); color: #2d6a2d; }
  .pill.warn { background: rgba(179, 39, 31, 0.12); color: #b3271f; }

  .suggestion {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    padding: 0.5rem 0.8rem;
    background: rgba(179, 39, 31, 0.05);
    border: 1px solid rgba(179, 39, 31, 0.15);
    border-radius: 6px;
    margin: 0.5rem 0;
    font-size: 0.85rem;
  }

  .binlist {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
    display: flex;
    gap: 1.5rem;
    font-size: 0.9rem;
  }
  .ok { color: #2d6a2d; font-weight: 600; }
  .warn { color: #b3271f; font-weight: 600; }

  code {
    background: rgba(28, 25, 23, 0.05);
    padding: 0.05rem 0.3rem;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  a { color: #b3271f; }
</style>
