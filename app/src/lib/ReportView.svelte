<script lang="ts">
  import Icon from './Icon.svelte'
  import { exportReport, printReport, reportHtml } from './files.svelte'
  import { project } from './project.svelte'

  let { crsLabel }: { crsLabel: string } = $props()

  const empty = $derived(project.report.length === 0)
  /** Podgląd to ten sam dokument HTML, który trafia do pliku i do druku. */
  const html = $derived(empty ? '' : reportHtml(crsLabel))

  function clear() {
    if (confirm(`Usunąć cały raport (${project.report.length} pozycji)?`)) project.report = []
  }
</script>

<div class="calc report">
  <div class="actions start">
    <div class="group">
      <button onclick={() => exportReport(crsLabel, 'txt')} disabled={empty}><Icon name="download" /> TXT</button>
      <button onclick={() => exportReport(crsLabel, 'html')} disabled={empty}><Icon name="download" /> HTML</button>
      <button class="primary" onclick={() => printReport(crsLabel)} disabled={empty}>
        <Icon name="report" /> Drukuj / PDF
      </button>
    </div>
    <button onclick={clear} disabled={empty}><Icon name="trash" /> Wyczyść</button>
  </div>

  {#if empty}
    <p class="hint">Raport jest pusty. Wynik trafia tu po kliknięciu „Zapisz w raporcie” w oknie obliczenia.</p>
  {:else}
    <ol class="entries">
      {#each project.report as entry, i (i)}
        <li>
          <span>{i + 1}. {entry.title}</span>
          <button class="ib" title={`Usuń pozycję ${i + 1}`} onclick={() => project.report.splice(i, 1)}>
            <Icon name="x" size={15} />
          </button>
        </li>
      {/each}
    </ol>
    <iframe class="preview" title="Podgląd raportu" srcdoc={html} sandbox=""></iframe>
    <p class="hint">W oknie drukowania wybierz „Zapisz jako PDF”, żeby otrzymać plik PDF.</p>
  {/if}
</div>

<style>
  .group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .entries {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 2px;
    max-height: 160px;
    overflow: auto;
  }
  .entries li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 4px 2px 10px;
    border-radius: 10px;
    font-size: 0.85rem;
  }
  .entries li:hover {
    background: color-mix(in srgb, var(--ink) 7%, transparent);
  }
  .entries span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .preview {
    width: 100%;
    height: max(60vh, 420px);
    border: 1px solid var(--field-edge);
    border-radius: 14px;
    background: #fff;
  }
</style>
