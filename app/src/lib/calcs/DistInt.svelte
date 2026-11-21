<script lang="ts">
  import Icon from '../Icon.svelte'
  import type { DistIntData } from '../calcs'
  import { distanceIntersection } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import PointField from './PointField.svelte'
  import { coords, f2, f3, num } from './util'

  interface Props {
    data: DistIntData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  const a = $derived(findPoint(data.a))
  const b = $derived(findPoint(data.b))
  const ra = $derived(num(data.ra))
  const rb = $derived(num(data.rb))
  const id = $derived(data.id.trim() || nextId())

  const outcome = $derived.by(() => {
    if (!a || !b || ra === null || rb === null) return null
    const r = distanceIntersection(a, ra, b, rb)
    return r ? { p: r[data.side], other: r[data.side === 'left' ? 'right' : 'left'] } : 'none'
  })
  const p = $derived(outcome && outcome !== 'none' ? outcome.p : null)

  $effect(() =>
    onsketch({
      paths: p ? [[a!, p, b!]] : a && b ? [[a, b]] : [],
      results: p ? [{ ...p, label: id }] : [],
    }),
  )

  function save() {
    addReport(`${title} (${crsLabel})`, [
      `Punkt A ${a!.id}: ${coords(a!)}`,
      `Punkt B ${b!.id}: ${coords(b!)}`,
      `Długość od A: ${f3(ra!)} m, od B: ${f3(rb!)} m, strona ${data.side === 'left' ? 'lewa' : 'prawa'} względem A→B`,
      `Punkt ${id}: ${coords(p!)}`,
    ])
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Punkt A" bind:value={data.a} />
    <PointField label="Punkt B" bind:value={data.b} />
    <label class="field">Długość od A [m]<input inputmode="decimal" bind:value={data.ra} /></label>
    <label class="field">Długość od B [m]<input inputmode="decimal" bind:value={data.rb} /></label>
    <label class="field">
      Strona względem A→B
      <select bind:value={data.side}>
        <option value="left">lewa</option>
        <option value="right">prawa</option>
      </select>
    </label>
    <label class="field">Numer nowego punktu<input placeholder={nextId()} bind:value={data.id} /></label>
  </div>

  {#if outcome === 'none'}
    <p class="error" role="alert">Okręgi się nie przecinają albo punkty A i B się pokrywają.</p>
  {:else if p}
    <dl class="card result">
      <dt>X</dt><dd>{f2(p.x)}</dd>
      <dt>Y</dt><dd>{f2(p.y)}</dd>
      {#if outcome}
        <dt>Rozwiązanie po drugiej stronie</dt><dd class="muted">{coords(outcome.other)}</dd>
      {/if}
    </dl>
  {:else}
    <p class="hint">Uzupełnij punkty i długości.</p>
  {/if}

  <div class="actions">
    <button disabled={!p || !!findPoint(id)} onclick={() => {
        data.id = id
        addNewPoints([{ id, ...p!, h: null, code: '' }])
      }}>
      <Icon name="pointAdd" /> Dodaj punkt {id}
    </button>
    <button class="primary" disabled={!p} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
