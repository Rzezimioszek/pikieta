<script lang="ts" generics="R">
  import { tick } from 'svelte'
  import type { SvelteSet } from 'svelte/reactivity'
  import { picker } from './project.svelte'
  import type { Column } from './table'

  interface Props {
    columns: Column<R>[]
    rows: R[]
    rowKey: (row: R, index: number) => string
    /** Komórki z `edit` są polami do wpisywania. */
    editing?: boolean
    /** Zmiana trafia do danych po każdym znaku (obliczenia) zamiast po zatwierdzeniu (atrybuty). */
    live?: boolean
    sortable?: boolean
    selection?: SvelteSet<string>
    canDelete?: (row: R, index: number) => boolean
    ondelete?: (row: R, index: number) => void
    /** Zmiana kolejności wierszy; indeksy w tablicy `rows`. Niedostępna przy sortowaniu. */
    onmove?: (from: number, to: number) => void
    canMove?: (row: R, index: number) => boolean
    onactivate?: (row: R) => void
    empty?: string
    /** Wiersz zawsze na końcu (np. do wpisania nowego obiektu), poza sortowaniem i zaznaczaniem. */
    tail?: R
    /**
     * Zatwierdzenie wiersza końcowego: Enter (`explicit`) albo wyjście z wiersza.
     * Zwraca `true`, gdy wiersz został przyjęty (wtedy fokus wraca do nowego pustego wiersza).
     */
    ontailcommit?: (explicit: boolean) => boolean
    /** Wysokość tabeli dopasowana do treści, ale nie większa niż podana (CSS). */
    maxHeight?: string
  }

  let {
    columns,
    rows,
    rowKey,
    editing = false,
    live = false,
    sortable = false,
    selection,
    canDelete,
    ondelete,
    onmove,
    canMove,
    onactivate,
    empty = 'Brak danych.',
    tail,
    ontailcommit,
    maxHeight,
  }: Props = $props()

  const ROW_H = 30
  const OVERSCAN = 8

  let wrap = $state<HTMLDivElement>()
  let scrollTop = $state(0)
  let viewport = $state(400)
  let sort = $state<{ key: string; dir: 1 | -1 } | null>(null)
  let widths = $state<Record<string, number>>({})
  let anchor: number | null = null
  let drag = $state<{ from: number; to: number } | null>(null)

  const width = (c: Column<R>) => widths[c.key] ?? c.width
  const totalWidth = $derived(52 + columns.reduce((s, c) => s + width(c), 0))

  const collator = new Intl.Collator('pl', { numeric: true, sensitivity: 'base' })
  function compare(a: string, b: string) {
    const [na, nb] = [Number(a), Number(b)]
    if (a !== '' && b !== '' && !Number.isNaN(na) && !Number.isNaN(nb)) return na - nb
    return collator.compare(a, b)
  }

  const items = $derived.by(() => {
    const list = rows.map((row, index) => ({ row, index, key: rowKey(row, index) }))
    const col = sort && columns.find((c) => c.key === sort!.key)
    if (col) list.sort((a, b) => compare(col.value(a.row), col.value(b.row)) * sort!.dir)
    if (tail) list.push({ row: tail, index: rows.length, key: '#tail' })
    return list
  })

  const start = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN))
  const end = $derived(Math.min(items.length, Math.ceil((scrollTop + viewport) / ROW_H) + OVERSCAN))
  const visible = $derived(items.slice(start, end).map((it, i) => ({ ...it, pos: start + i })))

  function toggleSort(key: string) {
    if (!sortable) return
    if (sort?.key !== key) sort = { key, dir: 1 }
    else sort = sort.dir === 1 ? { key, dir: -1 } : null
  }

  function startResize(e: PointerEvent, c: Column<R>) {
    e.preventDefault()
    e.stopPropagation()
    const [x0, w0] = [e.clientX, width(c)]
    const move = (ev: PointerEvent) => (widths[c.key] = Math.max(48, Math.round(w0 + ev.clientX - x0)))
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', () => window.removeEventListener('pointermove', move), { once: true })
  }

  function select(pos: number, key: string, e: MouseEvent) {
    if (key === '#tail') return
    if (!selection || (e.target as HTMLElement).closest('input, button')) return
    if (e.shiftKey && anchor !== null) {
      if (!(e.ctrlKey || e.metaKey)) selection.clear()
      const [a, b] = [Math.min(anchor, pos), Math.max(anchor, pos)]
      for (let i = a; i <= b; i++) if (items[i].key !== '#tail') selection.add(items[i].key)
      return
    }
    if (e.ctrlKey || e.metaKey) {
      if (!selection.delete(key)) selection.add(key)
    } else {
      selection.clear()
      selection.add(key)
    }
    anchor = pos
  }

  /** Ustawia fokus w pierwszej komórce wiersza końcowego. */
  export function focusNew() {
    if (tail) focusTail(0)
  }

  /** Przewija tabelę tak, żeby wiersz był widoczny. */
  export function reveal(key: string) {
    const pos = items.findIndex((it) => it.key === key)
    if (pos < 0 || !wrap) return
    const top = pos * ROW_H
    const header = ROW_H
    if (top < wrap.scrollTop) wrap.scrollTop = top
    else if (top + ROW_H > wrap.scrollTop + wrap.clientHeight - header) wrap.scrollTop = top + ROW_H * 2 - wrap.clientHeight
  }

  async function focusCell(pos: number, ci: number) {
    await tick()
    if (pos < 0 || pos >= items.length || !wrap) return
    reveal(items[pos].key)
    await tick()
    const el = wrap.querySelector<HTMLInputElement>(`input[data-r="${pos}"][data-c="${ci}"]`)
    el?.focus()
    el?.select()
  }

  /** Przenosi wiersz z pozycji `from` na `to` (pozycje w kolejności wyświetlania). */
  function moveRow(from: number, to: number) {
    if (!onmove || from === to || to < 0 || to >= items.length) return false
    if (!(canMove?.(items[to].row, items[to].index) ?? true)) return false
    onmove(items[from].index, items[to].index)
    return true
  }

  /** Przeciąganie za kolumnę numerów: działa myszą i palcem (zdarzenia wskaźnika). */
  function startDrag(e: PointerEvent, pos: number) {
    if ((e.target as HTMLElement).closest('button') || !wrap) return
    e.preventDefault()
    const body = wrap.querySelector('tbody')!
    drag = { from: pos, to: pos }
    const move = (ev: PointerEvent) => {
      const r = wrap!.getBoundingClientRect()
      // Przy krawędzi tabela przewija się sama.
      if (ev.clientY < r.top + ROW_H * 1.5) wrap!.scrollTop -= ROW_H / 2
      else if (ev.clientY > r.bottom - ROW_H / 2) wrap!.scrollTop += ROW_H / 2
      const at = Math.floor((ev.clientY - body.getBoundingClientRect().top) / ROW_H)
      drag = { from: pos, to: Math.max(0, Math.min(items.length - 1, at)) }
    }
    const up = () => {
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointercancel', up)
      if (drag) moveRow(drag.from, drag.to)
      drag = null
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up, { once: true })
    window.addEventListener('pointercancel', up, { once: true })
  }

  async function focusTail(ci: number) {
    await tick()
    // Szkic wiersza zmienił się na pusty w jednym kroku, więc Svelte nie widzi zmiany wartości pól.
    const pos = items.length - 1
    for (const input of wrap?.querySelectorAll<HTMLInputElement>(`input[data-r="${pos}"]`) ?? []) {
      input.value = tail ? columns[Number(input.dataset.c)].value(tail) : ''
    }
    focusCell(pos, ci)
  }

  /**
   * Zatwierdza wartość komórki od razu (bez czekania na zdarzenie `change` przy utracie fokusu,
   * które przyszłoby już po przejściu do innego wiersza).
   */
  function commitNow(input: HTMLInputElement, c: Column<R>, row: R) {
    if (live || input.value === c.value(row)) return
    c.set?.(row, input.value)
    // Przypisanie wartości ustala ją jako zatwierdzoną; przeglądarka nie wyśle już `change`.
    input.value = c.value(row)
  }

  function onKey(e: KeyboardEvent, pos: number, ci: number, c: Column<R>, row: R) {
    if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      e.preventDefault()
      const to = pos + (e.key === 'ArrowUp' ? -1 : 1)
      if (moveRow(pos, to)) focusCell(to, ci)
    } else if (e.key === 'Enter' || e.key === 'ArrowDown') {
      e.preventDefault()
      commitNow(e.target as HTMLInputElement, c, row)
      if (items[pos]?.key === '#tail') {
        if (e.key === 'Enter' && ontailcommit?.(true)) focusTail(0)
        return
      }
      focusCell(pos + 1, ci)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      focusCell(pos - 1, ci)
    } else if (e.key === 'Escape') {
      ;(e.currentTarget as HTMLInputElement).blur()
    }
  }

  function arm(c: Column<R>, row: R, pos: number, ci: number) {
    if (c.edit !== 'point') return
    picker.label = c.label
    picker.fill = (id) => {
      c.set?.(row, id)
      focusCell(pos + 1, ci)
    }
  }

  async function commit(e: Event, c: Column<R>, row: R) {
    const input = e.currentTarget as HTMLInputElement
    c.set?.(row, input.value)
    if (live) return
    // Odrzucona zmiana (np. litera w polu liczbowym) wraca do zapisanej wartości.
    await tick()
    input.value = c.value(row)
  }
