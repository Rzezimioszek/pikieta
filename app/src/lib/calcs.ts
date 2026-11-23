// Rodzaje obliczeń: opis, ikona, dane początkowe i automatyczny tytuł sesji.
import type { IconName } from './icons'
import type { CalcSession } from './project.svelte'

export type CalcType =
  | 'azimuth'
  | 'polar'
  | 'offset'
  | 'fromOffset'
  | 'distInt'
  | 'lineInt'
  | 'area'
  | 'traverse'
  | 'tachy'
  | 'stakeout'
  | 'resection'
  | 'angleInt'

export interface AzimuthData {
  rows: { from: string; to: string }[]
}
export interface PolarData {
  station: string
  ref: string
  rows: { id: string; angle: string; dist: string }[]
}
export interface OffsetData {
  a: string
  b: string
  rows: { id: string }[]
}
export interface FromOffsetData {
  a: string
  b: string
  rows: { id: string; along: string; offset: string }[]
}
export interface DistIntData {
  a: string
  b: string
  ra: string
  rb: string
  side: 'left' | 'right'
  id: string
}
export interface LineIntData {
  a: string
  b: string
  c: string
  d: string
  id: string
}
export interface AreaData {
  rows: { id: string }[]
}
export interface AngleIntData {
  a: string
  b: string
  alpha: string
  beta: string
  side: 'left' | 'right'
  id: string
}
export interface ResectionData {
  id: string
  rows: { id: string; hz: string }[]
}
export interface StakeoutData {
  station: string
  ref: string
  /** Odczyt koła poziomego na nawiązaniu [g]; puste = 0. */
  refReading: string
  /** Długości w terenie (odwrócenie redukcji odwzorowawczej i na elipsoidę). */
  reduce: 'yes' | 'no'
  rows: { id: string }[]
}
export interface TachyData {
  station: string
  /** Wysokość stanowiska; puste: z punktu w projekcie. */
  stationH: string
  instrument: string
  /** Poprawka na krzywiznę Ziemi i refrakcję. */
  curvature: 'yes' | 'no'
  /** Redukcja na elipsoidę i odwzorowawcza. */
  reduce: 'yes' | 'no'
  refs: { id: string; hz: string }[]
  rows: { id: string; code: string; hz: string; v: string; s: string; target: string }[]
}
export interface TraverseData {
  /** Dwustronnie nawiązany, zamknięty albo wiszący. */
  kind: 'connected' | 'closed' | 'open'
  /** Kąty lewe albo prawe. */
  angles: 'left' | 'right'
  back: string
  forward: string
  /** Redukcja odwzorowawcza długości (skala odwzorowania boku). */
  reduce: 'yes' | 'no'
  /** Błąd średni pomiaru kąta [cc] do odchyłki dopuszczalnej; puste: bez sprawdzania. */
  mAngle: string
  /** Dopuszczalna odchyłka względna 1 : N; puste: bez sprawdzania. */
  relLimit: string
  rows: { id: string; angle: string; dist: string }[]
}

export interface CalcInfo {
  label: string
  /** Krótka nazwa na przycisk wstążki. */
  short: string
  icon: IconName
  hint: string
  init: () => object
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  title: (data: any) => string
}

const filled = (...v: string[]) => v.every((s) => s.trim() !== '')

