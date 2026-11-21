// Stan projektu: układ, punkty i raport obliczeń. Zapisywany w pamięci przeglądarki.
import type { Pt } from './core'

export interface ReportEntry {
  title: string
  lines: string[]
}

const STORAGE_KEY = 'pikieta.project.v1'

interface Saved {
  crs: string
  points: Pt[]
  report: ReportEntry[]
}

function load(): Saved {
  const empty: Saved = { crs: 'EPSG:2178', points: [], report: [] }
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return empty
    const s = JSON.parse(raw) as Partial<Saved>
    return {
      crs: typeof s.crs === 'string' ? s.crs : empty.crs,
      points: Array.isArray(s.points) ? s.points : [],
      report: Array.isArray(s.report) ? s.report : [],
    }
  } catch {
    return empty
  }
}

export const project = $state<Saved>(load())

/** Zapisuje projekt; brak dostępu do pamięci przeglądarki nie przerywa pracy. */
export function saveProject() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(project))
  } catch {
    // Tryb prywatny albo pełna pamięć: projekt zostaje tylko w tej sesji.
  }
}

export function findPoint(id: string): Pt | undefined {
  const key = id.trim()
  return key === '' ? undefined : project.points.find((p) => p.id === key)
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
  const index = new Map(project.points.map((p, i) => [p.id, i]))
  let replaced = 0
  for (const p of points) {
    const i = index.get(p.id)
    if (i === undefined) {
      index.set(p.id, project.points.length)
      project.points.push(p)
    } else {
      project.points[i] = p
      replaced++
    }
  }
  return replaced
}

export function removePoint(id: string) {
  project.points = project.points.filter((p) => p.id !== id)
}

export function downloadText(filename: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: 'text/plain;charset=utf-8' }))
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}

/** Raport obliczeń jako tekst do zapisania w pliku. */
export function reportText(crsLabel: string): string {
  const head = [
    'Pikieta: raport obliczeń',
    `Data: ${new Date().toLocaleString('pl-PL')}`,
    `Układ współrzędnych: ${crsLabel}`,
    'Współrzędne i długości w metrach, kąty w gradach.',
  ]
  const body = project.report.map((e, i) => [`${i + 1}. ${e.title}`, ...e.lines.map((l) => `   ${l}`)].join('\n'))
  return [head.join('\n'), ...body].join('\n\n') + '\n'
}
