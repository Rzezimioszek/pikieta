<script lang="ts">
  import { onMount } from 'svelte'
  import Calc from './lib/Calc.svelte'
  import Points from './lib/Points.svelte'
  import Report from './lib/Report.svelte'
  import { coreReady, epsgOf, toGeodetic, transformXY, type XY } from './lib/core'
  import { BASEMAPS, CRS_LIST, createMap, type BasemapId, type MapHandle, type Sketch } from './lib/map'
  import { addPoints, nextId, project, saveProject } from './lib/project.svelte'

  type Tab = 'points' | 'calc' | 'report'
  const TABS: [Tab, string][] = [
    ['points', 'Punkty'],
    ['calc', 'Obliczenia'],
    ['report', 'Raport'],
  ]

  let mapEl = $state<HTMLDivElement>()
  let panelEl = $state<HTMLElement>()
  let calc = $state<ReturnType<typeof Calc>>()
  let handle = $state.raw<MapHandle>()
  let coreVersion = $state('')
  let shownCrs = project.crs
  let basemap = $state<BasemapId>('orto')
  let basemapError = $state('')
  let cursor = $state<XY | null>(null)
  let tab = $state<Tab>('points')
  let selectedId = $state<string | null>(null)
  let addByClick = $state(false)
  let sketch = $state.raw<Sketch>({ paths: [] })

  const crsLabel = $derived(CRS_LIST.find((c) => c.code === project.crs)?.label ?? project.crs)

  const cursorGeo = $derived(coreVersion && cursor ? toGeodetic(epsgOf(project.crs), cursor) : null)

  /** Po zmianie układu przelicza wszystkie punkty, żeby zostały w tym samym miejscu w terenie. */
  function onCrsChange() {
    const [from, to] = [epsgOf(shownCrs), epsgOf(project.crs)]
    shownCrs = project.crs
    if (!coreVersion) return
    project.points = project.points.map((p) => ({ ...p, ...(transformXY(from, to, p) ?? p) }))
  }

  /** Na szerokim ekranie panel zasłania prawy pas mapy; na telefonie leży pod mapą. */
  function zoomTo(points: XY[]) {
    const wide = window.matchMedia('(min-width: 701px)').matches
    handle?.zoomTo(points, wide ? (panelEl?.offsetWidth ?? 0) + 16 : 0)
  }

  function onMapClick(p: XY, pointId: string | null) {
    if (pointId !== null) {
      if (tab === 'calc') calc?.pick(pointId)
      else selectedId = pointId
      return
    }
    if (tab === 'points' && addByClick) {
      const id = nextId()
      addPoints([{ id, x: p.x, y: p.y, h: null, code: '' }])
      selectedId = id
    }
  }

  $effect(() => handle?.setPoints(project.points, selectedId))
  $effect(() => handle?.setSketch(tab === 'calc' ? sketch : { paths: [] }))
  $effect(() => handle?.setCrs(project.crs))
  $effect(() => {
    const id = basemap
    handle?.setBasemap(id).catch(() => {
      if (basemap !== id) return
      basemapError = `Nie udało się wczytać podkładu ${BASEMAPS.find((b) => b.id === id)?.label}.`
      basemap = 'orto'
    })
  })
  $effect(saveProject)

  onMount(() => {
    coreReady.then((v) => (coreVersion = v))
    const h = createMap(mapEl!, project.crs, {
      onPointer: (p) => (cursor = p),
      onClick: onMapClick,
    })
    handle = h
    if (project.points.length) setTimeout(() => zoomTo(project.points))
    return () => h.destroy()
  })
</script>

