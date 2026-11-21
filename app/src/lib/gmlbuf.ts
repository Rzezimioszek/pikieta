// Dekoder bufora geometrii z rdzenia (`GmlDocument::render_buffer`).

export interface Decoded {
  count: number
  /** Pary wschód, północ w układzie mapy. */
  coords: Float64Array
  ops: Uint32Array
  /** Początek opisu każdego obiektu w `ops`. */
  start: Uint32Array
}

export function decode(buf: ArrayBuffer): Decoded {
  const head = new Uint32Array(buf, 0, 4)
  const [count, nOps, nCoords] = [head[0], head[1], head[2]]
  const coords = new Float64Array(buf, 16, nCoords)
  const ops = new Uint32Array(buf, 16 + nCoords * 8, nOps)
  const start = new Uint32Array(count)
  let at = 0
  for (let i = 0; i < count; i++) {
    start[i] = at
    const parts = ops[at++]
    for (let p = 0; p < parts; p++) {
      const kind = ops[at]
      at += kind === 1 ? 2 : kind === 2 ? 3 : 2 + 2 * ops[at + 1]
    }
  }
  return { count, coords, ops, start }
}

export type Part =
  | { t: 'point'; c: number }
  | { t: 'line'; c: number; n: number }
  | { t: 'polygon'; rings: [number, number][] }

/** Części geometrii obiektu (numery par współrzędnych). */
export function parts(d: Decoded, i: number): Part[] {
  const out: Part[] = []
  let at = d.start[i]
  const n = d.ops[at++]
  for (let p = 0; p < n; p++) {
    const kind = d.ops[at]
    if (kind === 1) {
      out.push({ t: 'point', c: d.ops[at + 1] })
      at += 2
    } else if (kind === 2) {
      out.push({ t: 'line', c: d.ops[at + 1], n: d.ops[at + 2] })
      at += 3
    } else {
      const r = d.ops[at + 1]
      const rings: [number, number][] = []
      for (let k = 0; k < r; k++) rings.push([d.ops[at + 2 + 2 * k], d.ops[at + 3 + 2 * k]])
      out.push({ t: 'polygon', rings })
      at += 2 + 2 * r
    }
  }
  return out
}

/** Wierzchołki obiektu w konwencji geodezyjnej (x północ, y wschód). */
export function vertices(d: Decoded, i: number): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = []
  const add = (c: number, n: number) => {
    for (let k = c; k < c + n; k++) out.push({ x: d.coords[2 * k + 1], y: d.coords[2 * k] })
  }
  for (const p of parts(d, i)) {
    if (p.t === 'point') add(p.c, 1)
    else if (p.t === 'line') add(p.c, p.n)
    else for (const [c, n] of p.rings) add(c, n)
  }
  return out
}
