<script lang="ts">
  import { epsgOf, fmt, type Pt } from './core'
  import { finishImport, importDraft } from './files.svelte'
  import { checkAxes, swap } from './importCheck'
  import { findPoint, project, ui } from './project.svelte'

  let { crsLabel }: { crsLabel: string } = $props()

  let dialog = $state<HTMLDialogElement>()
  let yx = $state(false)
  let replace = $state(true)

  const draft = $derived(importDraft.draft)
  const check = $derived(draft ? checkAxes(draft.points, epsgOf(project.crs)) : null)
  /** Kolejność wykryta przy otwarciu; użytkownik może ją zmienić. */
  let detected = $state<boolean | null>(null)

  $effect(() => {
    if (!dialog) return
    if (draft && !dialog.open) {
      detected = check?.guess ?? null
      yx = detected ?? ui.yx
      replace = true
      dialog.showModal()
    } else if (!draft && dialog.open) dialog.close()
  })

  const points = $derived(draft ? (yx ? draft.points.map(swap) : draft.points) : [])
  const inside = $derived(check ? (yx ? check.inside.yx : check.inside.xy) : 0)
  const share = $derived(check?.checked ? inside / check.checked : 0)
  const clash = $derived(
    points.flatMap((p) => {
      const old = findPoint(p.id)
      return old ? [{ p, d: Math.hypot(old.x - p.x, old.y - p.y) }] : []
    }),
  )
  const moved = $derived(clash.filter((c) => c.d >= 0.001))
  const PREVIEW = 8

  function rowState(p: Pt) {
    const old = findPoint(p.id)
    if (!old) return { text: 'nowy', tone: 'ok' }
    const d = Math.hypot(old.x - p.x, old.y - p.y)
    if (d < 0.001) return { text: 'taki sam', tone: 'muted' }
    return replace ? { text: `zastąpi (Δ ${fmt(d, 3)} m)`, tone: 'error' } : { text: 'pominięty', tone: 'muted' }
  }

  function confirm() {
    ui.yx = yx
    finishImport(points, replace)
  }
  const fc = (v: number) => fmt(v, project.precision.coord)
</script>

