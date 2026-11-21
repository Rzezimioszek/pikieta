<script lang="ts">
  import { parsePoints, writePoints, type Pt, type XY } from './core'
  import { addPoints, downloadText, findPoint, nextId, project, removePoint } from './project.svelte'

  interface Props {
    selectedId: string | null
    addByClick: boolean
    onzoom: (points: XY[]) => void
  }
  let { selectedId = $bindable(), addByClick = $bindable(), onzoom }: Props = $props()

  /** Powyżej tej liczby wierszy tabela pokazuje tylko początek listy. */
  const MAX_ROWS = 300

  let yx = $state(false)
  let filter = $state('')
  let message = $state('')
  let errors = $state<string[]>([])
  let draft = $state({ id: '', x: '', y: '', h: '' })
  let fileInput = $state<HTMLInputElement>()

  const filtered = $derived.by(() => {
    const q = filter.trim().toLowerCase()
    return q === ''
      ? project.points
      : project.points.filter((p) => p.id.toLowerCase().includes(q) || p.code.toLowerCase().includes(q))
  })
  const rows = $derived(filtered.slice(0, MAX_ROWS))

  const num = (s: string) => {
    const t = s.trim().replace(',', '.')
    const v = Number(t)
    return t !== '' && Number.isFinite(v) ? v : null
  }

  /** Pliki z programów geodezyjnych bywają w Windows-1250, nie w UTF-8. */
  async function readText(file: File): Promise<string> {
    const bytes = await file.arrayBuffer()
    try {
      return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
    } catch {
      return new TextDecoder('windows-1250').decode(bytes)
    }
  }

  async function importFile(e: Event) {
    const input = e.currentTarget as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file) return
    const parsed = parsePoints(await readText(file), yx)
    const replaced = addPoints(parsed.points)
    errors = parsed.errors
    message =
      `${file.name}: wczytano ${parsed.points.length} pkt` +
      (replaced ? `, w tym zastąpiono ${replaced}` : '') +
      (parsed.errors.length ? `, pominięto ${parsed.errors.length} wierszy` : '') +
      '.'
    if (parsed.points.length) onzoom(parsed.points)
  }

  function exportFile() {
    downloadText('punkty.txt', writePoints(project.points, yx, 2))
  }

  function addDraft() {
    const [x, y, h] = [num(draft.x), num(draft.y), num(draft.h)]
    const id = draft.id.trim() || nextId()
    errors = []
    if (x === null || y === null) {
      message = 'Podaj liczbowe X i Y.'
      return
    }
    if (draft.h.trim() !== '' && h === null) {
      message = 'H nie jest liczbą.'
      return
    }
    if (findPoint(id)) {
      message = `Punkt ${id} już istnieje.`
      return
    }
    addPoints([{ id, x, y, h, code: '' }])
    selectedId = id
    draft = { id: '', x: '', y: '', h: '' }
    message = `Dodano punkt ${id}.`
  }

  function clearAll() {
    if (!confirm(`Usunąć wszystkie punkty (${project.points.length})?`)) return
    project.points = []
    selectedId = null
    message = ''
    errors = []
  }

  function select(p: Pt) {
    selectedId = p.id
    onzoom([p])
  }
</script>

<div class="toolbar">
  <button onclick={() => fileInput?.click()}>Importuj TXT</button>
  <button onclick={exportFile} disabled={project.points.length === 0}>Eksportuj TXT</button>
  <select aria-label="Kolejność kolumn w pliku" bind:value={yx}>
    <option value={false}>Nr X Y</option>
    <option value={true}>Nr Y X</option>
  </select>
  <input bind:this={fileInput} type="file" accept=".txt,.csv,.xyz,text/plain" hidden onchange={importFile} />
</div>

<form
  class="draft"
  onsubmit={(e) => {
    e.preventDefault()
    addDraft()
  }}
