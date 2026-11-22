<script lang="ts">
  import DataTable from './DataTable.svelte'
  import Icon from './Icon.svelte'
  import type { Pt } from './core'
  import { fc, num } from './calcs/util'
  import {
    addPoints,
    findPoint,
    nextId,
    notify,
    project,
    removePoints,
    renamePoint,
    selection,
  } from './project.svelte'
  import type { Column } from './table'

  interface Props {
    onzoom: (points: Pt[]) => void
    onclose: () => void
  }
  let { onzoom, onclose }: Props = $props()

  let table = $state<ReturnType<typeof DataTable<Pt>>>()
  /** Tabela pozwala od razu wpisywać i poprawiać punkty; przycisk ołówka to blokada edycji. */
  let editing = $state(true)
  let filter = $state('')
  let onlySelected = $state(false)
  let draft = $state({ id: '', x: '', y: '', h: '', code: '' })

  const rows = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    return project.points.filter(
      (p) =>
        (!onlySelected || selection.has(p.id)) &&
        (q === '' || p.id.toLowerCase().includes(q) || p.code.toLowerCase().includes(q)),
    )
  })

  function setNumber(label: string, apply: (v: number | null) => void, optional = false) {
    return (raw: string) => {
      const v = num(raw)
      if (v === null && !(optional && raw.trim() === '')) notify(`${label}: „${raw}” nie jest liczbą.`, 'error')
      else apply(v)
    }
  }

  /** Wiersz na końcu tabeli do wpisania nowego punktu. */
  const NEW: Pt = { id: '', x: NaN, y: NaN, h: null, code: '' }
  const isNew = (p: Pt) => p === NEW
  type DraftKey = keyof typeof draft

  /** Kolumna punktu: istniejący punkt zmienia się od razu, nowy trafia do szkicu wiersza. */
  function col(key: DraftKey, label: string, width: number, edit: 'text' | 'number', value: (p: Pt) => string, set: (p: Pt, v: string) => void, align?: 'right'): Column<Pt> {
    return {
      key,
      label,
      width,
      align,
      edit,
      value: (p) => (isNew(p) ? draft[key] : value(p)),
      set: (p, v) => (isNew(p) ? (draft[key] = v) : set(p, v)),
      placeholder: (p) => (isNew(p) ? (key === 'id' ? `nowy (${nextId()})` : key === 'x' || key === 'y' ? label : '') : ''),
    }
  }

  const columns: Column<Pt>[] = [
    col('id', 'Nr', 96, 'text', (p) => p.id, (p, v) => renamePoint(p.id, v)),
    col('x', 'X', 128, 'number', (p) => fc(p.x), (p, v) => setNumber('X', (n) => (p.x = n!))(v), 'right'),
    col('y', 'Y', 128, 'number', (p) => fc(p.y), (p, v) => setNumber('Y', (n) => (p.y = n!))(v), 'right'),
    col('h', 'H', 90, 'number', (p) => (p.h === null ? '' : fc(p.h)), (p, v) => setNumber('H', (n) => (p.h = n), true)(v), 'right'),
    col('code', 'Kod', 160, 'text', (p) => p.code, (p, v) => (p.code = v.trim())),
  ]

  /** Dodaje punkt z wiersza końcowego, gdy ma liczbowe X i Y. */
  function commitNew(explicit: boolean): boolean {
    const filled = Object.values(draft).some((v) => v.trim() !== '')
    if (!filled) return false
    const [x, y, h] = [num(draft.x), num(draft.y), num(draft.h)]
    if (x === null || y === null) {
      if (explicit) notify('Nowy punkt: podaj liczbowe X i Y.', 'error')
      return false
    }
    if (draft.h.trim() !== '' && h === null) {
      notify('Nowy punkt: H nie jest liczbą.', 'error')
      return false
    }
    const id = draft.id.trim() || nextId()
    if (findPoint(id)) {
      notify(`Punkt ${id} już istnieje.`, 'error')
      return false
    }
    addPoints([{ id, x, y, h, code: draft.code.trim() }])
    draft = { id: '', x: '', y: '', h: '', code: '' }
    return true
  }

  const selectedPoints = () => project.points.filter((p) => selection.has(p.id))

  function remove() {
    const n = selection.size
    if (n === 0) return
    if (n > 1 && !confirm(`Usunąć zaznaczone punkty (${n})?`)) return
    removePoints([...selection])
  }

  function invert() {
    for (const p of project.points) if (!selection.delete(p.id)) selection.add(p.id)
  }

  export function reveal(id: string) {
    table?.reveal(id)
  }
