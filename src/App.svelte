<script lang="ts">
  import BuildBar from "./components/BuildBar.svelte";
  import DiagnosticsPanel from "./components/DiagnosticsPanel.svelte";
  import ProjectPane from "./components/ProjectPane.svelte";
  import QuickSettings from "./components/QuickSettings.svelte";
  import ExamplePage from "./components/ExamplePage.svelte";
  import PageDiagram from "./components/PageDiagram.svelte";
  import PresetPicker from "./components/PresetPicker.svelte";
  import SettingsForm from "./components/SettingsForm.svelte";
  import StartScreen from "./components/StartScreen.svelte";
  import StyleEditor from "./components/StyleEditor.svelte";
  import StyleInspector from "./components/StyleInspector.svelte";
  import { STYLE_TABS, TABS } from "./lib/labels";
  import { session } from "./lib/session.svelte";
  import { t } from "./lib/i18n";

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
      <button type="button" class="close" onclick={() => void session.close()}>{t("closeProject")}</button>
    </div>
  {/if}

  {#if session.fault}
    <p class="fault" role="alert">{session.fault}</p>
  {/if}

  {#if session.openingWhat}
    <!--
      The start screen goes the moment a folder is chosen, not when it has
      finished being read. Reading a whole Bible is seconds of parsing, and a
      start screen still offering the folder you just clicked is the
      application looking like it did not hear you.
    -->
    <section class="loading">
      <p class="what">{t("loading")}</p>
      <p class="where">{session.openingWhat}</p>
    </section>
  {:else if !session.project}
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
      <div class="right">
      <!-- One tab at a time: each is long on its own, and the page geometry, what
           appears on it, and how it is set are three decisions a publisher makes
           at three separate times. Tabs rather than an accordion so the choice
           survives an edit, which reopens the project and would otherwise
           collapse it. -->
      <nav class="tabs" aria-label={t("configurationRegion")}>
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

      <!-- The tabs stay put and the form moves under them. Outside a scroller
           the Page section alone pushes the build bar off the bottom of the
           window, and the control you press after changing something should not
           be the one you have to go looking for. -->
      <div class="body" class:sectioned={tab.styles}>
        {#if tab.styles}
          <!-- Down the side rather than across the top: there are seven of
               them, they are nouns rather than steps, and a row of seven wraps
               to two lines at any width this window is likely to be — which
               moves the section you are reading every time the window is
               resized. A column also leaves the names left-aligned, so they
               read as a list of what can be styled. -->
          <nav class="subtabs" aria-label={t("stylesSectionsRegion")}>
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

        {#if tab.books}
          <!-- Outside the scroller, because this pane does its own: a whole
               Bible is two columns of rows that each scroll, and a scroller
               around a scroller gives you two bars and no way to know which
               one you are dragging. -->
          <ProjectPane />
        {:else}
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
            {#if tab.diagram}
              <!-- Above the diagram, because it is the thing to reach for
                   before adjusting the measurements one at a time, and the
                   diagram beside it shows what each one did. -->
              <PresetPicker />
            {/if}
            {#if tab.diagram && session.geometry}
              <PageDiagram geometry={session.geometry} />
            {/if}
            {#if tab.example}
              <ExamplePage which={tab.example} />
            {/if}
            {#if tab.settingGroups.length > 0 || tab.orphans}
              <SettingsForm groups={tab.settingGroups} orphans={tab.orphans ?? false} />
            {/if}
          {/if}
        </div>
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
  .loading {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    flex: 1;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }
  .what {
    margin: 0;
    font-size: 1rem;
  }
  .where {
    overflow-wrap: anywhere;
    margin: 0;
    font-size: 0.8rem;
    opacity: 0.55;
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
  /* The pane, and — on the Styles tab — the list of sections beside it. */
  .body {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-block-size: 0;
  }
  .body.sectioned {
    flex-direction: row;
    gap: 0.9rem;
  }
  .subtabs {
    display: flex;
    flex-direction: column;
    flex: none;
    gap: 0.1rem;
    min-inline-size: 8rem;
    padding-inline-end: 0.6rem;
    border-inline-end: 1px solid color-mix(in oklab, currentColor 15%, transparent);
    overflow-y: auto;
  }
  .subtabs button {
    padding-block: 0.25rem;
    padding-inline: 0.5rem;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 0.82rem;
    /* Left-aligned, because down the side they read as a list of what can be
       styled rather than as a row of buttons. */
    text-align: start;
    cursor: pointer;
    opacity: 0.75;
  }
  .subtabs button:hover {
    background: color-mix(in oklab, currentColor 8%, transparent);
    opacity: 1;
  }
  .subtabs button.active {
    background: color-mix(in oklab, currentColor 14%, transparent);
    font-weight: 600;
    opacity: 1;
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
  /* The whole width now. The books used to hold a permanent column beside
     this, which cost every other tab a third of the window for a question
     already answered — and left the book list itself too narrow to put the
     two testaments side by side. They are a tab of their own instead. */
  main {
    display: flex;
    /* Takes the space the strips and the bar leave. `min-block-size: 0` is
       what lets it *shrink* too — without it a flex child refuses to go below
       its content, and a long book list would push the build bar off the
       bottom of the window rather than scrolling. */
    flex: 1;
    min-block-size: 0;
    padding-block-end: 0.75rem;
  }
  /* A column of rows that do not grow — the tabs, the hint — and one that
     does. The pane scrolls inside itself rather than the whole side
     scrolling, which is what keeps the section list beside the Styles pane
     from scrolling away with the form it chooses. */
  .right {
    display: flex;
    flex-direction: column;
    flex: 1;
    gap: 1rem;
    min-inline-size: 0;
    min-block-size: 0;
    max-block-size: 100%;
    overflow: hidden;
  }
</style>