>
  <input aria-label="Numer nowego punktu" placeholder={`Nr (${nextId()})`} bind:value={draft.id} />
  <input aria-label="X nowego punktu" placeholder="X" inputmode="decimal" bind:value={draft.x} />
  <input aria-label="Y nowego punktu" placeholder="Y" inputmode="decimal" bind:value={draft.y} />
  <input aria-label="H nowego punktu" placeholder="H" inputmode="decimal" bind:value={draft.h} />
  <button class="primary" type="submit" aria-label="Dodaj punkt">+</button>
</form>

<label class="check">
  <input type="checkbox" bind:checked={addByClick} />
  Dodawaj punkty kliknięciem na mapie
</label>

{#if message}<p class="hint" role="status">{message}</p>{/if}
{#if errors.length}
  <ul class="error">
    {#each errors.slice(0, 5) as e (e)}<li>{e}</li>{/each}
    {#if errors.length > 5}<li>… i {errors.length - 5} więcej</li>{/if}
  </ul>
{/if}

{#if project.points.length === 0}
  <p class="hint">Brak punktów. Zaimportuj wykaz współrzędnych albo dodaj punkt ręcznie.</p>
{:else}
  <div class="listhead">
    <input aria-label="Szukaj punktu" placeholder="Szukaj numeru lub kodu" bind:value={filter} />
    <button class="small" onclick={() => onzoom(project.points)}>Pokaż wszystkie</button>
  </div>

  <div class="card table">
    <table>
      <thead>
        <tr><th>Nr</th><th>X</th><th>Y</th><th>H</th><th><span class="sr">Usuń</span></th></tr>
      </thead>
      <tbody>
        {#each rows as p (p.id)}
          <tr class:selected={p.id === selectedId} onclick={() => select(p)}>
            <td title={p.code}>{p.id}</td>
            <td>{p.x.toFixed(2)}</td>
            <td>{p.y.toFixed(2)}</td>
            <td>{p.h === null ? '' : p.h.toFixed(2)}</td>
            <td>
              <button
                class="small"
                aria-label={`Usuń punkt ${p.id}`}
                onclick={(e) => {
                  e.stopPropagation()
                  removePoint(p.id)
                  if (selectedId === p.id) selectedId = null
                }}>×</button
              >
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <div class="listfoot">
    <span class="hint">
      {filtered.length > MAX_ROWS ? `Pokazano ${MAX_ROWS} z ${filtered.length}` : `${filtered.length} pkt`}
      {filter.trim() ? ` (wszystkich: ${project.points.length})` : ''}
    </span>
    <button class="small" onclick={clearAll}>Usuń wszystkie</button>
  </div>
{/if}

<style>
  .toolbar {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 8px;
  }
  .toolbar button,
  .toolbar select {
    padding-inline: 8px;
    font-size: 0.85rem;
  }
  .draft {
    display: grid;
    grid-template-columns: 1.1fr 1.6fr 1.6fr 1fr auto;
    gap: 6px;
  }
  .draft input {
    padding: 8px;
    font-size: 0.85rem;
  }
  .draft button {
    padding-inline: 12px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
  }
  ul {
    padding-left: 18px;
  }
  .listhead,
  .listfoot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .listhead input {
    flex: 1;
    padding: 7px 10px;
    font-size: 0.85rem;
  }
  .table {
    padding: 0;
    overflow: auto;
    max-height: 38dvh;
    min-height: 90px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
    font-variant-numeric: tabular-nums;
  }
  th {
    position: sticky;
    top: 0;
    background: var(--menu-bg);
    font-weight: 600;
    color: var(--ink-soft);
    z-index: 1;
  }
  th,
  td {
    padding: 6px 8px;
    text-align: right;
    white-space: nowrap;
  }
  th:first-child,
  td:first-child {
    text-align: left;
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  td:last-child {
    padding-block: 2px;
    width: 1%;
  }
  tbody tr {
    cursor: pointer;
    border-top: 1px solid var(--field-edge);
  }
  tbody tr:hover {
    background: color-mix(in srgb, var(--ink) 7%, transparent);
  }
  tbody tr.selected {
    background: color-mix(in srgb, var(--accent) 28%, transparent);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
