<script lang="ts">
  import DataTable from '../DataTable.svelte'
  import Icon from '../Icon.svelte'
  import type { AreaAdjustData } from '../calcs'
  import { distributeArea, fmt } from '../core'
  import type { Sketch } from '../map'
  import { addReport } from '../project.svelte'
  import type { Column } from '../table'
  import { fha, fm2, isBlank, keepTrailing, moveRow, num, reportTable } from './util'

  interface Props {
    data: AreaAdjustData
    epsg: number
    title: string
    crsLabel: string
    onsketch: (s: Sketch) => void
  }
  let { data, title, crsLabel, onsketch }: Props = $props()

  type Row = AreaAdjustData['rows'][number]
  const empty = (): Row => ({ name: '', area: '' })
  const unit = $derived(Number(data.unit))
  /** Dokładność wyświetlania: tyle miejsc, ile ma jednostka zaokrąglenia. */
  const dec = $derived(Math.max(0, -Math.floor(Math.log10(unit) + 1e-9)))

  $effect(() => onsketch({ paths: [] }))

  const total = $derived(num(data.total))
  const parsed = $derived(
    data.rows.map((src) => {
      if (isBlank(src)) return { src, status: '' }
      const a = num(src.area)
      if (a === null || a < 0) return { src, status: 'błędne pole', tone: 'error' as const }
      return { src, a, status: '' }
    }),
  )
  const valid = $derived(parsed.filter((p) => p.a !== undefined) as { src: Row; a: number; status: string }[])
  const sum = $derived(valid.reduce((s, p) => s + p.a, 0))
  const deviation = $derived(total !== null ? total - sum : null)
  const limit = $derived(num(data.limit))
  const tooBig = $derived(limit !== null && deviation !== null && Math.abs(deviation) > limit)

  const outcome = $derived.by(() => {
    if (total === null || valid.length === 0 || parsed.some((p) => p.tone === 'error')) return null
    if (total <= 0) return { error: 'Pole działki musi być dodatnie.' }
    try {
      return { ok: distributeArea(total, valid.map((p) => p.a), unit) }
    } catch (e) {
      return { error: e instanceof Error ? e.message : String(e) }
    }
  })
  const adjusted = $derived(outcome && 'ok' in outcome ? outcome.ok : null)

  const items = $derived.by(() => {
    let k = 0
    return parsed.map((p) => {
      if (p.a === undefined) return { ...p, v: undefined, out: undefined }
      const out = adjusted?.[k++]
      return { ...p, out, v: out === undefined ? undefined : out - p.a }
    })
  })

  const f = (v: number | undefined) => (v === undefined ? '' : fmt(v, dec))
  const columns: Column<(typeof items)[number]>[] = [
    {
      key: 'name',
      label: 'Oznaczenie (użytek, część)',
      width: 170,
      edit: 'text',
      value: (i) => i.src.name,
      set: (i, v) => {
        i.src.name = v
        keepTrailing(data.rows, empty)
      },
    },
    {
      key: 'area',
      label: 'Pole obliczone [m²]',
      width: 140,
      align: 'right',
      edit: 'number',
      value: (i) => i.src.area,
      set: (i, v) => {
        i.src.area = v
        keepTrailing(data.rows, empty)
      },
    },
    { key: 'v', label: 'Poprawka [m²]', width: 110, align: 'right', value: (i) => (i.v === undefined ? '' : fm2(i.v)) },
    { key: 'out', label: 'Pole wyrównane [m²]', width: 150, align: 'right', value: (i) => f(i.out) },
    { key: 'ha', label: '[ha]', width: 90, align: 'right', value: (i) => (i.out === undefined ? '' : fha(i.out)) },
    { key: 'status', label: 'Uwagi', width: 100, value: (i) => i.status, tone: (i) => i.tone },
  ]

  function save() {
    addReport({
      title: `${title} (${crsLabel})`,
      params: [
        ['Pole działki', `${fm2(total!)} m² (${fha(total!)} ha)`],
        ['Zaokrąglenie', `${fmt(unit, dec)} m²`],
        ...(limit !== null ? [['Odchyłka dopuszczalna', `${fm2(limit)} m²`] as [string, string]] : []),
      ],
      table: reportTable(
        ['Oznaczenie', 'Pole obliczone [m²]', 'Poprawka [m²]', 'Pole wyrównane [m²]', 'Pole [ha]'],
        items
          .filter((i) => i.out !== undefined)
          .map((i) => [i.src.name.trim() || '—', fm2(i.a!), fm2(i.v!), f(i.out), fha(i.out!)]),
      ),
      results: [
        ['Suma pól obliczonych', `${fm2(sum)} m²`],
        ['Odchyłka', `${fm2(deviation!)} m² (${fmt((deviation! / total!) * 100, 2)} %)`],
      ],
      notes: tooBig ? ['Odchyłka przekracza dopuszczalną.'] : [],
    })
  }
</script>

<div class="calc">
  <div class="fields">
    <label class="field">Pole działki [m²]<input inputmode="decimal" bind:value={data.total} /></label>
    <label class="field">
      Zaokrąglenie
      <select bind:value={data.unit}>
        <option value="1">1 m² (0,0001 ha)</option>
        <option value="0.1">0,1 m²</option>
        <option value="0.01">0,01 m²</option>
      </select>
    </label>
    <label class="field">Odchyłka dopuszczalna [m²]<input inputmode="decimal" placeholder="bez kontroli" bind:value={data.limit} /></label>
  </div>

  <p class="hint">
    Odchyłka między polem działki a sumą pól części jest rozrzucana proporcjonalnie do pól; po zaokrągleniu suma
    wyrównanych pól jest równa polu działki.
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
  {:else if adjusted}
    {#if tooBig}<p class="error" role="alert">Odchyłka {fm2(deviation!)} m² przekracza dopuszczalną {fm2(limit!)} m².</p>{/if}
    <dl class="card result">
      <dt>Suma pól obliczonych</dt><dd>{fm2(sum)} m²</dd>
      <dt>Odchyłka</dt><dd>{fm2(deviation!)} m² <span class="muted">· {fmt((deviation! / total!) * 100, 2)} %</span></dd>
      <dt>Suma pól wyrównanych</dt><dd>{f(adjusted.reduce((s, v) => s + v, 0))} m²</dd>
    </dl>
  {:else}
    <p class="hint">Podaj pole działki i pola części.</p>
  {/if}

  <div class="actions">
    <button class="primary" disabled={!adjusted} onclick={save}><Icon name="save" /> Zapisz w raporcie</button>
  </div>
</div>
