// Style obiektów GML według rozporządzeń (folder isap):
// - EGiB: rozporządzenie w sprawie ewidencji gruntów i budynków (tekst jednolity Dz.U. 2024 poz. 219),
//   załącznik nr 8, tabela nr 1: style usług WMS (kolory RGB, grubości w px, przerywania, zakresy skal);
// - GESUT: rozporządzenie w sprawie GESUT (Dz.U. 2021 poz. 1374), załącznik nr 3: linie 1 px z białym halo 1 px,
//   w skalach 0,5–0,1 px/m linie 2 px z halo 2 px; kolory sieci z katalogu znaków mapy zasadniczej;
// - BDOT500: rozporządzenie w sprawie BDOT500 i mapy zasadniczej (Dz.U. 2021 poz. 1385), załącznik nr 4:
//   kolory i grubości znaków (mm w skali 1:500 przeliczone na px, 1 mm ≈ 3,8 px przy 96 dpi).
// Zakresy skal w rozporządzeniach podane są w px/m; tu jako rozdzielczość w m/px (odwrotność).
import { VARIANT_ACCURATE, VARIANT_PLANNED, VARIANT_STABILIZED } from './gmlvariants'

export type Rgba = [number, number, number, number]

export interface LineStyle {
  color: Rgba
  /** Grubość w px (CSS). */
  width: number
  /** Kreska i odstęp w px. */
  dash?: [number, number]
  /** Biała obwódka (halo) w px z każdej strony. */
  halo?: number
  /** Grubość i halo przy oddaleniu (rozdzielczość powyżej `farRes`). */
  far?: { res: number; width: number; halo?: number }
}

export interface PointStyle {
  /** Promień koła w px. */
  radius: number
  stroke: Rgba
  strokeWidth: number
  fill?: Rgba
  /** Promień kropki w środku w px. */
  dot?: number
  dotColor?: Rgba
}

export interface LabelStyle {
  color: Rgba
  /** Opisy widoczne przy rozdzielczości nie większej niż ta [m/px]. */
  maxRes: number
  font: string
}

export interface ClassStyle {
  /** Kolejność rysowania: większa wartość wyżej. */
  order: number
  /** Obiekt widoczny przy rozdzielczości nie większej niż ta [m/px]. */
  maxRes: number
  line?: LineStyle
  fill?: Rgba
  point?: PointStyle
  label?: LabelStyle
}

const rgb = (r: number, g: number, b: number, a = 1): Rgba => [r / 255, g / 255, b / 255, a]
const BLACK = rgb(0, 0, 0)
const WHITE = rgb(255, 255, 255)
const label = (color: Rgba, maxRes: number, font = '12px sans-serif'): LabelStyle => ({ color, maxRes, font })

/** 0,1 px/m z rozporządzeń, czyli do 10 m/px. */
const RES_FAR = 10

// --- EGiB: załącznik nr 8, tabela nr 1 (kolejność na mapie: 1 = na wierzchu, 10 = na spodzie) ---
const EGIB: Record<string, (variant: number) => ClassStyle> = {
  EGB_KonturUzytkuGruntowego: () => ({
    order: 0,
    maxRes: RES_FAR,
    line: { color: rgb(0, 0, 128), width: 1, dash: [5, 5] },
    label: label(rgb(0, 0, 128), 0.7),
  }),
  EGB_KonturKlasyfikacyjny: () => ({
    order: 2,
    maxRes: RES_FAR,
    line: { color: rgb(36, 188, 36), width: 1, dash: [9, 7] },
    label: label(rgb(36, 188, 36), 0.7),
  }),
  EGB_DzialkaEwidencyjna: () => ({
    order: 4,
    maxRes: RES_FAR,
    line: { color: rgb(64, 160, 255), width: 1 },
    label: label(rgb(64, 160, 255), 1, 'bold 12px sans-serif'),
  }),
  EGB_Budynek: () => ({
    order: 6,
    maxRes: RES_FAR,
    line: { color: rgb(200, 0, 0), width: 2 },
    label: label(rgb(200, 0, 0), 0.5),
  }),
  // Koło o średnicy 6 px: niebieskie dla SPD = 1 i ISD = 1, czerwone w pozostałych przypadkach;
  // czarna kropka 3 px, gdy punkt jest stabilizowany (STB = 3, 4, 5, 6).
  EGB_PunktGraniczny: (v) => ({
    order: 8,
    maxRes: RES_FAR,
    point: {
      radius: 3,
      stroke: v & VARIANT_ACCURATE ? rgb(0, 0, 255) : rgb(255, 0, 0),
      // Rozporządzenie podaje 0,5 px; na ekranie i ortofotomapie taka linia jest prawie niewidoczna.
      strokeWidth: 1,
      dot: v & VARIANT_STABILIZED ? 1.5 : undefined,
      dotColor: BLACK,
    },
    label: label(BLACK, 0.25, '11px sans-serif'),
  }),
  // Obręby i jednostki nie mają stylu w tabeli usług EGiB; przyjęto grubszą linię przerywaną w kolorze działek.
  EGB_ObrebEwidencyjny: () => ({
    order: 3,
    maxRes: 200,
    line: { color: rgb(64, 160, 255), width: 2.5, dash: [12, 4] },
    label: label(rgb(64, 160, 255), 20, 'bold 13px sans-serif'),
  }),
  EGB_JednostkaEwidencyjna: () => ({
    order: 3,
    maxRes: 500,
    line: { color: rgb(160, 60, 200), width: 3, dash: [14, 5] },
  }),
}

