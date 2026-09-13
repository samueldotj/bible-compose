<script lang="ts">
  /**
   * What the window shows before a project is open (PRJ-001).
   *
   * On the left, what this is and the two ways to arrive at a project; on
   * the right, the projects this machine has opened, because the folder
   * somebody wants is usually one they opened yesterday. Under the name,
   * how to get a Bible for somebody who has none — which is most people
   * opening this for the first time.
   */
  import NewProject from "./NewProject.svelte";
  import { session } from "../lib/session.svelte";
  import { phrases, t } from "../lib/i18n";

  let starting = $state(false);

  const OPEN_BIBLE = "https://www.open.bible/bibles";
</script>

<div class="welcome">
  <section class="intro">
    <div class="kicker big">{t("welcomeKicker")}</div>
    <h1>{t("appName")}</h1>
    <p class="lede">{t("startIntro")}</p>
    <div class="actions">
      <button type="button" class="btn primary tall" data-search-key="action:open-project" onclick={() => void session.choose()}>
        {t("openProjectEllipsis")}
      </button>
      <button type="button" class="btn tall" data-search-key="action:new-project" onclick={() => (starting = true)}>
        {t("newProjectEllipsis")}
      </button>
    </div>
    <hr />
    <h3>{t("startExistingTitle")}</h3>
    <ol class="steps">
      <li>
        <span class="n">1</span>
        <span
          >{t("startStepDownloadBefore")}&nbsp;<a
            href={OPEN_BIBLE}
            onclick={(e) => {
              e.preventDefault();
              void session.openUrl(OPEN_BIBLE);
            }}>{t("openBible")}</a
          >{t("startStepDownloadAfter")}</span
        >
      </li>
      <li><span class="n">2</span><span>{t("startStepExtract")}</span></li>
      <li><span class="n">3</span><span>{t("startStepSelect")}</span></li>
    </ol>
    {#if session.versions}
      <p class="mono versions">
        biblecompose {session.versions.app} · {t("contract")} {session.versions.contract}
      </p>
    {/if}
  </section>

  <section class="recent">
    <div class="head">
      <h3>{t("recentProjects")}</h3>
      <span class="muted">{session.recent.length}</span>
    </div>
    <ul>
      {#each session.recent as item (item.root)}
        <li class:missing={item.missing}>
          <span class="thumb" aria-hidden="true"><span></span><span></span></span>
          <div class="about">
            <div class="name">{item.name}</div>
            <div class="mono path">{item.root}</div>
            {#if item.missing}
              <div class="muted small">{t("noLongerThere")}</div>
            {/if}
          </div>
          <div class="do">
            <button type="button" class="btn small" disabled={item.missing} onclick={() => void session.open(item.root)}>
              {t("open")}
            </button>
            <button
              type="button"
              class="forget"
              title={phrases().forgetProject(item.name)}
              aria-label={phrases().forgetProject(item.name)}
              onclick={() => void session.forget(item.root)}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
                <path d="M18 6 6 18"></path><path d="m6 6 12 12"></path>
              </svg>
            </button>
          </div>
        </li>
      {/each}
    </ul>
    <p class="muted small note">{t("recentNote")}</p>
  </section>
</div>

{#if starting}
  <NewProject onclose={() => (starting = false)} />
{/if}

<style>
  .welcome {
    flex: 1;
    min-block-size: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    overflow: auto;
  }
  @media (max-width: 980px) {
    .welcome {
      grid-template-columns: 1fr;
    }
  }
  .intro {
    display: flex;
    flex-direction: column;
    padding: 72px 64px 40px 88px;
  }
  .kicker.big {
    font-size: 15px;
  }
  h1 {
    margin: 10px 0 20px;
    font: 500 64px/1 var(--serif);
    letter-spacing: -0.02em;
  }
  .lede {
    max-inline-size: 440px;
    margin: 0 0 28px;
    font-size: 16px;
    line-height: 1.5;
    color: var(--mut);
  }
  .actions {
    display: flex;
    gap: 10px;
  }
  .btn.tall {
    block-size: 40px;
    padding: 0 20px;
    font-size: 14px;
  }
  hr {
    inline-size: 100%;
    max-inline-size: 480px;
    margin: 44px 0 22px;
    border: 0;
    border-block-start: 1px solid var(--line2);
  }
  h3 {
    font: italic 500 16px var(--serif);
    margin-block-end: 12px;
  }
  .steps {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 10px;
    max-inline-size: 480px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 14px;
    line-height: 1.5;
  }
  .steps li {
    display: contents;
  }
  .n {
    font: 500 16px var(--serif);
    color: var(--acc);
  }
  a {
    color: var(--acc);
  }
  .versions {
    margin: auto 0 0;
    padding-block-start: 40px;
    color: var(--mut);
  }

  .recent {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
    padding: 72px 88px 40px 40px;
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding-block-end: 10px;
    border-block-end: 1px solid var(--line2);
  }
  .head h3 {
    margin: 0;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
  }
  li {
    display: grid;
    grid-template-columns: 44px 1fr auto;
    gap: 0 16px;
    align-items: center;
    padding: 16px 0;
    border-block-end: 1px solid var(--line);
  }
  .thumb {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px;
    inline-size: 44px;
    block-size: 58px;
    padding: 8px 6px;
    border: 1px solid var(--line2);
    border-radius: 2px;
    background: var(--paper);
  }
  .thumb span {
    background: var(--line);
  }
  .about {
    min-inline-size: 0;
  }
  .name {
    font: 500 19px var(--serif);
  }
  li.missing .name {
    text-decoration: line-through;
    color: var(--mut);
  }
  .path {
    margin-block-start: 2px;
    color: var(--mut);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .small {
    font-size: 12px;
  }
  .do {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .forget {
    display: inline-flex;
    padding: 6px;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--mut);
    cursor: pointer;
  }
  .forget:hover {
    background: var(--line);
    color: var(--ink);
  }
  .note {
    padding: 14px 0;
    margin: 0;
  }
</style>
