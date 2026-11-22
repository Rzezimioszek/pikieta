// Warstwy GML na mapie: geometria rysowana własnym rendererem WebGL2 (GPU), etykiety
// i zaznaczenie w lekkich warstwach canvas, wybór obiektu przez siatkę indeksu przestrzennego.
import Feature from 'ol/Feature'
import type Map from 'ol/Map'
import { type Extent } from 'ol/extent'
import {
  GeometryCollection,
  LineString,
  MultiLineString,
  MultiPoint,
  MultiPolygon,
  Point,
  Polygon,
  type Geometry,
} from 'ol/geom'
import Layer from 'ol/layer/Layer'
import VectorLayer from 'ol/layer/Vector'
import VectorSource from 'ol/source/Vector'
import { Circle, Fill, Stroke, Style, Text } from 'ol/style'
import { GmlGpu } from './glrender'
import { timed } from './perf'
import { forEachSpan, parts, type Decoded } from './gmlbuf'
import { pickPriority, styleOf, type ClassStyle, type Rgba } from './gmlstyle'

export interface GmlLayerData {
  decoded: Decoded
  classes: string[]
  cls: ArrayLike<number>
  labels: ArrayLike<string>
  variant: ArrayLike<number>
  /** Kody znaków umownych (słownik) i numer kodu dla każdego obiektu. */
  codes: string[]
  code: ArrayLike<number>
}

const css = (c: Rgba) =>
  `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(c[2] * 255)}, ${c[3]})`
const HALO = new Stroke({ color: '#ffffff', width: 3 })

/** Siatka do szybkiego znajdowania obiektów pod kursorem (bez obiektów OpenLayers dla każdego elementu). */
class GridIndex {
  readonly bbox: Float64Array
  private cells = new globalThis.Map<number, number[]>()
  private big: number[] = []
  private x0 = 0
  private y0 = 0
  private size = 1
  readonly extent: Extent | null

  constructor(d: Decoded) {
    this.bbox = new Float64Array(d.count * 4)
    let [minX, minY, maxX, maxY] = [Infinity, Infinity, -Infinity, -Infinity]
    const xy = d.coords
    let [a, b, c, e] = [0, 0, 0, 0]
    const span = (_kind: number, s: number, n: number) => {
      for (let k = 2 * s; k < 2 * (s + n); k += 2) {
        const [x, y] = [xy[k], xy[k + 1]]
        if (x < a) a = x
        if (x > c) c = x
        if (y < b) b = y
        if (y > e) e = y
      }
    }
    for (let i = 0; i < d.count; i++) {
      ;[a, b, c, e] = [Infinity, Infinity, -Infinity, -Infinity]
      forEachSpan(d, i, span)
      const o = 4 * i
      this.bbox[o] = a
      this.bbox[o + 1] = b
      this.bbox[o + 2] = c
      this.bbox[o + 3] = e
      if (a <= c) {
        minX = Math.min(minX, a)
        minY = Math.min(minY, b)
        maxX = Math.max(maxX, c)
        maxY = Math.max(maxY, e)
      }
    }
    this.extent = minX <= maxX ? [minX, minY, maxX, maxY] : null
    if (!this.extent) return
    this.x0 = minX
    this.y0 = minY
    this.size = Math.max(maxX - minX, maxY - minY, 1) / 256
    for (let i = 0; i < d.count; i++) {
      const o = 4 * i
      const [a, b, c, e] = [this.bbox[o], this.bbox[o + 1], this.bbox[o + 2], this.bbox[o + 3]]
      if (!(a <= c)) continue
      const [i0, j0, i1, j1] = [this.cx(a), this.cy(b), this.cx(c), this.cy(e)]
      // Obiekty obejmujące wiele komórek (np. obręby) trzymamy osobno.
      if ((i1 - i0 + 1) * (j1 - j0 + 1) > 64) {
        this.big.push(i)
        continue
      }
      for (let ci = i0; ci <= i1; ci++) {
        for (let cj = j0; cj <= j1; cj++) {
          const k = ci * 4096 + cj
          let list = this.cells.get(k)
          if (!list) this.cells.set(k, (list = []))
          list.push(i)
        }
      }
    }
  }

