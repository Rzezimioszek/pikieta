// Kontrola wczytywanego wykazu współrzędnych: czy kolumny X i Y nie są zamienione i czy punkty leżą
// w Polsce w układzie projektu. Geodeta pisze X (północ) przed Y (wschód), GIS zwykle odwrotnie
// (easting, northing); pomyłka przesuwa punkty o setki kilometrów i nic o tym nie mówi.
import { toGeodetic, type Pt, type XY } from './core'

/** Polska z zapasem około 30 km. */
const inPoland = (g: { lat: number; lon: number } | null) =>
  !!g && g.lat >= 48.8 && g.lat <= 55.1 && g.lon >= 13.8 && g.lon <= 24.4

/** Strefy układu 2000: EPSG → numer strefy (cyfra milionów w Y). */
const ZONES: Record<number, number> = { 2176: 5, 2177: 6, 2178: 7, 2179: 8 }

export const swap = (p: Pt): Pt => ({ ...p, x: p.y, y: p.x })

export interface AxisCheck {
  /** Ile sprawdzonych punktów leży w Polsce przy kolejności Nr X Y i Nr Y X. */
  inside: { xy: number; yx: number }
  checked: number
  /** Kolejność, przy której więcej punktów leży w Polsce; `null`, gdy nie da się rozstrzygnąć. */
  guess: boolean | null
  /** Strefa układu 2000 odczytana z pierwszej cyfry Y przy danej kolejności kolumn, gdy różni się
   *  od strefy projektu. */
  otherZone: { xy: number | null; yx: number | null }
}

/** `points` w kolejności Nr X Y (tak, jak przeczytane bez zamiany). Sprawdza do 300 punktów. */
export function checkAxes(points: Pt[], epsg: number): AxisCheck {
  const step = Math.max(1, Math.floor(points.length / 300))
  const sample = points.filter((_, i) => i % step === 0)
  let xy = 0
  let yx = 0
  for (const p of sample) {
    if (inPoland(toGeodetic(epsg, p))) xy++
    if (inPoland(toGeodetic(epsg, swap(p)))) yx++
  }
  const guess = xy === yx ? null : yx > xy
  return {
    inside: { xy, yx },
    checked: sample.length,
    guess,
    otherZone: { xy: zoneHint(sample, epsg, false), yx: zoneHint(sample, epsg, true) },
  }
}

/** W układzie 2000 Y = strefa·10⁶ + 500 000 ± 140 000; inna cyfra milionów oznacza inną strefę. */
function zoneHint(sample: XY[], epsg: number, yx: boolean): number | null {
  const zone = ZONES[epsg]
  if (!zone || !sample.length) return null
  const counts = new Map<number, number>()
  for (const p of sample) {
    const e = yx ? p.x : p.y
    const z = Math.floor(e / 1e6)
    if (z >= 5 && z <= 8 && Math.abs((e % 1e6) - 5e5) < 2e5) counts.set(z, (counts.get(z) ?? 0) + 1)
  }
  const [best] = [...counts].sort((a, b) => b[1] - a[1])
  return best && best[0] !== zone && best[1] > sample.length / 2 ? best[0] : null
}
