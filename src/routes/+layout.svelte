<script lang="ts">
  import { page } from '$app/state';
  import { profile, refreshProfile } from '$lib/profile.svelte';

  let { children } = $props();

  $effect(() => {
    refreshProfile();
  });

  const navItems = [
    { href: '/', label: 'Knihovna' },
    { href: '/practice', label: 'Trénink' },
    { href: '/progress', label: 'Pokrok' },
    { href: '/settings', label: 'Nastavení' }
  ];

  function isActive(href: string, pathname: string): boolean {
    if (href === '/') return pathname === '/';
    return pathname === href || pathname.startsWith(href + '/');
  }
</script>

<main>
  <header>
    <div class="brand">
      <a href="/">
        <h1>Datlino</h1>
        <p class="tagline">Piš to, co se učíš.</p>
      </a>
    </div>
    <nav>
      {#each navItems as item (item.href)}
        <a href={item.href} class:active={isActive(item.href, page.url.pathname)}>
          {item.label}
        </a>
      {/each}
    </nav>
    {#if profile.data}
      <div class="profile-strip">
        <span title="XP z dokončených sezení"><strong>L{profile.data.level}</strong> · {profile.data.total_xp} XP</span>
        <span title="Série dnů s tréninkem">🔥 {profile.data.current_streak}</span>
      </div>
    {/if}
  </header>
  {@render children()}
</main>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: #f7f3ec;
    color: #1c1917;
    font-family: ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif;
  }

  :global(*) {
    box-sizing: border-box;
  }

  main {
    max-width: 56rem;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
  }

  header {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 2rem;
    border-bottom: 1px solid rgba(28, 25, 23, 0.12);
    padding-bottom: 1rem;
    margin-bottom: 2rem;
  }

  .brand a {
    text-decoration: none;
    color: inherit;
  }

  h1 {
    margin: 0;
    font-size: 1.8rem;
    letter-spacing: -0.02em;
    color: #b3271f;
  }

  .tagline {
    margin: 0.1rem 0 0;
    color: #57534e;
    font-size: 0.85rem;
  }

  nav {
    display: flex;
    gap: 1rem;
    justify-self: center;
  }

  nav a {
    color: #44403c;
    text-decoration: none;
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    font-size: 0.95rem;
  }

  nav a.active {
    background: rgba(179, 39, 31, 0.08);
    color: #b3271f;
    font-weight: 600;
  }

  .profile-strip {
    display: flex;
    gap: 1rem;
    font-size: 0.9rem;
    color: #44403c;
  }

  .profile-strip strong {
    color: #b3271f;
  }
</style>
