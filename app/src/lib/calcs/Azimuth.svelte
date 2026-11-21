<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { AzimuthData } from '../calcs'
  import { azimuthDistance, lineReduction, type Pt } from '../core'
  import type { Sketch } from '../map'
  import { addReport, findPoint } from '../project.svelte'
  import type { Column } from '../table'
  import { f3, f4, isBlank, keepTrailing, tableLines } from './util'

  interface Props {
    data: AzimuthData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Src = AzimuthData['rows'][number]
  interface Item {
    src: Src
    a?: Pt
    b?: Pt
    az?: number
    dist?: number
    ell?: number
    scale?: number
    status: string
    tone?: 'error' | 'muted'
  }

  const empty = (): Src => ({ from: '', to: '' })

  const items = $derived(
    data.rows.map((src): Item => {
      if (isBlank(src)) return { src, status: '' }
      const [a, b] = [findPoint(src.from), findPoint(src.to)]
      if (src.from.trim() === '' || src.to.trim() === '') return { src, a, b, status: 'niepełny', tone: 'muted' }
      if (!a || !b) return { src, a, b, status: `brak punktu ${!a ? src.from : src.to}`, tone: 'error' }
      const r = azimuthDistance(a, b)
      if (!r) return { src, a, b, status: 'punkty się pokrywają', tone: 'error' }
      const red = lineReduction(epsg, a, b)
      return { src, a, b, az: r.azimuth, dist: r.distance, ell: red?.ellipsoidal, scale: red?.scale, status: '' }
    }),
  )
  const done = $derived(items.filter((i) => i.az !== undefined))

  $effect(() => onsketch({ paths: done.map((i) => [i.a!, i.b!]) }))

  const edit = (key: keyof Src) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, empty)
  }
  const opt = (v: number | undefined, f: (n: number) => string) => (v === undefined ? '' : f(v))

  const columns: Column<Item>[] = [
    { key: 'from', label: 'Od', width: 84, edit: 'point', value: (i) => i.src.from, set: edit('from') },
    { key: 'to', label: 'Do', width: 84, edit: 'point', value: (i) => i.src.to, set: edit('to') },
    { key: 'az', label: 'Azymut [g]', width: 104, align: 'right', value: (i) => opt(i.az, f4) },
    { key: 'dist', label: 'Długość [m]', width: 104, align: 'right', value: (i) => opt(i.dist, f3) },
    { key: 'ell', label: 'Na elipsoidzie [m]', width: 132, align: 'right', value: (i) => opt(i.ell, f3) },
    { key: 'scale', label: 'Skala', width: 104, align: 'right', value: (i) => opt(i.scale, (n) => n.toFixed(8)) },
    { key: 'status', label: 'Uwagi', width: 150, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function save() {
    addReport(`${title} (${crsLabel})`, [
      ...tableLines(
        ['Od → Do', 'Azymut [g]', 'Długość [m]', 'Na elipsoidzie [m]'],
        done.map((i) => [`${i.a!.id} → ${i.b!.id}`, f4(i.az!), f3(i.dist!), opt(i.ell, f3)]),
      ),
    ])
  }
</script>

<div class="calc">
  <p class="hint">Wpisz pary punktów albo wskazuj je na mapie. Enter przechodzi do kolejnego wiersza.</p>

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
    maxHeight="min(60vh, 520px)"
  />

  <div class="actions">
    <span class="hint">Obliczone odcinki: {done.length}</span>
    <button class="primary" disabled={done.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
