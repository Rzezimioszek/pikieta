// Warstwy GML. Dokument żyje w silniku (Web Worker z WASM albo natywny Rust w Tauri),
// tu jest tylko lekki indeks, uwagi walidacji i bufor geometrii dla mapy.
import { SvelteSet } from 'svelte/reactivity'
import { epsgOf, type Pt, type XY } from './core'
import { engine, type GmlIndex, type GmlIssue, type GmlSource } from './gml-engine'
import { decode, parts, vertices, type Decoded } from './gmlbuf'
import { saveText, type FileType } from './files.svelte'
import { addPoints, findPoint, notify, project, ui } from './project.svelte'
import { points_to_gml } from './wasm/pikieta.js'

export type { GmlIssue } from './gml-engine'

export const GML: FileType = { name: 'GML', extensions: ['gml', 'xml'], mime: 'application/gml+xml' }

/** Skąd warstwa się wczytuje po ponownym uruchomieniu. */
type Origin = { path: string } | { stored: true }

/** Edycja atrybutu, odtwarzana po ponownym wczytaniu źródła. */
type Edit = [feature: number, key: string, value: string]

export class GmlLayer {
  readonly id: string
  name = $state('')
  visible = $state(true)
  hidden = $state<string[]>([])
  /** Zmieniony względem pliku źródłowego. */
  dirty = $state(false)
  status = $state<'loading' | 'ready' | 'error'>('loading')
  error = $state('')
  index = $state.raw<GmlIndex | null>(null)
  issues = $state.raw<GmlIssue[]>([])
  /** Zmienia się po każdym nowym buforze geometrii (dla mapy). */
  geomRev = $state(0)
  /** Dokument w silniku. */
  doc = ''
  origin: Origin
  edits: Edit[] = []
  decoded: Decoded | null = null
  renderedEpsg = 0
  /** Przybliżyć mapę do warstwy po pierwszym narysowaniu (świeżo wczytana). */
  zoomOnLoad = false

  constructor(id: string, name: string, origin: Origin) {
    this.id = id
    this.name = name
    this.origin = origin
  }

  get count() {
    return this.index?.cls.length ?? 0
  }

  get classes(): [string, number][] {
    const idx = this.index
    if (!idx) return []
    const n = new Array(idx.classes.length).fill(0)
    for (const c of idx.cls) n[c]++
    return idx.classes
      .map((c, i): [string, number] => [c, n[i]])
      .sort((a, b) => classOrder(a[0]) - classOrder(b[0]) || a[0].localeCompare(b[0]))
  }

  get epsg() {
    return this.index?.epsg ?? null
  }

  classOf(i: number) {
    return this.index ? this.index.classes[this.index.cls[i]] : ''
  }
}

const ORDER = [
  'EGB_PunktGraniczny',
  'EGB_Budynek',
  'EGB_DzialkaEwidencyjna',
  'EGB_KonturUzytkuGruntowego',
  'EGB_KonturKlasyfikacyjny',
  'EGB_ObrebEwidencyjny',
  'EGB_JednostkaEwidencyjna',
]
export const classOrder = (c: string) => {
  const i = ORDER.indexOf(c)
  return i < 0 ? ORDER.length : i
}

const CLASS_NAMES: Record<string, string> = {
  EGB_DzialkaEwidencyjna: 'Działki',
  EGB_PunktGraniczny: 'Punkty graniczne',
  EGB_Budynek: 'Budynki',
  EGB_KonturUzytkuGruntowego: 'Kontury użytków',
  EGB_KonturKlasyfikacyjny: 'Kontury klasyfikacyjne',
  EGB_ObrebEwidencyjny: 'Obręby',
  EGB_JednostkaEwidencyjna: 'Jednostki ewidencyjne',
  EGB_Lokal: 'Lokale',
}
export const className = (c: string) => CLASS_NAMES[c] ?? c

export const gml = $state<{ layers: GmlLayer[] }>({ layers: [] })
/** Zaznaczone obiekty GML: klucze `warstwa:numer`. */
export const gmlSelection = new SvelteSet<string>()
export const featureKey = (layer: string, i: number) => `${layer}:${i}`
/** Klasa pokazywana w tabeli obiektów dla każdej warstwy. */
export const gmlTableClass = $state<Record<string, string>>({})

