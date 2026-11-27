<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { DivisionData } from '../calcs'
  import { divideParcel, polygonArea, type Pt, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId, project, selection } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import {
    coords,
    fc,
    fha,
    fm2,
    isBlank,
    keepTrailing,
    moveRow,
    num,
    numberSeq,
    pointState,
    reportTable,
    selfIntersects,
  } from './util'

  interface Props {
    data: DivisionData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Vertex = DivisionData['rows'][number]
  type Part = DivisionData['parts'][number]
  const emptyVertex = (): Vertex => ({ id: '' })
  const emptyPart = (): Part => ({ area: '' })

  const vertexItems = $derived(
    data.rows.map((src) => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      return p ? { src, p, status: '' } : { src, status: 'brak punktu', tone: 'error' as const }
    }),
  )
  const vertices = $derived(vertexItems.filter((v) => v.p).map((v) => v.p!) as Pt[])
  const missing = $derived(vertexItems.some((v) => v.tone === 'error'))
  const area = $derived(vertices.length >= 3 && !missing ? polygonArea(epsg, vertices) : null)
  /** Pola zadaje się jako ewidencyjne (na elipsoidzie); w układzie są większe o ten współczynnik. */
  const k = $derived(area && area.ellipsoidal > 0 ? area.grid / area.ellipsoidal : 1)
  const crossed = $derived(selfIntersects(vertices))

  /** Pola części [m², ewidencyjne]: z tabeli albo równe części. */
  const targets = $derived.by((): number[] | string => {
    if (data.split === 'equal') {
      const n = num(data.n)
      if (n === null) return []
      if (!Number.isInteger(n) || n < 2) return 'Liczba części musi być liczbą całkowitą, co najmniej 2.'
      return area ? Array.from({ length: n - 1 }, () => area.ellipsoidal / n) : []
    }
    const list = data.parts.filter((p) => p.area.trim()).map((p) => num(p.area))
    if (list.some((v) => v === null || v <= 0)) return 'Pola części muszą być liczbami dodatnimi.'
    return list as number[]
  })

  const dir = $derived.by((): [XY, XY] | null => {
    if (data.mode !== 'parallel') return null
    const [a, b] = [findPoint(data.a), findPoint(data.b)]
    return a && b ? [a, b] : null
  })
  const startIndex = $derived(data.mode === 'point' ? vertices.findIndex((v) => v.id === data.start.trim()) : -1)

  const outcome = $derived.by(() => {
    if (!area || typeof targets === 'string' || targets.length === 0) return null
    if (data.mode === 'parallel' && !dir) return null
    if (data.mode === 'point' && startIndex < 0) {
      return data.start.trim() ? { error: 'Punkt początkowy musi być wierzchołkiem działki.' } : null
    }
    try {
      return { ok: divideParcel(vertices, dir, startIndex, targets.map((t) => t * k)) }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const res = $derived(outcome && 'ok' in outcome ? outcome.ok : null)

  /** Nowe punkty: końce linii podziału, bez tych, które pokrywają się z wierzchołkami działki. */
  const fresh = $derived.by(() => {
    if (!res) return []
    const pts: XY[] = []
    for (const cut of res.cuts) {
      for (const p of data.mode === 'point' ? cut.slice(1) : cut) {
        if (vertices.some((v) => Math.hypot(v.x - p.x, v.y - p.y) < 0.001)) continue
        if (pts.some((q) => Math.hypot(q.x - p.x, q.y - p.y) < 0.001)) continue
        pts.push(p)
      }
    }
    return pts
  })
  const ids = $derived(numberSeq(data.firstId, fresh.length))
  const points = $derived(fresh.map((p, i) => ({ id: ids[i], p, ...pointState(ids[i], p) })))
  /** Numer punktu linii podziału: nowy albo istniejący wierzchołek. */
  function label(p: XY) {
    return (
      vertices.find((v) => Math.hypot(v.x - p.x, v.y - p.y) < 0.001)?.id ??
      points.find((q) => Math.hypot(q.p.x - p.x, q.p.y - p.y) < 0.001)?.id ??
      '?'
    )
  }

  $effect(() => {
    const paths: XY[][] = vertices.length ? [[...vertices, vertices[0]]] : []
    if (res) paths.push(...res.cuts)
    onsketch({ paths, results: points.map((i) => ({ ...i.p, label: i.id })) })
  })

  const vertexColumns: Column<(typeof vertexItems)[number]>[] = [
    {
      key: 'id',
      label: 'Wierzchołek',
      width: 104,
      edit: 'point',
      value: (i) => i.src.id,
      set: (i, v) => {
        i.src.id = v
        keepTrailing(data.rows, emptyVertex)
      },
    },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.y) : '') },
    { key: 'status', label: 'Uwagi', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]
  const partItems = $derived(data.parts.map((src, i) => ({ src, i })))
  const partColumns: Column<(typeof partItems)[number]>[] = [
    {
      key: 'area',
      label: 'Pole części [m²]',
      width: 130,
      align: 'right',
      edit: 'number',
      value: (i) => i.src.area,
      set: (i, v) => {
        i.src.area = v
        keepTrailing(data.parts, emptyPart)
      },
    },
    {
      key: 'ha',
      label: '[ha]',
      width: 90,
      align: 'right',
      value: (i) => {
        const v = num(i.src.area)
        return v === null ? '' : fha(v)
      },
    },
  ]

  function fromSelection() {
    const ids = project.points.filter((p) => selection.has(p.id)).map((p) => ({ id: p.id }))
    data.rows = [...ids, emptyVertex()]
  }

  function add() {
    const list = points.filter((i) => i.tone === 'ok')
    if (list[0]) data.firstId = points[0].id
    addNewPoints(list.map((i) => ({ id: i.id, ...i.p, h: null, code: '' })))
  }

  function save() {
    const r = res!
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        ['Działka', vertices.map((v) => v.id).join('-')],
        ['Pole działki', `${fm2(area!.ellipsoidal)} m² (${fha(area!.ellipsoidal)} ha), w układzie ${fm2(area!.grid)} m²`],
        data.mode === 'parallel'
          ? ['Podział', `równolegle do prostej ${data.a.trim()}–${data.b.trim()}`]
          : ['Podział', `z punktu ${data.start.trim()}`],
      ],
      table: reportTable(
        ['Część', 'Pole [m²]', 'Pole [ha]', 'Linia podziału'],
        r.areas.map((a, i) => [
          String(i + 1),
          fm2(a / k),
          fha(a / k),
          r.cuts[i] ? r.cuts[i].map(label).join('–') : '(reszta)',
        ]),
      ),
      results: points.map((i) => [`Punkt ${i.id}`, coords(i.p)] as [string, string]),
      notes: [
        'Pola części ewidencyjne (na elipsoidzie GRS80); w układzie przeliczone współczynnikiem z pola działki.',
        ...(r.outside ? ['Linia podziału przecina granicę więcej niż dwa razy: działka wklęsła, sprawdź części.'] : []),
      ],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">
      Sposób podziału
      <select bind:value={data.mode}>
        <option value="parallel">równolegle do boku</option>
        <option value="point">z punktu (wierzchołka)</option>
      </select>
    </label>
    {#if data.mode === 'parallel'}
      <PointField label="Kierunek: punkt A" bind:value={data.a} />
      <PointField label="Kierunek: punkt B" bind:value={data.b} />
    {:else}
      <PointField label="Punkt początkowy" bind:value={data.start} />
    {/if}
    <label class="field">
      Części
      <select bind:value={data.split}>
        <option value="areas">o zadanych polach</option>
        <option value="equal">równe</option>
      </select>
    </label>
    {#if data.split === 'equal'}
      <label class="field">Liczba części<input inputmode="numeric" bind:value={data.n} /></label>
    {/if}
    <label class="field">Numer pierwszego punktu<input placeholder={nextId()} bind:value={data.firstId} /></label>
  </div>

  <p class="hint">
    {data.mode === 'parallel'
      ? 'Części liczone od strony działki najbliższej prostej A–B (zwykle bok działki).'
      : 'Części liczone od punktu początkowego wzdłuż granicy, w kolejności wierzchołków.'}
    Pola zadaje się jako ewidencyjne; ostatnia część to reszta działki.
  </p>

  <div class="tables">
    <section aria-label="Wierzchołki działki">
      <h3>Wierzchołki działki</h3>
      <DataTable
        columns={vertexColumns}
        rows={vertexItems}
        rowKey={(_, i) => String(i)}
        editing
        live
        canDelete={(i) => !isBlank(i.src)}
        canMove={(i) => !isBlank(i.src)}
        onmove={(from, to) => moveRow(data.rows, from, to, emptyVertex)}
        ondelete={(_, i) => {
          data.rows.splice(i, 1)
          keepTrailing(data.rows, emptyVertex)
        }}
        maxHeight="min(35vh, 300px)"
      />
      <button class="small" disabled={selection.size < 3} onclick={fromSelection}>Z zaznaczonych punktów</button>
    </section>
    {#if data.split === 'areas'}
      <section aria-label="Pola części">
        <h3>Pola części</h3>
        <DataTable
          columns={partColumns}
          rows={partItems}
          rowKey={(_, i) => String(i)}
          editing
          live
          canDelete={(i) => !isBlank(i.src)}
          canMove={(i) => !isBlank(i.src)}
          onmove={(from, to) => moveRow(data.parts, from, to, emptyPart)}
          ondelete={(_, i) => {
            data.parts.splice(i, 1)
            keepTrailing(data.parts, emptyPart)
          }}
          maxHeight="min(35vh, 300px)"
        />
      </section>
    {/if}
  </div>

  {#if crossed}<p class="error" role="alert">Boki działki się przecinają: sprawdź kolejność wierzchołków.</p>{/if}
  {#if typeof targets === 'string'}
    <p class="error" role="alert">{targets}</p>
  {:else if outcome && 'error' in outcome}
    <p class="error" role="alert">{outcome.error}</p>
  {:else if res}
    {#if res.outside}
      <p class="error" role="alert">Linia podziału przecina granicę więcej niż dwa razy (działka wklęsła): sprawdź części.</p>
    {/if}
    <dl class="card result">
      <dt>Pole działki</dt>
      <dd>{fm2(area!.ellipsoidal)} m² <span class="muted">· {fha(area!.ellipsoidal)} ha</span></dd>
      {#each res.areas as a, i (i)}
        <dt>Część {i + 1}{res.cuts[i] ? `: ${res.cuts[i].map(label).join('–')}` : ' (reszta)'}</dt>
        <dd>{fm2(a / k)} m²</dd>
      {/each}
      {#each points as p (p.id)}
        <dt>{p.id}</dt>
        <dd>{coords(p.p)} <span class="muted">· {p.status}</span></dd>
      {/each}
    </dl>
  {:else if area}
    <p class="hint">
      {data.mode === 'parallel' ? 'Podaj kierunek A–B' : 'Podaj punkt początkowy'} i pola części.
    </p>
  {:else}
    <p class="hint">Podaj co najmniej trzy wierzchołki działki w kolejności obejścia.</p>
  {/if}

  <div class="actions">
    <span class="hint">Nowe punkty: {points.filter((i) => i.tone === 'ok').length}</span>
    <button disabled={!points.some((i) => i.tone === 'ok')} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={!res} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>

<style>
  .tables {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
    gap: 12px;
    align-items: start;
  }
  section {
    display: grid;
    gap: 6px;
    justify-items: start;
  }
  section > :global(*) {
    justify-self: stretch;
  }
  section > button {
    justify-self: start;
  }
  h3 {
    margin: 0;
    font-size: 0.85rem;
  }
</style>
