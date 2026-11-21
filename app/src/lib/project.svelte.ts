// Stan projektu (punkty, obliczenia, raport) i interfejsu, zapisywany w pamięci przeglądarki.
import { SvelteSet } from 'svelte/reactivity'
import { CALCS, type CalcType } from './calcs'
import type { Pt } from './core'
import type { BasemapId } from './map'

/** Pozycja raportu; układ odpowiada `pikieta_report::Entry`. */
export interface ReportEntry {
  title: string
  params?: [string, string][]
  table?: { head: string[]; rows: string[][]; right: boolean[] }
  results?: [string, string][]
  notes?: string[]
  /** Wpisy z wcześniejszych wersji. */
  lines?: string[]
}

export interface Precision {
  coord: number
  length: number
  angle: number
  area: number
}
export const DEFAULT_PRECISION: Precision = { coord: 2, length: 3, angle: 4, area: 2 }

/** Jedno obliczenie, widoczne na liście po lewej jak rozmowa w czacie. */
export interface CalcSession {
  id: string
  type: CalcType
  /** Nazwa nadana przez użytkownika; `null` oznacza nazwę automatyczną. */
  name: string | null
  open: boolean
  created: number
  updated: number
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  data: any
}

export interface Saved {
  crs: string
  points: Pt[]
  report: ReportEntry[]
  calcs: CalcSession[]
  precision: Precision
}

export type ViewMode = 'map' | 'split' | 'calc'
export type RibbonTab = 'project' | 'calc' | 'map' | 'view'
export type ActiveDoc = { kind: 'calc'; id: string } | { kind: 'report' } | null

interface Ui {
  view: ViewMode
  sidebar: boolean
  table: boolean
  tableHeight: number
  ribbonTab: RibbonTab
  ribbonCollapsed: boolean
  active: ActiveDoc
  /** Kolejność kolumn w importowanych i eksportowanych plikach: Nr Y X. */
  yx: boolean
  basemap: BasemapId
  addByClick: boolean
}

const PROJECT_KEY = 'pikieta.project.v1'
const UI_KEY = 'pikieta.ui.v1'
/** Najwięcej tylu obliczeń zostaje w historii; otwarte nie są usuwane. */
const MAX_CALCS = 100

function read<T>(key: string): Partial<T> {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as Partial<T>) : {}
  } catch {
    return {}
  }
}

function write(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Tryb prywatny albo pełna pamięć: stan zostaje tylko w tej sesji.
  }
}

const saved = read<Saved>(PROJECT_KEY)
export const project = $state<Saved>({
  crs: typeof saved.crs === 'string' ? saved.crs : 'EPSG:2178',
  points: Array.isArray(saved.points) ? saved.points : [],
  report: Array.isArray(saved.report) ? saved.report : [],
  calcs: Array.isArray(saved.calcs) ? saved.calcs.filter((s) => s.type in CALCS) : [],
  precision: { ...DEFAULT_PRECISION, ...saved.precision },
})

export function saveProject() {
  write(PROJECT_KEY, project)
}

const UI_DEFAULTS: Ui = {
  view: 'map',
  sidebar: true,
  table: true,
  tableHeight: 240,
  ribbonTab: 'calc',
  ribbonCollapsed: false,
  active: null,
  yx: false,
  basemap: 'orto',
  addByClick: false,
}
export const ui = $state<Ui>({ ...UI_DEFAULTS, ...read<Ui>(UI_KEY), addByClick: false })
if (ui.active?.kind === 'calc' && !project.calcs.some((s) => s.id === (ui.active as { id: string }).id)) {
  ui.active = null
}

export function saveUi() {
  write(UI_KEY, ui)
}

/** Stan zależny od urządzenia, nie zapisywany. */
export const env = $state({ narrow: false, core: false })

/** Numery zaznaczonych punktów, wspólne dla tabeli i mapy. */
export const selection = new SvelteSet<string>()

/** Pole czekające na wskazanie punktu kliknięciem na mapie. */
export const picker = $state<{ fill: ((id: string) => void) | null; label: string }>({ fill: null, label: '' })

// --- Komunikaty ---

export interface Toast {
  id: number
  text: string
  kind: 'info' | 'error'
}
export const toasts = $state<Toast[]>([])
let toastSeq = 0

export function notify(text: string, kind: Toast['kind'] = 'info') {
  const id = ++toastSeq
  toasts.push({ id, text, kind })
  setTimeout(
    () => {
      const i = toasts.findIndex((t) => t.id === id)
      if (i >= 0) toasts.splice(i, 1)
    },
    kind === 'error' ? 8000 : 4500,
  )
}

// --- Punkty ---

class PointIndex {
  byId = $derived(new Map(project.points.map((p) => [p.id, p])))
}
const index = new PointIndex()

