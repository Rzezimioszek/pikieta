<script lang="ts">
  import * as core from './core'
  import type { Pt, XY } from './core'
  import type { Sketch } from './map'
  import { addPoints, findPoint, nextId, project } from './project.svelte'

  interface Props {
    ready: boolean
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { ready, crsLabel, onsketch }: Props = $props()

  type PointKey = 'a' | 'b' | 'c' | 'd'
  type NumKey = 'angle' | 'dist' | 'along' | 'offset' | 'ra' | 'rb'
  type TaskId = 'azimuth' | 'polar' | 'offset' | 'fromOffset' | 'distInt' | 'lineInt' | 'area'

  interface Task {
    id: TaskId
    label: string
    points: [PointKey, string][]
    nums: [NumKey, string][]
  }

  const TASKS: Task[] = [
    { id: 'azimuth', label: 'Azymut i długość', points: [['a', 'Punkt A'], ['b', 'Punkt B']], nums: [] },
    {
      id: 'polar',
      label: 'Metoda biegunowa',
      points: [['a', 'Stanowisko'], ['b', 'Nawiązanie']],
      nums: [['angle', 'Kąt od nawiązania [g]'], ['dist', 'Długość [m]']],
    },
    {
      id: 'offset',
      label: 'Rzutowanie na linię (domiary)',
      points: [['a', 'Początek linii'], ['b', 'Koniec linii'], ['c', 'Punkt rzutowany']],
      nums: [],
    },
    {
      id: 'fromOffset',
      label: 'Punkt z domiarów',
      points: [['a', 'Początek linii'], ['b', 'Koniec linii']],
      nums: [['along', 'Miara bieżąca [m]'], ['offset', 'Domiar [m], + w prawo']],
    },
    {
      id: 'distInt',
      label: 'Wcięcie liniowe',
      points: [['a', 'Punkt A'], ['b', 'Punkt B']],
      nums: [['ra', 'Długość od A [m]'], ['rb', 'Długość od B [m]']],
    },
    {
      id: 'lineInt',
      label: 'Przecięcie prostych',
      points: [['a', 'Prosta 1: początek'], ['b', 'Prosta 1: koniec'], ['c', 'Prosta 2: początek'], ['d', 'Prosta 2: koniec']],
      nums: [],
    },
    { id: 'area', label: 'Pole ze współrzędnych', points: [], nums: [] },
  ]

  let taskId = $state<TaskId>('azimuth')
  let ids = $state<Record<PointKey, string>>({ a: '', b: '', c: '', d: '' })
  let nums = $state<Record<NumKey, string>>({ angle: '', dist: '', along: '', offset: '', ra: '', rb: '' })
  let side = $state<'left' | 'right'>('left')
  let areaIds = $state('')
  let newId = $state('')
  let active = $state<PointKey>('a')
  let note = $state('')

  const task = $derived(TASKS.find((t) => t.id === taskId)!)

  const f2 = (v: number) => v.toFixed(2)
  const f3 = (v: number) => v.toFixed(3)
  const f4 = (v: number) => v.toFixed(4)
  const coords = (p: XY) => `X ${f2(p.x)}  Y ${f2(p.y)}`
  const turn = (g: number) => ((g % 400) + 400) % 400

  function parse(s: string): number | null {
    const t = s.trim().replace(',', '.')
    const v = Number(t)
    return t !== '' && Number.isFinite(v) ? v : null
  }

  interface Outcome {
    rows: [string, string][]
    result: XY | null
    sketch: Sketch
    error: string
    inputs: string[]
  }

  const outcome = $derived.by((): Outcome => {
    const out: Outcome = { rows: [], result: null, sketch: { paths: [] }, error: '', inputs: [] }
    if (!ready) return out
    const epsg = core.epsgOf(project.crs)

    if (task.id === 'area') {
      const pts: Pt[] = []
      for (const token of areaIds.split(/[\s,;]+/).filter(Boolean)) {
        const p = findPoint(token)
        if (!p) {
          out.error = `Nie ma punktu ${token}.`
          return out
        }
        pts.push(p)
      }
      out.sketch = { paths: [pts], closed: true }
      if (pts.length < 3) return out
      const a = core.polygonArea(epsg, pts)
      if (!a) return out
      out.inputs = [`Wierzchołki: ${pts.map((p) => p.id).join(', ')}`, ...pts.map((p) => `${p.id}: ${coords(p)}`)]
      out.rows = [
        ['Pole ze współrzędnych', `${f2(a.grid)} m²`],
        ['Pole w hektarach', `${(a.grid / 10000).toFixed(4)} ha`],
        ['Pole na elipsoidzie', `${f2(a.ellipsoidal)} m²`],
      ]
      return out
    }

    const P = {} as Record<PointKey, Pt>
    const N = {} as Record<NumKey, number>
    let complete = true
    for (const [key, label] of task.points) {
      const raw = ids[key].trim()
      if (raw === '') {
        complete = false
        continue
      }
      const p = findPoint(raw)
      if (!p) {
        out.error = `Nie ma punktu ${raw}.`
        return out
      }
      P[key] = p
      out.inputs.push(`${label} ${p.id}: ${coords(p)}`)
    }
    for (const [key, label] of task.nums) {
      if (nums[key].trim() === '') {
        complete = false
        continue
      }
      const v = parse(nums[key])
      if (v === null) {
        out.error = `${label}: „${nums[key]}” nie jest liczbą.`
        return out
      }
      N[key] = v
      out.inputs.push(`${label}: ${nums[key].trim().replace(',', '.')}`)
    }
    const { a, b, c, d } = P
    out.sketch = { paths: [[a, b].filter(Boolean), [c, d].filter(Boolean)] }
    if (!complete) return out

    const made = (p: XY) => {
      out.result = p
      out.rows.push(['X', f2(p.x)], ['Y', f2(p.y)])
      out.sketch.result = p
    }

    switch (task.id) {
      case 'azimuth': {
        const r = core.azimuthDistance(a, b)
        if (!r) {
          out.error = 'Punkty się pokrywają.'
          break
        }
        const red = core.lineReduction(epsg, a, b)
        out.rows = [
          ['Azymut A→B', `${f4(r.azimuth)} g`],
          ['Długość', `${f3(r.distance)} m`],
        ]
        if (red) {
          out.rows.push(
            ['Skala odcinka', red.scale.toFixed(8)],
            ['Długość na elipsoidzie', `${f3(red.ellipsoidal)} m`],
            ['Zbieżność południków w A', `${f4(red.convergence)} g`],
          )
        }
        break
      }
      case 'polar': {
        const r = core.azimuthDistance(a, b)
        if (!r) {
          out.error = 'Stanowisko i nawiązanie się pokrywają.'
          break
        }
        const az = turn(r.azimuth + N.angle)
        out.rows = [
          ['Azymut nawiązania', `${f4(r.azimuth)} g`],
          ['Azymut na punkt', `${f4(az)} g`],
        ]
        const p = core.polar(a, az, N.dist)
        out.sketch.paths = [[b, a, p]]
        made(p)
        break
      }
      case 'offset': {
        const m = core.lineOffset(a, b, c)
        if (!m) {
          out.error = 'Początek i koniec linii się pokrywają.'
          break
        }
        const foot = core.pointFromOffset(a, b, m.along, 0)
        out.sketch.paths = [[a, b], foot ? [foot, c] : []]
        out.rows = [
          ['Miara bieżąca', `${f3(m.along)} m`],
          ['Domiar', `${f3(Math.abs(m.offset))} m ${m.offset >= 0 ? 'w prawo' : 'w lewo'}`],
        ]
        break
      }
      case 'fromOffset': {
        const p = core.pointFromOffset(a, b, N.along, N.offset)
        if (!p) {
          out.error = 'Początek i koniec linii się pokrywają.'
          break
        }
        const foot = core.pointFromOffset(a, b, N.along, 0)
        out.sketch.paths = [[a, b], foot ? [foot, p] : []]
        made(p)
        break
      }
      case 'distInt': {
        const r = core.distanceIntersection(a, N.ra, b, N.rb)
        if (!r) {
          out.error = 'Okręgi się nie przecinają albo punkty się pokrywają.'
          break
        }
        const p = r[side]
        out.inputs.push(`Strona: ${side === 'left' ? 'lewa' : 'prawa'} względem A→B`)
        out.sketch.paths = [[a, p, b]]
        made(p)
        break
      }
      case 'lineInt': {
        const p = core.lineIntersection(a, b, c, d)
        if (!p) {
          out.error = 'Proste są równoległe albo punkty prostej się pokrywają.'
          break
        }
        made(p)
        break
      }
    }
    return out
  })

  $effect(() => onsketch(outcome.sketch))

  /** Wstawia kliknięty na mapie punkt do aktywnego pola i przechodzi do następnego. */
  export function pick(id: string) {
    note = ''
    if (task.id === 'area') {
      areaIds = `${areaIds.trim()} ${id}`.trim()
      return
    }
    const keys = task.points.map(([k]) => k)
    const target = keys.includes(active) ? active : keys[0]
    ids[target] = id
    active = keys[(keys.indexOf(target) + 1) % keys.length]
  }

  function save() {
    if (outcome.error || outcome.rows.length === 0) return
    const lines = [...outcome.inputs, ...outcome.rows.map(([k, v]) => `${k}: ${v}`)]
    let added = ''
    if (outcome.result) {
      const id = newId.trim() || nextId()
      if (findPoint(id)) {
        note = `Punkt ${id} już istnieje. Podaj inny numer.`
        return
      }
      addPoints([{ id, ...outcome.result, h: null, code: '' }])
      lines.push(`Nowy punkt: ${id}`)
      added = ` Dodano punkt ${id}.`
      newId = ''
    }
    project.report.push({ title: `${task.label} (${crsLabel})`, lines })
    note = `Zapisano w raporcie jako pozycję ${project.report.length}.${added}`
  }
</script>

<label class="field">
  Zadanie
  <select bind:value={taskId} onchange={() => (note = '')}>
    {#each TASKS as t (t.id)}
      <option value={t.id}>{t.label}</option>
    {/each}
  </select>
</label>

<datalist id="point-ids">
  {#each project.points.slice(0, 1000) as p (p.id)}
    <option value={p.id}></option>
  {/each}
</datalist>

{#if task.id === 'area'}
  <label class="field">
    Numery wierzchołków po kolei
    <input placeholder="np. 1 2 3 4" bind:value={areaIds} />
  </label>
{:else}
  <div class="fields">
    {#each task.points as [key, label] (key)}
      <label class="field" class:active={active === key}>
        {label}
        <input list="point-ids" placeholder="nr" bind:value={ids[key]} onfocus={() => (active = key)} />
      </label>
    {/each}
    {#each task.nums as [key, label] (key)}
      <label class="field">
        {label}
        <input inputmode="decimal" bind:value={nums[key]} />
      </label>
    {/each}
    {#if task.id === 'distInt'}
      <label class="field">
        Strona względem A→B
        <select bind:value={side}>
          <option value="left">lewa</option>
          <option value="right">prawa</option>
        </select>
      </label>
    {/if}
  </div>
{/if}

<p class="hint">Punkt można też wskazać kliknięciem na mapie.</p>

{#if outcome.error}
  <p class="error" role="alert">{outcome.error}</p>
{/if}

{#if outcome.rows.length}
  <dl class="card">
    {#each outcome.rows as [k, v] (k)}
      <dt>{k}</dt>
      <dd>{v}</dd>
    {/each}
  </dl>

  <div class="save">
    {#if outcome.result}
      <input aria-label="Numer nowego punktu" placeholder={`Nr nowego (${nextId()})`} bind:value={newId} />
    {/if}
    <button class="primary" onclick={save} disabled={!!outcome.error}>
      {outcome.result ? 'Dodaj punkt i zapisz' : 'Zapisz w raporcie'}
    </button>
  </div>
{/if}

{#if note && !outcome.error}<p class="hint" role="status">{note}</p>{/if}

<style>
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .active input {
    border-color: var(--accent);
  }
  dl {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px 12px;
    font-size: 0.9rem;
  }
  dt {
    color: var(--ink-soft);
  }
  dd {
    text-align: right;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .save {
    display: flex;
    gap: 8px;
  }
  .save button {
    flex: 1;
  }
</style>