  private cx(x: number) {
    return Math.floor((x - this.x0) / this.size)
  }

  private cy(y: number) {
    return Math.floor((y - this.y0) / this.size)
  }

  /** Kandydaci, których prostokąt otaczający zawiera punkt z tolerancją. */
  query(x: number, y: number, tol: number): number[] {
    const out = new Set<number>()
    for (let ci = this.cx(x - tol); ci <= this.cx(x + tol); ci++) {
      for (let cj = this.cy(y - tol); cj <= this.cy(y + tol); cj++) {
        for (const i of this.cells.get(ci * 4096 + cj) ?? []) out.add(i)
      }
    }
    for (const i of this.big) out.add(i)
    return [...out].filter((i) => {
      const [a, b, c, e] = this.bbox.subarray(4 * i, 4 * i + 4)
      return x >= a - tol && x <= c + tol && y >= b - tol && y <= e + tol
    })
  }
}

function segDist(px: number, py: number, ax: number, ay: number, bx: number, by: number) {
  const [dx, dy] = [bx - ax, by - ay]
  const l2 = dx * dx + dy * dy
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / l2)) : 0
  return Math.hypot(px - ax - t * dx, py - ay - t * dy)
}

/** Czy punkt trafia w obiekt: odległość od punktu lub linii albo wnętrze wieloboku. */
function hits(d: Decoded, i: number, x: number, y: number, tol: number): boolean {
  const c = d.coords
  for (const p of parts(d, i)) {
    if (p.t === 'point') {
      if (Math.hypot(c[2 * p.c] - x, c[2 * p.c + 1] - y) <= tol) return true
      continue
    }
    const spans = p.t === 'line' ? [[p.c, p.n]] : p.rings
    let inside = false
    for (const [s, n] of spans) {
      for (let k = s; k < s + n - 1; k++) {
        const [ax, ay, bx, by] = [c[2 * k], c[2 * k + 1], c[2 * k + 2], c[2 * k + 3]]
        if (segDist(x, y, ax, ay, bx, by) <= tol) return true
        if (p.t === 'polygon' && ay > y !== by > y && x < ((bx - ax) * (y - ay)) / (by - ay) + ax) inside = !inside
      }
    }
    if (inside) return true
  }
  return false
}

/** Geometria OpenLayers jednego obiektu (do zaznaczenia). */
function geometryOf(d: Decoded, i: number): Geometry | null {
  const ps = parts(d, i)
  if (ps.length === 0) return null
  const flat = (s: number, n: number) => Array.from(d.coords.subarray(2 * s, 2 * (s + n)))
  const one = (p: (typeof ps)[number]): Geometry => {
    if (p.t === 'point') return new Point(flat(p.c, 1))
    if (p.t === 'line') return new LineString(flat(p.c, p.n), 'XY')
    const coords: number[] = []
    const ends: number[] = []
    for (const [s, n] of p.rings) {
      coords.push(...flat(s, n))
      ends.push(coords.length)
    }
    return new Polygon(coords, 'XY', ends)
  }
  if (ps.length === 1) return one(ps[0])
  const kinds = new Set(ps.map((p) => p.t))
  if (kinds.size === 1) {
    const g = ps.map(one)
    if (ps[0].t === 'point') return new MultiPoint(g.map((x) => (x as Point).getCoordinates()))
    if (ps[0].t === 'line') return new MultiLineString(g.map((x) => (x as LineString).getCoordinates()))
    return new MultiPolygon(g.map((x) => (x as Polygon).getCoordinates()))
  }
  return new GeometryCollection(ps.map(one))
}

/** Miejsce etykiety: punkt albo środek wierzchołków pierścienia zewnętrznego lub linii. */
function labelAnchor(d: Decoded, i: number): number[] | null {
  const p = parts(d, i)[0]
  if (!p) return null
  if (p.t === 'point') return [d.coords[2 * p.c], d.coords[2 * p.c + 1]]
  const [s, n] = p.t === 'line' ? [p.c, p.n] : p.rings[0]
  const m = p.t === 'polygon' ? n - 1 : n
  let [x, y] = [0, 0]
  for (let k = s; k < s + m; k++) {
    x += d.coords[2 * k]
    y += d.coords[2 * k + 1]
  }
  return m > 0 ? [x / m, y / m] : null
}

