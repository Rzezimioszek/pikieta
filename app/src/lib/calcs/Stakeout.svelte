<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { StakeoutData } from '../calcs'
  import { gridToTerrain, lineReduction, stakeout, type Pt } from '../core'
  import type { Sketch } from '../map'
  import { addReport, findPoint, project, selection } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, reportTable } from './util'

  interface Props {
    data: StakeoutData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Src = StakeoutData['rows'][number]
  interface Item {
    src: Src
    p?: Pt
    azimuth?: number
    reading?: number
    angle?: number
    grid?: number
    terrain?: number
    dh?: number
    status: string
    tone?: 'error' | 'muted'
  }
  const empty = (): Src => ({ id: '' })

  const st = $derived(findPoint(data.station))
  const ref = $derived(findPoint(data.ref))
  const refReading = $derived(num(data.refReading) ?? 0)
  const badReading = $derived(data.refReading.trim() !== '' && num(data.refReading) === null)

  const items = $derived(
    data.rows.map((src): Item => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      if (!p) return { src, status: 'brak punktu', tone: 'error' }
      if (!st || !ref) return { src, p, status: 'brak stanowiska lub nawiązania', tone: 'muted' }
      const e = stakeout(st, ref, refReading, p)
      if (!e) return { src, p, status: 'punkt na stanowisku', tone: 'error' }
      // Długość w terenie: odwrócenie redukcji odwzorowawczej i na elipsoidę (wysokość średnia celowej).
      let terrain: number | undefined
      if (data.reduce === 'yes') {
        const scale = lineReduction(epsg, st, p)?.scale ?? 1
        const h = st.h !== null && p.h !== null ? (st.h + p.h) / 2 : (st.h ?? p.h)
        terrain = gridToTerrain(e.grid, scale, h)
      }
      const dh = st.h !== null && p.h !== null ? p.h - st.h : undefined
      return { src, p, ...e, terrain, dh, status: '' }
    }),
  )
  const done = $derived(items.filter((i) => i.reading !== undefined))

  $effect(() =>
    onsketch({
      paths: [...(st && ref ? [[ref, st]] : []), ...done.map((i) => [st!, i.p!])],
    }),
  )

  const opt = (v: number | undefined, f: (n: number) => string) => (v === undefined ? '' : f(v))
  const reduce = $derived(data.reduce === 'yes')

  const columns = $derived.by((): Column<Item>[] => [
    {
      key: 'id',
      label: 'Punkt',
      width: 96,
      edit: 'point',
      value: (i) => i.src.id,
      set: (i, v) => {
        i.src.id = v
        keepTrailing(data.rows, empty)
      },
    },
    { key: 'reading', label: 'Odczyt Hz [g]', width: 110, align: 'right', value: (i) => opt(i.reading, fa) },
    { key: 'angle', label: 'Kąt od naw. [g]', width: 116, align: 'right', value: (i) => opt(i.angle, fa) },
    { key: 'grid', label: 'Długość [m]', width: 100, align: 'right', value: (i) => opt(i.grid, fl) },
    ...(reduce
      ? [{ key: 'terrain', label: 'W terenie [m]', width: 108, align: 'right', value: (i: Item) => opt(i.terrain, fl) } as Column<Item>]
      : []),
    { key: 'dh', label: 'ΔH [m]', width: 80, align: 'right', value: (i) => opt(i.dh, fl) },
    { key: 'az', label: 'Azymut [g]', width: 104, align: 'right', value: (i) => opt(i.azimuth, fa) },
    { key: 'status', label: 'Stan', width: 120, value: (i) => i.status, tone: (i) => i.tone },
  ])

  function fromSelection() {
    const ids = project.points.filter((p) => selection.has(p.id) && p.id !== data.station && p.id !== data.ref)
    data.rows = [...ids.map((p) => ({ id: p.id })), empty()]
  }

  function save() {
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        [`Stanowisko ${st!.id}`, coords(st!)],
        [`Nawiązanie ${ref!.id}`, `${coords(ref!)}, odczyt ${fa(refReading)} g`],
        ['Długości w terenie', reduce ? 'z odwróceniem redukcji odwzorowawczej i na elipsoidę' : 'równe długościom ze współrzędnych'],
      ],
      table: reportTable(
        ['Punkt', 'X', 'Y', 'Odczyt Hz [g]', 'Kąt od naw. [g]', 'Długość [m]', ...(reduce ? ['W terenie [m]'] : []), 'ΔH [m]'],
        done.map((i) => [
          i.p!.id,
          fc(i.p!.x),
          fc(i.p!.y),
          fa(i.reading!),
          fa(i.angle!),
          fl(i.grid!),
          ...(reduce ? [opt(i.terrain, fl)] : []),
          opt(i.dh, fl),
        ]),
      ),
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Stanowisko" bind:value={data.station} />
    <PointField label="Nawiązanie" bind:value={data.ref} />
    <label class="field">
      Odczyt na nawiązaniu [g]<input inputmode="decimal" placeholder="0.0000" bind:value={data.refReading} />
    </label>
    <label class="field">
      Długości do odmierzenia
      <select bind:value={data.reduce}>
        <option value="no">ze współrzędnych</option>
        <option value="yes">w terenie (redukcje odwrotne)</option>
      </select>
    </label>
  </div>
  {#if badReading}<p class="error" role="alert">Odczyt na nawiązaniu nie jest liczbą.</p>{/if}

  <div class="actions start">
    <p class="hint">Punkty do wytyczenia z projektu. Odczyt Hz to wartość do ustawienia na kole przy podanym odczycie na nawiązaniu.</p>
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
    canMove={(i) => !isBlank(i.src)}
    onmove={(from, to) => moveRow(data.rows, from, to, empty)}
    ondelete={(_, i) => {
      data.rows.splice(i, 1)
      keepTrailing(data.rows, empty)
    }}
    maxHeight="min(50vh, 440px)"
  />

  <div class="actions">
    <span class="hint">Punkty: {done.length}</span>
    <button class="primary" disabled={done.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
