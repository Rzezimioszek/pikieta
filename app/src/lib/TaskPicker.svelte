<script lang="ts">
  import Icon from './Icon.svelte'
  import { CALC_GROUPS, CALCS, type CalcType } from './calcs'
  import { recentTypes, startCalc, taskPicker } from './project.svelte'

  /** Słowa, pod którymi ludzie szukają zadania, a których nie ma w nazwie ani opisie. */
  const KEYWORDS: Record<CalcType, string> = {
    azimuth: 'kierunek odległość długość odcinka',
    polar: 'biegunowe kąt odległość stanowisko nowe punkty',
    offset: 'miary bieżące domiary prostopadłe rzut ortogonalna',
    fromOffset: 'miary bieżące domiary ortogonalna',
    distInt: 'przecięcie łuków dwie odległości',
    angleInt: 'w przód kąty',
    combinedInt: 'kombinowane nadliczbowe wyrównanie mnk',
    lineInt: 'przecięcie kierunków prostych',
    resection: 'wstecz snellius pothenot stanowisko',
    construct: 'prostopadła równoległa okrąg łuk podział odcinka punkt ukryty mimośród',
    traverse: 'poligon poligonowy osnowa odchyłka',
    tachy: 'pikiety pomiar sytuacyjny wysokościowy xyh stanowisko orientacja',
    stakeout: 'tyczenie wytyczenie realizacja elementy biegunowe',
    transform: 'helmert afiniczna hausbrandt przeliczenie układu lokalny dopasowanie',
    area: 'powierzchnia gauss działka hektary',
    areaAdjust: 'użytki kontury odchyłka rozrzucenie zaokrąglenie',
    division: 'podział działki wydzielenie nowe granice',
  }

  /** Porównanie bez wielkości liter i polskich znaków: „wciecie” znajduje „Wcięcie”. */
  const fold = (s: string) =>
    s
      .toLowerCase()
      .replace(/ł/g, 'l')
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
  const GROUP_OF = Object.fromEntries(CALC_GROUPS.flatMap((g) => g.types.map((t) => [t, g.label]))) as Record<CalcType, string>
  const TEXT = Object.fromEntries(
    (Object.keys(CALCS) as CalcType[]).map((t) => [t, fold([CALCS[t].label, CALCS[t].hint, GROUP_OF[t], KEYWORDS[t]].join(' '))]),
  ) as Record<CalcType, string>

  let dialog = $state<HTMLDialogElement>()
  let search = $state<HTMLInputElement>()
  let list = $state<HTMLElement>()
  let query = $state('')

  const words = $derived(fold(query).split(/\s+/).filter(Boolean))
  const groups = $derived(
    CALC_GROUPS.filter((g) => words.length > 0 || !taskPicker.group || g.label === taskPicker.group)
      .map((g) => ({ label: g.label, types: g.types.filter((t) => words.every((w) => TEXT[t].includes(w))) }))
      .filter((g) => g.types.length),
  )
  const recent = $derived(taskPicker.open && !words.length && !taskPicker.group ? recentTypes(4) : [])

  $effect(() => {
    if (!dialog) return
    if (taskPicker.open && !dialog.open) {
      query = ''
      dialog.showModal()
      search?.focus()
    } else if (!taskPicker.open && dialog.open) dialog.close()
  })

  function pick(t: CalcType) {
    taskPicker.open = false
    startCalc(t)
  }

  /** Strzałki przechodzą po kafelkach w kolejności na liście; z pola wyszukiwania na pierwszy. */
  function move(e: KeyboardEvent) {
    const items = [...(list?.querySelectorAll<HTMLButtonElement>('button.item') ?? [])]
    const at = items.indexOf(document.activeElement as HTMLButtonElement)
    let to = -1
    if (e.key === 'ArrowDown') to = at + 1
    else if (e.key === 'ArrowUp') to = at - 1
    else if (e.key === 'Home' && at >= 0) to = 0
    else if (e.key === 'End' && at >= 0) to = items.length - 1
    else return
    e.preventDefault()
    if (to < 0) search?.focus()
    else items[Math.min(to, items.length - 1)]?.focus()
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      const first = groups[0]?.types[0]
      if (first) pick(first)
    } else move(e)
  }
</script>