<main>
  <div class="map" bind:this={mapEl}></div>

  <aside class="glass" bind:this={panelEl}>
    <header>
      <h1>Pikieta</h1>
      <span class="badge">{coreVersion ? `rdzeń ${coreVersion}` : 'ładowanie…'}</span>
    </header>

    <div class="selects">
      <label>
        <span>Układ</span>
        <select bind:value={project.crs} onchange={onCrsChange}>
          {#each CRS_LIST as c (c.code)}
            <option value={c.code}>{c.label}</option>
          {/each}
        </select>
      </label>

      <label>
        <span>Podkład</span>
        <select bind:value={basemap} onchange={() => (basemapError = '')}>
          {#each BASEMAPS as b (b.id)}
            <option value={b.id}>{b.label}</option>
          {/each}
        </select>
      </label>
    </div>
    {#if basemapError}<p class="error" role="alert">{basemapError}</p>{/if}

    <div class="tabs" role="tablist">
      {#each TABS as [id, label] (id)}
        <button role="tab" aria-selected={tab === id} class:on={tab === id} onclick={() => (tab = id)}>
          {label}
          {#if id === 'points' && project.points.length}<span class="count">{project.points.length}</span>{/if}
          {#if id === 'report' && project.report.length}<span class="count">{project.report.length}</span>{/if}
        </button>
      {/each}
    </div>

    <section role="tabpanel">
      {#if tab === 'points'}
        <Points bind:selectedId bind:addByClick onzoom={zoomTo} />
      {:else if tab === 'calc'}
        <Calc bind:this={calc} ready={!!coreVersion} {crsLabel} onsketch={(s) => (sketch = s)} />
      {:else}
        <Report {crsLabel} />
      {/if}
    </section>
  </aside>

  {#if cursor}
    <div class="glass cursor" aria-hidden="true">
      <span>X {cursor.x.toFixed(2)}</span>
      <span>Y {cursor.y.toFixed(2)}</span>
      {#if cursorGeo}
        <span class="sep"></span>
        <span>B {cursorGeo.lat.toFixed(6)}°</span>
        <span>L {cursorGeo.lon.toFixed(6)}°</span>
      {/if}
    </div>
  {/if}
</main>

<style>
  main {
    position: relative;
    height: 100dvh;
    overflow: hidden;
  }

  .map {
    position: absolute;
    inset: 0;
  }

  /* Tafla szkła: rozmycie tła, jasna krawędź od góry i miękki cień. */
  .glass {
    position: absolute;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(22px) saturate(180%);
    backdrop-filter: blur(22px) saturate(180%);
    border: 1px solid var(--glass-edge);
    box-shadow:
      0 12px 40px var(--glass-shadow),
      inset 0 1px 0 rgb(255 255 255 / 0.55),
      inset 0 -1px 0 rgb(255 255 255 / 0.08);
  }
  /* Połysk: ukośny refleks, który nie przechwytuje kliknięć. */
  .glass::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(
      135deg,
      rgb(255 255 255 / 0.28),
      rgb(255 255 255 / 0) 38%,
      rgb(255 255 255 / 0) 70%,
      rgb(255 255 255 / 0.1)
    );
    pointer-events: none;
  }

  aside {
    top: 16px;
    right: 16px;
    width: 372px;
    max-height: calc(100dvh - 32px);
    padding: 20px;
    border-radius: 28px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    scrollbar-width: thin;
  }
  aside > * {
    position: relative;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  h1 {
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .badge {
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
    font-size: 0.72rem;
    color: var(--ink-soft);
    white-space: nowrap;
  }

  .selects {
    display: grid;
    gap: 10px;
  }
  .selects label {
    display: grid;
    grid-template-columns: 64px 1fr;
    align-items: center;
    gap: 10px;
    font-size: 0.9rem;
    color: var(--ink-soft);
  }
  .selects select {
    color: var(--ink);
  }

  .tabs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
    padding: 4px;
    border-radius: 18px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
  }
  .tabs button {
    padding: 7px 6px;
    border: none;
    background: transparent;
    font-size: 0.88rem;
    color: var(--ink-soft);
  }
  .tabs button.on {
    background: var(--accent);
    color: var(--on-accent);
  }
  .count {
    margin-left: 4px;
    padding: 0 6px;
    border-radius: 999px;
    background: color-mix(in srgb, currentColor 18%, transparent);
    font-size: 0.72rem;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .cursor {
    left: 16px;
    bottom: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-radius: 999px;
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    pointer-events: none;
  }
  .cursor > * {
    position: relative;
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--field-edge);
  }

  /* Kontrolki OpenLayers w tym samym stylu. */
  .map :global(.ol-zoom) {
    top: 16px;
    left: 16px;
    padding: 4px;
    border-radius: 18px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(22px) saturate(180%);
    backdrop-filter: blur(22px) saturate(180%);
    border: 1px solid var(--glass-edge);
    box-shadow: 0 8px 24px var(--glass-shadow);
  }
  .map :global(.ol-zoom button) {
    width: 34px;
    height: 34px;
    margin: 0;
    padding: 0;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: var(--ink);
    font-size: 1.2rem;
  }
  .map :global(.ol-zoom button:hover),
  .map :global(.ol-zoom button:focus-visible) {
    background: var(--field-bg);
    outline: none;
  }
  .map :global(.ol-attribution) {
    right: auto;
    left: 16px;
    bottom: 60px;
    border-radius: 14px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(22px) saturate(180%);
    backdrop-filter: blur(22px) saturate(180%);
    color: var(--ink);
  }
  .map :global(.ol-attribution button) {
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    color: var(--ink);
  }
  .map :global(.ol-attribution ul) {
    color: var(--ink);
    text-shadow: none;
  }

  @media (max-width: 700px) {
    aside {
      top: auto;
      left: 8px;
      right: 8px;
      bottom: 8px;
      width: auto;
      max-height: 58dvh;
      padding: 16px;
      border-radius: 26px;
      gap: 12px;
    }
    .cursor {
      left: auto;
      right: 8px;
      top: 12px;
      bottom: auto;
      flex-wrap: wrap;
      max-width: calc(100vw - 80px);
      border-radius: 18px;
      gap: 4px 10px;
    }
    .map :global(.ol-zoom) {
      top: 8px;
      left: 8px;
    }
    .map :global(.ol-attribution) {
      top: 96px;
      bottom: auto;
      left: 8px;
    }
  }

  @media (prefers-reduced-transparency: reduce) {
    .glass,
    .map :global(.ol-zoom),
    .map :global(.ol-attribution) {
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
    }
  }
</style>
