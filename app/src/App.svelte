<script lang="ts">
  import { onMount } from 'svelte'
  import { BASEMAPS, CRS_LIST, createMap, type BasemapId, type GeoPoint, type MapHandle } from './lib/map'
  import init, { azimuth_distance, core_version, line_reduction, to_geodetic, transform } from './lib/wasm/pikieta.js'

  let mapEl = $state<HTMLDivElement>()
  let handle = $state.raw<MapHandle>()
  let coreVersion = $state('')
  let crs = $state('EPSG:2178')
  let shownCrs = 'EPSG:2178'
  let basemap = $state<BasemapId>('orto')
  let basemapError = $state('')
  let cursor = $state<GeoPoint | null>(null)
  let fields = $state({ xa: '', ya: '', xb: '', yb: '' })
  let nextClick = $state<'a' | 'b'>('a')

  function parse(s: string): number | null {
    const t = s.trim().replace(',', '.')
    const v = Number(t)
    return t !== '' && Number.isFinite(v) ? v : null
  }

  function point(xs: string, ys: string): GeoPoint | null {
    const x = parse(xs)
    const y = parse(ys)
    return x === null || y === null ? null : { x, y }
  }

  const a = $derived(point(fields.xa, fields.ya))
  const b = $derived(point(fields.xb, fields.yb))

  const result = $derived.by(() => {
    if (!coreVersion || !a || !b) return null
    const r = azimuth_distance(a.x, a.y, b.x, b.y)
    if (!r) return null
    const out = { azimuth: r.azimuth_grad, distance: r.distance }
    r.free()
    const red = line_reduction(epsg(crs), a.x, a.y, b.x, b.y)
    return { ...out, scale: red?.[0], ellipsoidal: red?.[1], convergence: red?.[2] }
  })

  const epsg = (code: string) => Number(code.replace('EPSG:', ''))

  const cursorGeo = $derived.by(() => {
    if (!coreVersion || !cursor) return null
    const g = to_geodetic(epsg(crs), cursor.x, cursor.y)
    return g ? { lat: g[0], lon: g[1] } : null
  })

  /** Po zmianie układu przelicza wpisane punkty, żeby zostały w tym samym miejscu w terenie. */
  function onCrsChange() {
    const [from, to] = [epsg(shownCrs), epsg(crs)]
    shownCrs = crs
    if (!coreVersion) return
    for (const [kx, ky] of [['xa', 'ya'], ['xb', 'yb']] as const) {
      const p = point(fields[kx], fields[ky])
      const t = p && transform(from, to, p.x, p.y)
      if (!t) continue
      fields[kx] = t[0].toFixed(2)
      fields[ky] = t[1].toFixed(2)
    }
  }

  $effect(() => handle?.setSegment(a, b))
  $effect(() => handle?.setCrs(crs))
  $effect(() => {
    const id = basemap
    handle?.setBasemap(id).catch(() => {
      if (basemap !== id) return
      basemapError = `Nie udało się wczytać podkładu ${BASEMAPS.find((b) => b.id === id)?.label}.`
      basemap = 'orto'
    })
  })

  onMount(() => {
    init().then(() => (coreVersion = core_version()))
    const h = createMap(mapEl!, crs, {
      onPointer: (p) => (cursor = p),
      onClick: (p) => {
        const [x, y] = [p.x.toFixed(2), p.y.toFixed(2)]
        if (nextClick === 'a') {
          fields.xa = x
          fields.ya = y
          nextClick = 'b'
        } else {
          fields.xb = x
          fields.yb = y
          nextClick = 'a'
        }
      },
    })
    handle = h
    return () => h.destroy()
  })
</script>