interface Entry {
  data: GmlLayerData
  index: GridIndex
  styles: ClassStyle[]
  labels: Feature[]
  visible: boolean
  hidden: Set<string>
}

export class GmlMapLayers {
  private gpu: GmlGpu
  private gpuLayer: Layer
  private entries = new globalThis.Map<string, Entry>()
  private labelSource = new VectorSource()
  private labelLayer: VectorLayer
  private selSource = new VectorSource()
  private labelStyles = new globalThis.Map<string, Style>()

  constructor(private map: Map) {
    this.gpu = new GmlGpu()
    this.gpuLayer = new Layer({
      render: (frame) => this.gpu.render(frame),
      zIndex: 10,
    })
    map.addLayer(this.gpuLayer)
    this.labelLayer = new VectorLayer({
      source: this.labelSource,
      declutter: true,
      maxResolution: 25,
      zIndex: 20,
      style: (f, res) => this.labelStyle(f as Feature, res),
    })
    map.addLayer(this.labelLayer)
    map.addLayer(
      new VectorLayer({
        source: this.selSource,
        zIndex: 21,
        style: new Style({
          stroke: new Stroke({ color: '#22d3ee', width: 4 }),
          fill: new Fill({ color: 'rgba(34, 211, 238, 0.25)' }),
          image: new Circle({
            radius: 7,
            fill: new Fill({ color: '#22d3ee' }),
            stroke: new Stroke({ color: '#111', width: 1.5 }),
          }),
        }),
      }),
    )
  }

  get gpuInfo() {
    return this.gpu.renderer
  }

  private labelStyle(f: Feature, res: number): Style | undefined {
    const s = f.get('style') as ClassStyle
    if (!s.label || res > s.label.maxRes || res > s.maxRes) return undefined
    const pt = !!f.get('pt')
    const key = `${css(s.label.color)}|${s.label.font}|${pt}`
    let st = this.labelStyles.get(key)
    if (!st) {
      st = new Style({
        text: new Text({
          font: s.label.font,
          offsetY: pt ? -10 : 0,
          fill: new Fill({ color: css(s.label.color) }),
          stroke: HALO,
          overflow: !pt,
        }),
      })
      this.labelStyles.set(key, st)
    }
    st.getText()!.setText(f.get('label'))
    return st
  }

  /** Wymienia geometrię warstwy (po wczytaniu albo po zmianie układu projektu). */
  set(id: string, data: GmlLayerData, visible: boolean, hidden: string[]) {
    this.remove(id)
    const d = data.decoded
    // Styl liczony raz na (klasa, kod znaku, wariant); grupy o tym samym stylu i klasie trafiają razem na GPU.
    const groups = new globalThis.Map<number, { cls: string; style: ClassStyle; list: number[] }>()
    const styles = new Array<ClassStyle>(d.count)
    for (let i = 0; i < d.count; i++) {
      const [c, k, v] = [data.cls[i], data.code?.[i] ?? 0, data.variant[i] ?? 0]
      const key = (c * 4096 + k) * 256 + v
      let g = groups.get(key)
      if (!g) {
        const cls = data.classes[c]
        groups.set(key, (g = { cls, style: styleOf(cls, data.codes?.[k] ?? '', v), list: [] }))
      }
      g.list.push(i)
      styles[i] = g.style
    }
    timed('GPU: bufory warstwy', () => this.gpu.set(id, d, [...groups.values()], visible, hidden))
    const index = timed('indeks przestrzenny', () => new GridIndex(d))
    const entry: Entry = {
      data,
      index,
      styles,
      labels: [],
      visible,
      hidden: new Set(hidden),
    }
    this.entries.set(id, entry)
    this.gpuLayer.changed()
    this.buildLabels(id, entry)
  }

