<script lang="ts">
  import BuildBar from "./components/BuildBar.svelte";
  import DiagnosticsPanel from "./components/DiagnosticsPanel.svelte";
  import ProjectPane from "./components/ProjectPane.svelte";
  import QuickSettings from "./components/QuickSettings.svelte";
  import AppearsExample from "./components/AppearsExample.svelte";
  import PageDiagram from "./components/PageDiagram.svelte";
  import SettingsForm from "./components/SettingsForm.svelte";
  import StartScreen from "./components/StartScreen.svelte";
  import StyleEditor from "./components/StyleEditor.svelte";
  import StyleInspector from "./components/StyleInspector.svelte";
  import { STYLE_TABS, TABS } from "./lib/labels";
  import { session } from "./lib/session.svelte";

  $effect(() => {
    void session.start();
    return () => session.stop();
  });

  // `TABS` is a non-empty constant, but its type does not say so — and a
  // stored pane id from an older build could name a tab that no longer exists.
  const tab = $derived(TABS.find((t) => t.id === session.pane) ?? TABS[0]!);
  const styleTab = $derived(STYLE_TABS.find((t) => t.id === session.stylePane) ?? STYLE_TABS[0]!);

</script>

<!--
  The header and the build bar belong to a project. Before one is open there is
  nothing to reload, nothing to build, and no publication to name — a strip of
  disabled controls over a start screen would be furniture standing in for an
  application that has not been given anything to do yet.
