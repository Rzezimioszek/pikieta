// Warstwy GML na mapie: geometria rysowana przez WebGL (GPU), etykiety i zaznaczenie
// w lekkich warstwach canvas, wybór obiektu przez indeks przestrzenny.
import Feature from 'ol/Feature'
import type Map from 'ol/Map'
import { createEmpty, extend, type Extent } from 'ol/extent'
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
import VectorLayer from 'ol/layer/Vector'
import WebGLVectorLayer from 'ol/layer/WebGLVector'
import VectorSource from 'ol/source/Vector'
import { Circle, Fill, Stroke, Style, Text } from 'ol/style'
import type { Rule } from 'ol/style/flat'
import { parts, type Decoded } from './gmlbuf'

interface ClassLook {
  stroke: string
  width: number
  fill?: string
  point?: string
  /** Etykiety poniżej tej rozdzielczości [m/px]. */
  labelBelow?: number
  /** Kolejność rysowania (większa wyżej). */
  z: number
  /** Pierwszeństwo przy wyborze kliknięciem (mniejsze wygrywa). */
  pick: number
}

/** Wygląd klas EGiB; pozostałe klasy mają wygląd domyślny. */
const LOOK: Record<string, ClassLook> = {
  EGB_ObrebEwidencyjny: { stroke: '#e879f9', width: 3, z: 1, pick: 6 },
  EGB_JednostkaEwidencyjna: { stroke: '#c084fc', width: 3.5, z: 0, pick: 7 },
  EGB_KonturKlasyfikacyjny: { stroke: '#a3e635', width: 1, z: 2, pick: 5 },
  EGB_KonturUzytkuGruntowego: { stroke: '#4ade80', width: 1.2, z: 3, pick: 4 },
  EGB_DzialkaEwidencyjna: { stroke: '#facc15', width: 1.6, fill: 'rgba(250, 204, 21, 0.06)', labelBelow: 1.2, z: 4, pick: 3 },
  EGB_Budynek: { stroke: '#fb923c', width: 1.2, fill: 'rgba(251, 146, 60, 0.35)', z: 5, pick: 2 },
  EGB_PunktGraniczny: { stroke: '#111111', width: 1, point: '#ef4444', labelBelow: 0.2, z: 6, pick: 1 },
}
const DEFAULT_LOOK: ClassLook = { stroke: '#38bdf8', width: 1.2, fill: 'rgba(56, 189, 248, 0.08)', point: '#38bdf8', z: 3, pick: 5 }
const look = (cls: string) => LOOK[cls] ?? DEFAULT_LOOK

/** Punkty rysowane dopiero poniżej tej rozdzielczości [m/px]; wyżej zlewają się w plamę. */
const POINTS_BELOW = 1.5

/**
 * Style WebGL: osobne reguły dla punktów i dla linii/wieloboków (styl z właściwościami
 * `circle-*` rysowałby kółko w każdym wierzchołku wieloboku), filtr po klasie.
 */
function webglRules(): Rule[] {
  const isPoint = ['==', ['geometry-type'], 'Point']
  const shape = (l: ClassLook) => ({
    'stroke-color': l.stroke,
    'stroke-width': l.width,
    'fill-color': l.fill ?? 'rgba(0, 0, 0, 0)',
  })
  const point = (l: ClassLook) => ({
    'circle-radius': 3.5,
    'circle-fill-color': l.point ?? l.stroke,
    'circle-stroke-color': '#111111',
    'circle-stroke-width': 1,
  })
  const known = Object.keys(LOOK)
  const ofClass = (cls: string) => ['==', ['get', 'cls'], cls]
  const other = ['!', ['in', ['get', 'cls'], ['literal', known]]]
  const pointsVisible = ['<', ['resolution'], POINTS_BELOW]
  return [
    ...Object.entries(LOOK).flatMap(([cls, l]) => [
      { filter: ['all', ofClass(cls), ['!', isPoint]], style: shape(l) },
      { filter: ['all', ofClass(cls), isPoint, pointsVisible], style: point(l) },
    ]),
    { filter: ['all', other, ['!', isPoint]], style: shape(DEFAULT_LOOK) },
    { filter: ['all', other, isPoint, pointsVisible], style: point(DEFAULT_LOOK) },
  ] as Rule[]
}

const HALO = new Stroke({ color: '#111', width: 3 })
const labelStyles = new globalThis.Map<string, Style>()

