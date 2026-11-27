<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { rovingKeys } from './lib/a11y'
  import CalcList from './lib/CalcList.svelte'
  import Icon from './lib/Icon.svelte'
  import ImportPreview from './lib/ImportPreview.svelte'
  import GmlTable from './lib/GmlTable.svelte'
  import PointTable from './lib/PointTable.svelte'
  import Ribbon from './lib/Ribbon.svelte'
  import TaskPicker from './lib/TaskPicker.svelte'
  import ToolPanel from './lib/ToolPanel.svelte'
  import ViewSwitch from './lib/ViewSwitch.svelte'
  import { coreReady, epsgOf, toGeodetic, transformXY, type XY } from './lib/core'
  import { openProjectFile, saveProjectFile } from './lib/files.svelte'
  import {
    featureKey,
    findLayer,
    gml,
    gmlHooks,
    gmlSelection,
    gmlTableClass,
    render as renderGml,
    restoreLayers,
    type GmlLayer,
  } from './lib/gml.svelte'
  import { BASEMAPS, CRS_LIST, createMap, type MapHandle, type Sketch } from './lib/map'
  import {
    addPoints,
    env,
    nextId,
    notify,
    openTaskPicker,
    picker,
    project,
    saveProject,
    saveUi,
    selection,
    toasts,
    ui,
  } from './lib/project.svelte'

  let mapEl = $state<HTMLDivElement>()
  let pointTable = $state<ReturnType<typeof PointTable>>()
  let gmlTable = $state<ReturnType<typeof GmlTable>>()
  let handle = $state.raw<MapHandle>()
  let coreVersion = $state('')
  let cursor = $state<XY | null>(null)
  let sketch = $state.raw<Sketch>({ paths: [] })

  const crsLabel = $derived(CRS_LIST.find((c) => c.code === project.crs)?.label ?? project.crs)
  const epsg = $derived(epsgOf(project.crs))
  const cursorGeo = $derived(coreVersion && cursor ? toGeodetic(epsg, cursor) : null)
  /** Panel narzędzia: zawsze w podziale i pełnym ekranie, w trybie mapy tylko z otwartym dokumentem. */
  const toolVisible = $derived(ui.view !== 'map' || ui.active !== null)

  /** Zmiana układu projektu: z przeliczeniem punktów (zostają w tym samym miejscu w terenie) albo bez
   *  (liczby bez zmian, np. gdy punkty wczytano w złym układzie). */
  function changeCrs(code: string, reproject: boolean) {
    const [from, to] = [epsg, epsgOf(code)]
    const n = project.points.length
    if (reproject && coreVersion) project.points = project.points.map((p) => ({ ...p, ...(transformXY(from, to, p) ?? p) }))
    project.crs = code
    if (n) notify(reproject ? `Przeliczono ${n} pkt do układu ${crsLabel}.` : `Zmieniono układ na ${crsLabel} bez przeliczania ${n} pkt.`)
  }

  function zoomTo(points: XY[]) {
    if (ui.view === 'calc') ui.view = 'split'
    handle?.zoomTo(points)
  }

  function onMapClick(p: XY, pointId: string | null, additive: boolean, gmlKey: string | null) {
    if (pointId !== null) {
      if (picker.fill) return picker.fill(pointId)
      if (!additive) selection.clear()
      if (!selection.delete(pointId) || !additive) selection.add(pointId)
      ui.tableTab = 'points'
      pointTable?.reveal(pointId)
      return
    }
    if (ui.addByClick) {
      const id = nextId()
      addPoints([{ id, x: p.x, y: p.y, h: null, code: '' }])
      selection.clear()
      selection.add(id)
      ui.tableTab = 'points'
      pointTable?.reveal(id)
      return
    }
    if (!additive) {
      selection.clear()
      gmlSelection.clear()
    }
    if (gmlKey) {
      if (!gmlSelection.delete(gmlKey) || !additive) gmlSelection.add(gmlKey)
      showGmlFeature(gmlKey, false)
    }
  }

  /** Pokazuje obiekt GML w tabeli (karta warstwy, właściwa klasa) i w panelu warstwy. */
  function showGmlFeature(key: string, zoom: boolean) {
    const [layerId, index] = key.split(':')
    const layer = findLayer(layerId)
    if (!layer?.index) return
    ui.tableTab = layer.id
    gmlTableClass[layer.id] = layer.classOf(Number(index))
    if (ui.active?.kind !== 'calc') ui.active = { kind: 'gml', id: layer.id }
    if (zoom) zoomToGml([key])
    // Tabela klasy wczytuje się asynchronicznie; przewijamy, gdy już jest.
    setTimeout(() => gmlTable?.reveal(key), 300)
  }

  function zoomToGml(target: string[] | { layer: string }) {
    if (ui.view === 'calc') ui.view = 'split'
    handle?.zoomToGml(target)
  }

  function focusGml(layer: GmlLayer, feature: number) {
    const key = featureKey(layer.id, feature)
    gmlSelection.clear()
    gmlSelection.add(key)
    showGmlFeature(key, true)
  }

  const tableLayer = $derived(ui.tableTab === 'points' ? undefined : findLayer(ui.tableTab))

  // Geometria GML w układzie projektu: po zmianie układu silnik przelicza bufor.
  $effect(() => {
    void project.crs
    for (const l of gml.layers) if (l.status === 'ready') renderGml(l)
  })
  // Nowy bufor geometrii → warstwa WebGL na mapie (raz na warstwę i wersję).
  $effect(() => {
    if (!handle) return
    handle.keepGmlLayers(gml.layers.map((l) => l.id))
    for (const l of gml.layers) {
      if (!l.geomRev || !l.decoded || !l.index) continue
      untrack(() => {
        handle!.setGmlLayer(
          l.id,
          { decoded: l.decoded!, classes: l.index!.classes, cls: l.index!.cls, labels: l.index!.label, variant: l.index!.variant, codes: l.index!.codes, code: l.index!.code, texts: l.texts },
          l.visible,
          l.hidden,
        )
        if (l.zoomOnLoad) {
          l.zoomOnLoad = false
          zoomToGml({ layer: l.id })
        }
      })
    }
  })
  $effect(() => {
    for (const l of gml.layers) handle?.setGmlVisibility(l.id, l.visible, [...l.hidden])
  })
  $effect(() => handle?.setGmlSelection([...gmlSelection]))
  gmlHooks.label = (layer, i, label) => handle?.setGmlLabel(layer, i, label)

  const tableMax = () => Math.round(window.innerHeight * 0.7)
  /** Wysokość tabeli z klawiatury: strzałki co 24 px, PageUp/PageDown co 120 px, Home/End skrajne. */
  function resizeTableByKey(e: KeyboardEvent) {
    const step = { ArrowUp: 24, ArrowDown: -24, PageUp: 120, PageDown: -120 }[e.key]
    let h = ui.tableHeight
    if (step !== undefined) h += step
    else if (e.key === 'Home') h = 120
    else if (e.key === 'End') h = tableMax()
    else return
    e.preventDefault()
    ui.tableHeight = Math.min(tableMax(), Math.max(120, h))
  }

  function startTableResize(e: PointerEvent) {
    const [y0, h0] = [e.clientY, ui.tableHeight]
    const move = (ev: PointerEvent) => {
      const h = h0 - (ev.clientY - y0)
      ui.tableHeight = Math.round(Math.min(window.innerHeight * 0.7, Math.max(120, h)))
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', () => window.removeEventListener('pointermove', move), { once: true })
  }

  $effect(() => handle?.setPoints(project.points, selection))
  $effect(() => handle?.setSketch(ui.active?.kind === 'calc' ? sketch : { paths: [] }))
  $effect(() => handle?.setCrs(project.crs))
  $effect(() => {
    const id = ui.basemap
    handle?.setBasemap(id).catch(() => {
      if (ui.basemap !== id) return
      toasts.push({ id: Date.now(), text: `Nie udało się wczytać podkładu ${BASEMAPS.find((b) => b.id === id)?.label}.`, kind: 'error' })
      ui.basemap = 'orto'
    })
  })
  $effect(() => handle?.setOverlays(ui.overlays))
  $effect(saveProject)
  $effect(saveUi)

  onMount(() => {
    coreReady.then((v) => {
      coreVersion = v
      restoreLayers()
    })

    const narrow = window.matchMedia('(max-width: 900px)')
    const sync = () => (env.narrow = narrow.matches)
    sync()
    if (env.narrow) ui.sidebar = false
    narrow.addEventListener('change', sync)

    // Pole czeka na punkt z mapy tylko, dopóki ostatnio aktywne było pole punktu.
    const onFocus = (e: FocusEvent) => {
      if (!(e.target as HTMLElement).closest?.('[data-pick]')) picker.fill = null
    }
    const onKey = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey
      if (ctrl && e.key.toLowerCase() === 's') {
        e.preventDefault()
        saveProjectFile()
      } else if (ctrl && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        openTaskPicker()
      } else if (ctrl && e.key.toLowerCase() === 'o') {
        e.preventDefault()
        openProjectFile().then((pts) => pts?.length && zoomTo(pts))
      } else if (e.key === 'Escape') {
        picker.fill = null
        ui.addByClick = false
      }
    }
    document.addEventListener('focusin', onFocus)
    window.addEventListener('keydown', onKey)

    const h = createMap(mapEl!, project.crs, { onPointer: (p) => (cursor = p), onClick: onMapClick })
    handle = h
    if (project.points.length) setTimeout(() => h.zoomTo(project.points))

    return () => {
      h.destroy()
      narrow.removeEventListener('change', sync)
      document.removeEventListener('focusin', onFocus)
      window.removeEventListener('keydown', onKey)
    }
  })