</script>

<div
  class="wrap"
  style:max-height={maxHeight}
  bind:this={wrap}
  bind:clientHeight={viewport}
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  <table style:width={`${totalWidth}px`}>
    <colgroup>
      <col style:width="52px" />
      {#each columns as c (c.key)}<col style:width={`${width(c)}px`} />{/each}
    </colgroup>
    <thead>
      <tr>
        <th class="rn"><span class="sr-only">Lp.</span></th>
        {#each columns as c (c.key)}
          <th
            class:right={c.align === 'right'}
            aria-sort={sort?.key === c.key ? (sort.dir === 1 ? 'ascending' : 'descending') : undefined}
          >
            {#if sortable}
              <button class="head" onclick={() => toggleSort(c.key)}>
                {c.label}{#if sort?.key === c.key}<span class="arrow">{sort.dir === 1 ? '▲' : '▼'}</span>{/if}
              </button>
            {:else}
              <span class="head">{c.label}</span>
            {/if}
            <span class="grip" role="presentation" onpointerdown={(e) => startResize(e, c)}></span>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if start > 0}<tr class="spacer" style:height={`${start * ROW_H}px`}><td colspan={columns.length + 1}></td></tr>{/if}
      {#each visible as it (it.key)}
        {@const movable = !!onmove && !sort && (canMove?.(it.row, it.index) ?? true)}
        <tr
          class:selected={selection?.has(it.key)}
          class:dragging={drag?.from === it.pos}
          class:drop-above={drag && drag.to === it.pos && drag.to < drag.from}
          class:drop-below={drag && drag.to === it.pos && drag.to > drag.from}
          style:height={`${ROW_H}px`}
          onclick={(e) => select(it.pos, it.key, e)}
          class:tail={it.key === '#tail'}
          ondblclick={(e) => !(e.target as HTMLElement).closest('input') && it.key !== '#tail' && onactivate?.(it.row)}
          onfocusout={it.key === '#tail'
            ? (e) => !e.currentTarget.contains(e.relatedTarget as Node) && ontailcommit?.(false)
            : undefined}
        >
          <td
            class="rn"
            class:movable
            title={movable ? 'Przeciągnij, żeby zmienić kolejność (albo Alt+↑/↓ w komórce)' : undefined}
            onpointerdown={movable ? (e) => startDrag(e, it.pos) : undefined}
          >
            <span>{it.pos + 1}</span>
            {#if ondelete && (canDelete?.(it.row, it.index) ?? true)}
              <button class="del" aria-label={`Usuń wiersz ${it.pos + 1}`} onclick={() => ondelete(it.row, it.index)}>×</button>
            {/if}
          </td>
          {#each columns as c, ci (c.key)}
            {@const tone = c.tone?.(it.row)}
            <td class:right={c.align === 'right'} class={tone}>
              {#if c.hidden?.(it.row)}
                <span class="cell"></span>
              {:else if c.edit === 'check' && c.set}
                <input
                  type="checkbox"
                  class="check"
                  aria-label={`${c.label}, wiersz ${it.pos + 1}`}
                  checked={c.value(it.row) === '1'}
                  disabled={!editing}
                  onchange={(e) => c.set?.(it.row, e.currentTarget.checked ? '1' : '')}
                />
              {:else if editing && c.edit && c.set}
                <input
                  data-r={it.pos}
                  data-c={ci}
                  data-pick={c.edit === 'point' ? '' : undefined}
                  list={c.edit === 'point' ? 'point-ids' : undefined}
                  inputmode={c.edit === 'number' ? 'decimal' : undefined}
                  aria-label={`${c.label}, wiersz ${it.pos + 1}`}
                  value={c.value(it.row)}
                  placeholder={c.placeholder?.(it.row)}
                  oninput={live ? (e) => commit(e, c, it.row) : undefined}
                  onchange={live ? undefined : (e) => commit(e, c, it.row)}
                  onkeydown={(e) => onKey(e, it.pos, ci, c, it.row)}
                  onfocus={() => arm(c, it.row, it.pos, ci)}
                />
              {:else}
                <span class="cell" title={c.value(it.row)}>{c.value(it.row)}</span>
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
      {#if end < items.length}
        <tr class="spacer" style:height={`${(items.length - end) * ROW_H}px`}><td colspan={columns.length + 1}></td></tr>
      {/if}
    </tbody>
  </table>
  {#if items.length === 0}<p class="empty">{empty}</p>{/if}
</div>

<style>
  .wrap {
    position: relative;
    overflow: auto;
    min-height: 0;
    border-radius: 14px;
    border: 1px solid var(--field-edge);
    background: var(--table-bg);
    scrollbar-width: thin;
  }
  table {
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
    min-width: 100%;
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
  }
  th {
    position: sticky;
    top: 0;
    z-index: 2;
    height: 30px;
    padding: 0;
    background: var(--table-head);
    border-bottom: 1px solid var(--field-edge);
    border-right: 1px solid var(--grid-line);
    font-weight: 600;
    color: var(--ink-soft);
    text-align: left;
  }
  th.right {
    text-align: right;
  }
  .head {
    display: block;
    width: 100%;
    height: 100%;
    padding: 0 10px;
    border: none;
    border-radius: 0;
    background: none;
    font: inherit;
    font-weight: 600;
    color: inherit;
    text-align: inherit;
    line-height: 30px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.head:hover {
    background: color-mix(in srgb, var(--ink) 8%, transparent);
  }
  .arrow {
    margin-left: 4px;
    font-size: 0.65rem;
  }
  .grip {
    position: absolute;
    top: 0;
    right: 0;
    width: 8px;
    height: 100%;
    cursor: col-resize;
    z-index: 3;
  }
  td {
    height: 30px;
    padding: 0 10px;
    border-bottom: 1px solid var(--grid-line);
    border-right: 1px solid var(--grid-line);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  td.right {
    text-align: right;
  }
  .cell {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  tbody tr:nth-child(even of :not(.spacer)) td {
    background: var(--table-stripe);
  }
  tbody tr:hover td {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  tbody tr.selected td {
    background: var(--select-bg);
  }
  /* Kolumna numerów jest przyklejona do lewej, więc jej tło musi być kryjące. */
  tbody tr td.rn {
    background: var(--table-head);
  }
  tbody tr.selected td.rn {
    background: var(--focus);
    color: #fff;
  }
  .rn {
    position: sticky;
    left: 0;
    z-index: 1;
    padding: 0 6px;
    text-align: right;
    color: var(--ink-soft);
    background: var(--table-head);
    font-size: 0.74rem;
    cursor: default;
  }
  th.rn {
    z-index: 3;
  }
  .rn.movable {
    cursor: grab;
    touch-action: none;
  }
  .rn.movable::before {
    content: '⋮⋮';
    float: left;
    letter-spacing: -2px;
    opacity: 0.35;
  }
  .del {
    position: absolute;
    top: 4px;
    right: 3px;
    display: none;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: var(--danger);
    color: #fff;
    font-size: 0.9rem;
    line-height: 1;
  }
  tr:hover .del {
    display: block;
  }
  tr:hover .rn > span:has(+ .del) {
    visibility: hidden;
  }
  tr.dragging td {
    opacity: 0.45;
  }
  tr.drop-above td {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  tr.drop-below td {
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  td:has(input) {
    padding: 0;
  }
  td input {
    width: 100%;
    height: 29px;
    padding: 0 10px;
    border: none;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    font-size: inherit;
    text-align: inherit;
  }
  td input:focus-visible {
    box-shadow: inset 0 0 0 2px var(--focus);
    background: var(--field-bg);
  }
  td input::placeholder {
    font-style: italic;
  }
  td input.check {
    display: block;
    width: 16px;
    height: 16px;
    margin: 0 auto;
    padding: 0;
    accent-color: var(--accent, currentColor);
  }
  td.error {
    color: var(--danger);
  }
  td.muted {
    color: var(--ink-soft);
  }
  td.ok {
    color: var(--ok);
    font-weight: 600;
  }
  tbody tr.tail td {
    background: color-mix(in srgb, var(--accent) 7%, transparent);
  }
  .spacer td {
    padding: 0;
    border: none;
    background: none !important;
  }
  .empty {
    padding: 18px 12px;
    font-size: 0.85rem;
    color: var(--ink-soft);
    text-align: center;
  }
</style>