export const CALCS: Record<CalcType, CalcInfo> = {
  azimuth: {
    label: 'Azymut i długość',
    short: 'Azymut',
    icon: 'azimuth',
    hint: 'Azymuty i długości odcinków, z redukcją na elipsoidę.',
    init: (): AzimuthData => ({ rows: [{ from: '', to: '' }] }),
    title: (d: AzimuthData) => {
      const rows = d.rows.filter((r) => filled(r.from, r.to))
      if (rows.length === 0) return 'Azymut i długość'
      const first = `${rows[0].from.trim()}→${rows[0].to.trim()}`
      return rows.length === 1 ? `Azymut ${first}` : `Azymuty ${first} +${rows.length - 1}`
    },
  },
  polar: {
    label: 'Metoda biegunowa',
    short: 'Biegunowa',
    icon: 'polar',
    hint: 'Nowe punkty ze stanowiska po kącie od nawiązania i długości.',
    init: (): PolarData => ({ station: '', ref: '', rows: [{ id: '', angle: '', dist: '' }] }),
    title: (d: PolarData) => (filled(d.station) ? `Biegunowe ze st. ${d.station.trim()}` : 'Metoda biegunowa'),
  },
  offset: {
    label: 'Rzutowanie na linię',
    short: 'Rzutowanie',
    icon: 'offset',
    hint: 'Miary bieżące i domiary istniejących punktów względem linii.',
    init: (): OffsetData => ({ a: '', b: '', rows: [{ id: '' }] }),
    title: (d: OffsetData) => (filled(d.a, d.b) ? `Rzutowanie na ${d.a.trim()}–${d.b.trim()}` : 'Rzutowanie na linię'),
  },
  fromOffset: {
    label: 'Punkty z domiarów',
    short: 'Z domiarów',
    icon: 'fromOffset',
    hint: 'Nowe punkty z miar bieżących i domiarów względem linii.',
    init: (): FromOffsetData => ({ a: '', b: '', rows: [{ id: '', along: '', offset: '' }] }),
    title: (d: FromOffsetData) => (filled(d.a, d.b) ? `Domiary od ${d.a.trim()}–${d.b.trim()}` : 'Punkty z domiarów'),
  },
  distInt: {
    label: 'Wcięcie liniowe',
    short: 'Liniowe',
    icon: 'distInt',
    hint: 'Punkt z długości od dwóch punktów znanych.',
    init: (): DistIntData => ({ a: '', b: '', ra: '', rb: '', side: 'left', id: '' }),
    title: (d: DistIntData) => (filled(d.a, d.b) ? `Wcięcie liniowe ${d.a.trim()}, ${d.b.trim()}` : 'Wcięcie liniowe'),
  },
  lineInt: {
    label: 'Przecięcie prostych',
    short: 'Przecięcie',
    icon: 'lineInt',
    hint: 'Punkt przecięcia dwóch prostych zadanych parami punktów.',
    init: (): LineIntData => ({ a: '', b: '', c: '', d: '', id: '' }),
    title: (d: LineIntData) =>
      filled(d.a, d.b, d.c, d.d)
        ? `Przecięcie ${d.a.trim()}–${d.b.trim()} × ${d.c.trim()}–${d.d.trim()}`
        : 'Przecięcie prostych',
  },
  angleInt: {
    label: 'Wcięcie kątowe w przód',
    short: 'Kątowe',
    icon: 'angleInt',
    hint: 'Punkt z kątów pomierzonych na dwóch punktach znanych (kąty wewnętrzne trójkąta).',
    init: (): AngleIntData => ({ a: '', b: '', alpha: '', beta: '', side: 'left', id: '' }),
    title: (d: AngleIntData) =>
      filled(d.a, d.b) ? `Wcięcie kątowe ${d.a.trim()}, ${d.b.trim()}` : 'Wcięcie kątowe w przód',
  },
  resection: {
    label: 'Wcięcie wstecz',
    short: 'Wstecz',
    icon: 'resection',
    hint: 'Stanowisko z kierunków na co najmniej trzy punkty znane; przy większej liczbie z wyrównaniem.',
    init: (): ResectionData => ({ id: '', rows: [{ id: '', hz: '' }] }),
    title: (d: ResectionData) => {
      const ids = d.rows.map((r) => r.id.trim()).filter(Boolean)
      return ids.length >= 3 ? `Wcięcie wstecz na ${ids.slice(0, 4).join(', ')}` : 'Wcięcie wstecz'
    },
  },
  stakeout: {
    label: 'Tyczenie',
    short: 'Tyczenie',
    icon: 'stakeout',
    hint: 'Elementy tyczenia biegunowego: odczyt koła, kąt od nawiązania i długość do odmierzenia.',
    init: (): StakeoutData => ({ station: '', ref: '', refReading: '', reduce: 'no', rows: [{ id: '' }] }),
    title: (d: StakeoutData) => (d.station.trim() ? `Tyczenie ze st. ${d.station.trim()}` : 'Tyczenie'),
  },
  tachy: {
    label: 'Tachimetria',
    short: 'Tachimetria',
    icon: 'tachy',
    hint: 'Orientacja stanowiska na nawiązania i pikiety z kierunku, kąta zenitalnego i odległości skośnej.',
    init: (): TachyData => ({
      station: '',
      stationH: '',
      instrument: '',
      curvature: 'yes',
      reduce: 'no',
      refs: [{ id: '', hz: '' }],
      rows: [{ id: '', code: '', hz: '', v: '', s: '', target: '' }],
    }),
    title: (d: TachyData) => (d.station.trim() ? `Tachimetria ze st. ${d.station.trim()}` : 'Tachimetria'),
  },
  traverse: {
    label: 'Ciąg poligonowy',
    short: 'Ciąg',
    icon: 'traverse',
    hint: 'Ciąg dwustronnie nawiązany, zamknięty lub wiszący z wyrównaniem przybliżonym.',
    init: (): TraverseData => ({
      kind: 'connected',
      angles: 'left',
      back: '',
      forward: '',
      reduce: 'no',
      mAngle: '',
      relLimit: '',
      rows: [{ id: '', angle: '', dist: '' }],
    }),
    title: (d: TraverseData) => {
      const ids = d.rows.map((r) => r.id.trim()).filter(Boolean)
      return ids.length >= 2 ? `Ciąg ${ids[0]}–${ids.at(-1)}` : 'Ciąg poligonowy'
    },
  },
  area: {
    label: 'Pole powierzchni',
    short: 'Pole',
    icon: 'area',
    hint: 'Pole i obwód wieloboku ze współrzędnych wierzchołków.',
    init: (): AreaData => ({ rows: [{ id: '' }] }),
    title: (d: AreaData) => {
      const ids = d.rows.map((r) => r.id.trim()).filter(Boolean)
      if (ids.length < 3) return 'Pole powierzchni'
      return `Pole ${ids.slice(0, 4).join('-')}${ids.length > 4 ? '…' : ''}`
    },
  },
}

export const CALC_GROUPS: { label: string; types: CalcType[] }[] = [
  { label: 'Podstawowe', types: ['azimuth', 'polar'] },
  { label: 'Osnowa', types: ['traverse'] },
  { label: 'Pomiary', types: ['tachy', 'stakeout'] },
  { label: 'Domiary', types: ['offset', 'fromOffset'] },
  { label: 'Wcięcia', types: ['distInt', 'angleInt', 'lineInt', 'resection'] },
  { label: 'Pola', types: ['area'] },
]

export const calcTitle = (s: CalcSession) => s.name ?? CALCS[s.type].title(s.data)
