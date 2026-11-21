<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { OffsetData } from '../calcs'
  import { lineOffset, pointFromOffset, type Pt, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addReport, findPoint, project, selection } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { coords, f2, f3, isBlank, keepTrailing, tableLines } from './util'

  interface Props {
    data: OffsetData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Src = OffsetData['rows'][number]
  interface Item {
    src: Src
    p?: Pt
    along?: number
    offset?: number
    foot?: XY
    status: string
    tone?: 'error' | 'muted'
  }

  const empty = (): Src => ({ id: '' })

  const a = $derived(findPoint(data.a))
  const b = $derived(findPoint(data.b))

  const items = $derived(
    data.rows.map((src): Item => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      if (!p) return { src, status: 'brak punktu', tone: 'error' }
      if (!a || !b) return { src, p, status: 'brak linii', tone: 'muted' }
      const m = lineOffset(a, b, p)
      if (!m) return { src, p, status: 'linia ma zerową długość', tone: 'error' }
      const side = Math.abs(m.offset) < 0.0005 ? 'na linii' : m.offset > 0 ? 'prawa' : 'lewa'
      return { src, p, ...m, foot: pointFromOffset(a, b, m.along, 0) ?? undefined, status: side }
    }),
  )
  const done = $derived(items.filter((i) => i.along !== undefined))

  $effect(() =>
    onsketch({
      paths: [...(a && b ? [[a, b]] : []), ...done.map((i) => [i.foot!, i.p!])],
    }),
  )

  const columns: Column<Item>[] = [
    {
      key: 'id',
      label: 'Nr punktu',
      width: 96,
      edit: 'point',
      value: (i) => i.src.id,
      set: (i, v) => {
        i.src.id = v
        keepTrailing(data.rows, empty)
      },
    },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? f2(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? f2(i.p.y) : '') },
    { key: 'along', label: 'Bieżąca [m]', width: 104, align: 'right', value: (i) => (i.along === undefined ? '' : f3(i.along)) },
    { key: 'offset', label: 'Domiar [m]', width: 104, align: 'right', value: (i) => (i.offset === undefined ? '' : f3(i.offset)) },
    { key: 'status', label: 'Strona', width: 120, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function fromSelection() {
    const ids = project.points.filter((p) => selection.has(p.id) && p.id !== a?.id && p.id !== b?.id).map((p) => p.id)
    data.rows = [...data.rows.filter((r) => !isBlank(r)), ...ids.map((id) => ({ id })), empty()]
  }

  function save() {
    addReport(`${title} (${crsLabel})`, [
      `Początek linii ${a!.id}: ${coords(a!)}`,
      `Koniec linii ${b!.id}: ${coords(b!)}`,
      'Domiar dodatni po prawej stronie linii.',
      ...tableLines(
        ['Nr', 'Bieżąca [m]', 'Domiar [m]'],
        done.map((i) => [i.p!.id, f3(i.along!), f3(i.offset!)]),
      ),
    ])
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Początek linii" bind:value={data.a} />
    <PointField label="Koniec linii" bind:value={data.b} />
  </div>

  <div class="actions start">
    <p class="hint">Domiar dodatni po prawej stronie linii, patrząc od początku do końca.</p>
    <button class="small" disabled={selection.size === 0} onclick={fromSelection}>
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
    ondelete={(_, i) => {
      data.rows.splice(i, 1)
      keepTrailing(data.rows, empty)
    }}
    maxHeight="min(55vh, 480px)"
  />

  <div class="actions">
    <span class="hint">Zrzutowane punkty: {done.length}</span>
    <button class="primary" disabled={done.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