</script>

<div class="pt">
  <div class="bar">
    <strong class="name"><Icon name="table" /> Punkty</strong>
    <div class="tools">
      <button
        class="ib"
        class:on={editing}
        title={editing ? 'Edycja włączona (kliknij, aby zablokować)' : 'Edycja zablokowana (kliknij, aby włączyć)'}
        aria-pressed={editing}
        onclick={() => (editing = !editing)}
      >
        <Icon name="pencil" />
      </button>
      <button class="ib" title="Nowy punkt: wpisz w ostatnim wierszu tabeli" disabled={!editing} onclick={() => table?.focusNew()}>
        <Icon name="plus" />
      </button>
      <button class="ib" title="Usuń zaznaczone" disabled={selection.size === 0} onclick={remove}>
        <Icon name="trash" />
      </button>
      <span class="sep"></span>
      <button class="ib" title="Zaznacz wszystkie" onclick={() => project.points.forEach((p) => selection.add(p.id))}>
        <Icon name="selectAll" />
      </button>
      <button class="ib" title="Odwróć zaznaczenie" onclick={invert}><Icon name="invert" /></button>
      <button class="ib" title="Odznacz wszystkie" disabled={selection.size === 0} onclick={() => selection.clear()}>
        <Icon name="deselect" />
      </button>
      <span class="sep"></span>
      <button
        class="ib"
        title="Przybliż do zaznaczonych"
        disabled={selection.size === 0}
        onclick={() => onzoom(selectedPoints())}
      >
        <Icon name="fit" />
      </button>
      <button
        class="ib"
        class:on={onlySelected}
        title="Pokaż tylko zaznaczone"
        aria-pressed={onlySelected}
        onclick={() => (onlySelected = !onlySelected)}
      >
        <Icon name="filter" />
      </button>
    </div>
    <label class="search">
      <Icon name="search" size={16} />
      <input placeholder="Szukaj numeru lub kodu" aria-label="Szukaj punktu" bind:value={filter} />
    </label>
    <button class="ib" title="Zamknij tabelę" onclick={onclose}><Icon name="x" /></button>
  </div>


  <DataTable
    bind:this={table}
    {columns}
    {rows}
    tail={editing ? NEW : undefined}
    ontailcommit={commitNew}
    rowKey={(p) => p.id}
    {editing}
    sortable
    {selection}
    onactivate={(p) => onzoom([p])}
    empty={project.points.length
      ? 'Żaden punkt nie pasuje do filtra.'
      : 'Brak punktów. Zaimportuj wykaz (Projekt → Importuj) albo dodaj punkt.'}
  />

  <footer>
    <span>Punkty: {project.points.length}</span>
    <span>widoczne: {rows.length}</span>
    <span>zaznaczone: {selection.size}</span>
    {#if editing}<span class="edit">nowy punkt wpisz w ostatnim wierszu; Enter zatwierdza</span>{/if}
  </footer>
</div>

<style>
  .pt {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    gap: 8px;
    height: 100%;
    min-height: 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.9rem;
    white-space: nowrap;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: 12px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .sep {
    width: 1px;
    height: 18px;
    margin: 0 3px;
    background: var(--field-edge);
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 120px;
    max-width: 280px;
    margin-left: auto;
    padding: 0 10px;
    border-radius: 12px;
    border: 1px solid var(--field-edge);
    background: var(--field-bg);
    color: var(--ink-soft);
  }
  .search input {
    padding: 6px 0;
    border: none;
    background: none;
    box-shadow: none;
    font-size: 0.85rem;
  }
  footer {
    display: flex;
    gap: 14px;
    font-size: 0.75rem;
    color: var(--ink-soft);
    font-variant-numeric: tabular-nums;
  }
  .edit {
    color: var(--accent);
  }
  @media (max-width: 700px) {
    .name {
      display: none;
    }
  }
</style>