export function findPoint(id: string | undefined): Pt | undefined {
  const key = id?.trim() ?? ''
  return key === '' ? undefined : index.byId.get(key)
}

/** Kolejny wolny numer: o jeden większy od największego numeru liczbowego. */
export function nextId(): string {
  let max = 0
  for (const p of project.points) {
    const n = Number(p.id)
    if (Number.isInteger(n) && n > max) max = n
  }
  return String(max + 1)
}

/** Dodaje punkty; punkt o istniejącym numerze zastępuje stary. Zwraca liczbę zastąpionych. */
export function addPoints(points: Pt[]): number {
  const at = new Map(project.points.map((p, i) => [p.id, i]))
  let replaced = 0
  for (const p of points) {
    const i = at.get(p.id)
    if (i === undefined) {
      at.set(p.id, project.points.length)
      project.points.push(p)
    } else {
      project.points[i] = p
      replaced++
    }
  }
  return replaced
}

/** Dodaje punkty wynikowe obliczeń; istniejących numerów nie nadpisuje. */
export function addNewPoints(points: Pt[]) {
  const added = points.filter((p) => !findPoint(p.id))
  const skipped = points.length - added.length
  addPoints(added)
  if (added.length === 0) {
    notify('Nie dodano punktów: wszystkie numery już istnieją.', 'error')
    return
  }
  const ids = added.map((p) => p.id)
  notify(
    `Dodano ${ids.length === 1 ? `punkt ${ids[0]}` : `${ids.length} pkt (${ids.slice(0, 5).join(', ')}${ids.length > 5 ? '…' : ''})`}` +
      (skipped ? `; pominięto ${skipped} istniejących.` : '.'),
  )
}

export function removePoints(ids: Iterable<string>) {
  const drop = new Set(ids)
  project.points = project.points.filter((p) => !drop.has(p.id))
  for (const id of drop) selection.delete(id)
}

export function renamePoint(from: string, to: string): boolean {
  const id = to.trim()
  if (id === from) return true
  if (id === '') {
    notify('Numer punktu nie może być pusty.', 'error')
    return false
  }
  if (findPoint(id)) {
    notify(`Punkt ${id} już istnieje.`, 'error')
    return false
  }
  const p = findPoint(from)
  if (!p) return false
  p.id = id
  if (selection.delete(from)) selection.add(id)
  return true
}

// --- Obliczenia ---

const newId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(36).slice(2)}`

export function findCalc(id: string): CalcSession | undefined {
  return project.calcs.find((s) => s.id === id)
}

function show(doc: ActiveDoc) {
  ui.active = doc
  if (env.narrow) {
    if (ui.view === 'map') ui.view = 'split'
    ui.sidebar = false
  }
}

export function startCalc(type: CalcType) {
  const now = Date.now()
  project.calcs.unshift({ id: newId(), type, name: null, open: true, created: now, updated: now, data: CALCS[type].init() })
  const old = project.calcs.filter((s) => !s.open).slice(MAX_CALCS)
  if (old.length) project.calcs = project.calcs.filter((s) => !old.includes(s))
  show({ kind: 'calc', id: project.calcs[0].id })
}

export function openCalc(id: string) {
  const s = findCalc(id)
  if (!s) return
  s.open = true
  show({ kind: 'calc', id })
}

/** Po zamknięciu aktywnego dokumentu pokazuje ostatnio używane otwarte obliczenie. */
function activateNext(closed: string) {
  if (ui.active?.kind !== 'calc' || ui.active.id !== closed) return
  const next = project.calcs.filter((s) => s.open && s.id !== closed).sort((a, b) => b.updated - a.updated)[0]
  ui.active = next ? { kind: 'calc', id: next.id } : null
}

export function closeCalc(id: string) {
  const s = findCalc(id)
  if (s) s.open = false
  activateNext(id)
}

export function deleteCalc(id: string) {
  activateNext(id)
  project.calcs = project.calcs.filter((s) => s.id !== id)
}

export function showReport() {
  show({ kind: 'report' })
}

export function addReport(entry: ReportEntry) {
  project.report.push(entry)
  notify(`Zapisano w raporcie jako pozycję ${project.report.length}.`)
}

// --- Pliki ---

export function downloadText(filename: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: 'text/plain;charset=utf-8' }))
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

/** Dane raportu dla rdzenia (`pikieta_report::Report`). */
export function reportData(crsLabel: string) {
  return {
    title: 'Raport obliczeń',
    date: new Date().toLocaleString('pl-PL'),
    crs: crsLabel,
    notes: ['Współrzędne i długości w metrach, kąty w gradach.', 'Wyniki zaokrąglone regułą „do parzystej”.'],
    entries: project.report,
  }
}
