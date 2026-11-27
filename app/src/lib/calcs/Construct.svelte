<script lang="ts">
  import Icon from '../Icon.svelte'
  import type { ConstructData } from '../calcs'
  import { divideSegment, lineCircle, parallelIntersection, perpendicularFoot, type XY } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import PointField from './PointField.svelte'
  import { coords, fc, fl, num, numberSeq, pointState, reportTable } from './util'

  interface Props {
    data: ConstructData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  const MODES = {
    lineCircle: 'Przecięcie prostej z okręgiem',
    parallel: 'Prosta równoległa przecięta prostą',
    divide: 'Podział odcinka',
    foot: 'Spodek prostopadłej',
  } as const

  const a = $derived(findPoint(data.a))
  const b = $derived(findPoint(data.b))
  const c = $derived(findPoint(data.c))
  const d = $derived(findPoint(data.d))
  const value = $derived(num(data.value))

  /** Wynik: punkty albo komunikat, dlaczego ich nie ma. */
  const outcome = $derived.by((): { points: XY[] } | { message: string } | null => {
    if (!a || !b) return null
    switch (data.mode) {
      case 'lineCircle': {
        if (!c || value === null) return null
        if (value <= 0) return { message: 'Promień musi być dodatni.' }
        const pts = lineCircle(a, b, c, value)
        return pts.length ? { points: pts } : { message: 'Prosta nie przecina okręgu.' }
      }
      case 'parallel': {
        if (!c || !d || value === null) return null
        const p = parallelIntersection(a, b, data.side === 'right' ? value : -value, c, d)
        return p ? { points: [p] } : { message: 'Prosta równoległa nie przecina prostej C→D.' }
      }
      case 'divide': {
        if (value === null) return null
        if (data.divideBy === 'parts' && (!Number.isInteger(value) || value < 2))
          return { message: 'Liczba części musi być liczbą całkowitą, co najmniej 2.' }
        if (data.divideBy === 'step' && value <= 0) return { message: 'Krok musi być dodatni.' }
        const pts = data.divideBy === 'parts' ? divideSegment(a, b, value, 0) : divideSegment(a, b, 0, value)
        return pts.length ? { points: pts } : { message: 'Krok jest dłuższy niż odcinek.' }
      }
      case 'foot': {
        if (!c) return null
        const p = perpendicularFoot(a, b, c)
        return p ? { points: [p] } : { message: 'Punkty A i B się pokrywają.' }
      }
    }
  })
  const points = $derived(outcome && 'points' in outcome ? outcome.points : [])
  const ids = $derived(numberSeq(data.firstId, points.length))
  const items = $derived(points.map((p, i) => ({ id: ids[i], p, ...pointState(ids[i], p) })))
  const fresh = $derived(items.filter((i) => i.tone === 'ok'))

  $effect(() => {
    const paths: XY[][] = a && b ? [[a, b]] : []
    if (data.mode === 'parallel' && c && d) paths.push([c, d])
    if (data.mode === 'foot' && c && points[0]) paths.push([c, points[0]])
    if (data.mode === 'lineCircle' && c) for (const p of points) paths.push([c, p])
    onsketch({ paths, results: items.map((i) => ({ ...i.p, label: i.id })) })
  })

  function add() {
    if (fresh[0]) data.firstId = items[0].id
    addNewPoints(fresh.map((i) => ({ id: i.id, ...i.p, h: null, code: '' })))
  }

  function save() {
    const params: [string, string][] = [
      ['Konstrukcja', MODES[data.mode]],
      [`Punkt A ${a!.id}`, coords(a!)],
      [`Punkt B ${b!.id}`, coords(b!)],
    ]
    if (data.mode === 'lineCircle') params.push([`Środek okręgu ${c!.id}`, coords(c!)], ['Promień', `${fl(value!)} m`])
    if (data.mode === 'parallel')
      params.push(
        ['Odsunięcie', `${fl(value!)} m w ${data.side === 'right' ? 'prawo' : 'lewo'} od A→B`],
        [`Prosta C ${c!.id} → D ${d!.id}`, `${coords(c!)} → ${coords(d!)}`],
      )
    if (data.mode === 'divide')
      params.push([data.divideBy === 'parts' ? 'Liczba części' : 'Krok', data.divideBy === 'parts' ? String(value) : `${fl(value!)} m`])
    if (data.mode === 'foot') params.push([`Punkt rzutowany ${c!.id}`, coords(c!)])
    addReport({
      title: `${title} (${crsLabel})`,
      params,
      table: reportTable(
        ['Nr', 'X', 'Y'],
        items.map((i) => [i.id, fc(i.p.x), fc(i.p.y)]),
      ),
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">
      Konstrukcja
      <select bind:value={data.mode}>
        {#each Object.entries(MODES) as [k, label] (k)}<option value={k}>{label}</option>{/each}
      </select>
    </label>
    <PointField label="Punkt A" bind:value={data.a} />
    <PointField label="Punkt B" bind:value={data.b} />
    {#if data.mode === 'lineCircle'}
      <PointField label="Środek okręgu" bind:value={data.c} />
      <label class="field">Promień [m]<input inputmode="decimal" bind:value={data.value} /></label>
    {:else if data.mode === 'parallel'}
      <label class="field">Odsunięcie [m]<input inputmode="decimal" bind:value={data.value} /></label>
      <label class="field">
        Strona względem A→B
        <select bind:value={data.side}>
          <option value="right">prawa</option>
          <option value="left">lewa</option>
        </select>
      </label>
      <PointField label="Prosta przecinająca C" bind:value={data.c} />
      <PointField label="Prosta przecinająca D" bind:value={data.d} />
    {:else if data.mode === 'divide'}
      <label class="field">
        Podział
        <select bind:value={data.divideBy}>
          <option value="parts">na równe części</option>
          <option value="step">co zadany krok</option>
        </select>
      </label>
      <label class="field">
        {data.divideBy === 'parts' ? 'Liczba części' : 'Krok [m]'}<input inputmode="decimal" bind:value={data.value} />
      </label>
    {:else}
      <PointField label="Punkt rzutowany" bind:value={data.c} />
    {/if}
    <label class="field">Numer pierwszego punktu<input placeholder={nextId()} bind:value={data.firstId} /></label>
  </div>

  {#if outcome && 'message' in outcome}
    <p class="error" role="alert">{outcome.message}</p>
  {:else if items.length}
    <dl class="card result">
      {#each items as i (i.id)}
        <dt>{i.id}</dt>
        <dd>{coords(i.p)} <span class="muted">· {i.status}</span></dd>
      {/each}
    </dl>
  {:else}
    <p class="hint">Uzupełnij punkty i wartości.</p>
  {/if}

  <div class="actions">
    <span class="hint">Punkty: {items.length}, nowe: {fresh.length}</span>
    <button disabled={fresh.length === 0} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={items.length === 0} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
