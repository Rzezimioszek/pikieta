<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { FromOffsetData } from '../calcs'
  import { pointFromOffset, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { assignIds, coords, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: FromOffsetData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Src = FromOffsetData['rows'][number]
  interface Item {
    src: Src
    id: string
    p?: XY
    foot?: XY
    status: string
    tone?: 'error' | 'muted' | 'ok'
  }

  const empty = (): Src => ({ id: '', along: '', offset: '' })

  const a = $derived(findPoint(data.a))
  const b = $derived(findPoint(data.b))
  const ids = $derived(assignIds(data.rows.map((r) => (isBlank(r) ? 'x' : r.id))))

  const items = $derived(
    data.rows.map((src, i): Item => {
      if (isBlank(src)) return { src, id: '', status: '' }
      const id = ids[i]
      if (src.along.trim() === '') return { src, id, status: 'niepełny', tone: 'muted' }
      const along = num(src.along)
      const offset = src.offset.trim() === '' ? 0 : num(src.offset)
      if (along === null || offset === null) return { src, id, status: 'błędna liczba', tone: 'error' }
      if (!a || !b) return { src, id, status: 'brak linii', tone: 'muted' }
      const p = pointFromOffset(a, b, along, offset)
      if (!p) return { src, id, status: 'linia ma zerową długość', tone: 'error' }
      const foot = pointFromOffset(a, b, along, 0) ?? undefined
      return { src, id, p, foot, ...pointState(id, p) }
    }),
  )
  const computed = $derived(items.filter((i) => i.p))
  const fresh = $derived(computed.filter((i) => i.tone === 'ok'))

  $effect(() =>
    onsketch({
      paths: [...(a && b ? [[a, b]] : []), ...computed.map((i) => [i.foot!, i.p!])],
      results: computed.map((i) => ({ ...i.p!, label: i.id })),
    }),
  )

  const edit = (key: keyof Src) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, empty)
  }

  const columns: Column<Item>[] = [
    {
      key: 'id',
      label: 'Nr nowego',
      width: 96,
      edit: 'text',
      value: (i) => i.src.id,
      set: edit('id'),
      placeholder: (i) => (isBlank(i.src) ? '' : i.id),
    },
    { key: 'along', label: 'Bieżąca [m]', width: 104, align: 'right', edit: 'number', value: (i) => i.src.along, set: edit('along') },
    { key: 'offset', label: 'Domiar [m]', width: 104, align: 'right', edit: 'number', value: (i) => i.src.offset, set: edit('offset') },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.y) : '') },
    { key: 'status', label: 'Stan', width: 130, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    // Nadane numery zostają w tabeli, żeby kolejne kliknięcie nie dodało tych samych punktów.
    for (const i of fresh) i.src.id = i.id
    addNewPoints(fresh.map((i) => ({ id: i.id, ...i.p!, h: null, code: '' })))
  }

  function save() {
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        [`Początek linii ${a!.id}`, coords(a!)],
        [`Koniec linii ${b!.id}`, coords(b!)],
      ],
      table: reportTable(
        ['Nr', 'Bieżąca [m]', 'Domiar [m]', 'X', 'Y'],
        computed.map((i) => [i.id, fl(num(i.src.along)!), fl(num(i.src.offset) ?? 0), fc(i.p!.x), fc(i.p!.y)]),
      ),
      notes: ['Domiar dodatni po prawej stronie linii.'],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Początek linii" bind:value={data.a} />
    <PointField label="Koniec linii" bind:value={data.b} />
  </div>

  <p class="hint">Domiar dodatni w prawo, ujemny w lewo, pusty oznacza punkt na linii. Puste numery dostaną kolejne wolne.</p>

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
    maxHeight="min(55vh, 480px)"
  />

  <div class="actions">
    <span class="hint">Obliczone: {computed.length}, nowe: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={computed.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
