// Style obiektów GML (EGiB, GESUT, BDOT500) według katalogu znaków umownych mapy zasadniczej:
// rozporządzenie w sprawie BDOT500 oraz mapy zasadniczej (Dz.U. 2021 poz. 1385), załącznik nr 4.
// Znaki są zdefiniowane w milimetrach dla skali 1 : 500; na ekranie 1 mm = 96 / 25,4 px (jak w CSS),
// a linie i symbole mają najmniejszą czytelną grubość lub promień.
// Kod znaku (np. EGDE, SUWP, OTKJ) wylicza rdzeń z klasy i atrybutu rodzaju albo z `kodObiektu`.
// Znaki złożone (kreski poprzeczne skarp, szrafy, symbole drzew i urządzeń) są uproszczone
// do linii, przerywania i koła; kolory, grubości, przerywania i kolejność są z rozporządzenia.
import { VARIANT_OVERHEAD, VARIANT_PLANNED, VARIANT_STABILIZED, VARIANT_UNDERGROUND } from './gmlvariants'

export type Rgba = [number, number, number, number]

export interface LineStyle {
  color: Rgba
  /** Grubość w px (CSS). */
  width: number
  /** Wzór przerywania w px: kreska, odstęp[, kreska, odstęp]. */
  dash?: number[]
  /** Biała obwódka (halo) w px z każdej strony. */
  halo?: number
  /** Grubość i halo przy oddaleniu (rozdzielczość powyżej `res`). */
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

/** Piksele CSS na milimetr. */
const MM = 96 / 25.4
const px = (mm: number) => mm * MM
/** Grubość linii znaku [mm] na ekranie, co najmniej 1 px. */
const lw = (mm: number) => Math.max(1, px(mm))
/** Biała obwódka, żeby czarne linie były widoczne na ortofotomapie (dodatek ekranowy, nie część znaku). */
const HALO = 0.5

// Kolejność prezentacji (zał. 4, rozdz. 1 ust. 12): 1 osnowy na wierzchu … 7 kontury, wody, rzeźba na spodzie.
const ORDER = {
  osnowy: 70,
  granice: 60,
  ewidencja: 50,
  budynki: 40,
  uzbrojenie: 30,
  zagospodarowanie: 20,
  kontury: 10,
}

/** Widoczność: obiekty główne do 10 m/px, szczegóły do 1,5 m/px (około 1 : 5000 na ekranie). */
const RES_MAIN = 10
const RES_DETAIL = 1.5

// Kolory sieci uzbrojenia (litera sieci w kodzie SU?…).
const NET: Record<string, Rgba> = {
  W: rgb(0, 0, 255),
  K: rgb(128, 51, 0),
  E: rgb(255, 0, 0),
  G: rgb(191, 191, 0),
  C: rgb(210, 0, 210),
  T: rgb(255, 145, 0),
  S: BLACK,
  N: BLACK,
  P: BLACK,
  L: BLACK,
}

/** Opis znaku z katalogu: grubość [mm], kolor, przerywanie [mm], symbol punktowy. */
interface Sym {
  w?: number
  c?: Rgba
  dash?: number[]
  order?: number
  maxRes?: number
  point?: 'circle' | 'dot' | 'tree'
  /** Średnica symbolu punktowego [mm]. */
  d?: number
  label?: number
}

// Element + odstęp (a, b) z katalogu; kreska z kropką: a, (b − kropka) / 2, kropka, (b − kropka) / 2.
const dashDot = (a: number, b: number, dot: number) => [a, (b - dot) / 2, dot, (b - dot) / 2]

const CATALOG: Record<string, Sym> = {
  // EGiB
  EGJE: { w: 0.35, dash: dashDot(3, 2, 0.35), order: ORDER.ewidencja + 3, maxRes: 50 },
  EGOE: { w: 0.35, dash: [3, 2], order: ORDER.ewidencja + 2, maxRes: 50, label: 20 },
  EGDE: { w: 0.18, order: ORDER.ewidencja, label: 1 },
  EGPG: { w: 0.18, point: 'circle', d: 1.0, order: ORDER.ewidencja + 5, maxRes: RES_DETAIL, label: 0.25 },
  EGUG: { w: 0.18, c: rgb(0, 128, 0), dash: [1, 1], order: ORDER.kontury, label: 0.7 },
  EGKK: { w: 0.18, c: rgb(36, 188, 36), dash: [3, 1], order: ORDER.kontury + 1, label: 0.7 },
  EGBU: { w: 0.5, order: ORDER.budynki + 5, label: 0.5 },
  EGBN: { w: 0.18, order: ORDER.budynki + 4 },
  EGBP: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  EGBL: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  EGBA: { w: 0.5, dash: [4, 1], order: ORDER.budynki + 4 },
  EGBZ: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  EGBY: { w: 0.18, order: ORDER.budynki + 4 },
  EGBJ: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 3 },
  // BDOT500: budynki i obiekty z nimi związane jak w EGiB
  OTBU: { w: 0.5, order: ORDER.budynki + 5, label: 0.5 },
  OTBB: { w: 0.5, order: ORDER.budynki + 5, label: 0.5 },
  OTBN: { w: 0.18, order: ORDER.budynki + 4 },
  OTBP: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  OTBL: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  OTBA: { w: 0.5, dash: [4, 1], order: ORDER.budynki + 4 },
  OTBZ: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 4 },
  OTBY: { w: 0.18, order: ORDER.budynki + 4 },
  OTBJ: { w: 0.18, dash: [2, 1], order: ORDER.budynki + 3 },
  // BDOT500: budowle
  OTDK: { w: 0.35 },
  OTDC: { w: 0.25 },
  OTDZ: { w: 0.25 },
  OTDS: { w: 0.25 },
  OTDW: { w: 0.25 },
  OTDM: { w: 0.25 },
  OTDA: { w: 0.18, dash: [2, 1] },
  OTDF: { point: 'circle', d: 2.0 },
  OTDT: { point: 'circle', d: 3.5 },
  OTDP: { point: 'circle', d: 3.0 },
  OTDD: { point: 'circle', d: 1.0 },
  OTDI: { point: 'dot', d: 0.8 },
  // BDOT500: ogrodzenia
  OTOO: { w: 0.35 },
  // BDOT500: komunikacja
  OTKJ: { w: 0.18, dash: [3, 1], label: 0.5 },
  OTKK: { w: 0.25 },
  OTKC: { w: 0.18, dash: [2, 1] },
  OTKG: { w: 0.18, dash: [2, 1] },
  OTKU: { w: 0.18, dash: [3, 1] },
  OTKM: { w: 0.35 },
  OTKZ: { w: 0.35 },
  OTKW: { w: 0.35 },
  OTKE: { w: 0.35 },
  OTKT: { w: 0.35 },
  OTKL: { w: 0.18, dash: [1.2, 2] },
  // BDOT500: sport, zagospodarowanie terenu
  OTSI: { w: 0.18, dash: [2, 1], order: ORDER.zagospodarowanie },
  OTZL: { point: 'tree', d: 2.6, c: rgb(0, 128, 0), order: ORDER.zagospodarowanie },
  OTZG: { point: 'tree', d: 3.2, c: rgb(0, 128, 0), order: ORDER.zagospodarowanie },
  // BDOT500: wody i rzeźba terenu
  OTWP: { c: rgb(89, 217, 255), order: ORDER.kontury },
  OTWS: { c: rgb(89, 217, 255), order: ORDER.kontury },
  OTWJ: { w: 0.35, order: ORDER.kontury },
  OTWL: { w: 0.35, order: ORDER.kontury },
  OTWZ: { w: 0.35, order: ORDER.kontury },
  OTRK: { w: 0.18, order: ORDER.kontury },
  OTRS: { point: 'dot', d: 0.6, order: ORDER.kontury, maxRes: 0.5, label: 0.25 },
  OTRN: { point: 'dot', d: 0.6, order: ORDER.kontury, maxRes: 0.5, label: 0.25 },
  // GESUT: wyjątki od koloru sieci i grubości 0,18 mm
  SUWR: { w: 0.25 },
  SUWD: { c: BLACK, point: 'circle', d: 2.0 },
  SUWG: { c: BLACK, point: 'circle', d: 2.0 },
  SUWH: { point: 'circle', d: 2.0 },
  SUWS: { point: 'circle', d: 2.0 },
  SUWW: { point: 'dot', d: 1.2 },
  SUKW: { point: 'dot', d: 1.2 },
  SUGW: { point: 'dot', d: 1.2 },
  SUCW: { point: 'dot', d: 1.2 },
  SUKS: { point: 'circle', d: 2.0 },
  SUKN: { w: 0.25 },
  SUEO: { c: BLACK, point: 'circle', d: 2.0 },
  SUEM: { c: BLACK, point: 'circle', d: 2.0 },
  SUEA: { c: BLACK, point: 'circle', d: 3.0 },
  SUEL: { c: rgb(255, 64, 64), point: 'circle', d: 2.5 },
  SUTW: { c: BLACK, point: 'circle', d: 3.0 },
  SULO: { w: 0.35 },
  SUPP: { point: 'circle', d: 1.5 },
  SUPY: { point: 'circle', d: 1.4 },
}

