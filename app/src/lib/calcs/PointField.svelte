<script lang="ts">
  import { tick } from 'svelte'
  import { coords } from './util'
  import { findPoint, picker } from '../project.svelte'

  interface Props {
    label: string
    value: string
  }
  let { label, value = $bindable() }: Props = $props()

  let input = $state<HTMLInputElement>()
  const p = $derived(findPoint(value))
  const missing = $derived(value.trim() !== '' && !p)

  /** Po wskazaniu punktu na mapie przechodzi do kolejnego pola punktu w tym samym oknie. */
  function arm() {
    picker.label = label
    picker.fill = async (id) => {
      value = id
      await tick()
      const all = [...(input?.closest('.calc')?.querySelectorAll<HTMLInputElement>('input[data-pick]') ?? [])]
      const next = all[all.indexOf(input!) + 1]
      next?.focus()
    }
  }
</script>

<label class="field pf">
  {label}
  <input
    bind:this={input}
    bind:value
    data-pick
    list="point-ids"
    placeholder="nr punktu"
    autocomplete="off"
    onfocus={arm}
  />
  <small class:error={missing}>{missing ? 'brak punktu w projekcie' : p ? coords(p) : ' '}</small>
</label>

<style>
  small {
    min-height: 1.2em;
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    color: var(--ink-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  small.error {
    color: var(--danger);
  }
</style>
