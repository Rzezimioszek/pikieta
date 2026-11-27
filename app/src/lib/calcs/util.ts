// Wspólne pomocnicze funkcje okien obliczeń.
import { fmt, type XY } from '../core'
import { findPoint, nextId, project } from '../project.svelte'

export function num(s: string | undefined): number | null {
  const t = (s ?? '').trim().replace(',', '.')
  const v = Number(t)
  return t !== '' && Number.isFinite(v) ? v : null
}

/** Współrzędne i wysokości. */
export const fc = (v: number) => fmt(v, project.precision.coord)
/** Długości i miary. */
export const fl = (v: number) => fmt(v, project.precision.length)
/** Kąty w gradach. */
export const fa = (v: number) => fmt(v, project.precision.angle)
/** Pola w m². */
export const fm2 = (v: number) => fmt(v, project.precision.area)
/** Pola w hektarach: zawsze 0,0001 ha, czyli 1 m², jak w ewidencji gruntów. */
export const fha = (v: number) => fmt(v / 10000, 4)
export const coords = (p: XY) => `X ${fc(p.x)}  Y ${fc(p.y)}`
/** Sprowadza kąt w gradach do zakresu [0, 400). */
export const turn = (g: number) => ((g % 400) + 400) % 400

export const isBlank = (row: object) => Object.values(row).every((v) => String(v).trim() === '')

/** Pilnuje, żeby na końcu tabeli zawsze był pusty wiersz do wpisania kolejnych danych. */
export function keepTrailing<T extends object>(rows: T[], empty: () => T) {
  const last = rows.at(-1)
  if (!last || !isBlank(last)) rows.push(empty())
}

/** Numery nowych punktów: wpisane albo kolejne wolne, bez kolizji z projektem i tabelą. */
export function assignIds(explicit: string[]): string[] {
  const used = new Set(explicit.map((s) => s.trim()).filter(Boolean))
  let n = Number(nextId())
  return explicit.map((s) => {
    const id = s.trim()
    if (id) return id
    while (used.has(String(n)) || findPoint(String(n))) n++
    used.add(String(n))
    return String(n++)
  })
}

/** Numery kolejnych `n` punktów od pierwszego (K1 → K2, K3; 010 → 011); bez cyfr na końcu albo
 *  pusty: pierwszy jak wpisany, reszta kolejne wolne. */
export function numberSeq(first: string, n: number): string[] {
  const m = first.trim().match(/^(.*?)(\d+)$/)
  if (!m) return assignIds(Array.from({ length: n }, (_, i) => (i === 0 ? first : '')))
  const start = Number(m[2])
  return Array.from({ length: n }, (_, i) => m[1] + String(start + i).padStart(m[2].length, '0'))
}

/** Stan punktu wynikowego względem projektu. */
export function pointState(id: string, p: XY): { status: string; tone: 'ok' | 'muted' | 'error' } {
  const existing = findPoint(id)
  if (!existing) return { status: 'nowy', tone: 'ok' }
  return Math.hypot(existing.x - p.x, existing.y - p.y) < 0.001
    ? { status: 'w projekcie', tone: 'muted' }
    : { status: 'numer zajęty', tone: 'error' }
}

/** Tabela raportu; pierwsza kolumna (numer) do lewej, liczby do prawej. */
export function reportTable(head: string[], rows: string[][], left = 1) {
  return { head, rows, right: head.map((_, i) => i >= left) }
}

/** Przenosi wiersz tabeli obliczenia i pilnuje pustego wiersza na końcu. */
export function moveRow<T extends object>(rows: T[], from: number, to: number, empty: () => T) {
  const [row] = rows.splice(from, 1)
  rows.splice(to, 0, row)
  // Pusty wiersz przeniesiony w środek nie jest potrzebny.
  for (let i = rows.length - 2; i >= 0; i--) if (isBlank(rows[i])) rows.splice(i, 1)
  keepTrailing(rows, empty)
}

/** Czy wielobok (zamknięty) ma przecinające się boki; wtedy pole ze wzoru Gaussa jest błędne. */
export function selfIntersects(pts: XY[]): boolean {
  const n = pts.length
  if (n < 4) return false
  const cross = (o: XY, a: XY, b: XY) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
  const hit = (p1: XY, p2: XY, q1: XY, q2: XY) => {
    const [d1, d2, d3, d4] = [cross(q1, q2, p1), cross(q1, q2, p2), cross(p1, p2, q1), cross(p1, p2, q2)]
    return ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0))
  }
  for (let i = 0; i < n; i++) {
    for (let j = i + 2; j < n; j++) {
      if (i === 0 && j === n - 1) continue // boki sąsiednie przez zamknięcie
      if (hit(pts[i], pts[(i + 1) % n], pts[j], pts[(j + 1) % n])) return true
    }
  }
  return false
}
