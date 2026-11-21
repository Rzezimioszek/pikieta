<script lang="ts">
  import Icon from './Icon.svelte'
  import { epsgOf } from './core'
  import { engine } from './gml-engine'
  import {
    boundaryPointsToProject,
    className,
    exportGml,
    featureKey,
    gmlSelection,
    persistLayer,
    removeLayer,
    type GmlLayer,
  } from './gml.svelte'
  import { CRS_LIST } from './map'
  import { project } from './project.svelte'

  interface Props {
    layer: GmlLayer
    onzoom: (target: string[] | { layer: string }) => void
    onfocus: (layer: GmlLayer, feature: number) => void
  }
  let { layer, onzoom, onfocus }: Props = $props()

  /** Lista uwag jest ucinana; przy dużych plikach mogą ich być tysiące. */
  const MAX_ISSUES = 300
  const SEVERITY = { error: 'Błąd', warning: 'Ostrzeżenie', info: 'Informacja' } as const

  const crsName = (code: number) => CRS_LIST.find((c) => c.code === `EPSG:${code}`)?.label ?? `EPSG:${code}`
  const counts = $derived({
    error: layer.issues.filter((i) => i.severity === 'error').length,
    warning: layer.issues.filter((i) => i.severity === 'warning').length,
  })
  const hasBoundary = $derived(layer.classes.some(([c]) => c === 'EGB_PunktGraniczny'))
  const selected = $derived(
    [...gmlSelection].filter((k) => k.startsWith(`${layer.id}:`)).map((k) => Number(k.split(':')[1])),
  )

  let attrs = $state.raw<[string, string, boolean][]>([])
  $effect(() => {
    const i = selected[0]
    attrs = []
    if (i === undefined || layer.status !== 'ready') return
    let stale = false
    engine.properties(layer.doc, i).then((p) => !stale && (attrs = p))
    return () => (stale = true)
  })

  function toggleClass(c: string) {
    layer.hidden = layer.hidden.includes(c) ? layer.hidden.filter((h) => h !== c) : [...layer.hidden, c]
    persistLayer(layer)
  }
</script>