// --- GESUT: kolory sieci z katalogu znaków mapy zasadniczej ---
const NETWORKS: [RegExp, Rgba][] = [
  [/Wodociag/i, rgb(0, 0, 255)],
  [/Kanalizac/i, rgb(128, 51, 0)],
  [/Elektroenerget/i, rgb(255, 0, 0)],
  [/Gaz/i, rgb(191, 191, 0)],
  [/Cieplow/i, rgb(210, 0, 210)],
  [/Telekomunik/i, rgb(255, 145, 0)],
]

function gesut(cls: string, v: number): ClassStyle {
  const color = NETWORKS.find(([re]) => re.test(cls))?.[1] ?? BLACK
  const device = /Urzadzeni/i.test(cls)
  return {
    order: device ? 7 : 5,
    maxRes: RES_FAR,
    line: {
      color,
      width: 1,
      halo: 1,
      far: { res: 2, width: 2, halo: 2 },
      // Projektowane: linia przerywana 3 mm na 1 mm.
      dash: v & VARIANT_PLANNED ? [11, 4] : undefined,
    },
    point: { radius: 3, stroke: color, strokeWidth: 1, fill: WHITE },
    label: label(color, 0.3, '10px sans-serif'),
  }
}

// --- BDOT500: katalog znaków mapy zasadniczej (w GML klasy są zgrubne, więc styl na klasę) ---
function bdot500(cls: string): ClassStyle {
  const base: ClassStyle = { order: 3, maxRes: RES_FAR, line: { color: BLACK, width: 1, halo: 0.5 } }
  switch (cls) {
    case 'OT_BudynekNiewykazanyWEGIB':
      // Budynek: linia 0,5 mm.
      return { ...base, order: 6, line: { color: BLACK, width: 2, halo: 0.5 } }
    case 'OT_BlokBudynku':
    case 'OT_ObiektTrwaleZwiazanyZBudynkami':
      return { ...base, order: 6 }
    case 'OT_Wody':
      return { ...base, order: 1, line: { color: rgb(89, 217, 255), width: 1 }, fill: rgb(89, 217, 255, 0.25) }
    case 'OT_ZagospodarowanieTerenu':
      return { ...base, order: 1, line: { color: rgb(0, 128, 0), width: 1, halo: 0.5 } }
    case 'OT_Ogrodzenia':
      return { ...base, order: 4, line: { color: BLACK, width: 1, dash: [6, 2], halo: 0.5 } }
    case 'OT_Komunikacja':
      return { ...base, order: 2 }
    case 'OT_Skarpa':
      return { ...base, order: 1, line: { color: BLACK, width: 1, dash: [2, 3], halo: 0.5 } }
    default:
      return { ...base, point: { radius: 2.5, stroke: BLACK, strokeWidth: 1, fill: WHITE } }
  }
}

const DEFAULT: ClassStyle = {
  order: 3,
  maxRes: 50,
  line: { color: rgb(56, 189, 248), width: 1.2 },
  fill: rgb(56, 189, 248, 0.08),
  point: { radius: 3, stroke: BLACK, strokeWidth: 1, fill: rgb(56, 189, 248) },
}

/** Styl obiektu danej klasy i wariantu (wariant z rdzenia: atrybuty takie jak SPD, ISD, STB, status). */
export function styleOf(cls: string, variant = 0): ClassStyle {
  if (cls in EGIB) return EGIB[cls](variant)
  if (cls.startsWith('GES_')) return gesut(cls, variant)
  // BDOT500: klasy OT_ bez przyrostka geometrii (BDOT10k ma postać OT_XXXX_A/L/P).
  if (cls.startsWith('OT_') && !/^OT_[A-Z]{4}_[APL]$/.test(cls)) return bdot500(cls)
  return DEFAULT
}

/** Pierwszeństwo przy wyborze kliknięciem: punkty przed liniami i powierzchniami, wyższe warstwy przed niższymi. */
export function pickPriority(s: ClassStyle) {
  return (s.point ? 100 : 0) + s.order
}
