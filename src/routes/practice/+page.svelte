<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  // /practice used to be the mode picker. After the IA reorg, the two
  // top-level paths are /learn (keystrokes) and /study (content). We
  // keep this URL live because older shortcuts and the summary screen
  // deep-link to /practice/... — but the landing redirects home, where
  // the student picks a door.
  onMount(() => {
    goto('/', { replaceState: true });
  });
</script>

<p class="muted">Přesměrovávám…</p>

<style>
  .muted { color: #78716c; }
</style>