function prefixOrder(code: string) {
  if (code.startsWith('EG')) return ORDER.ewidencja
  if (code.startsWith('SU')) return ORDER.uzbrojenie
  if (/^OT[SZ]/.test(code)) return ORDER.zagospodarowanie
  if (/^OT[WR]/.test(code)) return ORDER.kontury
  return ORDER.budynki
}

function fromCatalog(code: string, variant: number): ClassStyle {
  const sym = CATALOG[code] ?? {}
  const gesut = code.startsWith('SU')
  const color = sym.c ?? (gesut ? (NET[code[2]] ?? BLACK) : BLACK)
  let dash = sym.dash
  let width = sym.w ?? 0.18
  // Budynek podziemny: znak (B) przerywany 2 mm / 1 mm.
  if (variant & VARIANT_UNDERGROUND) dash = [2, 1]
  if (gesut && code.endsWith('P')) {
    // Przewody nadziemne (znaki B–E) uproszczone do kreski długiej; projektowane: 3 mm na 1 mm.
    if (variant & VARIANT_OVERHEAD) dash = code === 'SUTP' ? [15, 2] : [10, 1]
    if (variant & VARIANT_PLANNED) dash = [3, 1]
  }
  if (code === 'SULO') width = 0.35
  const style: ClassStyle = {
    order: sym.order ?? prefixOrder(code),
    maxRes: sym.maxRes ?? (gesut && !code.endsWith('P') ? RES_DETAIL : RES_MAIN),
    line: { color, width: lw(width), dash: dash?.map(px), halo: HALO },
  }
  // GESUT: opisy przewodów (bez nadziemnych elektroenergetycznych i telekomunikacyjnych) i etykiety „proj.”.
  const gesutLabel =
    gesut &&
    (code.endsWith('P')
      ? !(variant & VARIANT_OVERHEAD && (code === 'SUEP' || code === 'SUTP'))
      : !!(variant & VARIANT_PLANNED))
  if (sym.label !== undefined || gesutLabel) {
    // Opisy: tekst 2,0 mm (przewody 1,5 mm), na ekranie nie mniejszy niż 11 px.
    style.label = { color, maxRes: sym.label ?? 0.3, font: `${Math.max(11, Math.round(px(2)))}px sans-serif` }
  }
  const d = sym.d ?? (gesut ? 1.6 : 1.2)
  const r = Math.max(2.5, px(d) / 2)
  if (code === 'EGPG') {
    // Punkt graniczny stabilizowany (A): okrąg 1,0 mm przesłaniający tło; niestabilizowany (B): kropka.
    style.point =
      variant & VARIANT_STABILIZED
        ? { radius: r, stroke: BLACK, strokeWidth: lw(0.18), fill: WHITE }
        : { radius: 1.6, stroke: BLACK, strokeWidth: 1, fill: BLACK }
  } else if (sym.point === 'dot') {
    style.point = { radius: Math.max(1.5, px(d) / 2), stroke: color, strokeWidth: 1, fill: color }
  } else if (sym.point === 'tree') {
    style.point = { radius: r, stroke: color, strokeWidth: lw(0.18), dot: 1, dotColor: color }
  } else {
    style.point = { radius: r, stroke: color, strokeWidth: lw(0.18), fill: WHITE }
  }
  return style
}