{#snippet tile(t: CalcType)}
  <button class="item" onclick={() => pick(t)}>
    <span class="ic"><Icon name={CALCS[t].icon} size={22} /></span>
    <span class="txt">
      <strong>{CALCS[t].label}</strong>
      <small>{CALCS[t].hint}</small>
    </span>
  </button>
{/snippet}

<dialog
  bind:this={dialog}
  class="picker surface"
  aria-labelledby="picker-title"
  onclose={() => (taskPicker.open = false)}
  onclick={(e) => e.target === dialog && (taskPicker.open = false)}
  onkeydown={(e) => (e.target as HTMLElement).matches('button.item') && move(e)}
>
  <header>
    <h2 id="picker-title">Nowe obliczenie</h2>
    <span class="hint">Ctrl+K</span>
    <button class="close" aria-label="Zamknij" onclick={() => (taskPicker.open = false)}><Icon name="x" /></button>
  </header>

  <label class="search">
    <Icon name="search" size={16} />
    <input
      bind:this={search}
      bind:value={query}
      oninput={() => (taskPicker.group = '')}
      onkeydown={onSearchKey}
      placeholder="Szukaj: wcięcie, Helmert, pole…"
      aria-label="Szukaj zadania"
      aria-describedby="picker-help"
    />
  </label>
  <p id="picker-help" class="sr-only">Enter otwiera pierwsze pasujące zadanie, strzałka w dół przechodzi do listy.</p>

  <div class="chips" role="group" aria-label="Grupa zadań">
    <button class="chip" aria-pressed={!taskPicker.group && !words.length} onclick={() => ((taskPicker.group = ''), (query = ''))}>
      Wszystkie
    </button>
    {#each CALC_GROUPS as g (g.label)}
      <button class="chip" aria-pressed={taskPicker.group === g.label} onclick={() => ((taskPicker.group = g.label), (query = ''))}>
        {g.label}
      </button>
    {/each}
  </div>

  <div class="list" bind:this={list}>
    {#if recent.length}
      <section>
        <h3>Ostatnio używane</h3>
        <div class="grid">
          {#each recent as t (t)}{@render tile(t)}{/each}
        </div>
      </section>
    {/if}
    {#each groups as g (g.label)}
      <section>
        <h3>{g.label}</h3>
        <div class="grid">
          {#each g.types as t (t)}{@render tile(t)}{/each}
        </div>
      </section>
    {/each}
    {#if !groups.length}
      <p class="hint">Nic nie pasuje do „{query}”. Szukaj po nazwie albo słowie z opisu, np. „wcięcie”, „Helmert”, „pole”.</p>
    {/if}
  </div>
</dialog>

<style>
  .picker {
    width: min(820px, calc(100vw - 32px));
    max-height: min(82vh, 760px);
    /* Globalny reset marginesów zdejmuje wyśrodkowanie okna modalnego. */
    margin: auto;
    position: fixed;
    padding: 0;
    color: var(--ink);
    background: var(--menu-bg);
  }
  .picker[open] {
    display: grid;
    grid-template-rows: auto auto auto 1fr;
  }
  .picker::backdrop {
    background: rgb(0 0 0 / 0.45);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 14px 8px 18px;
  }
  h2 {
    margin: 0;
    font-size: 1.05rem;
  }
  header .hint {
    margin-right: auto;
  }
  .close {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 11px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 18px;
    padding: 0 12px;
    border-radius: 12px;
    border: 1px solid var(--control-edge, var(--field-edge));
    background: var(--field-bg);
    color: var(--ink-soft);
  }
  .search input {
    flex: 1;
    padding: 10px 0;
    border: none;
    background: none;
    box-shadow: none;
    font-size: 0.95rem;
  }
  .search:focus-within {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .search input:focus-visible {
    outline: none;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 18px 4px;
  }
  .chip {
    padding: 4px 12px;
    border-radius: 999px;
    font-size: 0.8rem;
    font-weight: 550;
  }
  .chip[aria-pressed='true'] {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    border-color: var(--accent);
  }
  .list {
    display: grid;
    align-content: start;
    gap: 14px;
    min-height: 0;
    overflow: auto;
    padding: 10px 18px 18px;
    scrollbar-width: thin;
  }
  h3 {
    margin: 0 0 6px;
    font-size: 0.72rem;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 8px;
  }
  .item {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 16px;
    text-align: left;
    font-weight: 400;
    white-space: normal;
  }
  .ic {
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
  }
  .txt {
    display: grid;
    gap: 3px;
  }
  strong {
    font-size: 0.9rem;
    font-weight: 600;
  }
  small {
    font-size: 0.78rem;
    line-height: 1.35;
    color: var(--ink-soft);
  }
  @media (max-width: 600px) {
    .picker {
      width: 100vw;
      max-width: 100vw;
      height: 100dvh;
      max-height: 100dvh;
      margin: 0;
      border-radius: 0;
    }
    header .hint {
      visibility: hidden;
    }
  }
</style>
