<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { TraverseData } from '../calcs'
  import { fmt, lineReduction, traverse, type Pt, type TraverseOut, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint } from '../project.svelte'
  import type { Column } from '../table'
  import PointField from './PointField.svelte'
  import { assignIds, coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: TraverseData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, epsg, title, crsLabel, onsketch }: Props = $props()

  type Src = TraverseData['rows'][number]
  interface Item {
    src: Src
    id: string
    p?: XY
    /** Poprawka kąta [cc]. */
    v?: number
    /** Azymut i długość boku do następnego wierzchołka. */
    az?: number
    dist?: number
    status: string
    tone?: 'error' | 'muted' | 'ok'
  }

  const KINDS = { connected: 'dwustronnie nawiązany', closed: 'zamknięty', open: 'wiszący' }
  const empty = (): Src => ({ id: '', angle: '', dist: '' })
  /** Grady na sekundy setne (cc). */
  const CC = 10000
  const fcc = (g: number) => fmt(g * CC, 1)

  const rows = $derived(data.rows.filter((r) => !isBlank(r)))
  const n = $derived(rows.length)
  const kind = $derived(data.kind)
  /** Wiersze z nowymi punktami: bez początkowego i (poza wiszącym) bez końcowego. */
  const isNew = (i: number) => i > 0 && (kind === 'open' ? i < n : i < n - 1)
  const ids = $derived(assignIds(rows.map((r, i) => (isNew(i) ? r.id : r.id.trim() || 'x'))))

  const back = $derived(findPoint(data.back))
  const forward = $derived(findPoint(data.forward))
  const start = $derived(n ? findPoint(rows[0].id) : undefined)
  const end = $derived(n ? findPoint(rows[n - 1].id) : undefined)

  /** Kąty biorące udział w obliczeniu: wiszący nie ma kąta na ostatnim punkcie. */
  const angleRows = $derived(kind === 'open' ? n - 1 : n)

  const problems = $derived.by(() => {
    const p: string[] = []
    if (n < 2) return ['Wpisz wierzchołki ciągu: od punktu początkowego do końcowego.']
    if (!back) p.push(data.back.trim() ? `Brak punktu nawiązania ${data.back.trim()}.` : 'Podaj punkt nawiązania na początku.')
    if (!start) p.push(`Punkt początkowy ${rows[0].id.trim() || '(pusty)'} musi być w projekcie.`)
    if (kind === 'connected') {
      if (!end) p.push(`Punkt końcowy ${rows[n - 1].id.trim() || '(pusty)'} musi być w projekcie.`)
      if (!forward) p.push(data.forward.trim() ? `Brak punktu nawiązania ${data.forward.trim()}.` : 'Podaj punkt nawiązania na końcu.')
    }
    if (kind === 'closed' && rows[n - 1].id.trim() !== rows[0].id.trim())
      p.push('Ciąg zamknięty kończy się na punkcie początkowym: powtórz jego numer w ostatnim wierszu.')
    for (let i = 0; i < n; i++) {
      const r = rows[i]
      if (i < angleRows && num(r.angle) === null) {
        p.push(`Brak lub błędny kąt w wierszu ${i + 1}.`)
        break
      }
      if (i < n - 1 && !(num(r.dist)! > 0)) {
        p.push(`Brak lub błędna długość w wierszu ${i + 1}.`)
        break
      }
    }
    return p
  })

  const outcome = $derived.by((): { r: TraverseOut; dists: number[] } | { error: string } | null => {
    if (problems.length) return null
    const angles = rows.slice(0, angleRows).map((r) => num(r.angle)!)
    const measured = rows.slice(0, n - 1).map((r) => num(r.dist)!)
    const known = { back: back!, start: start!, end: kind === 'connected' ? end : undefined, forward: kind === 'connected' ? forward : undefined }
    try {
      const run = (dists: number[]) => traverse(kind, known, angles, dists, data.angles === 'right')
      let r = run(measured)
      let dists = measured
      if (data.reduce === 'yes') {
        // Skala odwzorowania boku z położenia przybliżonego; drugi przebieg z długościami zredukowanymi.
        const at = (i: number) => r.points[i % r.points.length]
        dists = measured.map((d, i) => d * (lineReduction(epsg, at(i), at(i + 1))?.scale ?? 1))
        r = run(dists)
      }
      return { r, dists }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const result = $derived(outcome && 'r' in outcome ? outcome.r : null)

  const items = $derived(
    data.rows.map((src): Item => {
      const i = rows.indexOf(src)
      if (i < 0) return { src, id: '', status: '' }
      const id = ids[i]
      const p = result ? result.points[i % result.points.length] : undefined
      const leg = result && i < n - 1 ? { az: result.azimuths[i], dist: (outcome as { dists: number[] }).dists[i] } : {}
      const v = result && i < angleRows ? result.angleCorrections[i] * CC : undefined
      const base = { src, id, p, v, ...leg }
      if (!isNew(i)) return { ...base, status: findPoint(src.id) ? 'punkt znany' : '', tone: 'muted' }
      return p ? { ...base, ...pointState(id, p) } : { ...base, status: '' }
    }),
  )
  const fresh = $derived(items.filter((i) => isNew(rows.indexOf(i.src)) && i.p && i.tone === 'ok'))

  // Odchyłki i dopuszczalne.
  const fAlpha = $derived(result?.angularMisclosure ?? null)
  const nAngles = $derived(kind === 'closed' ? angleRows - 1 : angleRows)
  const mAngle = $derived(num(data.mAngle))
  const fAlphaMax = $derived(mAngle !== null && mAngle > 0 ? 2 * mAngle * Math.sqrt(nAngles) : null)
  const fL = $derived(result && result.fx !== null && result.fy !== null ? Math.hypot(result.fx, result.fy) : null)
  const rel = $derived(fL && result ? result.length / fL : null)
  const relLimit = $derived(num(data.relLimit))
  const angleOk = $derived(fAlpha === null || fAlphaMax === null ? null : Math.abs(fAlpha * CC) <= fAlphaMax)
  const linearOk = $derived(rel === null || relLimit === null || relLimit <= 0 ? null : rel >= relLimit)

  $effect(() => {
    const pts = result?.points ?? []
    const line = kind === 'closed' && pts.length ? [...pts, pts[0]] : pts
    onsketch({
      paths: [
        ...(back && start ? [[back, start]] : []),
        line,
        ...(kind === 'connected' && end && forward ? [[end, forward]] : []),
      ],
      results: items.filter((i) => isNew(rows.indexOf(i.src)) && i.p).map((i) => ({ ...i.p!, label: i.id })),
    })
  })

  const edit = (key: keyof Src) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, empty)
  }
  const opt = (v: number | undefined, f: (n: number) => string) => (v === undefined ? '' : f(v))

  const columns: Column<Item>[] = [
    {
      key: 'id',
      label: 'Punkt',
      width: 96,
      edit: 'point',
      value: (i) => i.src.id,
      set: edit('id'),
      placeholder: (i) => (isBlank(i.src) ? '' : i.id),
    },
    { key: 'angle', label: 'Kąt [g]', width: 100, align: 'right', edit: 'number', value: (i) => i.src.angle, set: edit('angle') },
    { key: 'dist', label: 'Bok [m]', width: 96, align: 'right', edit: 'number', value: (i) => i.src.dist, set: edit('dist') },
    { key: 'v', label: 'Popr. [cc]', width: 88, align: 'right', value: (i) => opt(i.v, (v) => fmt(v, 1)) },
    { key: 'az', label: 'Azymut [g]', width: 100, align: 'right', value: (i) => opt(i.az, fa) },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => opt(i.p?.x, fc) },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => opt(i.p?.y, fc) },
    { key: 'status', label: 'Stan', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    for (const i of fresh) i.src.id = i.id
    addNewPoints(fresh.map((i) => ({ id: i.id, ...i.p!, h: null, code: '' })))
  }

  const judge = (ok: boolean | null) => (ok === null ? '' : ok ? ' (w granicy)' : ' (przekroczona)')

  function save() {
    const r = result!
    const pts = r.points
    const at = (i: number) => pts[i % pts.length]
    const known = (label: string, p: Pt | undefined) => (p ? [[`${label} ${p.id}`, coords(p)] as [string, string]] : [])
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        ['Rodzaj', KINDS[kind]],
        ['Kąty', data.angles === 'left' ? 'lewe' : 'prawe'],
        ...known('Nawiązanie początkowe', back),
        ...known('Punkt początkowy', start),
        ...(kind === 'connected' ? [...known('Punkt końcowy', end), ...known('Nawiązanie końcowe', forward)] : []),
        ['Redukcja odwzorowawcza długości', data.reduce === 'yes' ? 'tak' : 'nie'],
      ],
      table: reportTable(
        ['Punkt', 'Kąt [g]', 'v [cc]', 'Azymut [g]', 'Bok [m]', 'Δx [m]', 'Δy [m]', 'vΔx [m]', 'vΔy [m]', 'X', 'Y'],
        rows.map((row, i) => {
          const leg = i < n - 1
          const [a, b] = [at(i), at(i + 1)]
          return [
            ids[i],
            i < angleRows ? fa(num(row.angle)!) : '',
            i < angleRows ? fcc(r.angleCorrections[i]) : '',
            leg ? fa(r.azimuths[i]) : '',
            leg ? fl((outcome as { dists: number[] }).dists[i]) : '',
            leg ? fl(b.x - a.x) : '',
            leg ? fl(b.y - a.y) : '',
            leg ? fl(r.dxCorrections[i]) : '',
            leg ? fl(r.dyCorrections[i]) : '',
            fc(a.x),
            fc(a.y),
          ]
        }),
      ),
      results: [
        ...(fAlpha !== null
          ? [
              ['Odchyłka kątowa fα', `${fcc(fAlpha)} cc${fAlphaMax !== null ? `, dopuszczalna ${fmt(fAlphaMax, 0)} cc` : ''}${judge(angleOk)}`] as [string, string],
            ]
          : []),
        ...(r.fx !== null && r.fy !== null
          ? [
              ['Odchyłki fx, fy', `${fl(r.fx)} m, ${fl(r.fy)} m`] as [string, string],
              ['Odchyłka liniowa fL', `${fl(fL!)} m`] as [string, string],
              ['Odchyłka względna', `1 : ${Math.round(rel!)}${relLimit ? `, dopuszczalna 1 : ${relLimit}` : ''}${judge(linearOk)}`] as [string, string],
            ]
          : []),
        ['Długość ciągu', `${fl(r.length)} m`],
      ],
      notes: [
        kind === 'open' ? 'Ciąg wiszący: brak kontroli, wyniki bez wyrównania.' : 'Wyrównanie przybliżone: poprawki kątów po równo, przyrostów proporcjonalnie do długości boków.',
        ...(fAlphaMax !== null ? [`Odchyłka kątowa dopuszczalna 2·mα·√n, mα = ${data.mAngle.trim()} cc, n = ${nAngles}.`] : []),
      ],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">
      Rodzaj ciągu
      <select bind:value={data.kind}>
        {#each Object.entries(KINDS) as [k, label] (k)}<option value={k}>{label}</option>{/each}
      </select>
    </label>
    <label class="field">
      Kąty
      <select bind:value={data.angles}>
        <option value="left">lewe</option>
        <option value="right">prawe</option>
      </select>
    </label>
    <PointField label="Nawiązanie na początku" bind:value={data.back} />
    {#if kind === 'connected'}<PointField label="Nawiązanie na końcu" bind:value={data.forward} />{/if}
    <label class="field">
      Redukcja odwzorowawcza
      <select bind:value={data.reduce}>
        <option value="no">nie</option>
        <option value="yes">tak</option>
      </select>
    </label>
    <label class="field">mα do fα dop. [cc]<input inputmode="decimal" placeholder="bez sprawdzania" bind:value={data.mAngle} /></label>
    <label class="field">fL dop. względna 1 :<input inputmode="decimal" placeholder="bez sprawdzania" bind:value={data.relLimit} /></label>
  </div>

  <p class="hint">
    Wiersze od punktu początkowego: kąt na punkcie (pierwszy od nawiązania) i bok do następnego.
    {#if kind === 'connected'}Ostatni wiersz to znany punkt końcowy z kątem na nawiązanie końcowe.{:else if kind === 'closed'}Ostatni
      wiersz powtarza punkt początkowy z kątem zamykającym (od ostatniego punktu do pierwszego nowego).{:else}Ostatni wiersz to ostatni
      nowy punkt, bez kąta.{/if}
    Puste numery dostaną kolejne wolne.
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
    maxHeight="min(50vh, 440px)"
  />

  {#if problems.length && n >= 2}
    <p class="hint">{problems[0]}</p>
  {:else if outcome && 'error' in outcome}
    <p class="error" role="alert">{outcome.error}</p>
  {/if}

  {#if result}
    <dl class="card result">
      {#if fAlpha !== null}
        <dt>Odchyłka kątowa fα</dt>
        <dd class:bad={angleOk === false}>{fcc(fAlpha)} cc{fAlphaMax !== null ? ` / ${fmt(fAlphaMax, 0)}` : ''}</dd>
      {/if}
      {#if result.fx !== null && result.fy !== null}
        <dt>Odchyłki fx, fy</dt><dd>{fl(result.fx)} / {fl(result.fy)} m</dd>
        <dt>Odchyłka liniowa fL</dt><dd>{fl(fL!)} m</dd>
        <dt>Odchyłka względna</dt>
        <dd class:bad={linearOk === false}>1 : {Math.round(rel!)}{relLimit ? ` / 1 : ${relLimit}` : ''}</dd>
      {:else}
        <dt>Kontrola</dt><dd class="muted">brak (ciąg wiszący)</dd>
      {/if}
      <dt>Długość ciągu</dt><dd>{fl(result.length)} m</dd>
    </dl>
  {/if}

  <div class="actions">
    <span class="hint">Nowe punkty: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={!result} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>

<style>
  dd.bad {
    color: var(--danger, #e5484d);
  }
</style>
