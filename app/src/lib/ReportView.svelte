<script lang="ts">
  import Icon from './Icon.svelte'
  import { exportReport } from './files'
  import { project } from './project.svelte'

  let { crsLabel }: { crsLabel: string } = $props()

  function clear() {
    if (confirm(`Usunąć cały raport (${project.report.length} pozycji)?`)) project.report = []
  }
</script>

<div class="calc">
  <div class="actions start">
    <button onclick={() => exportReport(crsLabel)} disabled={project.report.length === 0}>
      <Icon name="download" /> Eksportuj TXT
    </button>
    <button onclick={clear} disabled={project.report.length === 0}><Icon name="trash" /> Wyczyść</button>
  </div>

  {#if project.report.length === 0}
    <p class="hint">Raport jest pusty. Wynik trafia tu po kliknięciu „Zapisz w raporcie” w oknie obliczenia.</p>
  {:else}
    <ol>
      {#each project.report as entry, i (i)}
        <li class="card">
          <div class="head">
            <strong>{i + 1}. {entry.title}</strong>
            <button class="small" aria-label={`Usuń pozycję ${i + 1}`} onclick={() => project.report.splice(i, 1)}>×</button>
          </div>
          <pre>{entry.lines.join('\n')}</pre>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  ol {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 8px;
    font-size: 0.88rem;
  }
  pre {
    margin-top: 8px;
    font-family: ui-monospace, 'Cascadia Mono', Consolas, monospace;
    font-size: 0.78rem;
    line-height: 1.5;
    white-space: pre-wrap;
    color: var(--ink-soft);
  }
</style>
