<script lang="ts">
  import { onMount } from 'svelte'
  import { CRS_LIST, createMap, type GeoPoint, type MapHandle } from './lib/map'
  import init, { azimuth_distance, core_version } from './lib/wasm/pikieta.js'

  let mapEl = $state<HTMLDivElement>()
  let handle = $state.raw<MapHandle>()
  let coreVersion = $state('')
  let crs = $state('EPSG:2178')
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
    return out
  })

  $effect(() => handle?.setSegment(a, b))
  $effect(() => handle?.setCrs(crs))

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

  <aside>
    <h1>Pikieta</h1>

    <label class="row">
      Układ
      <select bind:value={crs}>
        {#each CRS_LIST as c (c.code)}
          <option value={c.code}>{c.label}</option>
        {/each}
      </select>
    </label>

    <h2>Azymut i długość</h2>
    <p class="hint">Wpisz współrzędne albo kliknij na mapie (następny punkt: {nextClick.toUpperCase()}).</p>

    <div class="grid">
      <span></span><span>X (północ)</span><span>Y (wschód)</span>
      <strong>A</strong>
      <input aria-label="X punktu A" inputmode="decimal" bind:value={fields.xa} />
      <input aria-label="Y punktu A" inputmode="decimal" bind:value={fields.ya} />
      <strong>B</strong>
      <input aria-label="X punktu B" inputmode="decimal" bind:value={fields.xb} />
      <input aria-label="Y punktu B" inputmode="decimal" bind:value={fields.yb} />
    </div>

    <dl>
      <dt>Azymut A→B</dt>
      <dd>{result ? `${result.azimuth.toFixed(4)} g` : '—'}</dd>
      <dt>Długość</dt>
      <dd>{result ? `${result.distance.toFixed(3)} m` : '—'}</dd>
    </dl>

    <footer>
      <span>{cursor ? `X ${cursor.x.toFixed(2)}  Y ${cursor.y.toFixed(2)}` : ''}</span>
      <span>{coreVersion ? `rdzeń ${coreVersion}` : 'ładowanie rdzenia…'}</span>
    </footer>
  </aside>
</main>

<style>
  main {
    display: flex;
    height: 100dvh;
  }
  .map {
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  aside {
    width: 320px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    border-left: 1px solid color-mix(in srgb, currentColor 20%, transparent);
  }
  h1 {
    font-size: 1.25rem;
  }
  h2 {
    font-size: 1rem;
  }
  .hint {
    font-size: 0.85rem;
    opacity: 0.75;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row select {
    flex: 1;
    min-width: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: auto 1fr 1fr;
    gap: 6px 8px;
    align-items: center;
    font-size: 0.85rem;
  }
  input,
  select {
    font: inherit;
    padding: 8px;
    min-width: 0;
  }
  input {
    font-variant-numeric: tabular-nums;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 12px;
  }
  dd {
    text-align: right;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  footer {
    margin-top: auto;
    display: flex;
    justify-content: space-between;
    gap: 8px;
    font-size: 0.75rem;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
    white-space: pre;
  }
  @media (max-width: 700px) {
    main {
      flex-direction: column;
    }
    aside {
      width: auto;
      max-height: 50dvh;
      border-left: none;
      border-top: 1px solid color-mix(in srgb, currentColor 20%, transparent);
    }
    footer {
      margin-top: 0;
    }
  }
</style>