  /** Etykiety tworzone partiami w tle, żeby nie blokować interfejsu przy dużych warstwach. */
  private buildLabels(id: string, entry: Entry) {
    const { data, styles } = entry
    const d = data.decoded
    let i = 0
    const step = () => {
      if (this.entries.get(id) !== entry) return
      const end = Math.min(d.count, i + 20000)
      for (; i < end; i++) {
        const s = styles[i]
        if (!s.label || !data.labels[i]) continue
        const at = labelAnchor(d, i)
        if (!at) continue
        const f = new Feature(new Point(at))
        f.setProperties(
          {
            label: data.labels[i],
            style: s,
            cls: data.classes[data.cls[i]],
            key: `${id}:${i}`,
            pt:
              entry.index.bbox[4 * i] === entry.index.bbox[4 * i + 2] &&
              entry.index.bbox[4 * i + 1] === entry.index.bbox[4 * i + 3],
          },
          true,
        )
        entry.labels.push(f)
      }
      if (i < d.count) setTimeout(step)
      else this.refreshLabels()
    }
    setTimeout(step)
  }

  private refreshLabels() {
    this.labelSource.clear(true)
    const all: Feature[] = []
    for (const e of this.entries.values()) {
      if (!e.visible) continue
      for (const f of e.labels) if (!e.hidden.has(f.get('cls'))) all.push(f)
    }
    this.labelSource.addFeatures(all)
  }

  setVisibility(id: string, visible: boolean, hidden: string[]) {
    const e = this.entries.get(id)
    if (!e) return
    const same = e.visible === visible && hidden.length === e.hidden.size && hidden.every((h) => e.hidden.has(h))
    if (same) return
    e.visible = visible
    e.hidden = new Set(hidden)
    this.gpu.setVisibility(id, visible, hidden)
    this.refreshLabels()
    this.gpuLayer.changed()
  }

  setLabel(id: string, i: number, label: string) {
    this.entries
      .get(id)
      ?.labels.find((f) => f.get('key') === `${id}:${i}`)
      ?.set('label', label)
  }

  remove(id: string) {
    if (!this.entries.has(id)) return
    this.gpu.remove(id)
    this.entries.delete(id)
    this.refreshLabels()
    this.gpuLayer.changed()
  }

  keep(ids: string[]) {
    for (const id of [...this.entries.keys()]) if (!ids.includes(id)) this.remove(id)
  }

  setSelection(keys: Iterable<string>) {
    this.selSource.clear(true)
    const out: Feature[] = []
    for (const k of keys) {
      const [id, i] = k.split(':')
      const e = this.entries.get(id)
      const g = e && geometryOf(e.data.decoded, Number(i))
      if (g) out.push(new Feature(g))
    }
    this.selSource.addFeatures(out)
  }

  /** Zasięg obiektów (kluczy) albo całej warstwy. */
  extent(target: string[] | { layer: string }): Extent | null {
    if ('layer' in target) return this.entries.get(target.layer)?.index.extent ?? null
    let ext: Extent | null = null
    for (const k of target) {
      const [id, si] = k.split(':')
      const e = this.entries.get(id)
      if (!e) continue
      const b = e.index.bbox.subarray(4 * Number(si), 4 * Number(si) + 4)
      if (!(b[0] <= b[2])) continue
      ext = ext
        ? [Math.min(ext[0], b[0]), Math.min(ext[1], b[1]), Math.max(ext[2], b[2]), Math.max(ext[3], b[3])]
        : [...b]
    }
    return ext
  }

  /** Obiekt pod kursorem: punkty przed liniami i powierzchniami, wyższe warstwy przed niższymi, mniejsze przed większymi. */
  pick(coordinate: number[], tolerance: number, resolution: number): string | null {
    const [x, y] = coordinate
    let best: { key: string; prio: number; area: number } | null = null
    for (const [id, e] of this.entries) {
      if (!e.visible) continue
      for (const i of e.index.query(x, y, tolerance)) {
        const s = e.styles[i]
        if (resolution > s.maxRes || e.hidden.has(e.data.classes[e.data.cls[i]])) continue
        if (!hits(e.data.decoded, i, x, y, tolerance)) continue
        const b = e.index.bbox.subarray(4 * i, 4 * i + 4)
        const area = (b[2] - b[0]) * (b[3] - b[1])
        const cand = { key: `${id}:${i}`, prio: pickPriority(s, area === 0), area }
        if (!best || cand.prio > best.prio || (cand.prio === best.prio && cand.area < best.area)) best = cand
      }
    }
    return best?.key ?? null
  }
}
