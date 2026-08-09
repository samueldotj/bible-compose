<script lang="ts">
  import BuildBar from "./components/BuildBar.svelte";
  import BuildLog from "./components/BuildLog.svelte";
  import DiagnosticsPanel from "./components/DiagnosticsPanel.svelte";
  import ProjectPane from "./components/ProjectPane.svelte";
  import SettingsForm from "./components/SettingsForm.svelte";
  import { session } from "./lib/session.svelte";

  $effect(() => {
    void session.start();
    return () => session.stop();
  });

  function folderName(path: string): string {
    return path.split(/[/\\]/).filter(Boolean).pop() ?? path;
  }
</script>

<header class="top">
  <h1>BibleCompose</h1>

  <button type="button" class="open" onclick={() => void session.choose()} disabled={session.opening}>
    {session.opening ? "Opening…" : "Open project…"}
  </button>

  {#if session.project}
    <span class="project" title={session.project.root}>{folderName(session.project.root)}</span>
    <button type="button" class="reload" onclick={() => void session.reopen()}>Reload</button>
  {/if}

  {#if session.versions}
    <span class="versions">
      {session.versions.app} · contract {session.versions.contract}
    </span>
  {/if}
</header>

{#if session.fault}
  <p class="fault" role="alert">{session.fault}</p>
{/if}

{#if !session.project}
  <p class="welcome">
    Open a folder of USFM. Anything the project does not configure uses the built-in defaults, so a
    folder with nothing but Scripture in it will build.
  </p>
{/if}

<main>
  <div class="left">
    <ProjectPane />
    <DiagnosticsPanel />
  </div>
  <div class="right">
    <SettingsForm />
    <BuildLog />
  </div>
</main>

<BuildBar />

<style>
  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.8rem;
    align-items: baseline;
    padding-block-end: 0.6rem;
  }
  .open,
  .reload {
    padding-block: 0.25rem;
    padding-inline: 0.7rem;
    border: 1px solid color-mix(in oklab, currentColor 30%, transparent);
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .open:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .project {
    font-weight: 600;
  }
  .versions {
    margin-inline-start: auto;
    font-size: 0.78rem;
    opacity: 0.5;
  }
  .fault {
    margin: 0 0 0.6rem;
    padding: 0.5rem 0.7rem;
    border-inline-start: 3px solid #c0392b;
    background: color-mix(in oklab, #c0392b 8%, transparent);
    font-size: 0.85rem;
  }
  .welcome {
    max-inline-size: 46rem;
    margin-block: 0 1rem;
    opacity: 0.75;
  }
  main {
    display: grid;
    grid-template-columns: minmax(16rem, 22rem) 1fr;
    gap: 1rem;
    align-items: start;
  }
  .left,
  .right {
    display: grid;
    gap: 1rem;
    min-inline-size: 0;
  }
  /* One column below the two-pane threshold, which is also where a settings
     form stops fitting beside a book list. */
  @media (width < 62rem) {
    main {
      grid-template-columns: 1fr;
    }
  }
</style>
