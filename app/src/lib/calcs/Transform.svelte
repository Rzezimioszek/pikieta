<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { TransformData } from '../calcs'
  import { fmt, planeTransform, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, project, selection } from '../project.svelte'
  import type { Column } from '../table'
  import { coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: TransformData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Ctrl = TransformData['control'][number]
  type Row = TransformData['rows'][number]
  const emptyCtrl = (): Ctrl => ({ id: '', xs: '', ys: '', xt: '', yt: '' })
  const emptyRow = (): Row => ({ id: '', x: '', y: '' })
  const METHODS = { helmert: 'Helmerta (przez podobieństwo)', affine: 'Afiniczna' } as const

  /** Współrzędne z pól albo, gdy puste, z punktu projektu o tym numerze. */
  function pick(id: string, x: string, y: string): { p: XY | null; fromProject: boolean; error?: string } {
    if (!x.trim() && !y.trim()) {
      const p = findPoint(id)
      return p ? { p, fromProject: true } : { p: null, fromProject: false, error: id.trim() ? 'brak w projekcie' : 'brak współrzędnych' }
    }
    const [px, py] = [num(x), num(y)]
    return px === null || py === null ? { p: null, fromProject: false, error: 'błędne współrzędne' } : { p: { x: px, y: py }, fromProject: false }
  }

  const ctrl = $derived(
    data.control.map((src) => {
      if (isBlank(src)) return { src, status: '' }
      const from = pick(src.id, src.xs, src.ys)
      const to = pick(src.id, src.xt, src.yt)
      if (from.fromProject && to.fromProject) return { src, status: 'podaj jeden z układów', tone: 'error' as const }
      const error = from.error ?? to.error
      if (error || !from.p || !to.p) return { src, status: error ?? 'niepełny', tone: 'error' as const }
      return { src, from: from.p, to: to.p, fromProject: from.fromProject, toProject: to.fromProject, status: '' }
    }),
  )
  const used = $derived(ctrl.filter((c) => c.from && c.to) as { src: Ctrl; from: XY; to: XY; status: string }[])

  const rows = $derived(
    data.rows.map((src) => {
      if (isBlank(src)) return { src, status: '' }
      const s = pick(src.id, src.x, src.y)
      return s.p ? { src, p: s.p, fromProject: s.fromProject, status: '' } : { src, status: s.error!, tone: 'error' as const }
    }),
  )
  const toMove = $derived(rows.filter((r) => r.p) as { src: Row; p: XY }[])
  const hausbrandt = $derived(data.hausbrandt === 'yes')

  const outcome = $derived.by(() => {
    if (used.length < (data.method === 'helmert' ? 2 : 3)) return null
    try {
      return {
        ok: planeTransform(
          data.method,
          used.map((c) => c.from),
          used.map((c) => c.to),
          toMove.map((r) => r.p),
          hausbrandt,
        ),
      }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const res = $derived(outcome && 'ok' in outcome ? outcome.ok : null)
  /** Odchyłka liniowa na punkcie dostosowania, w kolejności `used`. */
  const vLen = $derived(res ? res.residuals.map((v) => Math.hypot(v.x, v.y)) : [])
  /** Próg odstawania: trzykrotny błąd średni położenia punktu. */
  const limit = $derived(res?.m0 != null ? 3 * Math.SQRT2 * res.m0 : null)

  const ctrlItems = $derived.by(() => {
    let k = 0
    return ctrl.map((c) => {
      if (!('from' in c) || !c.from) return { ...c, v: undefined, vl: undefined }
      const i = k++
      const v = res?.residuals[i]
      const vl = vLen[i]
      const out = limit !== null && vl > limit
      return { ...c, v, vl, status: out ? 'odstaje' : c.status, tone: out ? ('error' as const) : undefined }
    })
  })

  const items = $derived.by(() => {
    let k = 0
    return rows.map((r) => {
      if (!r.p) return { ...r, q: undefined, newId: '' }
      const q = res?.points[k++]
      const newId = data.prefix + r.src.id.trim()
      const st = q && r.src.id.trim() ? pointState(newId, q) : null
      return { ...r, q, newId, status: st?.status ?? (r.src.id.trim() ? '' : 'brak numeru'), tone: st?.tone ?? (r.src.id.trim() ? undefined : ('error' as const)) }
    })
  })
  const fresh = $derived(items.filter((i) => i.q && i.tone === 'ok'))

  $effect(() => {
    onsketch({
      paths: [],
      results: items.filter((i) => i.q).map((i) => ({ ...i.q!, label: i.newId })),
    })
  })

  const editCtrl = (key: keyof Ctrl) => (it: (typeof ctrlItems)[number], v: string) => {
    it.src[key] = v
    keepTrailing(data.control, emptyCtrl)
  }
  const editRow = (key: keyof Row) => (it: (typeof items)[number], v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, emptyRow)
  }
  /** Podpowiedź współrzędnej z projektu, gdy pole puste. */
  const hint = (id: string, axis: 'x' | 'y') => {
    const p = findPoint(id)
    return p ? fc(p[axis]) : ''
  }
  const mm = (v: number | undefined) => (v === undefined ? '' : fmt(v * 1000, 1))

  const ctrlColumns: Column<(typeof ctrlItems)[number]>[] = [
    { key: 'id', label: 'Nr', width: 80, edit: 'point', value: (i) => i.src.id, set: editCtrl('id') },
    { key: 'xs', label: 'x pierwotny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.xs, set: editCtrl('xs'), placeholder: (i) => (i.src.xt.trim() ? hint(i.src.id, 'x') : '') },
    { key: 'ys', label: 'y pierwotny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.ys, set: editCtrl('ys'), placeholder: (i) => (i.src.yt.trim() ? hint(i.src.id, 'y') : '') },
    { key: 'xt', label: 'X wtórny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.xt, set: editCtrl('xt'), placeholder: (i) => (i.src.xs.trim() ? hint(i.src.id, 'x') : '') },
    { key: 'yt', label: 'Y wtórny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.yt, set: editCtrl('yt'), placeholder: (i) => (i.src.ys.trim() ? hint(i.src.id, 'y') : '') },
    { key: 'vx', label: 'vX [mm]', width: 80, align: 'right', value: (i) => mm(i.v?.x) },
    { key: 'vy', label: 'vY [mm]', width: 80, align: 'right', value: (i) => mm(i.v?.y) },
    { key: 'v', label: 'v [mm]', width: 80, align: 'right', value: (i) => mm(i.vl) },
    { key: 'status', label: 'Stan', width: 120, value: (i) => i.status, tone: (i) => i.tone },
  ]
  const rowColumns: Column<(typeof items)[number]>[] = [
    { key: 'id', label: 'Nr', width: 80, edit: 'point', value: (i) => i.src.id, set: editRow('id') },
    { key: 'x', label: 'x pierwotny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.x, set: editRow('x'), placeholder: (i) => hint(i.src.id, 'x') },
    { key: 'y', label: 'y pierwotny', width: 110, align: 'right', edit: 'number', value: (i) => i.src.y, set: editRow('y'), placeholder: (i) => hint(i.src.id, 'y') },
    { key: 'nid', label: 'Nr wynikowy', width: 90, value: (i) => (i.q ? i.newId : '') },
    { key: 'X', label: 'X', width: 110, align: 'right', value: (i) => (i.q ? fc(i.q.x) : '') },
    { key: 'Y', label: 'Y', width: 110, align: 'right', value: (i) => (i.q ? fc(i.q.y) : '') },
    { key: 'status', label: 'Stan', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]

  /** Wklejanie wierszy: numer i współrzędne oddzielone spacją, tabulatorem albo średnikiem. */
  let pasteCtrl = $state('')
  let pasteRows = $state('')
  const split = (text: string) =>
    text
      .split(/\r?\n/)
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => l.split(/[\t; ]+/))
  function appendCtrl() {
    const add = split(pasteCtrl).map(([id = '', xs = '', ys = '', xt = '', yt = '']) => ({ id, xs, ys, xt, yt }))
    data.control = [...data.control.filter((r) => !isBlank(r)), ...add]
    keepTrailing(data.control, emptyCtrl)
    pasteCtrl = ''
  }
  function appendRows(add: Row[]) {
    const have = new Set(data.rows.map((r) => r.id.trim()))
    data.rows = [...data.rows.filter((r) => !isBlank(r)), ...add.filter((r) => !r.id || !have.has(r.id))]
    keepTrailing(data.rows, emptyRow)
  }
  function pasteToRows() {
    appendRows(split(pasteRows).map(([id = '', x = '', y = '']) => ({ id, x, y })))
    pasteRows = ''
  }
  /** Zaznaczone punkty projektu, a bez zaznaczenia wszystkie. */
  function fromProject() {
    const ids = selection.size ? [...selection] : project.points.map((p) => p.id)
    appendRows(ids.map((id) => ({ id, x: '', y: '' })))
  }

  function add() {
    addNewPoints(fresh.map((i) => ({ ...(findPoint(i.src.id) ?? { h: null, code: '' }), id: i.newId, x: i.q!.x, y: i.q!.y })))
  }

  function save() {
    const r = res!
    const helmert = data.method === 'helmert'
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        ['Metoda', METHODS[data.method]],
        ['Korekta Hausbrandta', hausbrandt ? 'tak' : 'nie'],
        ['Punkty dostosowania', String(used.length)],
      ],
      table: reportTable(
        ['Nr', 'x pierwotny', 'y pierwotny', 'X wtórny', 'Y wtórny', 'vX [mm]', 'vY [mm]', 'v [mm]'],
        used.map((c, i) => [
          c.src.id.trim(),
          fc(c.from.x),
          fc(c.from.y),
          fc(c.to.x),
          fc(c.to.y),
          mm(r.residuals[i].x),
          mm(r.residuals[i].y),
          mm(vLen[i]),
        ]),
      ),
      results: [
        ...(r.m0 !== null ? [['Błąd średni transformacji m₀', `${fl(r.m0)} m`] as [string, string]] : []),
        helmert
          ? ['Skala', `${fmt(r.scale.x, 8)} (${fmt((r.scale.x - 1) * 1e6, 1)} mm/km)`]
          : ['Skale osi x, y', `${fmt(r.scale.x, 8)}, ${fmt(r.scale.y, 8)}`],
        helmert ? ['Kąt obrotu', `${fa(r.rotation.x)} g`] : ['Kąty obrotu osi x, y', `${fa(r.rotation.x)} g, ${fa(r.rotation.y)} g`],
        ...items.filter((i) => i.q).map((i) => [`Punkt ${i.newId}`, coords(i.q!)] as [string, string]),
      ],
      notes: [
        ...(r.m0 === null ? ['Brak punktów nadliczbowych: odchyłki nie kontrolują transformacji.'] : []),
        ...ctrlItems.filter((c) => c.status === 'odstaje').map((c) => `Punkt ${c.src.id.trim()} odstaje: v = ${mm(c.vl)} mm.`),
      ],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">
      Metoda
      <select bind:value={data.method}>
        {#each Object.entries(METHODS) as [k, label] (k)}<option value={k}>{label}</option>{/each}
      </select>
    </label>
    <label class="field">
      Korekta Hausbrandta
      <select bind:value={data.hausbrandt}>
        <option value="yes">tak</option>
        <option value="no">nie</option>
      </select>
    </label>
    <label class="field">Przedrostek numerów wynikowych<input bind:value={data.prefix} /></label>
  </div>

  <h3>Punkty dostosowania</h3>
  <p class="hint">
    Te same punkty w obu układach. Puste współrzędne jednego z układów są brane z punktu projektu o tym numerze.
  </p>
  <DataTable
    columns={ctrlColumns}
    rows={ctrlItems}
    rowKey={(_, i) => String(i)}
    editing
    live
    canDelete={(i) => !isBlank(i.src)}
    canMove={(i) => !isBlank(i.src)}
    onmove={(from, to) => moveRow(data.control, from, to, emptyCtrl)}
    ondelete={(_, i) => {
      data.control.splice(i, 1)
      keepTrailing(data.control, emptyCtrl)
    }}
    maxHeight="min(35vh, 300px)"
  />
  <details>
    <summary>Wklej punkty dostosowania</summary>
    <textarea rows="4" placeholder="Nr x y X Y" bind:value={pasteCtrl}></textarea>
    <button disabled={!pasteCtrl.trim()} onclick={appendCtrl}>Dodaj wiersze</button>
  </details>

  {#if outcome && 'error' in outcome}
    <p class="error" role="alert">{outcome.error}</p>
  {:else if res}
    <dl class="card result">
      {#if res.m0 !== null}
        <dt>Błąd średni transformacji m₀</dt><dd>{fl(res.m0)} m</dd>
      {:else}
        <dt>Kontrola</dt><dd class="muted">brak punktów nadliczbowych</dd>
      {/if}
      {#if data.method === 'helmert'}
        <dt>Skala</dt><dd>{fmt(res.scale.x, 8)} <span class="muted">· {fmt((res.scale.x - 1) * 1e6, 1)} mm/km</span></dd>
        <dt>Kąt obrotu</dt><dd>{fa(res.rotation.x)} g</dd>
      {:else}
        <dt>Skale osi x, y</dt><dd>{fmt(res.scale.x, 8)} / {fmt(res.scale.y, 8)}</dd>
        <dt>Kąty obrotu osi x, y</dt><dd>{fa(res.rotation.x)} / {fa(res.rotation.y)} g</dd>
      {/if}
    </dl>
  {:else}
    <p class="hint">Podaj co najmniej {data.method === 'helmert' ? 'dwa' : 'trzy'} punkty dostosowania.</p>
  {/if}

  <h3>Punkty do przeliczenia</h3>
  <DataTable
    columns={rowColumns}
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
    maxHeight="min(35vh, 300px)"
  />
  <details>
    <summary>Wklej punkty do przeliczenia</summary>
    <textarea rows="4" placeholder="Nr x y" bind:value={pasteRows}></textarea>
    <button disabled={!pasteRows.trim()} onclick={pasteToRows}>Dodaj wiersze</button>
  </details>

  <div class="actions">
    <button onclick={fromProject}>
      <Icon name="pointAdd" /> {selection.size ? `Zaznaczone punkty (${selection.size})` : 'Wszystkie punkty projektu'}
    </button>
    <span class="hint">Nowe: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={!res} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>

<style>
  h3 {
    margin: 4px 0 0;
    font-size: 0.85rem;
  }
  details textarea {
    width: 100%;
    box-sizing: border-box;
    font-family: ui-monospace, monospace;
    margin: 4px 0;
  }
</style>
