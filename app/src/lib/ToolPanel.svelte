<script lang="ts">
  import type { Component } from 'svelte'
  import { untrack } from 'svelte'
  import Icon from './Icon.svelte'
  import Launcher from './Launcher.svelte'
  import ReportView from './ReportView.svelte'
  import GmlView from './GmlView.svelte'
  import { findLayer, type GmlLayer } from './gml.svelte'
  import ViewSwitch from './ViewSwitch.svelte'
  import { CALCS, calcTitle, type CalcType } from './calcs'
  import AngleInt from './calcs/AngleInt.svelte'
  import CombinedInt from './calcs/CombinedInt.svelte'
  import Construct from './calcs/Construct.svelte'
  import Area from './calcs/Area.svelte'
  import Azimuth from './calcs/Azimuth.svelte'
  import DistInt from './calcs/DistInt.svelte'
  import FromOffset from './calcs/FromOffset.svelte'
  import LineInt from './calcs/LineInt.svelte'
  import Offset from './calcs/Offset.svelte'
  import Polar from './calcs/Polar.svelte'
  import Resection from './calcs/Resection.svelte'
  import Stakeout from './calcs/Stakeout.svelte'
  import Tachy from './calcs/Tachy.svelte'
  import Traverse from './calcs/Traverse.svelte'
  import type { Sketch } from './map'
  import { closeCalc, findCalc, picker, project, startCalc, ui } from './project.svelte'

  interface Props {
    ready: boolean
    epsg: number
    crsLabel: string
    onsketch: (s: Sketch) => void
    onzoomgml: (target: string[] | { layer: string }) => void
    onfocusgml: (layer: GmlLayer, feature: number) => void
  }
  let { ready, epsg, crsLabel, onsketch, onzoomgml, onfocusgml }: Props = $props()

  const gmlLayer = $derived(ui.active?.kind === 'gml' ? findLayer(ui.active.id) : undefined)

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const VIEWS: Record<CalcType, Component<any>> = {
    azimuth: Azimuth,
    polar: Polar,
    offset: Offset,
    fromOffset: FromOffset,
    distInt: DistInt,
    lineInt: LineInt,
    area: Area,
    traverse: Traverse,
    tachy: Tachy,
    stakeout: Stakeout,
    resection: Resection,
    angleInt: AngleInt,
    combinedInt: CombinedInt,
    construct: Construct,
  }

  const session = $derived(ui.active?.kind === 'calc' ? findCalc(ui.active.id) : undefined)
  const View = $derived(session ? VIEWS[session.type] : null)

  let renaming = $state(false)
  let nameDraft = $state('')

  // Zmiana danych obliczenia przesuwa je na górę historii; samo otwarcie nie.
  let seen = { id: '', snap: '' }
  $effect(() => {
    const s = session
    if (!s) return
    const snap = JSON.stringify(s.data)
    untrack(() => {
      if (seen.id === s.id && seen.snap !== snap) s.updated = Date.now()
      seen = { id: s.id, snap }
    })
  })

  // Pole czekające na punkt z mapy należy do poprzedniego okna.
  $effect(() => {
    void ui.active
    untrack(() => (picker.fill = null))
  })

  $effect(() => {
    if (!session) onsketch({ paths: [] })
  })

  function finishRename() {
    if (!session) return
    const name = nameDraft.trim()
    session.name = name === '' || name === CALCS[session.type].title(session.data) ? null : name
    renaming = false
  }

  function focusOnMount(el: HTMLInputElement) {
    el.focus()
    el.select()
  }
</script>

<section class="tool surface">
  <header>
    {#if session}
      <span class="ic"><Icon name={CALCS[session.type].icon} size={20} /></span>
      <div class="titles">
        {#if renaming}
          <input
            class="rename"
            aria-label="Nazwa obliczenia"
            bind:value={nameDraft}
            use:focusOnMount
            onblur={finishRename}
            onkeydown={(e) => {
              if (e.key === 'Enter') finishRename()
              if (e.key === 'Escape') renaming = false
            }}
          />
        {:else}
          <button
            class="title"
            title="Kliknij, aby zmienić nazwę"
            onclick={() => {
              nameDraft = calcTitle(session)
              renaming = true
            }}>{calcTitle(session)}</button
          >
        {/if}
        <span class="sub">{CALCS[session.type].label} · {crsLabel}</span>
      </div>
    {:else if ui.active?.kind === 'report'}
      <span class="ic"><Icon name="report" size={20} /></span>
      <div class="titles">
        <strong class="title">Raport obliczeń</strong>
        <span class="sub">Pozycje: {project.report.length} · {crsLabel}</span>
      </div>
    {:else if gmlLayer}
      <span class="ic"><Icon name="layers" size={20} /></span>
      <div class="titles">
        <strong class="title" title={gmlLayer.name}>{gmlLayer.name}</strong>
        <span class="sub">
          Warstwa GML · {gmlLayer.status === 'loading' ? 'wczytywanie…' : `${gmlLayer.count.toLocaleString('pl-PL')} obiektów`}{gmlLayer.dirty ? ' · zmieniona' : ''}
        </span>
      </div>
    {:else}
      <span class="ic"><Icon name="panel" size={20} /></span>
      <div class="titles">
        <strong class="title">Obliczenia</strong>
        <span class="sub">Wybierz zadanie, żeby otworzyć nowe obliczenie</span>
      </div>
    {/if}
    <div class="right">
      <ViewSwitch />
      {#if session}
        <button class="close" title="Zamknij (zostaje w historii)" onclick={() => closeCalc(session.id)}>
          <Icon name="x" />
        </button>
      {:else if ui.active?.kind === 'report' || gmlLayer}
        <button class="close" title="Zamknij" onclick={() => (ui.active = null)}><Icon name="x" /></button>
      {/if}
    </div>
  </header>

  <div class="body">
    {#if !ready}
      <p class="hint">Ładowanie rdzenia obliczeniowego…</p>
    {:else if session && View}
      {#key session.id}
        <View data={session.data} {epsg} title={calcTitle(session)} {crsLabel} {onsketch} />
      {/key}
    {:else if ui.active?.kind === 'report'}
      <ReportView {crsLabel} />
    {:else if gmlLayer}
      {#key gmlLayer.id}<GmlView layer={gmlLayer} onzoom={onzoomgml} onfocus={onfocusgml} />{/key}
    {:else}
      <Launcher onpick={startCalc} />
    {/if}
  </div>
</section>

<style>
  .tool {
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 16px;
    border-bottom: 1px solid var(--field-edge);
  }
  .ic {
    display: grid;
    place-items: center;
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    color: var(--accent);
  }
  .titles {
    display: grid;
    min-width: 0;
    flex: 1;
  }
  .title {
    justify-self: start;
    max-width: 100%;
    padding: 0;
    border: none;
    background: none;
    font-size: 1.02rem;
    font-weight: 650;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  button.title:hover {
    background: none !important;
    text-decoration: underline dotted;
  }
  .rename {
    padding: 4px 8px;
    font-size: 1rem;
  }
  .sub {
    font-size: 0.76rem;
    color: var(--ink-soft);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .close {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border-radius: 11px;
  }
  .body {
    overflow: auto;
    min-height: 0;
    padding: 16px;
    container-type: inline-size;
    scrollbar-width: thin;
  }
</style>
