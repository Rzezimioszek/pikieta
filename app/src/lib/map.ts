import 'ol/ol.css'
import Feature from 'ol/Feature'
import Map from 'ol/Map'
import View from 'ol/View'
import { boundingExtent } from 'ol/extent'
import { LineString, Point, Polygon } from 'ol/geom'
import ImageLayer from 'ol/layer/Image'
import VectorLayer from 'ol/layer/Vector'
import { transform } from 'ol/proj'
import { register } from 'ol/proj/proj4'
import ImageWMS from 'ol/source/ImageWMS'
import VectorSource from 'ol/source/Vector'
import { Circle, Fill, Stroke, Style, Text } from 'ol/style'
import proj4 from 'proj4'
import { GmlMapLayers, type GmlLayerData } from './gmlmap'

/** Punkt w konwencji geodezyjnej: x na północ, y na wschód. */
export interface GeoPoint {
  x: number
  y: number
}

export const CRS_LIST = [
  { code: 'EPSG:2176', label: 'PL-2000 strefa 5 (15°E)' },
  { code: 'EPSG:2177', label: 'PL-2000 strefa 6 (18°E)' },
  { code: 'EPSG:2178', label: 'PL-2000 strefa 7 (21°E)' },
  { code: 'EPSG:2179', label: 'PL-2000 strefa 8 (24°E)' },
  { code: 'EPSG:2180', label: 'PL-1992' },
] as const

const GRS80 = '+ellps=GRS80 +towgs84=0,0,0,0,0,0,0 +units=m +no_defs'
for (const [zone, lon] of [[5, 15], [6, 18], [7, 21], [8, 24]]) {
  proj4.defs(
    `EPSG:${2171 + zone}`,
    `+proj=tmerc +lat_0=0 +lon_0=${lon} +k=0.999923 +x_0=${zone}500000 +y_0=0 ${GRS80}`,
  )
}
proj4.defs('EPSG:2180', `+proj=tmerc +lat_0=0 +lon_0=19 +k=0.9993 +x_0=500000 +y_0=-5300000 ${GRS80}`)
register(proj4)

export const BASEMAPS = [
  { id: 'orto', label: 'Ortofotomapa' },
  { id: 'bdot10k', label: 'BDOT10k' },
] as const
export type BasemapId = (typeof BASEMAPS)[number]['id']

const GEOPORTAL = 'https://mapy.geoportal.gov.pl/wss/service'
const ORTO_WMS = `${GEOPORTAL}/PZGIK/ORTO/WMS/StandardResolution`
const BDOT_WMS = `${GEOPORTAL}/pub/guest/kompozycja_BDOT10k_WMS/MapServer/WMSServer`

// WMS 1.1.1: kolejność osi w BBOX zawsze wschód, północ.
function wmsSource(url: string, params: Record<string, string>, attributions: string): ImageWMS {
  return new ImageWMS({
    url,
    params: { VERSION: '1.1.1', ...params },
    ratio: 1,
    crossOrigin: 'anonymous',
    attributions,
  })
}

/**
 * Usługa BDOT10k nie ma warstwy zbiorczej, więc listę warstw bierzemy z GetCapabilities.
 * Capabilities wymienia warstwy od wierzchniej, a LAYERS rysuje od spodniej, stąd odwrócenie.
 */
async function bdotSource(): Promise<ImageWMS> {
  const res = await fetch(`${BDOT_WMS}?SERVICE=WMS&REQUEST=GetCapabilities&VERSION=1.1.1`)
  if (!res.ok) throw new Error(`GetCapabilities BDOT10k: HTTP ${res.status}`)
  const doc = new DOMParser().parseFromString(await res.text(), 'text/xml')
  const names = Array.from(doc.querySelectorAll('Layer > Name'), (n) => n.textContent ?? '')
  if (names.length === 0) throw new Error('GetCapabilities BDOT10k: brak warstw')
  return wmsSource(
    BDOT_WMS,
    { LAYERS: names.reverse().join(','), FORMAT: 'image/png', TRANSPARENT: 'FALSE', BGCOLOR: '0xFFFFFF' },
    'BDOT10k © GUGiK (geoportal.gov.pl)',
  )
}

