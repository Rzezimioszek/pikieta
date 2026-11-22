// Pomiary czasów w konsoli, włączane wpisem `localStorage.setItem('pikieta.debug', '1')`.

let enabled = false
try {
  enabled = localStorage.getItem('pikieta.debug') === '1'
} catch {
  // Brak dostępu do pamięci przeglądarki: pomiary wyłączone.
}

/** Mierzy czas wykonania funkcji i wypisuje go w konsoli, gdy pomiary są włączone. */
export function timed<T>(label: string, fn: () => T): T {
  if (!enabled) return fn()
  const t = performance.now()
  const out = fn()
  console.info(`[pikieta] ${label}: ${Math.round(performance.now() - t)} ms`)
  return out
}

export function note(label: string, since: number) {
  if (enabled) console.info(`[pikieta] ${label}: ${Math.round(performance.now() - since)} ms`)
}
