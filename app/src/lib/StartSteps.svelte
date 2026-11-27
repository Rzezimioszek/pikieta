<script lang="ts">
  // Pasek startowy w pustym panelu: trzy kroki typowej pracy, odhaczane w miarę postępu.
  import Icon from './Icon.svelte'
  import type { XY } from './core'
  import { importPoints } from './files.svelte'
  import { openTaskPicker, project, showReport, ui } from './project.svelte'

  let { onzoom }: { onzoom: (points: XY[]) => void } = $props()

  const done = $derived([project.points.length > 0, project.calcs.length > 0, project.report.length > 0])
  /** Pierwszy krok do zrobienia; po wszystkich -1. */
  const next = $derived(done.indexOf(false))

  const plural = (k: number) =>
    k === 1 ? 'pozycja' : k % 10 >= 2 && k % 10 <= 4 && (k % 100 < 12 || k % 100 > 14) ? 'pozycje' : 'pozycji'

  async function load() {
    const pts = await importPoints()
    if (pts.length) onzoom(pts)
  }
</script>

<ol class="steps" aria-label="Pierwsze kroki">
  <li class:done={done[0]} class:next={next === 0}>
    <span class="no" aria-hidden="true">{done[0] ? '✓' : '1'}</span>
    <div class="txt">
      <h3>Punkty <span class="sr-only">{done[0] ? '(zrobione)' : ''}</span></h3>
      <p>
        {#if done[0]}
          W projekcie: {project.points.length} pkt.
        {:else}
          Wczytaj wykaz współrzędnych (TXT), wpisz punkty w tabeli pod mapą albo dodaj je kliknięciem na mapie.
        {/if}
      </p>
      <div class="acts">
        <button class:primary={next === 0} onclick={load}><Icon name="upload" size={16} /> Wczytaj wykaz</button>
        <button aria-pressed={ui.addByClick} onclick={() => (ui.addByClick = !ui.addByClick)}>
          <Icon name="pointAdd" size={16} /> Dodawaj na mapie
        </button>
      </div>
    </div>
  </li>
  <li class:done={done[1]} class:next={next === 1}>
    <span class="no" aria-hidden="true">{done[1] ? '✓' : '2'}</span>
    <div class="txt">
      <h3>Obliczenie <span class="sr-only">{done[1] ? '(zrobione)' : ''}</span></h3>
      <p>Wybierz zadanie, np. tachimetrię albo transformację. Okno wyboru otwiera też Ctrl+K.</p>
      <div class="acts">
        <button class:primary={next === 1} onclick={() => openTaskPicker()} aria-haspopup="dialog">
          <Icon name="plus" size={16} /> Wybierz zadanie
        </button>
      </div>
    </div>
  </li>
  <li class:done={done[2]} class:next={next === 2}>
    <span class="no" aria-hidden="true">{done[2] ? '✓' : '3'}</span>
    <div class="txt">
      <h3>Raport <span class="sr-only">{done[2] ? '(zrobione)' : ''}</span></h3>
      <p>
        {#if done[2]}
          W raporcie: {project.report.length} {plural(project.report.length)}.
        {:else}
          W oknie obliczenia naciśnij „Zapisz w raporcie”. Raport wydrukujesz albo zapiszesz jako PDF, TXT lub HTML.
        {/if}
      </p>
      {#if done[2]}
        <div class="acts">
          <button class:primary={next === -1} onclick={showReport}><Icon name="report" size={16} /> Pokaż raport</button>
        </div>
      {/if}
    </div>
  </li>
</ol>

<style>
  .steps {
    display: grid;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 16px;
    border: 1px solid var(--field-edge);
    background: var(--field-bg);
  }
  /* Następny krok: pełna ramka w kolorze akcentu i tło panelu. */
  li.next {
    border: 2px solid var(--accent);
    padding: 11px 13px;
    background: var(--surface);
  }
  .no {
    display: grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid var(--control-edge);
    background: var(--surface);
    color: var(--ink);
    font-weight: 700;
    font-size: 0.85rem;
  }
  .done .no {
    border-color: var(--ok);
    background: var(--ok);
    color: var(--surface);
  }
  .next .no {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
  .txt {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
    color: var(--ink);
  }
  p {
    margin: 0;
    font-size: 0.86rem;
    line-height: 1.5;
    color: var(--ink-soft);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .acts button {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.84rem;
    background: var(--surface);
    border-color: var(--control-edge);
  }
  .acts button.primary {
    background: var(--accent);
    border-color: var(--accent);
  }
  .acts button[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
