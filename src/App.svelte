<script lang="ts">
  /**
   * The window: three panes around a page.
   *
   * A rail of sections on the left, the proof spread (or a table, or a
   * gallery) in the centre, and on the right the inspector holding the
   * settings of the open section. The top bar names the project and holds
   * the way to any setting and the button that makes the book; the status
   * bar along the bottom says what the build is doing and, under the
   * pointer, what any control does. Before a project is open there is only
   * the welcome screen: a rail of disabled sections over nothing would be
   * furniture standing in for an application that has not been given
   * anything to do yet.
   */
  import BooksInspector from "./components/BooksInspector.svelte";
  import BooksScreen from "./components/BooksScreen.svelte";
  import BuildInspector from "./components/BuildInspector.svelte";
  import BuildScreen from "./components/BuildScreen.svelte";
  import HeadersInspector from "./components/HeadersInspector.svelte";
  import PageInspector from "./components/PageInspector.svelte";
  import Palette from "./components/Palette.svelte";
  import ProofSpread from "./components/ProofSpread.svelte";
  import Rail from "./components/Rail.svelte";
  import SettingsInspector from "./components/SettingsInspector.svelte";
  import StatusBar from "./components/StatusBar.svelte";
  import StyleInspector from "./components/StyleInspector.svelte";
  import StylesInspector from "./components/StylesInspector.svelte";
  import TemplateInspector from "./components/TemplateInspector.svelte";
  import TemplateScreen from "./components/TemplateScreen.svelte";
  import TopBar from "./components/TopBar.svelte";
  import Welcome from "./components/Welcome.svelte";
  import { TABS } from "./lib/labels";
  import { session } from "./lib/session.svelte";
  import { applyPreferences, installViewShortcuts } from "./lib/preferences.svelte";
  import { installHoverHelp } from "./lib/hover.svelte";
  import { shownStyle, ui } from "./lib/ui.svelte";
  import { phrases, t } from "./lib/i18n";

  // The theme and zoom this person keeps, and the keys that change them.
  $effect(() => {
    applyPreferences();
    return installViewShortcuts();
  });
  // What the pointer is on, for the status bar.
  $effect(() => installHoverHelp());

  $effect(() => {
    void session.start();
    return () => session.stop();
  });

  // `TABS` is a non-empty constant, but its type does not say so — and a
  // stored pane id from an older build could name a section that no longer
  // exists.
  const tab = $derived(TABS.find((x) => x.id === session.pane) ?? TABS[0]!);
  const inspecting = $derived(tab.styles && session.stylePane === "inspect");

  /** Ctrl K opens the palette from anywhere, once there is something to search. */
  function onkeydown(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k") {
      if (!session.project) return;
      event.preventDefault();
      ui.palette = !ui.palette;
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="app" class:narrow={tab.narrow}>
  <TopBar />

  {#if session.project && session.changedCount > 0}
    <!--
      FUN-007 offers a reload rather than performing one: a reload throws
      away nothing, but doing it under someone mid-edit would move the form
      they were reading. The notice is the offer.
    -->
    <div class="notice">
      <span>{phrases().changedOnDisk(session.changedCount)}</span>
      <button
        type="button"
        class="link"
        data-search-key="action:reload"
        title={session.changedNames.join(", ")}
        onclick={() => void session.reopen()}
      >
        {t("reload")}
      </button>
    </div>
  {/if}

  {#if session.fault}
    <p class="fault" role="alert">{session.fault}</p>
  {/if}

  {#if session.openingWhat}
    <!-- The welcome screen goes the moment a folder is chosen, not when it
         has finished being read: a start screen still offering the folder
         you just clicked is the application looking like it did not hear. -->
    <section class="loading">
      <p class="what">{t("loading")}</p>
      <p class="mono where">{session.openingWhat}</p>
    </section>
  {:else if !session.project}
    <Welcome />
  {:else}
    {#if session.created}
      <!-- A project that has just been made has a settings file and no
           Scripture. Nothing else on screen would say what stands between it
           and a book. -->
      <p class="notice">
        <span>{t("copyUsfmBefore")}<span class="mono">{session.created}</span>{t("copyUsfmAfter")}</span>
      </p>
    {/if}

    <main>
      <Rail />

      <div class="centre">
        {#if tab.canvas === "books"}
          <BooksScreen />
        {:else if tab.canvas === "templates"}
          <TemplateScreen />
        {:else if tab.canvas === "build"}
          <BuildScreen />
        {:else if inspecting}
          <StyleInspector />
        {:else}
          <ProofSpread guides={tab.trim ?? false} slots={tab.headers ?? false} outline={shownStyle()} />
        {/if}
      </div>

      {#if tab.id === "books"}
        <BooksInspector />
      {:else if tab.id === "template"}
        <TemplateInspector />
      {:else if tab.trim}
        <PageInspector />
      {:else if tab.headers}
        <HeadersInspector />
      {:else if tab.styles}
        <StylesInspector />
      {:else if tab.canvas === "build"}
        <BuildInspector />
      {:else}
        <SettingsInspector {tab} />
      {/if}
    </main>

    <StatusBar />
  {/if}

  {#if ui.palette}
    <Palette />
  {/if}
</div>

<style>
  /* The window, top to bottom: the bar, the notices, the three panes, the
     status line. Only the panes grow. */
  .app {
    display: flex;
    flex-direction: column;
    block-size: 100%;
    background: var(--bg);
    color: var(--ink);
  }
  .app.narrow {
    --inspector-w: 340px;
  }
  main {
    display: flex;
    flex: 1;
    min-block-size: 0;
  }
  .centre {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-inline-size: 0;
    min-block-size: 0;
  }
  .notice {
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 0;
    padding: 6px 20px;
    border-block-end: 1px solid var(--line);
    background: var(--warn-bg);
    color: var(--warn-ink);
    font-size: 12.5px;
  }
  .notice .mono {
    overflow-wrap: anywhere;
  }
  .fault {
    flex: none;
    margin: 0;
    padding: 8px 20px;
    border-inline-start: 3px solid var(--err);
    background: var(--err-bg);
    color: var(--err-ink);
    font-size: 12.5px;
  }
  .loading {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }
  .what {
    margin: 0;
    font: 500 20px var(--serif);
  }
  .where {
    margin: 0;
    color: var(--mut);
    overflow-wrap: anywhere;
  }
</style>