<main>
  <div class="map" bind:this={mapEl}></div>

  <aside class="glass">
    <header>
      <h1>Pikieta</h1>
      <span class="badge">{coreVersion ? `rdzeń ${coreVersion}` : 'ładowanie…'}</span>
    </header>

    <div class="selects">
      <label>
        <span>Układ</span>
        <select bind:value={crs} onchange={onCrsChange}>
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
    {#if basemapError}<p class="hint" role="alert">{basemapError}</p>{/if}

    <section>
      <h2>Azymut i długość</h2>
      <p class="hint">
        Wpisz współrzędne albo kliknij na mapie. Następny punkt:
        <span class="next">{nextClick.toUpperCase()}</span>
      </p>

      <div class="grid">
        <span></span><span class="col">X (północ)</span><span class="col">Y (wschód)</span>
        <strong>A</strong>
        <input aria-label="X punktu A" inputmode="decimal" bind:value={fields.xa} />
        <input aria-label="Y punktu A" inputmode="decimal" bind:value={fields.ya} />
        <strong>B</strong>
        <input aria-label="X punktu B" inputmode="decimal" bind:value={fields.xb} />
        <input aria-label="Y punktu B" inputmode="decimal" bind:value={fields.yb} />
      </div>
    </section>

    <dl class="results">
      <dt>Azymut A→B</dt>
      <dd>{result ? `${result.azimuth.toFixed(4)} g` : '—'}</dd>
      <dt>Długość</dt>
      <dd>{result ? `${result.distance.toFixed(3)} m` : '—'}</dd>
      <dt>Skala odcinka</dt>
      <dd>{result?.scale !== undefined ? result.scale.toFixed(8) : '—'}</dd>
      <dt>Długość na elipsoidzie</dt>
      <dd>{result?.ellipsoidal !== undefined ? `${result.ellipsoidal.toFixed(3)} m` : '—'}</dd>
      <dt>Zbieżność południków w A</dt>
      <dd>{result?.convergence !== undefined ? `${result.convergence.toFixed(4)} g` : '—'}</dd>
    </dl>
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
    --glass-bg: rgb(255 255 255 / 0.58);
    --glass-edge: rgb(255 255 255 / 0.7);
    --glass-shadow: rgb(15 23 42 / 0.28);
    --field-bg: rgb(255 255 255 / 0.5);
    --field-edge: rgb(15 23 42 / 0.14);
    --menu-bg: #ffffff;
    --ink: #0f172a;
    --ink-soft: rgb(15 23 42 / 0.66);
    --accent: #b45309;
    --focus: #2563eb;

    position: relative;
    height: 100dvh;
    overflow: hidden;
    color: var(--ink);
  }
  @media (prefers-color-scheme: dark) {
    main {
      --glass-bg: rgb(17 24 39 / 0.42);
      --glass-edge: rgb(255 255 255 / 0.22);
      --glass-shadow: rgb(0 0 0 / 0.5);
      --field-bg: rgb(255 255 255 / 0.09);
      --field-edge: rgb(255 255 255 / 0.16);
      --menu-bg: #1f2937;
      --ink: #f8fafc;
      --ink-soft: rgb(248 250 252 / 0.7);
      --accent: #fbbf24;
      --focus: #93c5fd;
    }
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
    width: 348px;
    max-height: calc(100dvh - 32px);
    padding: 20px;
    border-radius: 28px;
    display: flex;
    flex-direction: column;
    gap: 16px;
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
  h2 {
    font-size: 1rem;
    font-weight: 650;
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

  section {
    display: grid;
    gap: 10px;
  }
  .hint {
    font-size: 0.82rem;
    line-height: 1.4;
    color: var(--ink-soft);
  }
  .next {
    display: inline-block;
    min-width: 1.5em;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    font-weight: 700;
    text-align: center;
  }
  @media (prefers-color-scheme: dark) {
    .next {
      color: #111827;
    }
  }

  .grid {
    display: grid;
    grid-template-columns: auto 1fr 1fr;
    gap: 8px;
    align-items: center;
  }
  .col {
    font-size: 0.75rem;
    color: var(--ink-soft);
    padding-left: 4px;
  }

  input,
  select {
    font: inherit;
    color: inherit;
    min-width: 0;
    width: 100%;
    padding: 9px 12px;
    border-radius: 14px;
    border: 1px solid var(--field-edge);
    background: var(--field-bg);
    box-shadow: inset 0 1px 2px rgb(15 23 42 / 0.08);
    transition:
      border-color 0.15s,
      box-shadow 0.15s;
  }
  input {
    font-variant-numeric: tabular-nums;
  }
  /* Rozwijana lista jest rysowana przez system i nie dziedziczy szkła,
     więc pozycje dostają własne, nieprzezroczyste tło. */
  option {
    background-color: var(--menu-bg);
    color: var(--ink);
  }
  input:focus-visible,
  select:focus-visible {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--focus) 35%, transparent);
  }

  .results {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 9px 12px;
    padding: 14px 16px;
    border-radius: 20px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
    font-size: 0.9rem;
  }
  dt {
    color: var(--ink-soft);
  }
  dd {
    text-align: right;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
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
      max-height: 56dvh;
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
    main {
      --glass-bg: Canvas;
    }
    .glass,
    .map :global(.ol-zoom),
    .map :global(.ol-attribution) {
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
    }
  }
</style>
