<script lang="ts">
  import { rovingKeys } from './a11y'
  import Icon from './Icon.svelte'
  import { CALC_GROUPS, CALCS } from './calcs'
  import {
    exportPoints,
    exportReport,
    importPoints,
    openProjectFile,
    printReport,
    projectFile,
    saveProjectFile,
  } from './files.svelte'
  import { exportPointsGml, gml, importGml, replaceLayers } from './gml.svelte'
  import type { IconName } from './icons'
  import { BASEMAPS, CRS_LIST } from './map'
  import ServicesMenu from './ServicesMenu.svelte'
  import { grouped } from './services'
  import {
    notify,
    openTaskPicker,
    project,
    recentTypes,
    selection,
    showReport,
    startCalc,
    ui,
    type Precision,
    type RibbonTab,
    type ViewMode,
  } from './project.svelte'
  import type { XY } from './core'

  interface Props {
    crsLabel: string
    onzoom: (points: XY[]) => void
    onchangecrs: (code: string) => void
  }
  let { crsLabel, onzoom, onchangecrs }: Props = $props()

  const recent = $derived(recentTypes(4))

  const TABS: [RibbonTab, string][] = [
    ['project', 'Projekt'],
    ['calc', 'Obliczenia'],
    ['map', 'Mapa'],
    ['view', 'Widok'],
  ]

  const VIEWS: [ViewMode, IconName, string][] = [
    ['map', 'map', 'Mapa'],
    ['split', 'split', 'Podział'],
    ['calc', 'panel', 'Obliczenia'],
  ]

  function pickTab(t: RibbonTab) {
    if (ui.ribbonTab === t && !ui.ribbonCollapsed) ui.ribbonCollapsed = true
    else {
      ui.ribbonTab = t
      ui.ribbonCollapsed = false
    }
  }

  const PRECISION: [keyof Precision, string, number][] = [
    ['coord', 'Współrzędne', 4],
    ['length', 'Długości', 4],
    ['angle', 'Kąty [g]', 6],
    ['area', 'Pola [m²]', 4],
  ]

  function newProject() {
    const n = project.points.length + project.calcs.length + project.report.length + gml.layers.length
    if (n && !confirm('Zacząć nowy projekt? Punkty, obliczenia, raport i warstwy GML zostaną usunięte.')) return
    replaceLayers([])
    project.points = []
    project.calcs = []
    project.report = []
    selection.clear()
    ui.active = null
    projectFile.name = ''
    notify('Nowy projekt.')
  }

  async function doOpen() {
    const pts = await openProjectFile()
    if (pts?.length) onzoom(pts)
  }

  function doImportGml() {
    importGml()
  }

  async function doImport() {
    const pts = await importPoints()
    if (pts.length) onzoom(pts)
  }
</script>

