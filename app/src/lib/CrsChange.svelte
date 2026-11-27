<script lang="ts">
  // Zmiana układu projektu z punktami: dwie różne operacje, które łatwo pomylić. Przeliczenie
  // zmienia liczby, a punkty zostają w tym samym miejscu w terenie; zmiana bez przeliczania
  // zostawia liczby, a punkty „przeskakują” (to naprawia punkty wczytane w złym układzie).
  import { epsgOf, toGeodetic } from './core'
  import { inPoland } from './importCheck'
  import { CRS_LIST } from './map'
  import { project } from './project.svelte'

  interface Props {
    /** Docelowy układ; `null` zamyka okno. */
    target: string | null
    onchoose: (reproject: boolean) => void
    oncancel: () => void
  }
  let { target, onchoose, oncancel }: Props = $props()

  let dialog = $state<HTMLDialogElement>()
  const label = (code: string) => CRS_LIST.find((c) => c.code === code)?.label ?? code

  $effect(() => {
    if (!dialog) return
    if (target && !dialog.open) dialog.showModal()
    else if (!target && dialog.open) dialog.close()
  })

  /** Ile punktów leży w Polsce w obecnym układzie i po samej zmianie oznaczenia układu. */
  const fit = $derived.by(() => {
    if (!target) return null
    const step = Math.max(1, Math.floor(project.points.length / 300))
    const sample = project.points.filter((_, i) => i % step === 0)
    const [from, to] = [epsgOf(project.crs), epsgOf(target)]
    const now = sample.filter((p) => inPoland(toGeodetic(from, p))).length
    const relabel = sample.filter((p) => inPoland(toGeodetic(to, p))).length
    return { n: sample.length, now, relabel }
  })
  /** Zalecenie: bez przeliczania tylko wtedy, gdy punkty dziś leżą poza Polską, a po zmianie oznaczenia w Polsce. */
  const relabelLikely = $derived(!!fit && fit.now < fit.n / 2 && fit.relabel > fit.n / 2)
  const n = $derived(project.points.length)
  /** „jest 1 punkt”, „są 3 punkty”, „jest 5 punktów”, „są 22 punkty”. */
  function count(k: number) {
    if (k === 1) return 'jest 1 punkt'
    const few = k % 10 >= 2 && k % 10 <= 4 && (k % 100 < 12 || k % 100 > 14)
    return few ? `są ${k} punkty` : `jest ${k} punktów`
  }
</script>

<dialog bind:this={dialog} class="crs surface" aria-labelledby="crs-title" onclose={() => target && oncancel()}>
  {#if target && fit}
    <h2 id="crs-title">Zmiana układu współrzędnych</h2>
    <p>
      Z <strong>{label(project.crs)}</strong> na <strong>{label(target)}</strong>. W projekcie {count(n)}.
      Co zrobić z ich współrzędnymi?
    </p>

    {#if relabelLikely}
      <p class="warn" role="status">
        W obecnym układzie punkty leżą poza Polską ({fit.now} z {fit.n}), a w układzie {label(target)} w Polsce
        ({fit.relabel} z {fit.n}). Wygląda na to, że zostały wczytane w złym układzie: wybierz „Zmień układ bez
        przeliczania”.
      </p>
    {:else if fit.now === fit.n}
      <p class="ok">Punkty leżą w Polsce w obecnym układzie, więc zwykle trzeba je przeliczyć.</p>
    {/if}

    <div class="choices">
      <button class:primary={!relabelLikely} onclick={() => onchoose(true)}>
        <strong>Przelicz punkty do nowego układu</strong>
        <small>Współrzędne się zmienią, a punkty zostaną w tym samym miejscu w terenie.</small>
      </button>
      <button class:primary={relabelLikely} onclick={() => onchoose(false)}>
        <strong>Zmień układ bez przeliczania</strong>
        <small>Liczby zostaną bez zmian, a punkty przesuną się na mapie. Dla punktów wczytanych w złym układzie.</small>
      </button>
    </div>

    <p class="hint">
      Warstwy GML mają własny układ i zawsze są rysowane na swoim miejscu. Współrzędne wpisane ręcznie w oknach
      obliczeń i raport się nie zmieniają.
    </p>
    <footer><button onclick={oncancel}>Anuluj</button></footer>
  {/if}
</dialog>

<style>
  .crs {
    width: min(560px, calc(100vw - 32px));
    margin: auto;
    position: fixed;
    padding: 16px 18px;
    color: var(--ink);
    background: var(--menu-bg);
    font-size: 0.88rem;
  }
  .crs::backdrop {
    background: rgb(0 0 0 / 0.45);
  }
  h2 {
    margin: 0 0 8px;
    font-size: 1.05rem;
  }
  p {
    margin: 0 0 10px;
    line-height: 1.45;
  }
  .ok {
    color: var(--ok);
  }
  .warn {
    padding: 8px 12px;
    border: 1px solid var(--danger);
    border-radius: 12px;
    color: var(--danger);
  }
  .choices {
    display: grid;
    gap: 8px;
    margin-bottom: 12px;
  }
  .choices button {
    display: grid;
    gap: 3px;
    padding: 10px 14px;
    border-radius: 14px;
    text-align: left;
    white-space: normal;
  }
  .choices small {
    font-weight: 400;
    font-size: 0.8rem;
    opacity: 0.85;
  }
  footer {
    display: flex;
    justify-content: flex-end;
  }
</style>