function labelStyle(cls: string, label: string, resolution: number): Style | undefined {
  const l = look(cls)
  if (l.labelBelow === undefined || resolution >= l.labelBelow || !label) return undefined
  let s = labelStyles.get(cls)
  if (!s) {
    const point = cls === 'EGB_PunktGraniczny'
    s = new Style({
      text: new Text({
        font: point ? '11px sans-serif' : 'bold 13px sans-serif',
        offsetY: point ? -11 : 0,
        fill: new Fill({ color: '#fff' }),
        stroke: HALO,
        overflow: !point,
      }),
    })
    labelStyles.set(cls, s)
  }
  s.getText()!.setText(label)
  return s
}

const selectedStyle = new Style({
  stroke: new Stroke({ color: '#22d3ee', width: 4 }),
  fill: new Fill({ color: 'rgba(34, 211, 238, 0.25)' }),
  image: new Circle({ radius: 7, fill: new Fill({ color: '#22d3ee' }), stroke: new Stroke({ color: '#111', width: 1.5 }) }),
  zIndex: 10,
})

/** Geometria obiektu OpenLayers z bufora (współrzędne już w kolejności mapy). */
function geometryOf(d: Decoded, i: number): Geometry | null {
  const ps = parts(d, i)
  if (ps.length === 0) return null
  const flat = (c: number, n: number) => Array.from(d.coords.subarray(2 * c, 2 * (c + n)))
  const one = (p: (typeof ps)[number]): Geometry => {
    if (p.t === 'point') return new Point(flat(p.c, 1))
    if (p.t === 'line') return new LineString(flat(p.c, p.n), 'XY')
    const coords: number[] = []
    const ends: number[] = []
    for (const [c, n] of p.rings) {
      coords.push(...flat(c, n))
      ends.push(coords.length)
    }
    return new Polygon(coords, 'XY', ends)
  }
  if (ps.length === 1) return one(ps[0])
  const kinds = new Set(ps.map((p) => p.t))
  if (kinds.size === 1) {
    const geoms = ps.map(one)
    if (ps[0].t === 'point') return new MultiPoint(geoms.map((g) => (g as Point).getCoordinates()))
    if (ps[0].t === 'line') return new MultiLineString(geoms.map((g) => (g as LineString).getCoordinates()))
    return new MultiPolygon(geoms.map((g) => (g as Polygon).getCoordinates()))
  }
  return new GeometryCollection(ps.map(one))
}

interface LayerEntry {
  source: VectorSource
  layer: WebGLVectorLayer
  /** Wszystkie obiekty warstwy (także ukrytych klas), według numeru obiektu. */
  features: (Feature | null)[]
  classes: string[]
  visible: boolean
  hidden: Set<string>
}

export interface GmlLayerData {
  decoded: Decoded
  classes: string[]
  cls: ArrayLike<number>
  labels: ArrayLike<string>
}

export class GmlMapLayers {
  private layers = new globalThis.Map<string, LayerEntry>()
  private labelSource = new VectorSource({ useSpatialIndex: true })
  private labelLayer: VectorLayer
  private selSource = new VectorSource()
  private selLayer: VectorLayer

  constructor(private map: Map) {
    this.labelLayer = new VectorLayer({
      source: this.labelSource,
      declutter: true,
      maxResolution: 1.2,
      zIndex: 20,
      style: (f, res) => labelStyle(f.get('cls'), f.get('label'), res),
    })
    this.selLayer = new VectorLayer({ source: this.selSource, style: selectedStyle, zIndex: 21 })
    map.addLayer(this.labelLayer)
    map.addLayer(this.selLayer)
  }

  /** Wymienia geometrię warstwy (po wczytaniu albo po zmianie układu projektu). */
  set(id: string, data: GmlLayerData, visible: boolean, hidden: string[]) {
    this.remove(id)
    const features: (Feature | null)[] = new Array(data.decoded.count)
    // Kolejność rysowania: klasy niżej w legendzie pierwsze.
    const order = [...Array(data.decoded.count).keys()].sort(
      (a, b) => look(data.classes[data.cls[a]]).z - look(data.classes[data.cls[b]]).z,
    )
    for (const i of order) {
      const g = geometryOf(data.decoded, i)
      if (!g) {
        features[i] = null
        continue
      }
      const f = new Feature(g)
      f.setProperties({ key: `${id}:${i}`, cls: data.classes[data.cls[i]], label: data.labels[i] }, true)
      features[i] = f
    }
    const source = new VectorSource()
    const layer = new WebGLVectorLayer({ source, style: webglRules(), zIndex: 10, disableHitDetection: true })
    this.map.addLayer(layer)
    const entry: LayerEntry = { source, layer, features, classes: data.classes, visible, hidden: new Set(hidden) }
    this.layers.set(id, entry)
    this.fill(entry)
  }

