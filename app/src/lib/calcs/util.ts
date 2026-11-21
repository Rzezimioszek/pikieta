// Wspólne pomocnicze funkcje okien obliczeń.
import type { XY } from '../core'
import { findPoint, nextId } from '../project.svelte'

export function num(s: string | undefined): number | null {
  const t = (s ?? '').trim().replace(',', '.')
  const v = Number(t)
  return t !== '' && Number.isFinite(v) ? v : null
}

export const f2 = (v: number) => v.toFixed(2)
export const f3 = (v: number) => v.toFixed(3)
export const f4 = (v: number) => v.toFixed(4)
export const coords = (p: XY) => `X ${f2(p.x)}  Y ${f2(p.y)}`
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

/** Stan punktu wynikowego względem projektu. */
export function pointState(id: string, p: XY): { status: string; tone: 'ok' | 'muted' | 'error' } {
  const existing = findPoint(id)
  if (!existing) return { status: 'nowy', tone: 'ok' }
  return Math.hypot(existing.x - p.x, existing.y - p.y) < 0.001
    ? { status: 'w projekcie', tone: 'muted' }
    : { status: 'numer zajęty', tone: 'error' }
}

/** Wiersze raportu wyrównane w kolumny. */
export function tableLines(head: string[], rows: string[][]): string[] {
  const widths = head.map((h, i) => Math.max(h.length, ...rows.map((r) => r[i].length)))
  const line = (cells: string[]) => cells.map((c, i) => (i === 0 ? c.padEnd(widths[i]) : c.padStart(widths[i]))).join('  ')
  return [line(head), ...rows.map(line)]
}
