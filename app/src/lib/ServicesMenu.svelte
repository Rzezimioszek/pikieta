<script lang="ts">
  import Icon from './Icon.svelte'
  import { project, ui } from './project.svelte'
  import { CRS_LIST } from './map'
  import { grouped, OVERLAYS, requestCrs, type WmsService } from './services'

  let open = $state(false)
  let button = $state<HTMLButtonElement>()
  let panel = $state<HTMLDivElement>()
  let pos = $state({ top: 0, left: 0 })

  const groups = grouped(OVERLAYS)
  const crsName = (code: string) => CRS_LIST.find((c) => c.code === code)?.label ?? code

  /** Powód, dla którego usługi nie da się pokazać w układzie projektu (albo pusty). */
  function unavailable(s: WmsService) {
    if (requestCrs(s, project.crs)) return ''
    return `Usługa działa tylko w układach: ${s.crs!.map(crsName).join(', ')}.`
  }

  function toggle(id: string) {
    ui.overlays = ui.overlays.includes(id) ? ui.overlays.filter((o) => o !== id) : [...ui.overlays, id]
  }

  function show() {
    const r = button!.getBoundingClientRect()
    // Panel pod przyciskiem, ale w granicach okna.
    pos = { top: r.bottom + 6, left: Math.max(8, Math.min(r.left, window.innerWidth - 380)) }
    open = !open
  }

  /** Fokus do panelu po otwarciu: panel jest na końcu dokumentu, więc Tab sam by do niego nie doszedł. */
  function focusFirst(node: HTMLElement) {
    queueMicrotask(() => node.querySelector<HTMLElement>('input:not(:disabled), button:not(:disabled)')?.focus())
  }

  function close() {
    open = false
    button?.focus()
  }

  /** Tab z ostatniego elementu panelu wraca do przycisku, a nie na koniec strony. */
  function onPanelKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation()
      close()
    } else if (e.key === 'Tab') {
      const items = [...panel!.querySelectorAll<HTMLElement>('input:not(:disabled), button:not(:disabled)')]
      const edge = e.shiftKey ? items[0] : items.at(-1)
      if (document.activeElement === edge) {
        e.preventDefault()
        close()
      }
    }
  }

  /** Panel w `body`: wstążka ma `backdrop-filter`, który zamyka `position: fixed` w jej warstwie. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node)
    return { destroy: () => node.remove() }
  }

  function outside(e: PointerEvent) {
    if (open && !panel?.contains(e.target as Node) && !button?.contains(e.target as Node)) open = false
  }
</script>

<svelte:window onpointerdown={outside} onkeydown={(e) => e.key === 'Escape' && open && close()} />

<button
  bind:this={button}
  class="big"
  class:on={ui.overlays.length > 0}
  aria-expanded={open}
  aria-haspopup="dialog"
  aria-controls={open ? 'services-panel' : undefined}
  title="Nakładki z usług GUGiK i innych instytucji"
  onclick={show}
>
  <Icon name="layers" size={22} />
  <span>Nakładki{ui.overlays.length ? ` (${ui.overlays.length})` : ''}</span>
</button>

{#if open}
  <div
    use:portal
    use:focusFirst
    bind:this={panel}
    id="services-panel"
    class="panel surface"
    style:top="{pos.top}px"
    style:left="{pos.left}px"
    role="dialog"
    aria-label="Nakładki"
    tabindex="-1"
    onkeydown={onPanelKey}
  >
    <div class="head">
      <strong>Nakładki</strong>
      <button class="small" disabled={ui.overlays.length === 0} onclick={() => (ui.overlays = [])}>Wyłącz wszystkie</button>
    </div>
    <div class="list">
      {#each groups as [group, list] (group)}
        <p class="group">{group}</p>
        {#each list as s (s.id)}
          {@const why = unavailable(s)}
          <label class="item" class:off={!!why} title={why || s.attribution}>
            <input type="checkbox" checked={ui.overlays.includes(s.id)} disabled={!!why} onchange={() => toggle(s.id)} />
            <span>{s.label}</span>
          </label>
        {/each}
      {/each}
    </div>
    <p class="note">Dane z usług WMS, rysowane nad podkładem. Część usług pokazuje treść dopiero po przybliżeniu.</p>
  </div>
{/if}

<style>
  .big {
    display: grid;
    justify-items: center;
    gap: 3px;
    min-width: 64px;
    padding: 6px 8px;
    border: none;
    border-radius: 12px;
    background: transparent;
    font-size: 0.74rem;
    font-weight: 550;
    color: var(--ink);
  }
  .big :global(svg) {
    color: var(--accent);
  }
  .big.on {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .panel {
    position: fixed;
    z-index: 50;
    display: grid;
    gap: 8px;
    width: min(370px, calc(100vw - 16px));
    max-height: min(70vh, 560px);
    padding: 12px;
    border-radius: 16px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .list {
    overflow: auto;
    display: grid;
    gap: 2px;
  }
  .group {
    margin: 8px 0 2px;
    font-size: 0.7rem;
    font-weight: 650;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }
  .group:first-child {
    margin-top: 0;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px;
    border-radius: 8px;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .item:hover {
    background: var(--field-bg);
  }
  .item.off {
    opacity: 0.5;
    cursor: default;
  }
  .item input {
    width: auto;
    margin: 0;
  }
  .note {
    font-size: 0.72rem;
    color: var(--ink-soft);
  }
</style>
