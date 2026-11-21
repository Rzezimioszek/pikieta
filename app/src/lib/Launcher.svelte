<script lang="ts">
  import Icon from './Icon.svelte'
  import { CALC_GROUPS, CALCS, type CalcType } from './calcs'

  let { onpick, compact = false }: { onpick: (t: CalcType) => void; compact?: boolean } = $props()
</script>

<div class="launcher" class:compact>
  {#each CALC_GROUPS as g (g.label)}
    <section>
      <h4>{g.label}</h4>
      <div class="grid">
        {#each g.types as t (t)}
          <button class="item" onclick={() => onpick(t)}>
            <span class="ic"><Icon name={CALCS[t].icon} size={compact ? 18 : 22} /></span>
            <span class="txt">
              <strong>{CALCS[t].label}</strong>
              {#if !compact}<small>{CALCS[t].hint}</small>{/if}
            </span>
          </button>
        {/each}
      </div>
    </section>
  {/each}
</div>

<style>
  .launcher {
    display: grid;
    gap: 16px;
  }
  .compact {
    gap: 8px;
  }
  h4 {
    margin-bottom: 6px;
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
  .compact .grid {
    grid-template-columns: 1fr;
    gap: 2px;
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
  .compact .item {
    align-items: center;
    padding: 7px 10px;
    border-radius: 10px;
    border-color: transparent;
    background: transparent;
  }
  .ic {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
  }
  .compact .ic {
    width: 28px;
    height: 28px;
    border-radius: 8px;
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
</style>
