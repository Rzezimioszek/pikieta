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

/** Wcięcie kątowe w przód z kątów wewnętrznych α (przy A) i β (przy B) [g]. */
export function angleIntersection(a: XY, b: XY, alpha: number, beta: number, left: boolean): XY | null {
  return xy(w.angle_intersection(flat([a, b]), alpha, beta, left))
}

export type IntObs = { kind: 'dist'; from: XY; d: number } | { kind: 'angle'; at: XY; ref: XY; angle: number }

/** Wcięcie z wyrównaniem z kątów [g] i długości [m]; poprawki w cc (kąty) i mm (długości). */
export function intersectionLsq(obs: IntObs[], sigmaCc: number, sigmaMm: number, pick: number) {
  const flatObs = obs.flatMap((o) =>
    o.kind === 'dist' ? [0, o.from.x, o.from.y, 0, 0, o.d] : [1, o.at.x, o.at.y, o.ref.x, o.ref.y, o.angle],
  )
  const r = JSON.parse(w.intersection_lsq(new Float64Array(flatObs), sigmaCc, sigmaMm, pick))
  return {
    p: { x: r.x, y: r.y } as XY,
    residuals: r.residuals as number[],
    m0: r.m0 as number | null,
    mx: r.mx as number,
    my: r.my as number,
    solutions: (r.solutions as [number, number][]).map(([x, y]) => ({ x, y })),
  }
}

export function distanceIntersection(a: XY, ra: number, b: XY, rb: number): { left: XY; right: XY } | null {
  const r = w.distance_intersection(a.x, a.y, ra, b.x, b.y, rb)
  return r ? { left: xy(r, 0)!, right: xy(r, 2)! } : null
}

export function polygonArea(epsg: number, pts: XY[]): { grid: number; ellipsoidal: number } | null {
  const r = w.polygon_area(epsg, flat(pts))
  return r ? { grid: r[0], ellipsoidal: r[1] } : null
}

export interface TraverseOut {
  /** Wierzchołki od początkowego (zamknięty bez powtórzenia początku). */
  points: XY[]
  /** Azymuty boków po wyrównaniu [g]. */
  azimuths: number[]
  angleCorrections: number[]
  dxCorrections: number[]
  dyCorrections: number[]
  angularMisclosure: number | null
  fx: number | null
  fy: number | null
  length: number
}

const TRAVERSE_KIND = { connected: 0, closed: 1, open: 2 } as const

/** Ciąg poligonowy z wyrównaniem przybliżonym (rdzeń). Kąty w gradach. Rzuca wyjątek z opisem błędu. */
export function traverse(
  kind: keyof typeof TRAVERSE_KIND,
  known: { back: XY; start: XY; end?: XY; forward?: XY },
  anglesGrad: number[],
  distances: number[],
  right: boolean,
): TraverseOut {
  const k = [known.back, known.start, known.end, known.forward].flatMap((p) => (p ? [p.x, p.y] : [NaN, NaN]))
  const r = JSON.parse(
    w.traverse(
      TRAVERSE_KIND[kind],
      new Float64Array(k),
      new Float64Array(anglesGrad),
      new Float64Array(distances),
      right,
    ),
  )
  const pts: XY[] = []
  for (let i = 0; i < r.points.length; i += 2) pts.push({ x: r.points[i], y: r.points[i + 1] })
  return { ...r, points: pts }
}

/** Orientacja stanowiska: stała [g] i dla każdego nawiązania odchyłka [g] oraz długość celowej. */
export function tachyOrientation(station: XY, refs: { p: XY; hz: number }[]) {
  const r = w.tachy_orientation(station.x, station.y, new Float64Array(refs.flatMap((x) => [x.p.x, x.p.y, x.hz])))
  return {
    constant: r[0],
    refs: refs.map((_, i) => ({ residual: r[1 + 2 * i], distance: r[2 + 2 * i] })),
  }
}

export interface TachyShot {
  p: XY
  azimuth: number
  horizontal: number
  reduced: number
  dh: number | null
  h: number | null
}

/** Pikieta ze stanowiska zorientowanego stałą [g]. Kąty w gradach; brak wartości jako null. */
export function tachyShot(
  station: XY & { h: number | null; instrument: number },
  constant: number,
  m: { hz: number; zenith: number | null; distance: number; target: number },
  o: { curvature: boolean; toEllipsoid: boolean; scale: number },
): TachyShot {
  const r = w.tachy_shot(
    station.x,
    station.y,
    station.h ?? NaN,
    station.instrument,
    constant,
    m.hz,
    m.zenith ?? NaN,
    m.distance,
    m.target,
    o.curvature,
    o.toEllipsoid,
    o.scale,
  )
  const n = (v: number) => (Number.isNaN(v) ? null : v)
  return { p: { x: r[0], y: r[1] }, azimuth: r[2], horizontal: r[3], reduced: r[4], dh: n(r[5]), h: n(r[6]) }
}

/** Elementy tyczenia [g, m]; null, gdy punkty się pokrywają. */
export function stakeout(station: XY, reference: XY, refReading: number, target: XY) {
  const r = w.stakeout(flat([station, reference, target]), refReading)
  return r ? { azimuth: r[0], reading: r[1], angle: r[2], grid: r[3] } : null
}

/** Długość w układzie na długość do odmierzenia w terenie. */
export function gridToTerrain(grid: number, scale: number, height: number | null): number {
  return w.grid_to_terrain(grid, scale, height ?? NaN)
}

/** Wcięcie wstecz z odczytów [g] na co najmniej trzy punkty znane. Rzuca wyjątek z opisem błędu. */
export function resection(obs: { p: XY; reading: number }[]) {
  const r = w.resection(new Float64Array(obs.flatMap((o) => [o.p.x, o.p.y, o.reading])))
  const n = (v: number) => (Number.isNaN(v) ? null : v)
  return {
    p: { x: r[0], y: r[1] },
    orientation: r[2],
    m0cc: n(r[3]),
    mx: n(r[4]),
    my: n(r[5]),
    danger: r[6],
    residualsCc: Array.from(r.slice(7)),
  }
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
