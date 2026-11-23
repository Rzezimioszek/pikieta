<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { TachyData } from '../calcs'
  import { azimuthDistance, fmt, lineReduction, tachyOrientation, tachyShot, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { assignIds, coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: TachyData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Ref = TachyData['refs'][number]
  type Row = TachyData['rows'][number]
  const emptyRef = (): Ref => ({ id: '', hz: '' })
  const emptyRow = (): Row => ({ id: '', code: '', hz: '', v: '', s: '', target: '' })
  const CC = 10000

  const st = $derived(findPoint(data.station))
  const stationH = $derived(data.stationH.trim() ? num(data.stationH) : (st?.h ?? null))
  const instrument = $derived(num(data.instrument) ?? 0)

  // --- orientacja ---
  interface RefItem {
    src: Ref
    p?: XY
    az?: number
    residual?: number
    distance?: number
    status: string
    tone?: 'error' | 'muted'
  }
  const refsIn = $derived(
    data.refs
      .filter((r) => !isBlank(r))
      .map((r) => ({ r, p: findPoint(r.id), hz: num(r.hz) }))
      .filter((x) => x.p && x.hz !== null && st),
  )
  const orient = $derived.by(() => {
    if (!st || refsIn.length === 0) return null
    try {
      return tachyOrientation(
        st,
        refsIn.map((x) => ({ p: x.p!, hz: x.hz! })),
      )
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const constant = $derived(orient && 'constant' in orient ? orient.constant : null)

  const refItems = $derived(
    data.refs.map((src): RefItem => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      if (!p) return { src, status: src.id.trim() ? 'brak punktu' : 'niepełny', tone: src.id.trim() ? 'error' : 'muted' }
      if (num(src.hz) === null) return { src, p, status: 'brak kierunku', tone: 'muted' }
      const k = refsIn.findIndex((x) => x.r === src)
      const o = orient && 'refs' in orient && k >= 0 ? orient.refs[k] : undefined
      const az = st ? azimuthDistance(st, p)?.azimuth : undefined
      return { src, p, az, residual: o?.residual, distance: o?.distance, status: '' }
    }),
  )

  // --- pikiety ---
  interface Item {
    src: Row
    id: string
    p?: XY
    h?: number | null
    status: string
    tone?: 'error' | 'muted' | 'ok'
  }
  const ids = $derived(assignIds(data.rows.map((r) => (isBlank(r) ? 'x' : r.id))))

  const items = $derived.by(() => {
    let target = 0
    return data.rows.map((src, i): Item => {
      if (isBlank(src)) return { src, id: '', status: '' }
      // Puste pole wysokości celu: jak w wierszu wyżej.
      if (src.target.trim() !== '') target = num(src.target) ?? target
      const id = ids[i]
      const [hz, s] = [num(src.hz), num(src.s)]
      const zenith = src.v.trim() === '' ? null : num(src.v)
      if (src.hz.trim() === '' || src.s.trim() === '') return { src, id, status: 'niepełny', tone: 'muted' }
      if (hz === null || s === null || (src.v.trim() !== '' && zenith === null))
        return { src, id, status: 'błędna liczba', tone: 'error' }
      if (!st || constant === null) return { src, id, status: 'brak orientacji', tone: 'muted' }
      try {
        const station = { x: st.x, y: st.y, h: stationH, instrument }
        const m = { hz, zenith, distance: s, target }
        const reduce = data.reduce === 'yes'
        let r = tachyShot(station, constant, m, { curvature: data.curvature === 'yes', toEllipsoid: reduce, scale: 1 })
        if (reduce) {
          // Skala odwzorowania celowej z położenia przybliżonego.
          const scale = lineReduction(epsg, st, r.p)?.scale ?? 1
          r = tachyShot(station, constant, m, { curvature: data.curvature === 'yes', toEllipsoid: true, scale })
        }
        return { src, id, p: r.p, h: r.h, ...pointState(id, r.p) }
      } catch (e) {
        return { src, id, status: e instanceof Error ? e.message : String(e), tone: 'error' }
      }
    })
  })
  const computed = $derived(items.filter((i) => i.p))
  const fresh = $derived(computed.filter((i) => i.tone === 'ok'))

  $effect(() =>
    onsketch({
      paths: [
        ...refItems.filter((r) => r.p && st).map((r) => [st!, r.p!]),
        ...computed.map((i) => [st!, i.p!]),
      ],
      results: computed.map((i) => ({ ...i.p!, label: i.id })),
    }),
  )

  const opt = (v: number | null | undefined, f: (n: number) => string) => (v === null || v === undefined ? '' : f(v))
  const editRef = (key: keyof Ref) => (it: RefItem, v: string) => {
    it.src[key] = v
    keepTrailing(data.refs, emptyRef)
  }
  const edit = (key: keyof Row) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, emptyRow)
  }

  const refColumns: Column<RefItem>[] = [
    { key: 'id', label: 'Nawiązanie', width: 104, edit: 'point', value: (i) => i.src.id, set: editRef('id') },
    { key: 'hz', label: 'Kierunek [g]', width: 104, align: 'right', edit: 'number', value: (i) => i.src.hz, set: editRef('hz') },
    { key: 'az', label: 'Azymut [g]', width: 104, align: 'right', value: (i) => opt(i.az, fa) },
    { key: 'res', label: 'Odchyłka [cc]', width: 104, align: 'right', value: (i) => opt(i.residual, (v) => fmt(v * CC, 1)) },
    { key: 'd', label: 'Celowa [m]', width: 96, align: 'right', value: (i) => opt(i.distance, fl) },
    { key: 'status', label: 'Stan', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]

  const columns: Column<Item>[] = [
    {
      key: 'id',
      label: 'Nr',
      width: 84,
      edit: 'text',
      value: (i) => i.src.id,
      set: edit('id'),
      placeholder: (i) => (isBlank(i.src) ? '' : i.id),
    },
    { key: 'code', label: 'Kod', width: 70, edit: 'text', value: (i) => i.src.code, set: edit('code') },
    { key: 'hz', label: 'Kierunek [g]', width: 100, align: 'right', edit: 'number', value: (i) => i.src.hz, set: edit('hz') },
    { key: 'v', label: 'Zenit [g]', width: 92, align: 'right', edit: 'number', value: (i) => i.src.v, set: edit('v') },
    { key: 's', label: 'Odl. [m]', width: 88, align: 'right', edit: 'number', value: (i) => i.src.s, set: edit('s') },
    { key: 'target', label: 'Cel [m]', width: 72, align: 'right', edit: 'number', value: (i) => i.src.target, set: edit('target') },
    { key: 'x', label: 'X', width: 112, align: 'right', value: (i) => opt(i.p?.x, fc) },
    { key: 'y', label: 'Y', width: 112, align: 'right', value: (i) => opt(i.p?.y, fc) },
    { key: 'h', label: 'H', width: 80, align: 'right', value: (i) => opt(i.h, fc) },
    { key: 'status', label: 'Stan', width: 104, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    for (const i of fresh) i.src.id = i.id
    addNewPoints(fresh.map((i) => ({ id: i.id, ...i.p!, h: i.h ?? null, code: i.src.code.trim() })))
  }

  function save() {
    const refs = refItems.filter((r) => r.residual !== undefined)
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        [`Stanowisko ${st!.id}`, coords(st!) + (stationH !== null ? `  H ${fc(stationH)}` : '')],
        ['Wysokość instrumentu', `${fl(instrument)} m`],
        ['Stała orientacji', `${fa(constant!)} g`],
        ...refs.map(
          (r): [string, string] => [
            `Nawiązanie ${r.src.id.trim()}`,
            `kierunek ${fa(num(r.src.hz)!)} g, azymut ${fa(r.az!)} g, odchyłka ${fmt(r.residual! * CC, 1)} cc, celowa ${fl(r.distance!)} m`,
          ],
        ),
        ['Krzywizna Ziemi i refrakcja', data.curvature === 'yes' ? 'tak (k = 0,13)' : 'nie'],
        ['Redukcje odległości', data.reduce === 'yes' ? 'na elipsoidę i odwzorowawcza' : 'nie'],
      ],
      table: reportTable(
        ['Nr', 'Kod', 'Kierunek [g]', 'Zenit [g]', 'Odl. [m]', 'Cel [m]', 'X', 'Y', 'H'],
        computed.map((i) => [
          i.id,
          i.src.code,
          fa(num(i.src.hz)!),
          i.src.v.trim() ? fa(num(i.src.v)!) : '',
          fl(num(i.src.s)!),
          i.src.target,
          fc(i.p!.x),
          fc(i.p!.y),
          opt(i.h, fc),
        ]),
      ),
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Stanowisko" bind:value={data.station} />
    <label class="field">
      H stanowiska [m]<input inputmode="decimal" placeholder={st?.h != null ? fc(st.h) : 'brak'} bind:value={data.stationH} />
    </label>
    <label class="field">Wysokość instrumentu [m]<input inputmode="decimal" placeholder="0" bind:value={data.instrument} /></label>
    <label class="field">
      Krzywizna i refrakcja
      <select bind:value={data.curvature}>
        <option value="yes">tak</option>
        <option value="no">nie</option>
      </select>
    </label>
    <label class="field">
      Redukcje odległości
      <select bind:value={data.reduce}>
        <option value="no">nie</option>
        <option value="yes">na elipsoidę i odwzorowawcza</option>
      </select>
    </label>
    <div class="field">
      Stała orientacji
      <output>{constant === null ? '—' : `${fa(constant)} g`}</output>
    </div>
  </div>

  <DataTable
    columns={refColumns}
    rows={refItems}
    rowKey={(_, i) => `r${i}`}
    editing
    live
    canDelete={(i) => !isBlank(i.src)}
    ondelete={(_, i) => {
      data.refs.splice(i, 1)
      keepTrailing(data.refs, emptyRef)
    }}
    maxHeight="160px"
  />
  {#if orient && 'error' in orient}<p class="error" role="alert">{orient.error}</p>{/if}

  <p class="hint">
    Kierunki i kąty zenitalne w gradach, odległości skośne. Bez kąta zenitalnego odległość jest poziomą i punkt nie dostaje
    wysokości. Pusta wysokość celu: jak w wierszu wyżej.
  </p>

  <DataTable
    {columns}
    rows={items}
    rowKey={(_, i) => String(i)}
    editing
    live
    canDelete={(i) => !isBlank(i.src)}
    canMove={(i) => !isBlank(i.src)}
    onmove={(from, to) => moveRow(data.rows, from, to, emptyRow)}
    ondelete={(_, i) => {
      data.rows.splice(i, 1)
      keepTrailing(data.rows, emptyRow)
    }}
    maxHeight="min(45vh, 400px)"
  />

  <div class="actions">
    <span class="hint">Obliczone: {computed.length}, nowe: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={computed.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
