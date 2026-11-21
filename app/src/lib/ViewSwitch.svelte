<script lang="ts">
  import Icon from './Icon.svelte'
  import { ui, type ViewMode } from './project.svelte'
  import type { IconName } from './icons'

  const MODES: [ViewMode, IconName, string][] = [
    ['map', 'map', 'Mapa z panelem narzędzia'],
    ['split', 'split', 'Podział 50/50'],
    ['calc', 'panel', 'Obliczenia na pełnym ekranie'],
  ]
</script>

<div class="switch" role="radiogroup" aria-label="Tryb widoku">
  {#each MODES as [mode, icon, label] (mode)}
    <button
      role="radio"
      aria-checked={ui.view === mode}
      aria-label={label}
      title={label}
      class:on={ui.view === mode}
      onclick={() => (ui.view = mode)}
    >
      <Icon name={icon} size={17} />
    </button>
  {/each}
</div>

<style>
  .switch {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 13px;
    background: var(--glass-bg);
    -webkit-backdrop-filter: blur(18px) saturate(170%);
    backdrop-filter: blur(18px) saturate(170%);
    border: 1px solid var(--glass-edge);
    box-shadow: 0 6px 18px var(--glass-shadow);
  }
  button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 30px;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: transparent;
    color: var(--ink-soft);
  }
  button.on {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