/** Powiadomienia dla mapy (ustawia App). */
export const gmlHooks: { label?: (layer: string, i: number, label: string) => void } = {}

export const findLayer = (id: string) => gml.layers.find((l) => l.id === id)

// --- IndexedDB: ustawienia warstw i źródła plików (localStorage jest za mały) ---

interface Meta {
  id: string
  name: string
  visible: boolean
  hidden: string[]
  dirty: boolean
  order: number
  origin: Origin
  edits: Edit[]
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open('pikieta', 2)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains('gml')) db.createObjectStore('gml', { keyPath: 'id' })
      if (!db.objectStoreNames.contains('gmlsrc')) db.createObjectStore('gmlsrc')
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function tx<T>(store: string, fn: (s: IDBObjectStore) => IDBRequest<T> | void): Promise<T | undefined> {
  try {
    const db = await openDb()
    return await new Promise((resolve, reject) => {
      const t = db.transaction(store, 'readwrite')
      const req = fn(t.objectStore(store))
      t.oncomplete = () => resolve(req ? req.result : undefined)
      t.onerror = () => reject(t.error)
    })
  } catch {
    // Brak IndexedDB (np. tryb prywatny): warstwy działają do zamknięcia strony.
    return undefined
  }
}

export function persistLayer(l: GmlLayer) {
  const meta: Meta = {
    id: l.id,
    name: l.name,
    visible: l.visible,
    hidden: [...l.hidden],
    dirty: l.dirty,
    order: gml.layers.indexOf(l),
    origin: l.origin,
    edits: l.edits,
  }
  tx('gml', (s) => s.put(meta))
}

// --- Wczytywanie ---

async function load(layer: GmlLayer, src: GmlSource) {
  layer.status = 'loading'
  try {
    const opened = await engine.open(src)
    layer.doc = opened.id
    const index = opened.index
    for (const [i, k, v] of layer.edits) index.label[i] = await engine.setProperty(opened.id, i, k, v)
    layer.index = index
    layer.issues = opened.issues
    layer.status = 'ready'
    await render(layer)
  } catch (e) {
    layer.status = 'error'
    layer.error = e instanceof Error ? e.message : String(e)
  }
}

/** Geometria w układzie projektu; po zmianie układu wywoływana ponownie. */
export async function render(layer: GmlLayer) {
  if (layer.status !== 'ready') return
  const epsg = epsgOf(project.crs)
  if (layer.decoded && layer.renderedEpsg === epsg) return
  const buf = await engine.render(layer.doc, epsg)
  layer.decoded = decode(buf)
  layer.renderedEpsg = epsg
  layer.geomRev++
}

