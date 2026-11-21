<script lang="ts">
  import Icon from '../Icon.svelte'
  import type { LineIntData } from '../calcs'
  import { lineIntersection } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import PointField from './PointField.svelte'
  import { coords, fc } from './util'

  interface Props {
    data: LineIntData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  const pts = $derived([data.a, data.b, data.c, data.d].map((id) => findPoint(id)))
  const ready = $derived(pts.every(Boolean))
  const p = $derived(ready ? lineIntersection(pts[0]!, pts[1]!, pts[2]!, pts[3]!) : null)
  const id = $derived(data.id.trim() || nextId())

  $effect(() => {
    const [a, b, c, d] = pts
    onsketch({
      paths: [a && b ? (p ? [a, b, p] : [a, b]) : [], c && d ? (p ? [c, d, p] : [c, d]) : []],
      results: p ? [{ ...p, label: id }] : [],
    })
  })

  function save() {
    const [a, b, c, d] = pts
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        [`Prosta 1: początek ${a!.id}`, coords(a!)],
        [`Prosta 1: koniec ${b!.id}`, coords(b!)],
        [`Prosta 2: początek ${c!.id}`, coords(c!)],
        [`Prosta 2: koniec ${d!.id}`, coords(d!)],
      ],
      results: [[`Punkt ${id}`, coords(p!)]],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Prosta 1: początek" bind:value={data.a} />
    <PointField label="Prosta 1: koniec" bind:value={data.b} />
    <PointField label="Prosta 2: początek" bind:value={data.c} />
    <PointField label="Prosta 2: koniec" bind:value={data.d} />
    <label class="field">Numer nowego punktu<input placeholder={nextId()} bind:value={data.id} /></label>
  </div>

  {#if ready && !p}
    <p class="error" role="alert">Proste są równoległe albo punkty jednej prostej się pokrywają.</p>
  {:else if p}
    <dl class="card result">
      <dt>X</dt><dd>{fc(p.x)}</dd>
      <dt>Y</dt><dd>{fc(p.y)}</dd>
    </dl>
  {:else}
    <p class="hint">Uzupełnij cztery punkty.</p>
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