export interface MapPoint extends GeoPoint {
  id: string
}

/** Szkic bieżącego zadania: łamane przez punkty wejściowe i ewentualny punkt wynikowy. */
export interface Sketch {
  paths: GeoPoint[][]
  /** Pierwsza łamana jest wielobokiem. */
  closed?: boolean
  /** Punkty wynikowe, jeszcze nie dodane do projektu. */
  results?: (GeoPoint & { label?: string })[]
}

export interface MapHandle {
  setCrs(code: string): void
  /** Odrzuca obietnicę, gdy podkładu nie da się wczytać; poprzedni podkład zostaje. */
  setBasemap(id: BasemapId): Promise<void>
  setPoints(points: MapPoint[], selected: ReadonlySet<string>): void
  setSketch(sketch: Sketch): void
  zoomTo(points: GeoPoint[]): void
  setGmlLayer(id: string, data: GmlLayerData, visible: boolean, hidden: string[]): void
  setGmlVisibility(id: string, visible: boolean, hidden: string[]): void
  setGmlLabel(id: string, i: number, label: string): void
  keepGmlLayers(ids: string[]): void
  setGmlSelection(keys: Iterable<string>): void
  /** Przybliża do obiektów GML (klucze `warstwa:numer`) albo całej warstwy. */
  zoomToGml(target: string[] | { layer: string }): void
  destroy(): void
}

interface Callbacks {
  onPointer(p: GeoPoint | null): void
  /**
   * `pointId` to numer klikniętego punktu projektu, `gmlKey` klucz obiektu GML (gdy nie trafiono punktu);
   * `additive` to kliknięcie z Ctrl lub Shift.
   */
  onClick(p: GeoPoint, pointId: string | null, additive: boolean, gmlKey: string | null): void
}

// OpenLayers trzyma współrzędne jako [wschód, północ].
const toGeo = (c: number[]): GeoPoint => ({ x: c[1], y: c[0] })
const toOl = (p: GeoPoint): number[] => [p.y, p.x]

const HALO = new Stroke({ color: '#111', width: 3 })

function labelled(radius: number, color: string, font: string): Style {
  return new Style({
    image: new Circle({
      radius,
      fill: new Fill({ color }),
      stroke: new Stroke({ color: '#111', width: 2 }),
    }),
    text: new Text({
      offsetY: -(radius + 9),
      font,
      fill: new Fill({ color: '#fff' }),
      stroke: HALO,
    }),
  })
}