</script>

<datalist id="point-ids">
  {#each project.points.slice(0, 2000) as p (p.id)}<option value={p.id}></option>{/each}
</datalist>

<a class="skip-link" href="#workspace">Przejdź do obszaru roboczego</a>

<TaskPicker />
<ImportPreview {crsLabel} />

<div class="app">
  <Ribbon {crsLabel} onzoom={zoomTo} onchangecrs={changeCrs} />

  <div class="main" class:with-sidebar={ui.sidebar}>
    {#if ui.sidebar}
      {#if env.narrow}<button class="scrim" aria-label="Zamknij listę" onclick={() => (ui.sidebar = false)}></button>{/if}
      <CalcList {coreVersion} />
    {/if}

    <main
      id="workspace"
      tabindex="-1"
      class="workspace"
      class:with-table={ui.table}
      style:--table-h={`${ui.tableHeight}px`}
    >
      <div class="top view-{ui.view}" class:has-tool={toolVisible}>
        <div class="mapcard surface">
          <div
            class="map"
            class:picking={picker.fill !== null || ui.addByClick}
            role="application"
            aria-label="Mapa. Strzałki przesuwają widok, plus i minus zmieniają skalę. Punkty są też w tabeli punktów."
            tabindex="0"
            bind:this={mapEl}
          ></div>

          {#if !toolVisible}<div class="corner"><ViewSwitch /></div>{/if}

          {#if picker.fill}
            <div class="pill hintpill" role="status">
              Kliknij punkt na mapie: <strong>{picker.label}</strong> <kbd>Esc</kbd>
            </div>
          {:else if ui.addByClick}
            <div class="pill hintpill" role="status">Kliknij na mapie, żeby dodać punkt {nextId()} <kbd>Esc</kbd></div>
          {/if}

          {#if cursor}
            <div class="pill cursor" aria-hidden="true">
              <span>X {cursor.x.toFixed(2)}</span>
              <span>Y {cursor.y.toFixed(2)}</span>
              {#if cursorGeo}
                <span class="sep"></span>
                <span>B {cursorGeo.lat.toFixed(6)}°</span>
                <span>L {cursorGeo.lon.toFixed(6)}°</span>
              {/if}
            </div>
          {/if}
        </div>

        {#if toolVisible}
          <ToolPanel
            ready={!!coreVersion}
            {epsg}
            {crsLabel}
            onsketch={(s) => (sketch = s)}
            onzoomgml={zoomToGml}
            onfocusgml={focusGml}
          />
        {/if}
      </div>

      {#if ui.table}
        <div
          class="splitter"
          role="separator"
          tabindex="0"
          aria-orientation="horizontal"
          aria-label="Wysokość tabeli"
          aria-valuemin={120}
          aria-valuemax={tableMax()}
          aria-valuenow={ui.tableHeight}
          onpointerdown={startTableResize}
          onkeydown={resizeTableByKey}
        ></div>
        <div class="tabledock surface">
          {#if gml.layers.length}
            <div class="docktabs" role="tablist" aria-label="Tabele" use:rovingKeys>
              <button role="tab" aria-selected={!tableLayer} tabindex={tableLayer ? -1 : 0} class:on={!tableLayer} onclick={() => (ui.tableTab = 'points')}>
                <Icon name="table" size={15} /> Punkty
              </button>
              {#each gml.layers as l (l.id)}
                <button role="tab" aria-selected={tableLayer === l} tabindex={tableLayer === l ? 0 : -1} class:on={tableLayer === l} onclick={() => (ui.tableTab = l.id)}>
                  <Icon name="layers" size={15} /> {l.name}
                </button>
              {/each}
            </div>
          {/if}
          {#if tableLayer}
            {#key tableLayer.id}
              <GmlTable bind:this={gmlTable} layer={tableLayer} onzoom={zoomToGml} onclose={() => (ui.table = false)} />
            {/key}
          {:else}
            <PointTable bind:this={pointTable} onzoom={zoomTo} onclose={() => (ui.table = false)} />
          {/if}
        </div>
      {:else}
        <button class="reopen" onclick={() => (ui.table = true)}><Icon name="table" size={16} /> Tabela punktów ({project.points.length})</button>
      {/if}
    </main>
  </div>

  <div class="toasts" role="status" aria-live="polite">
    {#each toasts as t (t.id)}
      <div class="toast surface" class:error={t.kind === 'error'}>
        {#if t.kind === 'error'}<span class="sr-only">Błąd: </span>{/if}{t.text}
      </div>
    {/each}
  </div>
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 8px;
    height: 100dvh;
    padding: 8px;
    overflow: hidden;
  }
  .main {
    position: relative;
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    min-height: 0;
  }
  .main.with-sidebar {
    grid-template-columns: 272px 1fr;
  }
  .workspace {
    display: grid;
    grid-template-rows: 1fr auto;
    min-width: 0;
    min-height: 0;
  }
  .workspace.with-table {
    grid-template-rows: 1fr 8px var(--table-h);
  }
  .top {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    min-height: 0;
  }
  .top.view-map.has-tool {
    grid-template-columns: 1fr minmax(360px, 34%);
  }
  .top.view-split {
    grid-template-columns: 1fr 1fr;
  }
  .top.view-calc .mapcard {
    display: none;
  }

  .mapcard {
    position: relative;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 0;
  }
  .map {
    position: absolute;
    inset: 0;
  }
  .map.picking :global(.ol-viewport) {
    cursor: crosshair;
  }
  .corner {
    position: absolute;
    top: 10px;
    right: 10px;
    z-index: 2;
  }

  .pill {
    position: absolute;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 14px;
    border-radius: 999px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(18px) saturate(170%);
    backdrop-filter: blur(18px) saturate(170%);
    border: 1px solid var(--glass-edge);
    box-shadow: 0 6px 18px var(--glass-shadow);
    font-size: 0.78rem;
    white-space: nowrap;
    pointer-events: none;
  }
  .cursor {
    left: 10px;
    bottom: 10px;
    font-variant-numeric: tabular-nums;
  }
  .hintpill {
    top: 10px;
    left: 50%;
    transform: translateX(-50%);
    background: color-mix(in srgb, var(--accent) 30%, var(--glass-bg));
  }
  kbd {
    padding: 0 5px;
    border-radius: 5px;
    border: 1px solid var(--field-edge);
    font-size: 0.7rem;
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--field-edge);
  }

  .splitter {
    cursor: row-resize;
    touch-action: none;
    position: relative;
  }
  .splitter::after {
    content: '';
    position: absolute;
    left: 50%;
    top: 2px;
    width: 44px;
    height: 4px;
    margin-left: -22px;
    border-radius: 2px;
    background: var(--field-edge);
  }
  .tabledock {
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 6px;
    min-height: 0;
    padding: 10px 12px;
  }
  .docktabs {
    display: flex;
    gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .docktabs button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    /* Przezroczysta ramka, żeby aktywna karta nie przesuwała sąsiednich. */
    border: 1px solid transparent;
    border-radius: 10px;
    background: transparent;
    font-size: 0.8rem;
    font-weight: 550;
    color: var(--ink-soft);
    white-space: nowrap;
  }
  .docktabs button.on {
    background: var(--field-bg);
    color: var(--ink);
    border-color: var(--accent);
  }
  .reopen {
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 5px 12px;
    font-size: 0.8rem;
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 30;
    border: none;
    border-radius: 0;
    background: rgb(0 0 0 / 0.35);
  }

  .toasts {
    position: fixed;
    left: 50%;
    bottom: 20px;
    z-index: 50;
    display: grid;
    gap: 8px;
    transform: translateX(-50%);
    width: min(520px, calc(100vw - 32px));
    pointer-events: none;
  }
  .toast {
    padding: 10px 16px;
    background: var(--menu-bg);
    font-size: 0.86rem;
  }
  .toast.error {
    border-color: var(--danger);
    color: var(--danger);
  }

  /* Kontrolki OpenLayers w tym samym stylu. */
  .map :global(.ol-zoom) {
    top: 10px;
    left: 10px;
    padding: 3px;
    border-radius: 14px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(18px) saturate(170%);
    backdrop-filter: blur(18px) saturate(170%);
    border: 1px solid var(--glass-edge);
    box-shadow: 0 6px 18px var(--glass-shadow);
  }
  .map :global(.ol-zoom button) {
    width: 32px;
    height: 32px;
    margin: 0;
    padding: 0;
    border: none;
    border-radius: 11px;
    background: transparent;
    color: var(--ink);
    font-size: 1.15rem;
  }
  .map :global(.ol-zoom button:hover),
  .map :global(.ol-zoom button:focus-visible) {
    background: var(--field-bg);
  }
  /* Fokus mapy: obrys do środka karty, bo karta przycina to, co wystaje. */
  .map:focus-visible {
    outline-offset: -3px;
    border-radius: 20px;
  }
  .workspace:focus {
    outline: none;
  }
  .splitter:focus-visible::after,
  .splitter:hover::after {
    background: var(--accent);
  }
  .map :global(.ol-attribution) {
    right: 10px;
    bottom: 10px;
    border-radius: 12px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(18px) saturate(170%);
    backdrop-filter: blur(18px) saturate(170%);
    color: var(--ink);
  }
  .map :global(.ol-attribution button) {
    padding: 0;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--ink);
  }
  .map :global(.ol-attribution ul) {
    color: var(--ink);
    text-shadow: none;
  }

  @media (max-width: 900px) {
    .app {
      padding: 6px;
      gap: 6px;
    }
    .main.with-sidebar {
      grid-template-columns: 1fr;
    }
    .main :global(.sidebar) {
      position: fixed;
      top: 6px;
      bottom: 6px;
      left: 6px;
      z-index: 31;
      width: min(300px, calc(100vw - 48px));
      background: var(--menu-bg);
    }
    .top.view-map.has-tool,
    .top.view-split {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr 1fr;
    }
    .cursor {
      flex-wrap: wrap;
      max-width: calc(100% - 80px);
      gap: 2px 8px;
      border-radius: 14px;
    }
    .hintpill {
      top: auto;
      bottom: 52px;
    }
  }
</style>
