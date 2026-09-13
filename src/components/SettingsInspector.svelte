<script lang="ts">
  /**
   * The inspector for a section whose settings are plain rows: the switch
   * groups of the proof-spread sections, the setting groups of the others,
   * and — on the one section that claims them — every key nobody has given
   * a home, so a setting added to the schema is visible somewhere rather
   * than nowhere.
   */
  import Inspector from "./ui/Inspector.svelte";
  import SettingRow from "./ui/SettingRow.svelte";
  import { EDITED_ELSEWHERE, GROUPS, navTitle, tabTitle, type Tab } from "../lib/labels";
  import { session } from "../lib/session.svelte";
  import { SWITCH_GROUPS } from "../lib/switches";
  import { locale, word } from "../lib/i18n";

  const { tab }: { tab: Tab } = $props();

  const switchGroups = $derived(SWITCH_GROUPS.filter((g) => g.tab === tab.example));
  const settingGroups = $derived(GROUPS.filter((g) => tab.settingGroups.includes(g.id)));
  const strays = $derived.by(() => {
    if (!tab.orphans) return [];
    const homed = new Set([
      ...GROUPS.flatMap((g) => g.keys),
      ...SWITCH_GROUPS.flatMap((g) => g.switches.flatMap((s) => [s.key, s.combined?.where ?? ""])),
    ]);
    return session.settings.filter((s) => !homed.has(s.key) && !EDITED_ELSEWHERE.has(s.key));
  });
</script>

<Inspector kicker={navTitle(tab.nav)} title={tabTitle(tab)} description={locale().help[`tab:${tab.id}`]}>
  {#each switchGroups as group (group.title)}
    <div class="section-title">{word(`group:${group.title}`, group.title)}</div>
    {#each group.switches as s (s.key)}
      <SettingRow
        key={s.key}
        label={word(`setting:${s.key}`, s.label)}
        hint={s.note}
        combined={s.combined}
        under={s.under}
        implied={s.implied}
        range={s.range}
      />
    {/each}
  {/each}

  {#each settingGroups as group (group.id)}
    <div class="section-title">{word(`group:${group.id}`, group.title)}</div>
    {#each group.keys as key (key)}
      <SettingRow {key} />
    {/each}
  {/each}

  {#if strays.length > 0}
    <div class="section-title">{word("group:other", "Other")}</div>
    {#each strays as s (s.key)}
      <SettingRow key={s.key} />
    {/each}
  {/if}
</Inspector>