<div class="calc">
  {#if layer.status === 'loading'}
    <p class="hint loading">Wczytywanie i sprawdzanie pliku w tle… Interfejs działa w tym czasie.</p>
  {:else if layer.status === 'error'}
    <p class="error">{layer.error}</p>
    <div class="actions start"><button onclick={() => removeLayer(layer)}><Icon name="trash" /> Usuń warstwę</button></div>
  {:else}
    <div class="actions start">
      <div class="group">
        <button class="primary" onclick={() => exportGml(layer)}><Icon name="download" /> Eksportuj GML</button>
        <button onclick={() => onzoom({ layer: layer.id })}><Icon name="fit" /> Pokaż</button>
        {#if hasBoundary}
          <button onclick={() => boundaryPointsToProject(layer)} title="Dodaj punkty graniczne do punktów projektu">
            <Icon name="pointAdd" /> Punkty graniczne → projekt
          </button>
        {/if}
      </div>
      <button onclick={() => removeLayer(layer)}><Icon name="trash" /> Usuń</button>
    </div>

    <dl class="card result">
      <dt>Układ współrzędnych</dt>
      <dd>{layer.epsg ? crsName(layer.epsg) : 'nierozpoznany'}</dd>
      {#if layer.epsg && layer.epsg !== epsgOf(project.crs)}
        <dt></dt>
        <dd class="muted">na mapie przeliczony do układu projektu</dd>
      {/if}
      <dt>Obiekty</dt>
      <dd>{layer.count.toLocaleString('pl-PL')}</dd>
      <dt>Stan</dt>
      <dd class:muted={!layer.dirty}>{layer.dirty ? 'zmieniony, niezapisany' : 'bez zmian'}</dd>
    </dl>

    <section>
      <h3>Klasy obiektów</h3>
      <ul class="classes">
        {#each layer.classes as [c, n] (c)}
          <li>
            <label>
              <input type="checkbox" checked={!layer.hidden.includes(c)} onchange={() => toggleClass(c)} />
              <span>{className(c)}</span>
              {#if className(c) !== c}<small>{c}</small>{/if}
            </label>
            <span class="n">{n.toLocaleString('pl-PL')}</span>
          </li>
        {/each}
      </ul>
    </section>

    {#if selected.length}
      <section>
        <h3>Zaznaczony obiekt{selected.length > 1 ? ` (1 z ${selected.length})` : ''}</h3>
        <dl class="card props">
          <dt>Klasa</dt><dd>{className(layer.classOf(selected[0]))}</dd>
          <dt>gml:id</dt><dd>{layer.index?.id[selected[0]]}</dd>
          {#each attrs as [k, v] (k)}<dt>{k}</dt><dd>{v}</dd>{/each}
        </dl>
      </section>
    {/if}

    <section>
      <h3>
        Walidacja
        <span class="badges">
          {#if counts.error}<span class="b error">{counts.error} błędów</span>{/if}
          {#if counts.warning}<span class="b warning">{counts.warning} ostrzeżeń</span>{/if}
          {#if !counts.error && !counts.warning}<span class="b ok">bez uwag</span>{/if}
        </span>
      </h3>
      {#if layer.issues.length}
        <ul class="issues">
          {#each layer.issues.slice(0, MAX_ISSUES) as issue, k (k)}
            <li class={issue.severity}>
              {#if issue.feature !== null}
                <button onclick={() => onfocus(layer, issue.feature!)}>
                  <strong>{SEVERITY[issue.severity]}:</strong> {issue.message}
                </button>
              {:else}
                <span><strong>{SEVERITY[issue.severity]}:</strong> {issue.message}</span>
              {/if}
            </li>
          {/each}
        </ul>
        {#if layer.issues.length > MAX_ISSUES}
          <p class="hint">Pokazano {MAX_ISSUES} z {layer.issues.length} uwag.</p>
        {/if}
      {/if}
      <p class="hint">
        Sprawdzane: powtórzone gml:id, odwołania xlink:href, układ współrzędnych, zamknięcie i samoprzecięcia pierścieni,
        wymagana geometria i idDzialki, położenie punktów granicznych w wierzchołkach działek.
      </p>
    </section>
  {/if}
</div>

<style>
  .loading {
    padding: 8px 0;
  }
  .group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 0.86rem;
  }
  .classes,
  .issues {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .classes li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 8px;
    border-radius: 10px;
  }
  .classes li:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }
  .classes label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 0.86rem;
  }
  .classes small {
    color: var(--ink-soft);
    font-size: 0.7rem;
  }
  .n {
    font-variant-numeric: tabular-nums;
    color: var(--ink-soft);
    font-size: 0.8rem;
  }
  .props {
    grid-template-columns: max-content 1fr;
    font-size: 0.8rem;
    max-height: 260px;
    overflow: auto;
  }
  .props dd {
    text-align: left;
    font-weight: 500;
    word-break: break-word;
  }
  .badges {
    display: flex;
    gap: 6px;
  }
  .b {
    padding: 1px 8px;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 600;
  }
  .b.error {
    background: color-mix(in srgb, var(--danger) 25%, transparent);
    color: var(--danger);
  }
  .b.warning {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent);
  }
  .b.ok {
    background: color-mix(in srgb, var(--ok) 22%, transparent);
    color: var(--ok);
  }
  .issues li > * {
    display: block;
    width: 100%;
    padding: 6px 10px;
    border: none;
    border-left: 3px solid var(--ink-soft);
    border-radius: 8px;
    background: var(--field-bg);
    font-size: 0.8rem;
    font-weight: 400;
    text-align: left;
    white-space: normal;
  }
  .issues li.error > * {
    border-left-color: var(--danger);
  }
  .issues li.warning > * {
    border-left-color: var(--accent);
  }
</style>
