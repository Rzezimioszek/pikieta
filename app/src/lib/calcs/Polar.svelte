<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { PolarData } from '../calcs'
  import { azimuthDistance, polar, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { assignIds, coords, pointState, f2, f3, f4, isBlank, keepTrailing, num, tableLines, turn } from './util'

  interface Props {
    data: PolarData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Src = PolarData['rows'][number]
  interface Item {
    src: Src
    id: string
    p?: XY
    status: string
    tone?: 'error' | 'muted' | 'ok'
  }

  const empty = (): Src => ({ id: '', angle: '', dist: '' })

  const st = $derived(findPoint(data.station))
  const ref = $derived(findPoint(data.ref))
  const refAz = $derived(st && ref ? (azimuthDistance(st, ref)?.azimuth ?? null) : null)
  const ids = $derived(assignIds(data.rows.map((r) => (isBlank(r) ? 'x' : r.id))))

  const items = $derived(
    data.rows.map((src, i): Item => {
      if (isBlank(src)) return { src, id: '', status: '' }
      const id = ids[i]
      if (src.angle.trim() === '' || src.dist.trim() === '') return { src, id, status: 'niepełny', tone: 'muted' }
      const [a, d] = [num(src.angle), num(src.dist)]
      if (a === null || d === null) return { src, id, status: 'błędna liczba', tone: 'error' }
      if (refAz === null) return { src, id, status: 'brak stanowiska lub nawiązania', tone: 'muted' }
      const p = polar(st!, turn(refAz + a), d)
      return { src, id, p, ...pointState(id, p) }
    }),
  )
  const computed = $derived(items.filter((i) => i.p))
  const fresh = $derived(computed.filter((i) => i.tone === 'ok'))

  $effect(() =>
    onsketch({
      paths: [...(st && ref ? [[ref, st]] : []), ...computed.map((i) => [st!, i.p!])],
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
    { key: 'angle', label: 'Kąt [g]', width: 96, align: 'right', edit: 'number', value: (i) => i.src.angle, set: edit('angle') },
    { key: 'dist', label: 'Długość [m]', width: 104, align: 'right', edit: 'number', value: (i) => i.src.dist, set: edit('dist') },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? f2(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? f2(i.p.y) : '') },
    { key: 'status', label: 'Stan', width: 130, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    // Nadane numery zostają w tabeli, żeby kolejne kliknięcie nie dodało tych samych punktów.
    for (const i of fresh) i.src.id = i.id
    addNewPoints(fresh.map((i) => ({ id: i.id, ...i.p!, h: null, code: '' })))
  }

  function save() {
    addReport(`${title} (${crsLabel})`, [
      `Stanowisko ${st!.id}: ${coords(st!)}`,
      `Nawiązanie ${ref!.id}: ${coords(ref!)}`,
      `Azymut nawiązania: ${f4(refAz!)} g`,
      ...tableLines(
        ['Nr', 'Kąt [g]', 'Długość [m]', 'X', 'Y'],
        computed.map((i) => [i.id, f4(num(i.src.angle)!), f3(num(i.src.dist)!), f2(i.p!.x), f2(i.p!.y)]),
      ),
    ])
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Stanowisko" bind:value={data.station} />
    <PointField label="Nawiązanie" bind:value={data.ref} />
    <div class="field">
      Azymut nawiązania
      <output>{refAz === null ? '—' : `${f4(refAz)} g`}</output>
    </div>
  </div>

  <p class="hint">Kąty mierzone od kierunku na nawiązanie, zgodnie z ruchem wskazówek zegara. Puste numery dostaną kolejne wolne.</p>

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
    <span class="hint">Obliczone: {computed.length}, nowe: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={computed.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