  private fill(e: LayerEntry) {
    e.source.clear(true)
    const shown = e.features.filter((f): f is Feature => !!f && !e.hidden.has(f.get('cls')))
    e.source.addFeatures(order(shown))
    e.layer.setVisible(e.visible)
    this.refreshLabels()
  }

  private refreshLabels() {
    this.labelSource.clear(true)
    const all: Feature[] = []
    for (const e of this.layers.values()) {
      if (!e.visible) continue
      for (const f of e.features) {
        if (f && !e.hidden.has(f.get('cls')) && look(f.get('cls')).labelBelow !== undefined) all.push(f)
      }
    }
    this.labelSource.addFeatures(all)
  }

  setVisibility(id: string, visible: boolean, hidden: string[]) {
    const e = this.layers.get(id)
    if (!e) return
    const sameHidden = hidden.length === e.hidden.size && hidden.every((h) => e.hidden.has(h))
    if (e.visible === visible && sameHidden) return
    e.visible = visible
    e.hidden = new Set(hidden)
    this.fill(e)
  }

  setLabel(id: string, i: number, label: string) {
    this.layers.get(id)?.features[i]?.set('label', label)
  }

  remove(id: string) {
    const e = this.layers.get(id)
    if (!e) return
    this.map.removeLayer(e.layer)
    e.layer.dispose()
    this.layers.delete(id)
    this.refreshLabels()
  }

  /** Usuwa warstwy spoza listy. */
  keep(ids: string[]) {
    for (const id of [...this.layers.keys()]) if (!ids.includes(id)) this.remove(id)
  }

  setSelection(keys: Iterable<string>) {
    this.selSource.clear(true)
    const out: Feature[] = []
    for (const k of keys) {
      const f = this.feature(k)
      const g = f?.getGeometry()
      if (g) out.push(new Feature(g.clone()))
    }
    this.selSource.addFeatures(out)
  }

  private feature(key: string): Feature | null | undefined {
    const [id, i] = key.split(':')
    return this.layers.get(id)?.features[Number(i)]
  }

  /** Zasięg obiektów (kluczy) albo całej warstwy. */
  extent(keys: string[] | { layer: string }): Extent | null {
    if ('layer' in keys) {
      const e = this.layers.get(keys.layer)
      if (!e) return null
      const ext = createEmpty()
      for (const f of e.features) if (f) extend(ext, f.getGeometry()!.getExtent())
      return ext[0] <= ext[2] ? ext : null
    }
    const ext = createEmpty()
    for (const k of keys) {
      const g = this.feature(k)?.getGeometry()
      if (g) extend(ext, g.getExtent())
    }
    return ext[0] <= ext[2] ? ext : null
  }

  /** Obiekt pod kursorem: punkty i budynki mają pierwszeństwo przed działkami i konturami. */
  pick(coordinate: number[], tolerance: number): string | null {
    const [x, y] = coordinate
    let best: { f: Feature; pick: number; area: number } | null = null
    for (const e of this.layers.values()) {
      if (!e.visible) continue
      for (const f of e.source.getFeaturesInExtent([x - tolerance, y - tolerance, x + tolerance, y + tolerance])) {
        const g = f.getGeometry()!
        const type = g.getType()
        let hit: boolean
        if (type === 'Point' || type === 'MultiPoint' || type === 'LineString' || type === 'MultiLineString') {
          const c = g.getClosestPoint(coordinate)
          hit = Math.hypot(c[0] - x, c[1] - y) <= tolerance
        } else {
          hit = g.intersectsCoordinate(coordinate)
          if (!hit) {
            const c = g.getClosestPoint(coordinate)
            hit = Math.hypot(c[0] - x, c[1] - y) <= tolerance
          }
        }
        if (!hit) continue
        const ext = g.getExtent()
        const cand = { f, pick: look(f.get('cls')).pick, area: (ext[2] - ext[0]) * (ext[3] - ext[1]) }
        if (!best || cand.pick < best.pick || (cand.pick === best.pick && cand.area < best.area)) best = cand
      }
    }
    return best ? (best.f.get('key') as string) : null
  }
}

/** Kolejność dodawania do źródła WebGL zgodna z kolejnością rysowania klas. */
function order(features: Feature[]): Feature[] {
  return features.sort((a, b) => look(a.get('cls')).z - look(b.get('cls')).z)
}
