import 'ol/ol.css'
import Feature from 'ol/Feature'
import Map from 'ol/Map'
import View from 'ol/View'
import { LineString, Point } from 'ol/geom'
import ImageLayer from 'ol/layer/Image'
import VectorLayer from 'ol/layer/Vector'
import { transform } from 'ol/proj'
import { register } from 'ol/proj/proj4'
import ImageWMS from 'ol/source/ImageWMS'
import VectorSource from 'ol/source/Vector'
import { Circle, Fill, Stroke, Style, Text } from 'ol/style'
import proj4 from 'proj4'

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

const ORTO_WMS = 'https://mapy.geoportal.gov.pl/wss/service/PZGIK/ORTO/WMS/StandardResolution'

export interface MapHandle {
  setCrs(code: string): void
  setSegment(a: GeoPoint | null, b: GeoPoint | null): void
  destroy(): void
}

interface Callbacks {
  onPointer(p: GeoPoint | null): void
  onClick(p: GeoPoint): void
}

// OpenLayers trzyma współrzędne jako [wschód, północ].
const toGeo = (c: number[]): GeoPoint => ({ x: c[1], y: c[0] })
const toOl = (p: GeoPoint): number[] => [p.y, p.x]

function pointStyle(label: string): Style {
  return new Style({
    image: new Circle({
      radius: 6,
      fill: new Fill({ color: '#fbbf24' }),
      stroke: new Stroke({ color: '#111', width: 2 }),
    }),
    text: new Text({
      text: label,
      offsetY: -16,
      font: 'bold 14px sans-serif',
      fill: new Fill({ color: '#fff' }),
      stroke: new Stroke({ color: '#111', width: 3 }),
    }),
  })
}

export function createMap(target: HTMLElement, crs: string, cb: Callbacks): MapHandle {
  const segment = new VectorSource()
  const map = new Map({
    target,
    layers: [
      new ImageLayer({
        source: new ImageWMS({
          url: ORTO_WMS,
          // WMS 1.1.1: kolejność osi w BBOX zawsze wschód, północ.
          params: { LAYERS: 'Raster', VERSION: '1.1.1', FORMAT: 'image/jpeg' },
          ratio: 1,
          crossOrigin: 'anonymous',
          attributions: 'Ortofotomapa © GUGiK (geoportal.gov.pl)',
        }),
      }),
      new VectorLayer({
        source: segment,
        style: new Style({ stroke: new Stroke({ color: '#fbbf24', width: 3 }) }),
      }),
    ],
    view: new View({
      projection: crs,
      center: transform([21.0122, 52.2297], 'EPSG:4326', crs),
      resolution: 2,
    }),
  })

  map.on('pointermove', (e) => cb.onPointer(toGeo(e.coordinate)))
  map.on('singleclick', (e) => cb.onClick(toGeo(e.coordinate)))
  target.addEventListener('pointerleave', () => cb.onPointer(null))

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
    setSegment(a, b) {
      segment.clear()
      for (const [p, label] of [[a, 'A'], [b, 'B']] as const) {
        if (!p) continue
        const f = new Feature(new Point(toOl(p)))
        f.setStyle(pointStyle(label))
        segment.addFeature(f)
      }
      if (a && b) segment.addFeature(new Feature(new LineString([toOl(a), toOl(b)])))
    },
    destroy() {
      map.setTarget(undefined)
    },
  }
}
