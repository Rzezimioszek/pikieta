<script lang="ts">
  import DataTable from './DataTable.svelte'
  import Icon from './Icon.svelte'
  import { engine, type ClassTable } from './gml-engine'
  import { note } from './perf'
  import {
    className,
    featureKey,
    gmlSelection,
    gmlTableClass,
    setGmlProperty,
    type GmlLayer,
  } from './gml.svelte'
  import type { Column } from './table'

  interface Props {
    layer: GmlLayer
    onzoom: (keys: string[]) => void
    onclose: () => void
  }
  let { layer, onzoom, onclose }: Props = $props()

  interface Row {
    i: number
    values: string[]
  }

  const GEOM = ['brak', 'punkt', 'linia', 'wielobok', 'wieloczęściowa']

  let table = $state<ReturnType<typeof DataTable<Row>>>()
  let editing = $state(false)
  let filter = $state('')
  let onlySelected = $state(false)
  let data = $state.raw<ClassTable | null>(null)
  let loading = $state(false)
  let error = $state('')

  const classes = $derived(layer.classes)
  const cls = $derived(
    classes.some(([c]) => c === gmlTableClass[layer.id]) ? gmlTableClass[layer.id] : (classes[0]?.[0] ?? ''),
  )

  // Atrybuty klasy pobierane z silnika dopiero, gdy klasa jest pokazana.
  $effect(() => {
    // Najpierw geometria dla mapy (silnik wykonuje zadania po kolei), potem atrybuty tabeli.
    const [doc, c, ready] = [layer.doc, cls, layer.status === 'ready' && layer.geomRev > 0]
    data = null
    error = ''
    if (!ready || !c) return
    loading = true
    let stale = false
    const t0 = performance.now()
    engine
      .classTable(doc, c)
      .then((t) => {
        note(`tabela klasy ${c}`, t0)
        if (!stale) data = t
      })
      .catch((e) => !stale && (error = String(e)))
      .finally(() => !stale && (loading = false))
    return () => (stale = true)
  })

  const all = $derived(data ? data.rows.map(([i, values]): Row => ({ i, values })) : [])
  const rows = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    const labels = layer.index?.label ?? []
    return all.filter(
      (r) =>
        (!onlySelected || gmlSelection.has(featureKey(layer.id, r.i))) &&
        (q === '' || labels[r.i]?.toLowerCase().includes(q) || r.values.some((v) => v.toLowerCase().includes(q))),
    )
  })

  const columns = $derived.by((): Column<Row>[] => {
    const idx = layer.index
    const keys = data?.keys ?? []
    return [
      { key: '#label', label: 'Etykieta', width: 110, value: (r) => idx?.label[r.i] ?? '' },
      {
        key: '#geom',
        label: 'Geometria',
        width: 104,
        value: (r) => (idx?.errors[r.i] ? 'błąd' : GEOM[idx?.geom[r.i] ?? 0]),
        tone: (r) => (idx?.errors[r.i] || !idx?.geom[r.i] ? 'error' : 'muted'),
      },
      ...keys.map(
        (k, c): Column<Row> => ({
          key: k,
          label: k,
          width: Math.min(260, Math.max(96, k.length * 8 + 24)),
          edit: data?.editable[c] ? 'text' : undefined,
          value: (r) => r.values[c] ?? '',
          set: (r, v) => {
            if (v === r.values[c]) return
            setGmlProperty(layer, r.i, k, v).then((ok) => {
              if (ok && data) {
                // Wartość w tabeli bez ponownego pobierania całej klasy.
                r.values[c] = v
                data = { ...data }
              }
            })
          },
        }),
      ),
      { key: '#id', label: 'gml:id', width: 160, value: (r) => idx?.id[r.i] ?? '', tone: () => 'muted' },
    ]
  })

  const selectedKeys = () => all.map((r) => featureKey(layer.id, r.i)).filter((k) => gmlSelection.has(k))

  export function reveal(key: string) {
    table?.reveal(key)
  }
</script>

<div class="gt">
  <div class="bar">
    <strong class="name"><Icon name="layers" /> {layer.name}</strong>
    <select aria-label="Klasa obiektów" value={cls} onchange={(e) => (gmlTableClass[layer.id] = e.currentTarget.value)}>
      {#each classes as [c, n] (c)}<option value={c}>{className(c)} ({n})</option>{/each}
    </select>
    <div class="tools">
      <button class="ib" class:on={editing} title="Tryb edycji atrybutów" aria-pressed={editing} onclick={() => (editing = !editing)}>
        <Icon name="pencil" />
      </button>
      <span class="sep"></span>
      <button class="ib" title="Zaznacz wszystkie widoczne" onclick={() => rows.forEach((r) => gmlSelection.add(featureKey(layer.id, r.i)))}>
        <Icon name="selectAll" />
      </button>
      <button class="ib" title="Odznacz wszystkie" disabled={gmlSelection.size === 0} onclick={() => gmlSelection.clear()}>
        <Icon name="deselect" />
      </button>
      <button class="ib" title="Przybliż do zaznaczonych" disabled={gmlSelection.size === 0} onclick={() => onzoom(selectedKeys())}>
        <Icon name="fit" />
      </button>
      <button class="ib" class:on={onlySelected} title="Pokaż tylko zaznaczone" aria-pressed={onlySelected} onclick={() => (onlySelected = !onlySelected)}>
        <Icon name="filter" />
      </button>
    </div>
    <label class="search">
      <Icon name="search" size={16} />
      <input placeholder="Szukaj w atrybutach" aria-label="Szukaj obiektu" bind:value={filter} />
    </label>
    <button class="ib" title="Zamknij tabelę" onclick={onclose}><Icon name="x" /></button>
  </div>

  {#if layer.status === 'loading'}
    <p class="hint pad">Wczytywanie warstwy…</p>
  {:else if layer.status === 'error'}
    <p class="error pad">{layer.error}</p>
  {:else if error}
    <p class="error pad">{error}</p>
  {:else if loading && !data}
    <p class="hint pad">Wczytywanie atrybutów klasy {className(cls)}…</p>
  {:else}
    {#key cls}
      <DataTable
        bind:this={table}
        {columns}
        {rows}
        rowKey={(r) => featureKey(layer.id, r.i)}
        {editing}
        sortable
        selection={gmlSelection}
        onactivate={(r) => onzoom([featureKey(layer.id, r.i)])}
        empty={all.length ? 'Żaden obiekt nie pasuje do filtra.' : 'Brak obiektów tej klasy.'}
      />
    {/key}
  {/if}

  <footer>
    <span>{className(cls)}: {all.length}</span>
    <span>widoczne: {rows.length}</span>
    <span>zaznaczone: {gmlSelection.size}</span>
    {#if editing}<span class="edit">edytowalne są proste atrybuty tekstowe; zmiana trafia do pliku po eksporcie</span>{/if}
  </footer>
</div>

<style>
  .gt {
    display: grid;
    grid-template-rows: auto 1fr auto;
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
    max-width: 200px;
    overflow: hidden;
    font-size: 0.9rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bar select {
    width: auto;
    padding: 5px 8px;
    font-size: 0.85rem;
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px;
    border-radius: 12px;
    background: var(--field-bg);
    border: 1px solid var(--field-edge);
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
  .pad {
    padding: 12px;
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
    .name,
    .tools {
      display: none;
    }
  }
</style>
