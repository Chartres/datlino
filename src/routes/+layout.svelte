<script lang="ts">
  let { children } = $props();
</script>

<main>
  <header>
    <h1>Datlino</h1>
    <p class="tagline">Piš to, co se učíš.</p>
  </header>
  {@render children()}
</main>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: #f7f3ec;
    color: #1c1917;
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  }

  main {
    max-width: 56rem;
    margin: 0 auto;
    padding: 2rem 1.5rem 4rem;
  }

  header {
    border-bottom: 1px solid rgba(28, 25, 23, 0.12);
    padding-bottom: 1rem;
    margin-bottom: 2rem;
  }

  h1 {
    margin: 0;
    font-size: 2rem;
    letter-spacing: -0.02em;
    color: #b3271f; /* woodpecker red */
  }

  .tagline {
    margin: 0.25rem 0 0;
    color: #57534e;
    font-size: 0.95rem;
  }
</style>