const DEFAULT: ClassStyle = {
  order: 3,
  maxRes: 50,
  line: { color: rgb(56, 189, 248), width: 1.2 },
  fill: rgb(56, 189, 248, 0.08),
  point: { radius: 3, stroke: BLACK, strokeWidth: 1, fill: rgb(56, 189, 248) },
}

/** Domyślny kod dla obiektów bez znaku w katalogu, zależny od bazy danych. */
function fallbackCode(cls: string) {
  if (cls.startsWith('EGB_')) return 'EG'
  if (cls.startsWith('GES_')) return 'SUN'
  // BDOT500: klasy OT_ bez przyrostka geometrii (BDOT10k ma postać OT_XXXX_A/L/P).
  if (cls.startsWith('OT_') && !/^OT_[A-Z]{4}_[APL]$/.test(cls)) return 'OT'
  return ''
}

/** Styl obiektu: klasa, kod znaku umownego z rdzenia i wariant (bity `VARIANT_*`). */
export function styleOf(cls: string, code = '', variant = 0): ClassStyle {
  const c = code || fallbackCode(cls)
  return c ? fromCatalog(c, variant) : DEFAULT
}

/** Pierwszeństwo przy wyborze kliknięciem: punkty przed liniami i powierzchniami, wyższe warstwy przed niższymi. */
export function pickPriority(s: ClassStyle, point: boolean) {
  return (point ? 1000 : 0) + s.order
}
