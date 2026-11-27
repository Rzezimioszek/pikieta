<script lang="ts">
  import Icon from './Icon.svelte'
  import Launcher from './Launcher.svelte'
  import { CALCS, calcTitle } from './calcs'
  import { gml, persistLayer } from './gml.svelte'
  import {
    closeCalc,
    deleteCalc,
    openCalc,
    project,
    showReport,
    startCalc,
    ui,
    type CalcSession,
  } from './project.svelte'

  let { coreVersion }: { coreVersion: string } = $props()

  let query = $state('')
  let menu = $state(false)
  let renaming = $state<string | null>(null)
  let nameDraft = $state('')

  const DAY = 86_400_000
  const startOfToday = () => new Date().setHours(0, 0, 0, 0)

  const matching = $derived.by(() => {
    const q = query.trim().toLowerCase()
    return [...project.calcs]
      .sort((a, b) => b.updated - a.updated)
      .filter((s) => q === '' || calcTitle(s).toLowerCase().includes(q) || CALCS[s.type].label.toLowerCase().includes(q))
  })
  const open = $derived(matching.filter((s) => s.open))
  const history = $derived.by(() => {
    const today = startOfToday()
    const groups: [string, CalcSession[]][] = [
      ['Dzisiaj', []],
      ['Wczoraj', []],
      ['Ostatnie 7 dni', []],
      ['Starsze', []],
    ]
    for (const s of matching.filter((s) => !s.open)) {
      const g = s.updated >= today ? 0 : s.updated >= today - DAY ? 1 : s.updated >= today - 7 * DAY ? 2 : 3
      groups[g][1].push(s)
    }
    return groups.filter(([, list]) => list.length)
  })

  const isActive = (s: CalcSession) => ui.active?.kind === 'calc' && ui.active.id === s.id

  function when(t: number) {
    const d = new Date(t)
    return t >= startOfToday()
      ? d.toLocaleTimeString('pl-PL', { hour: '2-digit', minute: '2-digit' })
      : d.toLocaleDateString('pl-PL', { day: 'numeric', month: 'short' })
  }

  function startRename(s: CalcSession) {
    renaming = s.id
    nameDraft = calcTitle(s)
  }

  function finishRename(s: CalcSession) {
    const name = nameDraft.trim()
    s.name = name === '' || name === CALCS[s.type].title(s.data) ? null : name
    renaming = null
  }

  function remove(s: CalcSession) {
    if (confirm(`Usunąć obliczenie „${calcTitle(s)}” z historii?`)) deleteCalc(s.id)
  }

  function focusOnMount(el: HTMLInputElement) {
    el.focus()
    el.select()
  }
</script>

