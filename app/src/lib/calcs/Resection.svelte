<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { ResectionData } from '../calcs'
  import { fmt, resection, type Pt } from '../core'
  import type { Sketch } from '../map'
  import { addNewPoints, addReport, findPoint, nextId } from '../project.svelte'
  import type { Column } from '../table'
  import { coords, fa, fc, fl, isBlank, keepTrailing, moveRow, num, pointState, reportTable } from './util'

  interface Props {
    data: ResectionData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Src = ResectionData['rows'][number]
  interface Item {
    src: Src
    p?: Pt
    residual?: number
    status: string
    tone?: 'error' | 'muted'
  }
  const empty = (): Src => ({ id: '', hz: '' })
  /** Wskaźnik bliskości okręgu niebezpiecznego, poniżej którego ostrzegamy. */
  const DANGER = 0.15

  const used = $derived(
    data.rows
      .filter((r) => !isBlank(r))
      .map((r) => ({ r, p: findPoint(r.id), hz: num(r.hz) }))
      .filter((x) => x.p && x.hz !== null),
  )
  const outcome = $derived.by(() => {
    if (used.length < 3) return null
    try {
      return { ok: resection(used.map((x) => ({ p: x.p!, reading: x.hz! }))) }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const res = $derived(outcome && 'ok' in outcome ? outcome.ok : null)
  const id = $derived(data.id.trim() || nextId())
  const state = $derived(res ? pointState(id, res.p) : null)

  const items = $derived(
    data.rows.map((src): Item => {
      if (isBlank(src)) return { src, status: '' }
      const p = findPoint(src.id)
      if (!p) return { src, status: src.id.trim() ? 'brak punktu' : 'niepełny', tone: src.id.trim() ? 'error' : 'muted' }
      if (num(src.hz) === null) return { src, p, status: src.hz.trim() ? 'błędna liczba' : 'brak kierunku', tone: 'muted' }
      const k = used.findIndex((x) => x.r === src)
      return { src, p, residual: res && k >= 0 ? res.residualsCc[k] : undefined, status: '' }
    }),
  )

  $effect(() =>
    onsketch({
      paths: res ? used.map((x) => [res.p, x.p!]) : [],
      results: res ? [{ ...res.p, label: id }] : [],
    }),
  )

  const edit = (key: keyof Src) => (it: Item, v: string) => {
    it.src[key] = v
    keepTrailing(data.rows, empty)
  }
  const columns: Column<Item>[] = [
    { key: 'id', label: 'Punkt znany', width: 110, edit: 'point', value: (i) => i.src.id, set: edit('id') },
    { key: 'hz', label: 'Kierunek [g]', width: 110, align: 'right', edit: 'number', value: (i) => i.src.hz, set: edit('hz') },
    { key: 'x', label: 'X', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.x) : '') },
    { key: 'y', label: 'Y', width: 116, align: 'right', value: (i) => (i.p ? fc(i.p.y) : '') },
    { key: 'v', label: 'Odchyłka [cc]', width: 108, align: 'right', value: (i) => (i.residual === undefined ? '' : fmt(i.residual, 1)) },
    { key: 'status', label: 'Stan', width: 110, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function add() {
    data.id = id
    addNewPoints([{ id, ...res!.p, h: null, code: '' }])
  }

  function save() {
    const r = res!
    addReport({
      title: `${title} (${crsLabel})`,
      table: reportTable(
        ['Punkt', 'X', 'Y', 'Kierunek [g]', 'Odchyłka [cc]'],
        used.map((x, k) => [x.p!.id, fc(x.p!.x), fc(x.p!.y), fa(x.hz!), fmt(r.residualsCc[k], 1)]),
      ),
      results: [
        [`Stanowisko ${id}`, coords(r.p)],
        ['Stała orientacji', `${fa(r.orientation)} g`],
        ...(r.m0cc !== null
          ? [
              ['Błąd średni kierunku m₀', `${fmt(r.m0cc, 1)} cc`] as [string, string],
              ['Błędy średnie mx, my', `${fl(r.mx!)} m, ${fl(r.my!)} m`] as [string, string],
            ]
          : []),
      ],
      notes: [
        used.length > 3
          ? 'Wyrównanie metodą najmniejszych kwadratów (niewiadome X, Y i stała orientacji).'
          : 'Rozwiązanie z trzech kierunków, bez kontroli.',
        ...(r.danger < DANGER ? ['Stanowisko blisko okręgu przez punkty znane: wynik słabo wyznaczony.'] : []),
      ],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">Numer stanowiska<input placeholder={nextId()} bind:value={data.id} /></label>
  </div>

  <p class="hint">
    Kierunki (odczyty koła poziomego) na co najmniej trzy punkty znane. Przy czterech i więcej wynik jest wyrównany, z
    odchyłkami i błędami średnimi.
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
    {#if res.danger < DANGER}
      <p class="error" role="alert">Stanowisko leży blisko okręgu przez punkty znane: wynik jest słabo wyznaczony.</p>
    {/if}
    <dl class="card result">
      <dt>X</dt><dd>{fc(res.p.x)}</dd>
      <dt>Y</dt><dd>{fc(res.p.y)}</dd>
      <dt>Stała orientacji</dt><dd>{fa(res.orientation)} g</dd>
      {#if res.m0cc !== null}
        <dt>Błąd średni kierunku m₀</dt><dd>{fmt(res.m0cc, 1)} cc</dd>
        <dt>Błędy średnie mx, my</dt><dd>{fl(res.mx!)} / {fl(res.my!)} m</dd>
      {:else}
        <dt>Kontrola</dt><dd class="muted">brak (trzy kierunki)</dd>
      {/if}
      <dt>Stan punktu {id}</dt><dd class="muted">{state?.status}</dd>
    </dl>
  {:else}
    <p class="hint">Podaj co najmniej trzy punkty znane z kierunkami.</p>
  {/if}

  <div class="actions">
    <button disabled={!res || state?.tone !== 'ok'} onclick={add}><Icon name="pointAdd" /> Dodaj do projektu</button>
    <button class="primary" disabled={!res} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
