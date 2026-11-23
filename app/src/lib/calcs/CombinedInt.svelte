<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { CombinedIntData } from '../calcs'
  import { fmt, intersectionLsq, type IntObs, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import type { Column } from '../table'
  import { coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: CombinedIntData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Src = CombinedIntData['rows'][number]
  interface Item {
    src: Src
    /** Poprawka kąta [cc] i długości [mm] (gdy obserwacja wzięta do obliczenia). */
    vAngle?: number
    vDist?: number
    status: string
    tone?: 'error' | 'muted'
  }
  const empty = (): Src => ({ at: '', ref: '', angle: '', dist: '' })

  /** Obserwacje z wierszy: kąt (potrzebne nawiązanie) i/lub długość ze stanowiska. */
  const parsed = $derived(
    data.rows.map((src) => {
      if (isBlank(src)) return { src, obs: [] as { kind: 'angle' | 'dist'; o: IntObs }[], status: '' }
      const at = findPoint(src.at)
      if (!at) return { src, obs: [], status: src.at.trim() ? 'brak stanowiska' : 'niepełny', tone: 'error' as const }
      const obs: { kind: 'angle' | 'dist'; o: IntObs }[] = []
      if (src.angle.trim()) {
        const ref = findPoint(src.ref)
        const a = num(src.angle)
        if (!ref || a === null) return { src, obs: [], status: !ref ? 'kąt bez nawiązania' : 'błędny kąt', tone: 'error' as const }
        obs.push({ kind: 'angle', o: { kind: 'angle', at, ref, angle: a } })
      }
      if (src.dist.trim()) {
        const d = num(src.dist)
        if (d === null || d <= 0) return { src, obs: [], status: 'błędna długość', tone: 'error' as const }
        obs.push({ kind: 'dist', o: { kind: 'dist', from: at, d } })
      }
      return { src, obs, status: obs.length ? '' : 'brak kąta i długości', tone: obs.length ? undefined : ('muted' as const) }
    }),
  )
  const allObs = $derived(parsed.flatMap((p) => p.obs.map((x) => x.o)))
  const sigmaCc = $derived(num(data.sigmaAngle) ?? 10)
  const sigmaMm = $derived(num(data.sigmaDist) ?? 5)

  const outcome = $derived.by(() => {
    if (allObs.length < 2) return null
    try {
      return { ok: intersectionLsq(allObs, sigmaCc, sigmaMm, data.pick) }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const res = $derived(outcome && 'ok' in outcome ? outcome.ok : null)
  const id = $derived(data.id.trim() || nextId())
  const state = $derived(res ? pointState(id, res.p) : null)

  const items = $derived.by(() => {
    let k = 0
    return parsed.map((p): Item => {
      const item: Item = { src: p.src, status: p.status, tone: p.tone }
      for (const x of p.obs) {
        const v = res?.residuals[k++]
        if (x.kind === 'angle') item.vAngle = v
        else item.vDist = v
      }
      return item
    })
  })

  $effect(() => {
    const p = res?.p
    const lines: XY[][] = []
    if (p) for (const x of parsed) if (x.obs.length) lines.push([findPoint(x.src.at)!, p])
    for (const x of parsed) {
      const at = findPoint(x.src.at)
      const ref = findPoint(x.src.ref)
      if (at && ref && x.src.angle.trim()) lines.push([ref, at])
    }
    onsketch({ paths: lines, results: p ? [{ ...p, label: id }] : [] })
  })

  const edit = (key: keyof Src) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, empty)
  }
  const opt = (v: number | undefined, d: number) => (v === undefined ? '' : fmt(v, d))
  const columns: Column<Item>[] = [
    { key: 'at', label: 'Stanowisko', width: 100, edit: 'point', value: (i) => i.src.at, set: edit('at') },
    { key: 'ref', label: 'Nawiązanie', width: 100, edit: 'point', value: (i) => i.src.ref, set: edit('ref') },
    { key: 'angle', label: 'Kąt [g]', width: 100, align: 'right', edit: 'number', value: (i) => i.src.angle, set: edit('angle') },
    { key: 'dist', label: 'Długość [m]', width: 100, align: 'right', edit: 'number', value: (i) => i.src.dist, set: edit('dist') },
    { key: 'va', label: 'v kąta [cc]', width: 92, align: 'right', value: (i) => opt(i.vAngle, 1) },
    { key: 'vd', label: 'v dług. [mm]', width: 96, align: 'right', value: (i) => opt(i.vDist, 1) },
    { key: 'status', label: 'Stan', width: 120, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    data.id = id
    addNewPoints([{ id, ...res!.p, h: null, code: '' }])
  }

  function save() {
    const r = res!
    let k = 0
    const rows: string[][] = []
    for (const p of parsed) {
      for (const x of p.obs) {
        const v = r.residuals[k++]
        rows.push(
          x.o.kind === 'angle'
            ? [p.src.at.trim(), p.src.ref.trim(), `${fa(x.o.angle)} g`, `${fmt(v, 1)} cc`]
            : [p.src.at.trim(), '', `${fl(x.o.d)} m`, `${fmt(v, 1)} mm`],
        )
      }
    }
    addReport({
      title: `${title} (${crsLabel})`,
      params: [['Błędy średnie pomiaru (wagi)', `kąt ${fmt(sigmaCc, 1)} cc, długość ${fmt(sigmaMm, 1)} mm`]],
      table: reportTable(['Stanowisko', 'Nawiązanie', 'Obserwacja', 'Poprawka'], rows, 2),
      results: [
        [`Punkt ${id}`, coords(r.p)],
        ...(r.m0 !== null ? [['Błąd średni jednostkowy m₀', fmt(r.m0, 2)] as [string, string]] : []),
        ['Błędy średnie mx, my', `${fl(r.mx)} m, ${fl(r.my)} m${r.m0 === null ? ' (a priori)' : ''}`],
      ],
      notes:
        r.solutions.length > 1
          ? [`Bez obserwacji nadliczbowych: wybrano rozwiązanie ${data.pick + 1} z ${r.solutions.length}.`]
          : [],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">Numer nowego punktu<input placeholder={nextId()} bind:value={data.id} /></label>
    <label class="field">Błąd kąta [cc]<input inputmode="decimal" bind:value={data.sigmaAngle} /></label>
    <label class="field">Błąd długości [mm]<input inputmode="decimal" bind:value={data.sigmaDist} /></label>
    {#if res && res.solutions.length > 1}
      <label class="field">
        Rozwiązanie
        <select bind:value={data.pick}>
          {#each res.solutions as s, i (i)}<option value={i}>{i + 1}: X {fc(s.x)} Y {fc(s.y)}</option>{/each}
        </select>
      </label>
    {/if}
  </div>

  <p class="hint">
    Każdy wiersz to obserwacje z jednego punktu znanego: kąt od nawiązania do nowego punktu (zgodnie z ruchem wskazówek
    zegara) i/lub długość do nowego punktu. Kąt i długość w jednym wierszu to pomiar biegunowy.
  </p>

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
    maxHeight="min(45vh, 400px)"
  />

  {#if outcome && 'error' in outcome}
    <p class="error" role="alert">{outcome.error}</p>
  {:else if res}
    <dl class="card result">
      <dt>X</dt><dd>{fc(res.p.x)}</dd>
      <dt>Y</dt><dd>{fc(res.p.y)}</dd>
      {#if res.m0 !== null}
        <dt>Błąd średni jednostkowy m₀</dt><dd>{fmt(res.m0, 2)}</dd>
        <dt>Błędy średnie mx, my</dt><dd>{fl(res.mx)} / {fl(res.my)} m</dd>
      {:else}
        <dt>Błędy średnie mx, my (a priori)</dt><dd>{fl(res.mx)} / {fl(res.my)} m</dd>
        <dt>Kontrola</dt><dd class="muted">brak obserwacji nadliczbowych</dd>
      {/if}
      <dt>Stan punktu {id}</dt><dd class="muted">{state?.status}</dd>
    </dl>
  {:else}
    <p class="hint">Podaj co najmniej dwie obserwacje (kąty lub długości).</p>
  {/if}

  <div class="actions">
    <button disabled={!res || state?.tone !== 'ok'} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={!res} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
