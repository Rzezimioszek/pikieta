// Odczyt i zapis plików: okna systemowe w Tauri, File System Access w przeglądarkach
// Chromium, a w pozostałych przeglądarkach zwykłe pobieranie i wybór pliku.
import { isTauri } from '@tauri-apps/api/core'
import { parsePoints, renderReport, writePoints, type Pt } from './core'
import { layersForProject, replaceLayers } from './gml.svelte'
import {
  addPoints,
  DEFAULT_PRECISION,
  downloadText,
  notify,
  project,
  reportData,
  selection,
  ui,
  type Saved,
} from './project.svelte'

export interface FileType {
  name: string
  extensions: string[]
  mime: string
}

export const TXT: FileType = { name: 'Plik tekstowy', extensions: ['txt', 'csv', 'xyz'], mime: 'text/plain' }
export const HTML: FileType = { name: 'Strona HTML', extensions: ['html'], mime: 'text/html' }
export const PROJECT: FileType = { name: 'Projekt Pikiety', extensions: ['pikieta'], mime: 'application/json' }

interface PickerWindow {
  showSaveFilePicker?: (o: object) => Promise<{
    name: string
    createWritable: () => Promise<{ write: (d: string) => Promise<void>; close: () => Promise<void> }>
  }>
}

const isAbort = (e: unknown) => e instanceof DOMException && e.name === 'AbortError'
const basename = (path: string) => path.split(/[\\/]/).pop() ?? path

/** Zapisuje tekst do pliku wskazanego przez użytkownika. Zwraca nazwę pliku albo `null` po anulowaniu. */
export async function saveText(suggested: string, text: string, type: FileType): Promise<string | null> {
  try {
    if (isTauri()) {
      const { save } = await import('@tauri-apps/plugin-dialog')
      const { writeTextFile } = await import('@tauri-apps/plugin-fs')
      const path = await save({ defaultPath: suggested, filters: [{ name: type.name, extensions: type.extensions }] })
      if (!path) return null
      await writeTextFile(path, text)
      return basename(path)
    }
    const picker = (window as PickerWindow).showSaveFilePicker
    if (picker) {
      const handle = await picker({
        suggestedName: suggested,
        types: [{ description: type.name, accept: { [type.mime]: type.extensions.map((e) => `.${e}`) } }],
      })
      const out = await handle.createWritable()
      await out.write(text)
      await out.close()
      return handle.name
    }
    downloadText(suggested, text)
    return suggested
  } catch (e) {
    if (isAbort(e)) return null
    notify(`Nie udało się zapisać pliku: ${e instanceof Error ? e.message : String(e)}`, 'error')
    return null
  }
}

/** Pliki z programów geodezyjnych bywają w Windows-1250, nie w UTF-8. */
function decode(bytes: ArrayBuffer | Uint8Array): string {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    return new TextDecoder('windows-1250').decode(bytes)
  }
}

function pickInBrowser(type: FileType): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = [...type.extensions.map((e) => `.${e}`), type.mime].join(',')
    input.onchange = () => resolve(input.files?.[0] ?? null)
    input.oncancel = () => resolve(null)
    input.click()
  })
}

/** Wczytuje plik tekstowy wybrany przez użytkownika; `null` po anulowaniu. */
export async function openText(type: FileType): Promise<{ name: string; text: string } | null> {
  try {
    if (isTauri()) {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const { readFile } = await import('@tauri-apps/plugin-fs')
      const path = await open({ multiple: false, filters: [{ name: type.name, extensions: type.extensions }] })
      if (!path) return null
      return { name: basename(path), text: decode(await readFile(path)) }
    }
    const file = await pickInBrowser(type)
    return file ? { name: file.name, text: decode(await file.arrayBuffer()) } : null
  } catch (e) {
    notify(`Nie udało się wczytać pliku: ${e instanceof Error ? e.message : String(e)}`, 'error')
    return null
  }
}

// --- Punkty ---