const newId = () => crypto.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`

/** Okno wyboru pliku GML: ścieżka w Tauri, plik w przeglądarce. */
async function pickSource(): Promise<{ name: string; src: GmlSource } | null> {
  if (engine.native) {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const path = await open({ multiple: false, filters: [{ name: GML.name, extensions: GML.extensions }] })
    return path ? { name: path.split(/[\\/]/).pop() ?? path, src: { path } } : null
  }
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.gml,.xml'
    input.onchange = () => {
      const f = input.files?.[0]
      resolve(f ? { name: f.name, src: { file: f } } : null)
    }
    input.oncancel = () => resolve(null)
    input.click()
  })
}

function summary(l: GmlLayer) {
  const errors = l.issues.filter((i) => i.severity === 'error').length
  const warnings = l.issues.filter((i) => i.severity === 'warning').length
  return (
    `${l.name}: ${l.count} obiektów` +
    (errors || warnings ? `, błędy: ${errors}, ostrzeżenia: ${warnings}.` : ', bez błędów.')
  )
}

/** Wczytuje plik GML jako nową warstwę (w tle). */
export async function importGml(): Promise<GmlLayer | null> {
  const picked = await pickSource()
  if (!picked) return null
  return addLayer(picked.name, picked.src)
}

export async function addLayer(name: string, src: GmlSource): Promise<GmlLayer | null> {
  const origin: Origin = 'path' in src ? { path: src.path } : { stored: true }
  const layer = new GmlLayer(newId(), name, origin)
  layer.zoomOnLoad = true
  gml.layers.push(layer)
  ui.active = { kind: 'gml', id: layer.id }
  ui.tableTab = layer.id
  if ('file' in src) tx('gmlsrc', (s) => s.put(src.file, layer.id))
  if ('text' in src) tx('gmlsrc', (s) => s.put(new Blob([src.text], { type: 'application/gml+xml' }), layer.id))
  persistLayer(layer)
  await load(layer, src)
  if (layer.status === 'error') {
    notify(`${name}: ${layer.error}`, 'error')
    return null
  }
  notify(summary(layer), layer.issues.some((i) => i.severity === 'error') ? 'error' : 'info')
  return layer
}

export async function restoreLayers() {
  const metas = ((await tx<Meta[]>('gml', (s) => s.getAll() as IDBRequest<Meta[]>)) ?? []).sort(
    (a, b) => a.order - b.order,
  )
  for (const m of metas) {
    // Wpisy z wcześniejszej wersji (bez `origin`) są pomijane.
    if (!m.origin) {
      tx('gml', (s) => s.delete(m.id))
      continue
    }
    if (findLayer(m.id)) continue
    const layer = new GmlLayer(m.id, m.name, m.origin)
    layer.visible = m.visible
    layer.hidden = m.hidden
    layer.dirty = m.dirty
    layer.edits = m.edits ?? []
    gml.layers.push(layer)
  }
  // Wczytywanie po kolei, w tle; interfejs działa w tym czasie.
  for (const layer of gml.layers.filter((l) => l.status === 'loading' && !l.doc)) {
    let src: GmlSource | undefined
    if ('path' in layer.origin) src = { path: layer.origin.path }
    else {
      const blob = await tx<Blob>('gmlsrc', (s) => s.get(layer.id) as IDBRequest<Blob>)
      if (blob) src = { file: blob }
    }
    if (src) await load(layer, src)
    else {
      layer.status = 'error'
      layer.error = 'Brak zapisanego źródła warstwy. Wczytaj plik ponownie.'
    }
  }
}

export async function exportGml(layer: GmlLayer) {
  const suggested = layer.name.toLowerCase().endsWith('.gml') ? layer.name : `${layer.name}.gml`
  try {
    if (engine.native) {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const path = await save({ defaultPath: suggested, filters: [{ name: GML.name, extensions: GML.extensions }] })
      if (!path) return
      await engine.save(layer.doc, path)
      notify(`Zapisano ${path.split(/[\\/]/).pop()}.`)
    } else {
      const name = await saveText(suggested, await engine.xml(layer.doc), GML)
      if (!name) return
      notify(`Zapisano ${name}.`)
    }
    layer.dirty = false
    persistLayer(layer)
  } catch (e) {
    notify(`Nie udało się zapisać: ${e instanceof Error ? e.message : String(e)}`, 'error')
  }
}

export function removeLayer(layer: GmlLayer, ask = true) {
  if (ask && layer.dirty && !confirm(`Warstwa ${layer.name} ma niezapisane zmiany. Usunąć ją mimo to?`)) return
  if (layer.doc) engine.close(layer.doc)
  gml.layers = gml.layers.filter((l) => l !== layer)
  for (const k of [...gmlSelection]) if (k.startsWith(`${layer.id}:`)) gmlSelection.delete(k)
  if (ui.active?.kind === 'gml' && ui.active.id === layer.id) ui.active = null
  if (ui.tableTab === layer.id) ui.tableTab = 'points'
  tx('gml', (s) => s.delete(layer.id))
  tx('gmlsrc', (s) => s.delete(layer.id))
}

/** Zmienia atrybut obiektu; zwraca `false`, gdy rdzeń odmówi. */
export async function setGmlProperty(layer: GmlLayer, feature: number, key: string, value: string): Promise<boolean> {
  try {
    const label = await engine.setProperty(layer.doc, feature, key, value)
    if (layer.index && layer.index.label[feature] !== label) {
      layer.index.label[feature] = label
      layer.index = { ...layer.index }
      gmlHooks.label?.(layer.id, feature, label)
    }
  } catch (e) {
    notify(e instanceof Error ? e.message : String(e), 'error')
    return false
  }
  layer.edits.push([feature, key, value])
  layer.dirty = true
  persistLayer(layer)
  return true
}

// --- Plik projektu ---

export interface ProjectLayer {
  name: string
  text?: string
  path?: string
  visible?: boolean
  hidden?: string[]
  dirty?: boolean
  edits?: Edit[]
}

/** Opis warstw do pliku projektu: ścieżka (desktop) albo treść, gdy plik jest mały. */
export async function layersForProject(): Promise<ProjectLayer[]> {
  const out: ProjectLayer[] = []
  for (const l of gml.layers) {
    const base = { name: l.name, visible: l.visible, hidden: [...l.hidden], dirty: l.dirty }
    if ('path' in l.origin) out.push({ ...base, path: l.origin.path, edits: l.edits })
    else if (l.status === 'ready') {
      const text = await engine.xml(l.doc)
      if (text.length <= 30_000_000) out.push({ ...base, text })
      else notify(`${l.name} jest za duży, żeby zapisać go w pliku projektu; wczytaj go osobno.`, 'error')
    }
  }
  return out
}

/** Zastępuje warstwy warstwami z pliku projektu. */
export function replaceLayers(list: ProjectLayer[]) {
  for (const l of [...gml.layers]) removeLayer(l, false)
  for (const rec of list) {
    const src: GmlSource | null = rec.path ? { path: rec.path } : rec.text !== undefined ? { text: rec.text } : null
    if (!src) continue
    const layer = new GmlLayer(newId(), rec.name, rec.path ? { path: rec.path } : { stored: true })
    layer.visible = rec.visible ?? true
    layer.hidden = rec.hidden ?? []
    layer.dirty = rec.dirty ?? false
    layer.edits = rec.edits ?? []
    gml.layers.push(layer)
    if (rec.text !== undefined) tx('gmlsrc', (s) => s.put(new Blob([rec.text!]), layer.id))
    persistLayer(layer)
    load(layer, src)
  }
}

// --- Punkty ---

/** Wierzchołki obiektu w układzie projektu (np. do przybliżenia). */
export function featureXY(layer: GmlLayer, i: number): XY[] {
  return layer.decoded ? vertices(layer.decoded, i) : []
}

/** Przenosi punkty graniczne EGiB do punktów projektu (numer: część identyfikatora po ostatniej kropce). */
export function boundaryPointsToProject(layer: GmlLayer) {
  const idx = layer.index
  const d = layer.decoded
  if (!idx || !d) return
  const ci = idx.classes.indexOf('EGB_PunktGraniczny')
  const pts: Pt[] = []
  for (let i = 0; i < idx.cls.length; i++) {
    if (idx.cls[i] !== ci) continue
    const p = parts(d, i)[0]
    if (p?.t !== 'point') continue
    pts.push({ id: idx.label[i], x: d.coords[2 * p.c + 1], y: d.coords[2 * p.c], h: null, code: 'PG' })
  }
  if (pts.length === 0) return notify('Warstwa nie ma punktów granicznych.', 'error')
  const fresh = pts.filter((p) => !findPoint(p.id))
  addPoints(fresh)
  notify(
    `Dodano ${fresh.length} punktów granicznych` +
      (pts.length > fresh.length ? `; pominięto ${pts.length - fresh.length} o numerach już istniejących.` : '.'),
  )
}

export async function exportPointsGml() {
  if (project.points.length === 0) return notify('Brak punktów do eksportu.', 'error')
  const text = points_to_gml(
    project.points.map((p) => p.id),
    project.points.flatMap((p) => [p.x, p.y, p.h ?? NaN]),
    project.points.map((p) => p.code),
    epsgOf(project.crs),
    project.precision.coord,
  )
  const name = await saveText('punkty.gml', text, GML)
  if (name) notify(`Zapisano ${project.points.length} pkt do ${name}.`)
}
