// Silnik GML: dokument żyje poza wątkiem interfejsu.
// W przeglądarce to Web Worker z rdzeniem WASM, w aplikacji desktopowej natywny Rust (komendy Tauri).
import { invoke, isTauri } from '@tauri-apps/api/core'

export interface GmlIndex {
  classes: string[]
  /** Indeks klasy dla każdego obiektu. */
  cls: number[]
  label: string[]
  id: string[]
  /** 0 brak, 1 punkt, 2 linia, 3 wielobok, 4 wieloczęściowa. */
  geom: number[]
  /** Wariant stylu (bity `VARIANT_*`), np. punkt graniczny dokładny lub stabilizowany. */
  variant: number[]
  /** Kody znaków umownych mapy zasadniczej (słownik) i numer kodu dla obiektu. */
  codes: string[]
  code: number[]
  epsg: number | null
  /** Błędy geometrii: numer obiektu → opis. */
  errors: Record<string, string>
}

export interface GmlIssue {
  severity: 'error' | 'warning' | 'info'
  feature: number | null
  message: string
}

/** Tekst kartograficzny (PrezentacjaGraficzna) w układzie mapy. */
export interface CartoText {
  /** Obiekt prezentacji. */
  f: number
  /** Obiekt opisywany albo null. */
  t: number | null
  /** Wschód, północ. */
  x: number
  y: number
  s: string
  /** Justyfikacja 1–9 jak na klawiaturze numerycznej; 0 brak. */
  j: number
  /** Obrót [rad], przeciwnie do ruchu wskazówek zegara od osi wschodniej. */
  a: number
}

export interface ClassTable {
  keys: string[]
  editable: boolean[]
  /** Numer obiektu i wartości w kolejności `keys`. */
  rows: [number, string[]][]
}

export type GmlSource = { file: Blob } | { path: string } | { text: string }

export interface Opened {
  id: string
  index: GmlIndex
  issues: GmlIssue[]
}

export interface GmlEngine {
  /** Natywny Rust (Tauri): czyta i zapisuje pliki po ścieżkach. */
  native: boolean
  open(src: GmlSource): Promise<Opened>
  render(id: string, epsg: number): Promise<ArrayBuffer>
  texts(id: string, epsg: number): Promise<CartoText[]>
  classTable(id: string, cls: string): Promise<ClassTable>
  properties(id: string, feature: number): Promise<[string, string, boolean][]>
  /** Zwraca nową etykietę obiektu. */
  setProperty(id: string, feature: number, key: string, value: string): Promise<string>
  xml(id: string): Promise<string>
  /** Tylko natywnie: zapis do pliku. */
  save(id: string, path: string): Promise<void>
  close(id: string): void
}

function workerEngine(): GmlEngine {
  const worker = new Worker(new URL('./gml.worker.ts', import.meta.url), { type: 'module' })
  let seq = 0
  const pending = new Map<number, { resolve: (v: unknown) => void; reject: (e: Error) => void }>()
  worker.onmessage = (e: MessageEvent<{ seq: number; ok: boolean; result?: unknown; error?: string }>) => {
    const p = pending.get(e.data.seq)
    if (!p) return
    pending.delete(e.data.seq)
    if (e.data.ok) p.resolve(e.data.result)
    else p.reject(new Error(e.data.error))
  }
  const call = <T>(op: string, ...args: unknown[]) =>
    new Promise<T>((resolve, reject) => {
      const n = ++seq
      pending.set(n, { resolve: resolve as (v: unknown) => void, reject })
      worker.postMessage({ seq: n, op, args })
    })
  return {
    native: false,
    open: (src) => call('open', src),
    render: (id, epsg) => call('render', id, epsg),
    texts: (id, epsg) => call('texts', id, epsg),
    classTable: (id, cls) => call('classTable', id, cls),
    properties: (id, f) => call('properties', id, f),
    setProperty: (id, f, k, v) => call('setProperty', id, f, k, v),
    xml: (id) => call('xml', id),
    save: () => Promise.reject(new Error('zapis do ścieżki działa tylko w aplikacji desktopowej')),
    close: (id) => void call('close', id),
  }
}

function tauriEngine(): GmlEngine {
  const json = async <T>(p: Promise<string>) => JSON.parse(await p) as T
  return {
    native: true,
    async open(src) {
      if (!('path' in src)) throw new Error('w aplikacji desktopowej plik GML otwiera się po ścieżce')
      const r = await invoke<{ id: string; index: string; issues: string }>('gml_open', { path: src.path })
      return { id: r.id, index: JSON.parse(r.index), issues: JSON.parse(r.issues) }
    },
    render: (id, epsg) => invoke<ArrayBuffer>('gml_render', { id, epsg }),
    texts: (id, epsg) => json(invoke<string>('gml_texts', { id, epsg })),
    classTable: (id, cls) => json(invoke<string>('gml_class_table', { id, className: cls })),
    properties: (id, feature) => json(invoke<string>('gml_properties', { id, feature })),
    setProperty: (id, feature, key, value) => invoke<string>('gml_set', { id, feature, key, value }),
    xml: (id) => invoke<string>('gml_xml', { id }),
    save: (id, path) => invoke<void>('gml_save', { id, path }),
    close: (id) => void invoke('gml_close', { id }),
  }
}

/**
 * Natywny silnik na desktopie. Na Androidzie okno wyboru zwraca adresy `content://`,
 * których natywny odczyt pliku nie obsłuży, więc tam działa wątek roboczy z WASM.
 */
const android = /Android/i.test(navigator.userAgent)
export const engine: GmlEngine = isTauri() && !android ? tauriEngine() : workerEngine()
