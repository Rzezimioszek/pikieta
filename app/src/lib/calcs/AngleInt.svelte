<script lang="ts">
  import Icon from '../Icon.svelte'
  import type { AngleIntData } from '../calcs'
  import { angleIntersection, azimuthDistance } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import PointField from './PointField.svelte'
  import { coords, fa, fc, fl, num } from './util'

  interface Props {
    data: AngleIntData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  const a = $derived(findPoint(data.a))
  const b = $derived(findPoint(data.b))
  const alpha = $derived(num(data.alpha))
  const beta = $derived(num(data.beta))
  const id = $derived(data.id.trim() || nextId())

  const outcome = $derived.by(() => {
    if (!a || !b || alpha === null || beta === null) return null
    return angleIntersection(a, b, alpha, beta, data.side === 'left') ?? 'none'
  })
  const p = $derived(outcome && outcome !== 'none' ? outcome : null)
  /** Kąt przy punkcie wyznaczanym (γ = 200g − α − β) i długości boków trójkąta. */
  const gamma = $derived(alpha !== null && beta !== null ? 200 - alpha - beta : null)
  const sides = $derived(p && a && b ? { ap: azimuthDistance(a, p)?.distance, bp: azimuthDistance(b, p)?.distance } : null)

  $effect(() =>
    onsketch({
      paths: p ? [[a!, p, b!], [a!, b!]] : a && b ? [[a, b]] : [],
      results: p ? [{ ...p, label: id }] : [],
    }),
  )

  function save() {
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        [`Punkt A ${a!.id}`, coords(a!)],
        [`Punkt B ${b!.id}`, coords(b!)],
        ['Kąt α przy A', `${fa(alpha!)} g`],
        ['Kąt β przy B', `${fa(beta!)} g`],
        ['Strona względem A→B', data.side === 'left' ? 'lewa' : 'prawa'],
      ],
      results: [
        [`Punkt ${id}`, coords(p!)],
        ['Kąt γ przy punkcie', `${fa(gamma!)} g`],
        ['Długości A–P, B–P', `${fl(sides!.ap!)} m, ${fl(sides!.bp!)} m`],
      ],
      notes: gamma! < 30 || gamma! > 170 ? ['Kąt γ przy punkcie jest ostry lub rozwarty: wcięcie słabo wyznaczone.'] : [],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <PointField label="Punkt A" bind:value={data.a} />
    <PointField label="Punkt B" bind:value={data.b} />
    <label class="field">Kąt α przy A [g]<input inputmode="decimal" bind:value={data.alpha} /></label>
    <label class="field">Kąt β przy B [g]<input inputmode="decimal" bind:value={data.beta} /></label>
    <label class="field">
      Strona względem A→B
      <select bind:value={data.side}>
        <option value="left">lewa</option>
        <option value="right">prawa</option>
      </select>
    </label>
    <label class="field">Numer nowego punktu<input placeholder={nextId()} bind:value={data.id} /></label>
  </div>

  <p class="hint">α to kąt przy A między kierunkami na B i na punkt, β przy B między kierunkami na A i na punkt (kąty wewnętrzne trójkąta).</p>

  {#if outcome === 'none'}
    <p class="error" role="alert">Kąty nie wyznaczają punktu: oba muszą być dodatnie, a ich suma mniejsza niż 200g.</p>
  {:else if p}
    {#if gamma! < 30 || gamma! > 170}
      <p class="error" role="alert">Kąt przy punkcie wynosi {fa(gamma!)} g: wcięcie jest słabo wyznaczone.</p>
    {/if}
    <dl class="card result">
      <dt>X</dt><dd>{fc(p.x)}</dd>
      <dt>Y</dt><dd>{fc(p.y)}</dd>
      <dt>Kąt γ przy punkcie</dt><dd>{fa(gamma!)} g</dd>
      {#if sides}<dt>Długości A–P, B–P</dt><dd>{fl(sides.ap!)} / {fl(sides.bp!)} m</dd>{/if}
    </dl>
  {:else}
    <p class="hint">Uzupełnij punkty i kąty.</p>
  {/if}

  <div class="actions">
    <button
      disabled={!p || !!findPoint(id)}
      onclick={() => {
        data.id = id
        addNewPoints([{ id, ...p!, h: null, code: '' }])
      }}
    >
      <Icon name="pointAdd" /> Dodaj punkt {id}
    </button>
    <button class="primary" disabled={!p} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
