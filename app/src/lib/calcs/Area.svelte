<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { AreaData } from '../calcs'
  import { azimuthDistance, polygonArea, type Pt } from '../core'
  import type { Sketch } from '../map'
  import { addReport, findPoint, project, selection } from '../project.svelte'
  import type { Column } from '../table'
  import { fc, fha, fl, fm2, isBlank, keepTrailing, moveRow, reportTable, selfIntersects } from './util'

  interface Props {
    data: AreaData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Src = AreaData['rows'][number]
  interface Item {
    src: Src
    p?: Pt
    side?: number
    status: string
    tone?: 'error'
  }

  const empty = (): Src => ({ id: '' })

  const vertices = $derived(data.rows.map((r) => findPoint(r.id)).filter((p): p is Pt => !!p))
  /** Długość boku od wierzchołka do następnego (ostatni zamyka wielobok). */
  const sides = $derived(
    vertices.map((p, i) => (vertices.length > 1 ? (azimuthDistance(p, vertices[(i + 1) % vertices.length])?.distance ?? 0) : 0)),
  )

  const items = $derived.by(() => {
    let k = 0
    return data.rows.map((src): Item => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      if (!p) return { src, status: 'brak punktu', tone: 'error' }
      return { src, p, side: vertices.length >= 2 ? sides[k++] : undefined, status: '' }
    })
  })
  const missing = $derived(items.some((i) => i.tone === 'error'))
  const area = $derived(vertices.length >= 3 && !missing ? polygonArea(epsg, vertices) : null)
  const perimeter = $derived(sides.reduce((s, d) => s + d, 0))
  const crossed = $derived(selfIntersects(vertices))
  const CROSSED = 'Boki wieloboku się przecinają: pole jest błędne, sprawdź kolejność wierzchołków.'

  $effect(() => onsketch({ paths: [vertices], closed: true }))

  const columns: Column<Item>[] = [
    {
      key: 'id',
      label: 'Wierzchołek',
      width: 104,
      edit: 'point',
      value: (i) => i.src.id,
      set: (i, v) => {
        i.src.id = v
        keepTrailing(data.rows, empty)
      },
    },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.y) : '') },
    { key: 'side', label: 'Bok do następnego [m]', width: 160, align: 'right', value: (i) => (i.side === undefined ? '' : fl(i.side)) },
    { key: 'status', label: 'Uwagi', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function fromSelection() {
    const ids = project.points.filter((p) => selection.has(p.id)).map((p) => ({ id: p.id }))
    data.rows = [...ids, empty()]
  }

  function save() {
    addReport({
      title: `${title} (${crsLabel})`,
      table: reportTable(
        ['Wierzchołek', 'X', 'Y', 'Bok do następnego [m]'],
        vertices.map((p, i) => [p.id, fc(p.x), fc(p.y), fl(sides[i])]),
      ),
      results: [
        ['Pole ze współrzędnych', `${fm2(area!.grid)} m² (${fha(area!.grid)} ha)`],
        ['Pole na elipsoidzie', `${fm2(area!.ellipsoidal)} m²`],
        ['Obwód', `${fl(perimeter)} m`],
      ],
      notes: crossed ? [CROSSED] : [],
    })
  }
</script>

<div class="calc">
  <div class="actions start">
    <p class="hint">Wierzchołki po kolei, wielobok zamyka się sam. Zaznaczone punkty wstawią się w kolejności z tabeli.</p>
    <button class="small" disabled={selection.size < 3} onclick={fromSelection}>
      <Icon name="selectAll" size={16} /> Wstaw zaznaczone ({selection.size})
    </button>
  </div>

  <DataTable
    {columns}
    rows={items}
    rowKey={(_, i) => String(i)}
    editing
    live
    canDelete={(i) => !isBlank(i.src)}
    canMove={(i) => !isBlank(i.src)}
    onmove={(from, to) => moveRow(data.rows, from, to, empty)}
    ondelete={(_, i) => {
      data.rows.splice(i, 1)
      keepTrailing(data.rows, empty)
    }}
    maxHeight="min(50vh, 440px)"
  />

  {#if area && crossed}<p class="error" role="alert">{CROSSED}</p>{/if}
  {#if area}
    <dl class="card result">
      <dt>Pole ze współrzędnych</dt><dd>{fm2(area.grid)} m²</dd>
      <dt>Pole w hektarach</dt><dd>{fha(area.grid)} ha</dd>
      <dt>Pole na elipsoidzie</dt><dd>{fm2(area.ellipsoidal)} m²</dd>
      <dt>Obwód</dt><dd>{fl(perimeter)} m</dd>
      <dt>Wierzchołki</dt><dd>{vertices.length}</dd>
    </dl>
  {:else}
    <p class="hint">{missing ? 'Popraw wierzchołki oznaczone jako brakujące.' : 'Podaj co najmniej trzy wierzchołki.'}</p>
  {/if}

  <div class="actions">
    <button class="primary" disabled={!area} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