{#snippet big(icon: IconName, label: string, action: () => void, opts: { on?: boolean; disabled?: boolean; title?: string } = {})}
  <button
    class="big"
    class:on={opts.on}
    aria-pressed={opts.on}
    disabled={opts.disabled}
    title={opts.title ?? label}
    onclick={action}
  >
    <Icon name={icon} size={22} />
    <span>{label}</span>
  </button>
{/snippet}

<header class="ribbon surface" class:collapsed={ui.ribbonCollapsed}>
  <div class="strip">
    <button class="burger" title="Lista obliczeń" aria-pressed={ui.sidebar} onclick={() => (ui.sidebar = !ui.sidebar)}>
      <Icon name="menu" />
    </button>
    <h1 class="brand">Pikieta</h1>
    <div class="tabs" role="tablist" aria-label="Wstążka" use:rovingKeys>
      {#each TABS as [id, label] (id)}
        <button
          role="tab"
          id="ribbon-tab-{id}"
          aria-selected={ui.ribbonTab === id}
          aria-controls={ui.ribbonCollapsed ? undefined : 'ribbon-panel'}
          tabindex={ui.ribbonTab === id ? 0 : -1}
          class:on={ui.ribbonTab === id && !ui.ribbonCollapsed} onclick={() => pickTab(id)}>
          {label}
        </button>
      {/each}
    </div>
    <span class="crs" title="Układ współrzędnych projektu"><Icon name="globe" size={14} /> {crsLabel}</span>
    <button
      class="fold"
      title={ui.ribbonCollapsed ? 'Rozwiń wstążkę' : 'Zwiń wstążkę'}
      aria-expanded={!ui.ribbonCollapsed}
      onclick={() => (ui.ribbonCollapsed = !ui.ribbonCollapsed)}
    >
      <Icon name={ui.ribbonCollapsed ? 'chevronDown' : 'chevronUp'} />
    </button>
  </div>

  {#if !ui.ribbonCollapsed}
    <div class="body" id="ribbon-panel" role="tabpanel" aria-labelledby="ribbon-tab-{ui.ribbonTab}">
      {#if ui.ribbonTab === 'project'}
        <div class="group">
          <div class="btns">
            {@render big('file', 'Nowy', newProject, { title: 'Nowy projekt' })}
            {@render big('folder', 'Otwórz', doOpen, { title: 'Otwórz projekt z pliku (Ctrl+O)' })}
            {@render big('save', 'Zapisz', saveProjectFile, { title: 'Zapisz projekt do pliku (Ctrl+S)' })}
          </div>
          <span class="cap">Projekt{projectFile.name ? `: ${projectFile.name}` : ''}</span>
        </div>
        <div class="group">
          <div class="btns">
            {@render big('upload', 'Importuj', doImport, { title: 'Importuj wykaz współrzędnych (TXT)' })}
            {@render big('download', 'Eksportuj', exportPoints, {
              title: 'Eksportuj wykaz współrzędnych (TXT)',
              disabled: project.points.length === 0,
            })}
            <label class="stack">
              <span>Kolumny</span>
              <select bind:value={ui.yx} aria-label="Kolejność kolumn w pliku">
                <option value={false}>Nr X Y</option>
                <option value={true}>Nr Y X</option>
              </select>
            </label>
          </div>
          <span class="cap">Punkty</span>
        </div>
        <div class="group">
          <div class="btns">
            {@render big('layers', 'Wczytaj', doImportGml, { title: 'Wczytaj plik GML (np. EGiB) jako warstwę' })}
            {@render big('download', 'Punkty', exportPointsGml, {
              title: 'Zapisz punkty projektu jako GML 3.2',
              disabled: project.points.length === 0,
            })}
          </div>
          <span class="cap">GML</span>
        </div>
        <div class="group">
          <div class="btns">
            {@render big('report', 'Pokaż', showReport, { title: 'Pokaż raport obliczeń' })}
            {@render big('download', 'TXT', () => exportReport(crsLabel, 'txt'), {
              title: 'Zapisz raport jako tekst',
              disabled: project.report.length === 0,
            })}
            {@render big('download', 'HTML', () => exportReport(crsLabel, 'html'), {
              title: 'Zapisz raport jako stronę HTML',
              disabled: project.report.length === 0,
            })}
            {@render big('printer', 'Drukuj', () => printReport(crsLabel), {
              title: 'Drukuj raport albo zapisz jako PDF',
              disabled: project.report.length === 0,
            })}
          </div>
          <span class="cap">Raport</span>
        </div>
        <div class="group">
          <div class="btns precision">
            {#each PRECISION as [key, label, max] (key)}
              <label class="stack">
                <span>{label}</span>
                <select bind:value={project.precision[key]} aria-label={`Miejsca po przecinku: ${label}`}>
                  {#each Array.from({ length: max + 1 }, (_, i) => i) as d (d)}
                    <option value={d}>{d === 0 ? '1' : `0.${'0'.repeat(d - 1)}1`}</option>
                  {/each}
                </select>
              </label>
            {/each}
          </div>
          <span class="cap">Dokładność wyników (zaokrąglanie do parzystej)</span>
        </div>
      {:else if ui.ribbonTab === 'calc'}
        <div class="group">
          <div class="btns">
            {@render big('plus', 'Nowe obliczenie', () => openTaskPicker(), { title: 'Okno wyboru zadania (Ctrl+K)' })}
          </div>
          <span class="cap">Wszystkie zadania</span>
        </div>
        <div class="group">
          <div class="btns">
            {#each CALC_GROUPS as g (g.label)}
              {@render big(CALCS[g.types[0]].icon, g.label, () => openTaskPicker(g.label), {
                title: `${g.label}: ${g.types.map((t) => CALCS[t].label).join(', ')}`,
              })}
            {/each}
          </div>
          <span class="cap">Grupy zadań</span>
        </div>
        {#if recent.length}
          <div class="group">
            <div class="btns">
              {#each recent as t (t)}
                {@render big(CALCS[t].icon, CALCS[t].label, () => startCalc(t), { title: `Nowe: ${CALCS[t].label}. ${CALCS[t].hint}` })}
              {/each}
            </div>
            <span class="cap">Ostatnio używane</span>
          </div>
        {/if}
      {:else if ui.ribbonTab === 'map'}
        <div class="group">
          <div class="btns">
            <label class="stack wide">
              <span>Układ współrzędnych</span>
              <select value={project.crs} onchange={(e) => onchangecrs(e.currentTarget.value)}>
                {#each CRS_LIST as c (c.code)}<option value={c.code}>{c.label}</option>{/each}
              </select>
            </label>
            <label class="stack">
              <span>Podkład</span>
              <select bind:value={ui.basemap}>
                {#each grouped(BASEMAPS) as [group, list] (group)}
                  <optgroup label={group}>
                    {#each list as b (b.id)}<option value={b.id}>{b.label}</option>{/each}
                  </optgroup>
                {/each}
              </select>
            </label>
          </div>
          <span class="cap">Mapa</span>
        </div>
        <div class="group">
          <div class="btns">
            <ServicesMenu />
          </div>
          <span class="cap">Usługi</span>
        </div>
        <div class="group">
          <div class="btns">
            {@render big('pointAdd', 'Dodawaj', () => (ui.addByClick = !ui.addByClick), {
              on: ui.addByClick,
              title: 'Dodawaj punkty kliknięciem na mapie',
            })}
            {@render big('fit', 'Wszystkie', () => onzoom(project.points), {
              title: 'Pokaż wszystkie punkty',
              disabled: project.points.length === 0,
            })}
            {@render big('selectAll', 'Zaznaczone', () => onzoom(project.points.filter((p) => selection.has(p.id))), {
              title: 'Przybliż do zaznaczonych punktów',
              disabled: selection.size === 0,
            })}
          </div>
          <span class="cap">Punkty</span>
        </div>
      {:else}
        <div class="group">
          <div class="btns">
            {#each VIEWS as [mode, icon, label] (mode)}
              {@render big(icon, label, () => (ui.view = mode), { on: ui.view === mode })}
            {/each}
          </div>
          <span class="cap">Tryb widoku</span>
        </div>
        <div class="group">
          <div class="btns">
            {@render big('sidebar', 'Obliczenia', () => (ui.sidebar = !ui.sidebar), {
              on: ui.sidebar,
              title: 'Lista otwartych i ostatnich obliczeń',
            })}
            {@render big('table', 'Tabela', () => (ui.table = !ui.table), { on: ui.table, title: 'Tabela punktów' })}
          </div>
          <span class="cap">Panele</span>
        </div>
      {/if}
    </div>
  {/if}
</header>

<style>
  .ribbon {
    display: grid;
    min-width: 0;
    padding: 6px 8px 8px;
  }
  .collapsed {
    padding-bottom: 6px;
  }
  .strip {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .burger,
  .fold {
    display: grid;
    place-items: center;
    width: 34px;
    height: 32px;
    padding: 0;
    border: none;
    background: transparent;
  }
  .brand {
    margin: 0;
    font-weight: 750;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
    margin-right: 6px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tabs button {
    padding: 6px 14px;
    /* Przezroczysta ramka, żeby aktywna karta nie przesuwała sąsiednich. */
    border: 1px solid transparent;
    border-radius: 10px;
    background: transparent;
    color: var(--ink-soft);
    font-size: 0.88rem;
  }
  .tabs button.on {
    background: var(--field-bg);
    color: var(--ink);
    border-color: var(--accent);
  }
  .crs {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid var(--field-edge);
    background: var(--field-bg);
    font-size: 0.74rem;
    color: var(--ink-soft);
    white-space: nowrap;
  }
  .body {
    display: flex;
    align-items: stretch;
    gap: 4px;
    margin-top: 6px;
    padding: 6px 4px 0;
    border-top: 1px solid var(--field-edge);
    overflow-x: auto;
    scrollbar-width: thin;
  }
  .group {
    display: grid;
    grid-template-rows: 1fr auto;
    justify-items: center;
    gap: 2px;
    padding: 0 10px;
    border-right: 1px solid var(--field-edge);
  }
  .group:last-child {
    border-right: none;
  }
  .btns {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .cap {
    font-size: 0.68rem;
    color: var(--ink-soft);
    white-space: nowrap;
  }
  .big {
    display: grid;
    justify-items: center;
    gap: 3px;
    min-width: 64px;
    padding: 6px 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    font-size: 0.74rem;
    font-weight: 550;
  }
  .big span {
    max-width: 92px;
    line-height: 1.15;
    text-align: center;
  }
  .big :global(svg) {
    color: var(--accent);
  }
  .big.on {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .stack {
    display: grid;
    gap: 3px;
    padding: 0 6px;
    font-size: 0.7rem;
    color: var(--ink-soft);
  }
  .stack select {
    padding: 5px 8px;
    border-radius: 10px;
    font-size: 0.82rem;
    color: var(--ink);
  }
  .stack.wide select {
    min-width: 210px;
  }
  @media (max-width: 700px) {
    .crs,
    .brand {
      display: none;
    }
    .big {
      min-width: 56px;
    }
  }
</style>
