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
import { BASEMAPS, OVERLAYS, requestCrs, type WmsService } from './services'

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
for (const [zone, lon] of [
  [5, 15],
  [6, 18],
  [7, 21],
  [8, 24],
]) {
  proj4.defs(`EPSG:${2171 + zone}`, `+proj=tmerc +lat_0=0 +lon_0=${lon} +k=0.999923 +x_0=${zone}500000 +y_0=0 ${GRS80}`)
}
proj4.defs('EPSG:2180', `+proj=tmerc +lat_0=0 +lon_0=19 +k=0.9993 +x_0=500000 +y_0=-5300000 ${GRS80}`)
register(proj4)

export { BASEMAPS, OVERLAYS } from './services'
export type BasemapId = string

/** Warstwy z GetCapabilities (od spodniej), dla usług bez warstwy zbiorczej, np. BDOT10k. */
const capabilities = new globalThis.Map<string, Promise<string>>()
function allLayers(url: string): Promise<string> {
  let p = capabilities.get(url)
  if (!p) {
    p = (async () => {
      const res = await fetch(`${url}?SERVICE=WMS&REQUEST=GetCapabilities&VERSION=1.1.1`)
      if (!res.ok) throw new Error(`GetCapabilities: HTTP ${res.status}`)
      const doc = new DOMParser().parseFromString(await res.text(), 'text/xml')
      const names = Array.from(doc.querySelectorAll('Layer > Name'), (n) => n.textContent ?? '')
      if (names.length === 0) throw new Error('GetCapabilities: brak warstw')
      // Capabilities wymienia warstwy od wierzchniej, a LAYERS rysuje od spodniej.
      return names.reverse().join(',')
    })()
    p.catch(() => capabilities.delete(url))
    capabilities.set(url, p)
  }
  return p
}

/**
 * Źródło WMS 1.1.1 (kolejność osi w BBOX zawsze wschód, północ). Gdy usługa nie obsługuje układu
 * mapy, obraz jest pobierany w jej układzie i przeliczany; `null`, gdy w tym układzie się nie da.
 */
async function wmsFor(s: WmsService, mapCrs: string, base: boolean): Promise<ImageWMS | null> {
  const crs = requestCrs(s, mapCrs)
  if (!crs) return null
  const layers = s.layers === '*' ? await allLayers(s.url) : s.layers
  const format = s.format ?? 'image/png'
  return new ImageWMS({
    url: s.url,
    params: {
      VERSION: '1.1.1',
      LAYERS: layers,
      FORMAT: format,
      TRANSPARENT: base ? 'FALSE' : 'TRUE',
      ...(base && format === 'image/png' ? { BGCOLOR: '0xFFFFFF' } : {}),
    },
    projection: crs === mapCrs ? undefined : crs,
    ratio: 1,
    crossOrigin: s.cors ? 'anonymous' : undefined,
    attributions: s.attribution,
  })
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
  /** Nakładki z katalogu usług, w kolejności rysowania (od spodniej). */
  setOverlays(ids: string[]): void
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
  const basemap = new ImageLayer({ zIndex: 0 })
  let wanted: BasemapId = 'orto'
  let overlayIds: string[] = []
  const overlays = new globalThis.Map<string, ImageLayer<ImageWMS>>()
  const crsNow = () => map.getView().getProjection().getCode()

  async function applyBasemap() {
    const id = wanted
    const s = BASEMAPS.find((b) => b.id === id) ?? BASEMAPS[0]
    const source = await wmsFor(s, crsNow(), true)
    if (wanted !== id) return
    if (!source) throw new Error(`${s.label}: usługa nie obsługuje układu mapy`)
    basemap.setSource(source)
  }

  function applyOverlays() {
    const crs = crsNow()
    for (const [id, layer] of overlays) {
      if (!overlayIds.includes(id)) {
        map.removeLayer(layer)
        overlays.delete(id)
      }
    }
    overlayIds.forEach((id, i) => {
      const s = OVERLAYS.find((o) => o.id === id)
      if (!s) return
      let layer = overlays.get(id)
      if (!layer) {
        layer = new ImageLayer<ImageWMS>({ opacity: s.opacity ?? 1 })
        overlays.set(id, layer)
        map.addLayer(layer)
      }
      layer.setZIndex(1 + i)
      // Źródło zależy od układu mapy; przy zmianie układu budujemy je od nowa.
      if (layer.get('crs') !== crs) {
        layer.set('crs', crs)
        wmsFor(s, crs, false).then((src) => {
          if (layer.get('crs') === crs) layer.setSource(src)
        })
      }
    })
  }

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
    const res = map.getView().getResolution()!
    const gmlHit = hit ? null : gmlLayers.pick(e.coordinate, 5 * res, res)
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
      applyBasemap().catch(() => {})
      applyOverlays()
    },
    async setBasemap(id) {
      wanted = id
      await applyBasemap()
    },
    setOverlays(ids) {
      overlayIds = [...ids]
      applyOverlays()
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
