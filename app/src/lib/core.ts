// Typowana nakładka na rdzeń Pikiety (Rust → WebAssembly).
// Funkcji wolno używać dopiero po rozstrzygnięciu `coreReady`.
import { env } from './project.svelte'
import init, * as w from './wasm/pikieta.js'

/** Punkt w konwencji geodezyjnej: x na północ, y na wschód. */
export interface XY {
  x: number
  y: number
}

export interface Pt extends XY {
  id: string
  h: number | null
  code: string
}

/** Rozstrzyga się wersją rdzenia, gdy moduł WASM jest gotowy. */
export const coreReady: Promise<string> = init().then(() => {
  // Flaga jest reaktywna: widoki sformatowane przed wczytaniem rdzenia odświeżą się.
  env.core = true
  return w.core_version()
})

/** Zaokrąglenie regułą „do parzystej” (rdzeń); przed wczytaniem rdzenia zwykłe toFixed. */
export function fmt(value: number, decimals: number): string {
  return env.core ? w.fmt(value, decimals) : value.toFixed(decimals)
}

export function renderReport(report: object, format: 'txt' | 'html'): string {
  return w.render_report(JSON.stringify(report), format)
}

export const epsgOf = (code: string) => Number(code.replace('EPSG:', ''))

const xy = (a: Float64Array | undefined, i = 0): XY | null => (a ? { x: a[i], y: a[i + 1] } : null)
const flat = (pts: XY[]) => pts.flatMap((p) => [p.x, p.y])

export function azimuthDistance(a: XY, b: XY): { azimuth: number; distance: number } | null {
  const r = w.azimuth_distance(a.x, a.y, b.x, b.y)
  if (!r) return null
  const out = { azimuth: r.azimuth_grad, distance: r.distance }
  r.free()
  return out
}

export function lineReduction(epsg: number, a: XY, b: XY) {
  const r = w.line_reduction(epsg, a.x, a.y, b.x, b.y)
  return r ? { scale: r[0], ellipsoidal: r[1], convergence: r[2] } : null
}

export function transformXY(from: number, to: number, p: XY): XY | null {
  return xy(w.transform(from, to, p.x, p.y))
}

export function toGeodetic(epsg: number, p: XY): { lat: number; lon: number } | null {
  const g = w.to_geodetic(epsg, p.x, p.y)
  return g ? { lat: g[0], lon: g[1] } : null
}

export function polar(station: XY, azimuthGrad: number, distance: number): XY {
  return xy(w.polar(station.x, station.y, azimuthGrad, distance))!
}

export function lineOffset(a: XY, b: XY, p: XY): { along: number; offset: number } | null {
  const r = w.line_offset(a.x, a.y, b.x, b.y, p.x, p.y)
  return r ? { along: r[0], offset: r[1] } : null
}

export function pointFromOffset(a: XY, b: XY, along: number, offset: number): XY | null {
  return xy(w.point_from_offset(a.x, a.y, b.x, b.y, along, offset))
}

export function lineIntersection(a: XY, b: XY, c: XY, d: XY): XY | null {
  return xy(w.line_intersection(flat([a, b, c, d])))
}

export function distanceIntersection(a: XY, ra: number, b: XY, rb: number): { left: XY; right: XY } | null {
  const r = w.distance_intersection(a.x, a.y, ra, b.x, b.y, rb)
  return r ? { left: xy(r, 0)!, right: xy(r, 2)! } : null
}

export function polygonArea(epsg: number, pts: XY[]): { grid: number; ellipsoidal: number } | null {
  const r = w.polygon_area(epsg, flat(pts))
  return r ? { grid: r[0], ellipsoidal: r[1] } : null
}

export function parsePoints(text: string, yx: boolean): { points: Pt[]; errors: string[] } {
  const r = w.parse_points(text, yx)
  const [ids, coords, codes, errors] = [r.ids, r.coords, r.codes, r.errors]
  r.free()
  const points = ids.map((id, i) => {
    const h = coords[3 * i + 2]
    return { id, x: coords[3 * i], y: coords[3 * i + 1], h: Number.isNaN(h) ? null : h, code: codes[i] }
  })
  return { points, errors }
}

export function writePoints(points: Pt[], yx: boolean, decimals: number): string {
  return w.write_points(
    points.map((p) => p.id),
    points.flatMap((p) => [p.x, p.y, p.h ?? NaN]),
    points.map((p) => p.code),
    yx,
    decimals,
  )
}