export function createMap(target: HTMLElement, crs: string, cb: Callbacks): MapHandle {
  const orto = wmsSource(
    ORTO_WMS,
    { LAYERS: 'Raster', FORMAT: 'image/jpeg' },
    'Ortofotomapa © GUGiK (geoportal.gov.pl)',
  )
  const basemap = new ImageLayer({ source: orto })
  let bdot: Promise<ImageWMS> | undefined
  let wanted: BasemapId = 'orto'

  // Jeden styl na warstwę, z podmienianą etykietą: tani nawet dla tysięcy punktów.
  const pointStyle = labelled(4.5, '#38bdf8', '12px sans-serif')
  const selectedStyle = labelled(7, '#fbbf24', 'bold 13px sans-serif')
  const resultStyle = labelled(6, '#f43f5e', 'bold 13px sans-serif')

  const pointSource = new VectorSource()
  const pointLayer = new VectorLayer({
    source: pointSource,
    declutter: true,
    style: (f) => {
      const style = f.get('selected') ? selectedStyle : pointStyle
      style.getText()!.setText(f.get('pid') as string)
      return style
    },
  })
  const sketchSource = new VectorSource()
  const sketchLayer = new VectorLayer({
    source: sketchSource,
    style: (f) => {
      if (f.getGeometry()!.getType() === 'Point') {
        resultStyle.getText()!.setText((f.get('label') as string | undefined) ?? '')
        return resultStyle
      }
      return sketchStyle
    },
  })
  const sketchStyle = new Style({
    stroke: new Stroke({ color: '#fbbf24', width: 2.5, lineDash: [8, 5] }),
    fill: new Fill({ color: 'rgb(251 191 36 / 0.18)' }),
  })

  const map = new Map({
    target,
    layers: [basemap, sketchLayer, pointLayer],
    view: new View({
      projection: crs,
      center: transform([21.0122, 52.2297], 'EPSG:4326', crs),
      resolution: 2,
    }),
  })

  map.on('pointermove', (e) => cb.onPointer(toGeo(e.coordinate)))
  map.on('singleclick', (e) => {
    const hit = map.forEachFeatureAtPixel(e.pixel, (f) => f.get('pid') as string | undefined, {
      layerFilter: (l) => l === pointLayer,
      hitTolerance: 8,
    })
    const ev = e.originalEvent as MouseEvent
    const gmlHit = hit ? null : gmlLayers.pick(e.coordinate, 5 * map.getView().getResolution()!)
    cb.onClick(toGeo(e.coordinate), hit ?? null, ev.ctrlKey || ev.metaKey || ev.shiftKey, gmlHit ?? null)
  })
  target.addEventListener('pointerleave', () => cb.onPointer(null))
  const gmlLayers = new GmlMapLayers(map)
  sketchLayer.setZIndex(30)
  pointLayer.setZIndex(31)
  // Panel obok mapy zmienia jej rozmiar (tryby widoku, tabela, lista obliczeń).
  const resize = new ResizeObserver(() => map.updateSize())
  resize.observe(target)

  return {
    setCrs(code) {
      const old = map.getView()
      const from = old.getProjection()
      if (from.getCode() === code) return
      map.setView(
        new View({
          projection: code,
          center: transform(old.getCenter()!, from, code),
          resolution: old.getResolution(),
        }),
      )
    },
    async setBasemap(id) {
      wanted = id
      if (id === 'orto') {
        basemap.setSource(orto)
        return
      }
      bdot ??= bdotSource()
      try {
        const source = await bdot
        if (wanted === id) basemap.setSource(source)
      } catch (e) {
        bdot = undefined
        throw e
      }
    },
    setPoints(points, selected) {
      pointSource.clear(true)
      pointSource.addFeatures(
        points.map((p) => {
          const f = new Feature(new Point(toOl(p)))
          f.set('pid', p.id, true)
          f.set('selected', selected.has(p.id), true)
          return f
        }),
      )
    },
    setSketch({ paths, closed, results }) {
      sketchSource.clear(true)
      paths.forEach((path, i) => {
        const coords = path.map(toOl)
        if (closed && i === 0 && coords.length >= 3) {
          sketchSource.addFeature(new Feature(new Polygon([[...coords, coords[0]]])))
        } else if (coords.length >= 2) {
          sketchSource.addFeature(new Feature(new LineString(coords)))
        }
      })
      for (const r of results ?? []) {
        const f = new Feature(new Point(toOl(r)))
        f.set('label', r.label ?? '', true)
        sketchSource.addFeature(f)
      }
    },
    zoomTo(points) {
      if (points.length === 0) return
      const view = map.getView()
      const maxZoom = view.getZoomForResolution(0.25)
      view.fit(boundingExtent(points.map(toOl)), {
        padding: [60, 60, 60, 60],
        maxZoom,
        duration: 300,
      })
    },
    setGmlLayer: (id, data, visible, hidden) => gmlLayers.set(id, data, visible, hidden),
    setGmlVisibility: (id, visible, hidden) => gmlLayers.setVisibility(id, visible, hidden),
    setGmlLabel: (id, i, label) => gmlLayers.setLabel(id, i, label),
    keepGmlLayers: (ids) => gmlLayers.keep(ids),
    setGmlSelection: (keys) => gmlLayers.setSelection(keys),
    zoomToGml(target) {
      const ext = gmlLayers.extent(target)
      if (!ext) return
      const view = map.getView()
      view.fit(ext, { padding: [60, 60, 60, 60], maxZoom: view.getZoomForResolution(0.1), duration: 300 })
    },
    destroy() {
      resize.disconnect()
      map.setTarget(undefined)
    },
  }
}

