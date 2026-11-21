<script lang="ts">
  import { downloadText, project, reportText } from './project.svelte'

  interface Props {
    crsLabel: string
  }
  let { crsLabel }: Props = $props()

  function clear() {
    if (confirm(`Usunąć cały raport (${project.report.length} pozycji)?`)) project.report = []
  }
</script>

{#if project.report.length === 0}
  <p class="hint">Raport jest pusty. Wynik zadania trafia tu po kliknięciu „Zapisz” w zakładce Obliczenia.</p>
{:else}
  <div class="toolbar">
    <button onclick={() => downloadText('raport.txt', reportText(crsLabel))}>Eksportuj TXT</button>
    <button onclick={clear}>Wyczyść</button>
  </div>

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

<style>
  .toolbar {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
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
    margin-top: 6px;
    font: inherit;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    white-space: pre-wrap;
    color: var(--ink-soft);
  }
</style>