/** Wczytuje wykaz współrzędnych do projektu; zwraca wczytane punkty. */
export async function importPoints(): Promise<Pt[]> {
  const file = await openText(TXT)
  if (!file) return []
  const parsed = parsePoints(file.text, ui.yx)
  const replaced = addPoints(parsed.points)
  notify(
    `${file.name}: wczytano ${parsed.points.length} pkt` +
      (replaced ? `, zastąpiono ${replaced}` : '') +
      (parsed.errors.length ? `, pominięto ${parsed.errors.length} wierszy (${parsed.errors.slice(0, 2).join('; ')})` : '') +
      '.',
    parsed.points.length === 0 ? 'error' : 'info',
  )
  return parsed.points
}

export async function exportPoints() {
  if (project.points.length === 0) return notify('Brak punktów do eksportu.', 'error')
  const name = await saveText('punkty.txt', writePoints(project.points, ui.yx, project.precision.coord), TXT)
  if (name) notify(`Zapisano ${project.points.length} pkt do ${name}.`)
}

// --- Raport ---

export const reportHtml = (crsLabel: string) => renderReport(reportData(crsLabel), 'html')

export async function exportReport(crsLabel: string, format: 'txt' | 'html') {
  if (project.report.length === 0) return notify('Raport jest pusty.', 'error')
  const text = renderReport(reportData(crsLabel), format)
  const name = await saveText(`raport.${format}`, text, format === 'txt' ? { ...TXT, extensions: ['txt'] } : HTML)
  if (name) notify(`Zapisano raport do ${name}.`)
}

/** Drukuje raport; w oknie drukowania można wybrać zapis do PDF. */
export function printReport(crsLabel: string) {
  if (project.report.length === 0) return notify('Raport jest pusty.', 'error')
  const frame = document.createElement('iframe')
  frame.style.cssText = 'position:fixed;width:0;height:0;border:0;visibility:hidden'
  frame.srcdoc = reportHtml(crsLabel)
  frame.onload = () => {
    frame.contentWindow?.focus()
    frame.contentWindow?.print()
    setTimeout(() => frame.remove(), 1000)
  }
  document.body.append(frame)
}

// --- Projekt ---

const FORMAT = 'pikieta-projekt'
const VERSION = 1

export const projectFile = $state({ name: '' })

export async function saveProjectFile() {
  const doc = { format: FORMAT, version: VERSION, saved: new Date().toISOString(), ...project, gml: await layersForProject() }
  const name = await saveText(projectFile.name || 'projekt.pikieta', JSON.stringify(doc, null, 1), PROJECT)
  if (!name) return
  projectFile.name = name
  notify(`Zapisano projekt ${name}.`)
}

/** Wczytuje projekt z pliku i zastępuje bieżący. Zwraca wczytane punkty albo `null`. */
export async function openProjectFile(): Promise<Pt[] | null> {
  const file = await openText(PROJECT)
  if (!file) return null
  let doc: Partial<Saved> & { format?: string; version?: number; gml?: Parameters<typeof replaceLayers>[0] }
  try {
    doc = JSON.parse(file.text)
  } catch {
    notify(`${file.name} nie jest plikiem projektu (błędny JSON).`, 'error')
    return null
  }
  if (doc.format !== FORMAT || !Array.isArray(doc.points)) {
    notify(`${file.name} nie jest plikiem projektu Pikiety.`, 'error')
    return null
  }
  if ((doc.version ?? 0) > VERSION) {
    notify(`${file.name} pochodzi z nowszej wersji Pikiety; część danych może się nie wczytać.`, 'error')
  }
  const busy = project.points.length + project.calcs.length + project.report.length > 0
  if (busy && !confirm(`Otworzyć ${file.name}? Bieżący projekt zostanie zastąpiony.`)) return null

  project.crs = typeof doc.crs === 'string' ? doc.crs : project.crs
  project.points = doc.points
  project.calcs = Array.isArray(doc.calcs) ? doc.calcs : []
  project.report = Array.isArray(doc.report) ? doc.report : []
  project.precision = { ...DEFAULT_PRECISION, ...doc.precision }
  replaceLayers(Array.isArray(doc.gml) ? doc.gml : [])
  selection.clear()
  ui.active = null
  projectFile.name = file.name
  notify(
    `Otwarto projekt ${file.name}: ${doc.points.length} pkt, ${project.calcs.length} obliczeń` +
      (doc.gml?.length ? `, ${doc.gml.length} warstw GML.` : '.'),
  )
  return project.points
}