-->
<div class="app">
  <!--
    A header only when there is something to put in it. What used to be here —
    the folder name and the version — is in the title bar now, and an empty strip
    above the workspace is a margin pretending to be a toolbar.
  -->
  {#if session.project && session.changedCount > 0}
    <header class="top">
      <!--
        FUN-007 offers a reload rather than performing one: a reload throws away
        nothing, but doing it under someone mid-edit would move the form they
        were reading. The notice *is* the offer — a standing Reload button beside
        it was a control that did nothing on almost every page view, and the
        moment it is worth pressing is exactly the moment this appears.
      -->
      <button
        type="button"
        class="changed"
        title={`${session.changedNames.join(", ")} — click to read the project again`}
        onclick={() => void session.reopen()}
      >
        {session.changedCount}
        {session.changedCount === 1 ? "file has" : "files have"} changed on disk — reload
      </button>
    </header>

    <!-- What the publication is and what language it is in. They are answered
         once, when a folder is first opened, which is not a reason to keep them
         behind a tab for the rest of the project's life. -->
    <div class="identity">
      <QuickSettings keys={["project.name", "project.language"]} width="12rem" />
      <!-- Beside what it closes: the strip is what this project *is*, and
           putting it down belongs with the two things that name it. -->
      <button type="button" class="close" onclick={() => void session.close()}>Close project</button>
    </div>
  {/if}

  {#if session.fault}
    <p class="fault" role="alert">{session.fault}</p>
  {/if}

  {#if !session.project}
    <StartScreen />
  {:else}
    {#if session.created}
      <!-- A project that has just been made has a settings file and no
           Scripture. Nothing else on screen would say what stands between it
           and a book. -->
      <p class="next-step">
        Copy your USFM files into <code>{session.created}</code>, then Reload.
      </p>
    {/if}

    <main>
      <div class="left">
        <ProjectPane />
        <!-- Under the books, because that is what most of them are about, and
             because the count is the reason to open it at all. -->
        <button
          type="button"
          class="problems"
          class:bad={session.errorCount > 0}
          onclick={() => (session.showProblems = true)}
        >
          Problems
          <span class="tally">{session.problemCount}</span>
        </button>
      </div>
      <div class="right">
      <!-- One tab at a time: each is long on its own, and the page geometry, what
           appears on it, and how it is set are three decisions a publisher makes
           at three separate times. Tabs rather than an accordion so the choice
           survives an edit, which reopens the project and would otherwise
           collapse it. -->
      <nav class="tabs" aria-label="Configuration">
        {#each TABS as t (t.id)}
          <button
            type="button"
            class:active={session.pane === t.id}
            aria-current={session.pane === t.id ? "true" : undefined}
            onclick={() => (session.pane = t.id)}
          >
            {t.title}
          </button>
        {/each}
      </nav>

      {#if !session.editable}
        <p class="hint">
          The built-in defaults, which is what a folder with no project files gets. Open a project to
          change them.
        </p>
      {/if}

      {#if tab.styles}
        <nav class="tabs subtabs" aria-label="Styles sections">
          {#each STYLE_TABS as s (s.id)}
            <button
              type="button"
              class:active={session.stylePane === s.id}
              aria-current={session.stylePane === s.id ? "true" : undefined}
              onclick={() => (session.stylePane = s.id)}
            >
              {s.title}
            </button>
          {/each}
        </nav>
      {/if}

      <!-- The tabs stay put and the form moves under them. Outside a scroller
           the Page section alone pushes the Build button off the bottom of the
           window, and the control you press after changing something should not
           be the one you have to go looking for. -->
      <div class="scroller">
        {#if tab.styles}
          {#if styleTab.inspector}
            <StyleInspector />
          {/if}
          {#if styleTab.settingGroups.length > 0}
            <SettingsForm groups={styleTab.settingGroups} />
          {/if}
          {#if styleTab.styleGroups.length > 0}
            <StyleEditor groups={styleTab.styleGroups} />
          {/if}
        {:else}
          {#if tab.diagram && session.geometry}
            <PageDiagram geometry={session.geometry} />
          {/if}
          {#if tab.example}
            <AppearsExample />
          {/if}
          {#if tab.settingGroups.length > 0 || tab.orphans}
            <SettingsForm groups={tab.settingGroups} orphans={tab.orphans ?? false} />
          {/if}
        {/if}
      </div>
    </div>
    </main>
  {/if}

  {#if session.project}
    <BuildBar />
  {/if}

  {#if session.showProblems}
    <DiagnosticsPanel onclose={() => (session.showProblems = false)} />
  {/if}
</div>

<style>
  .top {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 0.8rem;
    align-items: baseline;
    padding-block-end: 0.6rem;
  }
  .close {
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
  .changed {
    padding-block: 0.2rem;
    padding-inline: 0.5rem;
    border: 1px solid #b8860b;
    border-radius: 5px;
    background: transparent;
    font: inherit;
    font-size: 0.78rem;
    color: #8a6100;
    cursor: pointer;
  }
  .problems {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    justify-content: center;
    padding-block: 0.4rem;
    border: 1px solid color-mix(in oklab, currentColor 25%, transparent);
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 0.88rem;
    cursor: pointer;
  }
  .problems:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
  }
  .problems.bad {
    border-color: #c0392b;
    color: #c0392b;
    font-weight: 600;
  }
  .problems .tally {
    padding-inline: 0.4rem;
    border-radius: 999px;
    background: color-mix(in oklab, currentColor 15%, transparent);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  .fault {
    flex: none;
    margin: 0 0 0.6rem;
    padding: 0.5rem 0.7rem;
    border-inline-start: 3px solid #c0392b;
    background: color-mix(in oklab, #c0392b 8%, transparent);
    font-size: 0.85rem;
  }
  .identity {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem 0.9rem;
    align-items: center;
    padding-block: 0.35rem 0.55rem;
    padding-inline: 1rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 12%, transparent);
  }
  .next-step {
    flex: none;
    margin: 0;
    padding-block: 0.5rem;
    padding-inline: 1rem;
    background: color-mix(in oklab, currentColor 8%, transparent);
    font-size: 0.85rem;
  }
  .next-step code {
    overflow-wrap: anywhere;
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    border-block-end: 1px solid color-mix(in oklab, currentColor 15%, transparent);
  }
  .tabs button {
    padding-block: 0.3rem;
    padding-inline: 0.8rem;
    border: 0;
    border-block-end: 2px solid transparent;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.85rem;
    opacity: 0.6;
    cursor: pointer;
  }
  /* Quieter than the tabs above them, so the two rows read as a hierarchy
     rather than as eleven equal choices. */
  .subtabs {
    flex-wrap: wrap;
    border-block-end-style: dashed;
    margin-block-start: -0.4rem;
  }
  .subtabs button {
    padding-block: 0.2rem;
    padding-inline: 0.55rem;
    font-size: 0.78rem;
  }
  .scroller {
    /* Bounded by the column now rather than by a share of the viewport, which
       was a guess at how much room the rest of the window wanted. */
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
    /* Room for the scrollbar, so a value in the rightmost column is never
       under it. */
    padding-inline-end: 0.4rem;
  }
  .hint {
    margin-block: 0;
    font-size: 0.82rem;
    opacity: 0.7;
  }
  .tabs button.active {
    border-block-end-color: currentColor;
    opacity: 1;
    font-weight: 600;
  }
  /* The window, top to bottom: the strips that describe the project, the
     workspace, and the build bar on the bottom edge. Only the middle grows. */
  .app {
    display: flex;
    flex-direction: column;
    block-size: 100%;
    padding-inline: 1rem;
    padding-block-start: 0.75rem;
  }
  main {
    display: grid;
    grid-template-columns: minmax(16rem, 22rem) 1fr;
    gap: 1rem;
    align-items: start;
    /* Takes the space the strips and the bar leave. `min-block-size: 0` is
       what lets it *shrink* too — without it a flex child refuses to go below
       its content, and a long book list would push the build bar off the
       bottom of the window rather than scrolling. */
    flex: 1;
    min-block-size: 0;
    padding-block-end: 0.75rem;
  }
  .left,
  .right {
    display: grid;
    gap: 1rem;
    min-inline-size: 0;
    /* Each column keeps to its own space, so a hundred diagnostics do not
       decide how much of the settings form is visible. */
    min-block-size: 0;
    max-block-size: 100%;
  }
  /* The book list takes the column and the Problems button sits under it,
     rather than the list stopping at some height and leaving the rest of the
     column empty. The list scrolls inside what it is given. */
  .left {
    grid-template-rows: 1fr auto;
    overflow: hidden;
  }
  .right {
    align-content: start;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  /* One column below the two-pane threshold, which is also where a settings
     form stops fitting beside a book list. */
  @media (width < 62rem) {
    main {
      grid-template-columns: 1fr;
    }
  }
</style>
