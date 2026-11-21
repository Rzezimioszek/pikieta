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
  let editing = $state(false)
  let filter = $state('')
  let onlySelected = $state(false)
  let adding = $state(false)
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

  const columns: Column<Pt>[] = [
    { key: 'id', label: 'Nr', width: 96, edit: 'text', value: (p) => p.id, set: (p, v) => renamePoint(p.id, v) },
    {
      key: 'x',
      label: 'X',
      width: 128,
      align: 'right',
      edit: 'number',
      value: (p) => fc(p.x),
      set: (p, v) => setNumber('X', (n) => (p.x = n!))(v),
    },
    {
      key: 'y',
      label: 'Y',
      width: 128,
      align: 'right',
      edit: 'number',
      value: (p) => fc(p.y),
      set: (p, v) => setNumber('Y', (n) => (p.y = n!))(v),
    },
    {
      key: 'h',
      label: 'H',
      width: 90,
      align: 'right',
      edit: 'number',
      value: (p) => (p.h === null ? '' : fc(p.h)),
      set: (p, v) => setNumber('H', (n) => (p.h = n), true)(v),
    },
    { key: 'code', label: 'Kod', width: 160, edit: 'text', value: (p) => p.code, set: (p, v) => (p.code = v.trim()) },
  ]

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

  function addDraft(e: SubmitEvent) {
    e.preventDefault()
    const [x, y, h] = [num(draft.x), num(draft.y), num(draft.h)]
    const id = draft.id.trim() || nextId()
    if (x === null || y === null) return notify('Podaj liczbowe X i Y.', 'error')
    if (draft.h.trim() !== '' && h === null) return notify('H nie jest liczbą.', 'error')
    if (findPoint(id)) return notify(`Punkt ${id} już istnieje.`, 'error')
    addPoints([{ id, x, y, h, code: draft.code.trim() }])
    selection.clear()
    selection.add(id)
    draft = { id: '', x: '', y: '', h: '', code: '' }
    table?.reveal(id)
  }

  export function reveal(id: string) {
    table?.reveal(id)
  }
</script>

<div class="pt">
  <div class="bar">
    <strong class="name"><Icon name="table" /> Punkty</strong>
    <div class="tools">
      <button class="ib" class:on={editing} title="Tryb edycji" aria-pressed={editing} onclick={() => (editing = !editing)}>
        <Icon name="pencil" />
      </button>
      <button class="ib" class:on={adding} title="Dodaj punkt" aria-pressed={adding} onclick={() => (adding = !adding)}>
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

  {#if adding}
    <form class="draft" onsubmit={addDraft}>
      <input aria-label="Numer" placeholder={`Nr (${nextId()})`} bind:value={draft.id} />
      <input aria-label="X" placeholder="X" inputmode="decimal" bind:value={draft.x} />
      <input aria-label="Y" placeholder="Y" inputmode="decimal" bind:value={draft.y} />
      <input aria-label="H" placeholder="H" inputmode="decimal" bind:value={draft.h} />
      <input aria-label="Kod" placeholder="Kod" bind:value={draft.code} />
      <button class="primary" type="submit">Dodaj</button>
    </form>
  {/if}

  <DataTable
    bind:this={table}
    {columns}
    {rows}
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
    {#if editing}<span class="edit">tryb edycji: zmiany zapisują się po zatwierdzeniu (Enter)</span>{/if}
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
  .draft {
    display: grid;
    grid-template-columns: 1fr 1.4fr 1.4fr 1fr 1.2fr auto;
    gap: 6px;
  }
  .draft input {
    padding: 6px 10px;
    font-size: 0.85rem;
  }
  .draft button {
    padding: 6px 14px;
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
    .draft {
      grid-template-columns: repeat(3, 1fr);
    }
    .name {
      display: none;
    }
  }
</style>
