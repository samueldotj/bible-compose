<script lang="ts">
  import { backend } from "./lib/services/backend";

  // Deliberately the whole of P2.1's interface: an empty window that proves
  // the shell, the bundle and the service boundary are wired together. The
  // project pane and the settings forms are P2.8.
  let versions = $state<{ app: string; contract: string; backend: string } | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    backend()
      .versions()
      .then((v) => (versions = v))
      .catch((e: unknown) => (error = String(e)));
  });
</script>

<main>
  <h1>BibleCompose</h1>
  {#if versions}
    <p class="status">
      {versions.app} · contract {versions.contract} · {versions.backend}
    </p>
  {:else if error}
    <p class="status">{error}</p>
  {:else}
    <p class="status">Starting…</p>
  {/if}
</main>