<dialog bind:this={dialog} class="preview surface" aria-labelledby="import-title" onclose={() => draft && finishImport(null, false)}>
  {#if draft && check}
    <header>
      <h2 id="import-title">Import punktów: {draft.name}</h2>
    </header>

    <div class="body">
      <fieldset>
        <legend>Kolejność kolumn w pliku</legend>
        <label><input type="radio" name="yx" value={false} bind:group={yx} /> Nr X Y (H kod): najpierw północ, jak w geodezji</label>
        <label><input type="radio" name="yx" value={true} bind:group={yx} /> Nr Y X (H kod): najpierw wschód, jak w GIS</label>
      </fieldset>

      {#if check.checked}
        {#if share >= 0.9}
          <p class="ok">
            W układzie projektu ({crsLabel}) {inside === check.checked ? 'wszystkie punkty leżą' : `${inside} z ${check.checked} sprawdzonych punktów leży`} w Polsce.
            {#if detected !== null && detected !== ui.yx && yx === detected}
              Kolejność kolumn wykryta automatycznie: {yx ? 'Nr Y X' : 'Nr X Y'}.
            {/if}
          </p>
        {:else}
          <div class="warn" role="status">
            <p>
              Przy tej kolejności {inside ? `tylko ${inside} z ${check.checked}` : 'żaden z'} sprawdzonych punktów
              {inside === 1 ? 'leży' : 'leżą'} w Polsce w układzie projektu ({crsLabel}).
            </p>
            {#if (yx ? check.inside.xy : check.inside.yx) > inside}
              <p>Przy drugiej kolejności kolumn w Polsce leży {yx ? check.inside.xy : check.inside.yx} punktów: kolumny są pewnie zamienione.</p>
            {/if}
            {#if yx ? check.otherZone.yx : check.otherZone.xy}
              <p>Współrzędne Y wyglądają na strefę {yx ? check.otherZone.yx : check.otherZone.xy} układu 2000. Zmień układ projektu na karcie Mapa przed importem albo przelicz punkty później.</p>
            {:else if !inside && !(yx ? check.inside.xy : check.inside.yx)}
              <p>Punkty mogą być w innym układzie (np. lokalnym). Można je wczytać i przeliczyć transformacją.</p>
            {/if}
          </div>
        {/if}
      {/if}

      <div class="scroll">
      <table>
        <caption class="sr-only">Pierwsze punkty z pliku</caption>
        <thead>
          <tr><th>Nr</th><th class="r">X (północ)</th><th class="r">Y (wschód)</th><th class="r">H</th><th>Kod</th><th>Stan</th></tr>
        </thead>
        <tbody>
          {#each points.slice(0, PREVIEW) as p, i (i)}
            {@const s = rowState(p)}
            <tr>
              <td>{p.id}</td>
              <td class="r">{fc(p.x)}</td>
              <td class="r">{fc(p.y)}</td>
              <td class="r">{p.h === null ? '' : fc(p.h)}</td>
              <td>{p.code}</td>
              <td class={s.tone}>{s.text}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      </div>
      {#if points.length > PREVIEW}<p class="hint">…i {points.length - PREVIEW} kolejnych.</p>{/if}

      {#if clash.length}
        <fieldset>
          <legend>
            Numery już w projekcie: {clash.length}{moved.length ? `, w tym ${moved.length} o innych współrzędnych` : ''}
          </legend>
          <label><input type="radio" name="replace" value={true} bind:group={replace} /> Zastąp współrzędne w projekcie</label>
          <label><input type="radio" name="replace" value={false} bind:group={replace} /> Pomiń te punkty</label>
          {#if moved.length}
            <p class="hint">
              {moved.slice(0, 6).map((c) => `${c.p.id} (Δ ${fmt(c.d, 3)} m)`).join(', ')}{moved.length > 6 ? '…' : ''}
            </p>
          {/if}
        </fieldset>
      {/if}

      {#if draft.errors.length}
        <details>
          <summary>Pominięte wiersze: {draft.errors.length}</summary>
          <ul>
            {#each draft.errors.slice(0, 50) as e, i (i)}<li>{e}</li>{/each}
          </ul>
          {#if draft.errors.length > 50}<p class="hint">…i {draft.errors.length - 50} kolejnych.</p>{/if}
        </details>
      {/if}
      {#if !points.length}<p class="error" role="alert">W pliku nie ma żadnego poprawnego wiersza z punktem.</p>{/if}
    </div>

    <footer>
      <button onclick={() => finishImport(null, false)}>Anuluj</button>
      <button class="primary" disabled={!points.length} onclick={confirm}>
        Wczytaj {replace ? points.length : points.length - clash.length} pkt
      </button>
    </footer>
  {/if}
</dialog>

<style>
  .preview {
    width: min(760px, calc(100vw - 32px));
    max-height: min(86vh, 800px);
    margin: auto;
    position: fixed;
    padding: 0;
    color: var(--ink);
    background: var(--menu-bg);
  }
  .preview[open] {
    display: grid;
    grid-template-rows: auto 1fr auto;
  }
  .preview::backdrop {
    background: rgb(0 0 0 / 0.45);
  }
  header {
    padding: 16px 18px 6px;
  }
  h2 {
    margin: 0;
    font-size: 1.05rem;
    overflow-wrap: anywhere;
  }
  .body {
    display: grid;
    align-content: start;
    gap: 12px;
    min-height: 0;
    overflow: auto;
    padding: 6px 18px 12px;
    font-size: 0.86rem;
  }
  fieldset {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 8px 12px 10px;
    border: 1px solid var(--field-edge);
    border-radius: 12px;
  }
  legend {
    padding: 0 4px;
    font-weight: 600;
  }
  label {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  label input {
    flex: none;
    width: auto;
    margin: 0;
    padding: 0;
    box-shadow: none;
  }
  .ok {
    margin: 0;
    color: var(--ok);
  }
  .warn {
    padding: 8px 12px;
    border-radius: 12px;
    border: 1px solid var(--danger);
    color: var(--danger);
  }
  .warn p {
    margin: 0 0 4px;
  }
  .scroll {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-variant-numeric: tabular-nums;
  }
  th,
  td {
    padding: 4px 8px;
    border-bottom: 1px solid var(--field-edge);
    text-align: left;
    white-space: nowrap;
  }
  th {
    font-size: 0.75rem;
    color: var(--ink-soft);
  }
  .r {
    text-align: right;
  }
  td.ok {
    color: var(--ok);
  }
  td.muted {
    color: var(--ink-soft);
  }
  td.error {
    color: var(--danger);
  }
  .hint {
    margin: 0;
  }
  details ul {
    margin: 6px 0 0;
    padding-left: 18px;
    font-family: ui-monospace, monospace;
    font-size: 0.78rem;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px 16px;
    border-top: 1px solid var(--field-edge);
  }
  @media (max-width: 600px) {
    .preview {
      width: 100vw;
      max-width: 100vw;
      height: 100dvh;
      max-height: 100dvh;
      border-radius: 0;
    }
    .body {
      overflow-x: auto;
    }
  }
</style>