{#snippet item(s: CalcSession)}
  <li class="row" class:active={isActive(s)}>
    {#if renaming === s.id}
      <input
        class="rename"
        aria-label="Nazwa obliczenia"
        bind:value={nameDraft}
        use:focusOnMount
        onblur={() => finishRename(s)}
        onkeydown={(e) => {
          if (e.key === 'Enter') finishRename(s)
          if (e.key === 'Escape') renaming = null
        }}
      />
    {:else}
      <button class="open" onclick={() => openCalc(s.id)} ondblclick={() => startRename(s)}>
        <span class="ic"><Icon name={CALCS[s.type].icon} size={16} /></span>
        <span class="txt">
          <span class="t">{calcTitle(s)}</span>
          <span class="sub">{CALCS[s.type].label} · {when(s.updated)}</span>
        </span>
      </button>
      <span class="acts">
        <button class="mini" title="Zmień nazwę" onclick={() => startRename(s)}><Icon name="pencil" size={14} /></button>
        {#if s.open}
          <button class="mini" title="Zamknij (zostaje w historii)" onclick={() => closeCalc(s.id)}><Icon name="x" size={14} /></button>
        {:else}
          <button class="mini" title="Usuń z historii" onclick={() => remove(s)}><Icon name="trash" size={14} /></button>
        {/if}
      </span>
    {/if}
  </li>
{/snippet}

<aside class="sidebar surface" aria-label="Lista obliczeń i warstw">
  <div class="new">
    <button class="primary" onclick={() => (menu = !menu)} aria-expanded={menu}>
      <Icon name="plus" /> Nowe obliczenie
    </button>
    {#if menu}
      <button class="backdrop" aria-label="Zamknij menu" onclick={() => (menu = false)}></button>
      <div class="menu surface">
        <Launcher
          compact
          onpick={(t) => {
            menu = false
            startCalc(t)
          }}
        />
      </div>
    {/if}
  </div>

  <label class="search">
    <Icon name="search" size={16} />
    <input placeholder="Szukaj obliczeń" aria-label="Szukaj obliczeń" bind:value={query} />
  </label>

  <nav aria-label="Obliczenia">
    {#if open.length}
      <h2>Otwarte</h2>
      <ul>
        {#each open as s (s.id)}{@render item(s)}{/each}
      </ul>
    {/if}
    {#each history as [label, list] (label)}
      <h2>{label}</h2>
      <ul>
        {#each list as s (s.id)}{@render item(s)}{/each}
      </ul>
    {/each}
    {#if project.calcs.length === 0}
      <p class="hint pad">Tu pojawią się otwarte i ostatnio używane obliczenia. Zacznij od „Nowe obliczenie” albo wstążki u góry.</p>
    {:else if matching.length === 0}
      <p class="hint pad">Nic nie pasuje do „{query}”.</p>
    {/if}
  </nav>

  <footer>
    {#if gml.layers.length}
      <h2>Warstwy GML</h2>
      <ul class="layers">
        {#each gml.layers as l (l.id)}
          <li class="row" class:active={ui.active?.kind === 'gml' && ui.active.id === l.id}>
            <button
              class="open"
              onclick={() => {
                ui.active = { kind: 'gml', id: l.id }
                ui.tableTab = l.id
              }}
            >
              <span class="ic"><Icon name="layers" size={16} /></span>
              <span class="txt">
                <span class="t">{l.name}{l.dirty ? ' •' : ''}</span>
                <span class="sub">
                  {#if l.status === 'loading'}wczytywanie…{:else if l.status === 'error'}błąd wczytywania{:else}{l.count.toLocaleString('pl-PL')} obiektów{l.issues.some((i) => i.severity === 'error') ? ' · błędy' : ''}{/if}
                </span>
              </span>
            </button>
            <input
              type="checkbox"
              class="vis"
              title="Widoczna na mapie"
              checked={l.visible}
              onchange={() => {
                l.visible = !l.visible
                persistLayer(l)
              }}
            />
          </li>
        {/each}
      </ul>
    {/if}
    <button class="report" class:active={ui.active?.kind === 'report'} onclick={showReport}>
      <Icon name="report" size={16} /> Raport
      {#if project.report.length}<span class="count">{project.report.length}</span>{/if}
    </button>
    <span class="ver">{coreVersion ? `rdzeń ${coreVersion}` : 'ładowanie rdzenia…'}</span>
  </footer>
</aside>

<style>
  .sidebar {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    gap: 10px;
    min-height: 0;
    padding: 12px;
  }
  .new {
    position: relative;
  }
  .new > .primary {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    border: none;
    border-radius: 0;
    background: transparent;
    cursor: default;
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 21;
    width: 280px;
    max-height: 70vh;
    overflow: auto;
    padding: 10px;
    background: var(--menu-bg);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border-radius: 12px;
    border: 1px solid var(--field-edge);
    background: var(--field-bg);
    color: var(--ink-soft);
  }
  .search input {
    padding: 7px 0;
    border: none;
    background: none;
    box-shadow: none;
    font-size: 0.85rem;
  }
  nav {
    overflow-y: auto;
    min-height: 0;
    margin: 0 -6px;
    padding: 0 6px;
    scrollbar-width: thin;
  }
  h2 {
    margin: 10px 4px 4px;
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }
  ul {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 12px;
  }
  .row:hover,
  .row:focus-within {
    background: color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .row.active {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .open {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 7px 8px;
    border: none;
    background: none;
    text-align: left;
    font-weight: 400;
  }
  .open:hover {
    background: none !important;
  }
  .ic {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 9px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
    color: var(--accent);
  }
  .txt {
    display: grid;
    min-width: 0;
  }
  .t,
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .t {
    font-size: 0.86rem;
    font-weight: 550;
  }
  .sub {
    font-size: 0.72rem;
    color: var(--ink-soft);
  }
  .acts {
    display: none;
    gap: 2px;
    padding-right: 4px;
  }
  .row:hover .acts,
  .row:focus-within .acts {
    display: flex;
  }
  .mini {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 8px;
    border: none;
    background: transparent;
  }
  .rename {
    margin: 3px;
    padding: 6px 8px;
    font-size: 0.86rem;
  }
  .pad {
    padding: 8px 4px;
  }
  footer {
    display: grid;
    gap: 6px;
    padding-top: 8px;
    border-top: 1px solid var(--field-edge);
  }
  .report {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    padding: 7px 10px;
    border: none;
    background: transparent;
    font-weight: 550;
  }
  .report.active {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .count {
    margin-left: auto;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
    font-size: 0.72rem;
  }
  .layers {
    max-height: 180px;
    overflow-y: auto;
    margin-bottom: 4px;
  }
  .vis {
    margin-right: 10px;
  }
  footer h2 {
    margin-top: 0;
  }
  .ver {
    padding-left: 10px;
    font-size: 0.7rem;
    color: var(--ink-soft);
  }
</style>
