// Wątek roboczy z rdzeniem WASM: wczytywanie i przetwarzanie GML bez blokowania interfejsu.
import init, { GmlDoc } from './wasm/pikieta.js'
import type { GmlSource } from './gml-engine'

const ready = init()
const docs = new Map<string, GmlDoc>()
let next = 0

/** Pliki z programów geodezyjnych bywają w Windows-1250, nie w UTF-8. */
async function textOf(src: GmlSource): Promise<string> {
  if ('text' in src) return src.text
  if (!('file' in src)) throw new Error('w przeglądarce plik trzeba wskazać w oknie wyboru')
  const bytes = await src.file.arrayBuffer()
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    return new TextDecoder('windows-1250').decode(bytes)
  }
}

function doc(id: string): GmlDoc {
  const d = docs.get(id)
  if (!d) throw new Error('dokument nie jest wczytany')
  return d
}

const ops: Record<string, (...args: never[]) => unknown> = {
  async open(src: GmlSource) {
    const d = new GmlDoc(await textOf(src))
    const id = `w${next++}`
    docs.set(id, d)
    return { id, index: JSON.parse(d.index_json()), issues: JSON.parse(d.issues_json()) }
  },
  render: (id: string, epsg: number) => doc(id).render_buffer(epsg).buffer,
  texts: (id: string, epsg: number) => JSON.parse(doc(id).texts_json(epsg)),
  classTable: (id: string, cls: string) => JSON.parse(doc(id).class_table_json(cls)),
  properties: (id: string, feature: number) => JSON.parse(doc(id).properties_json(feature)),
  setProperty: (id: string, feature: number, key: string, value: string) => doc(id).set_property(feature, key, value),
  xml: (id: string) => doc(id).to_xml(),
  close(id: string) {
    docs.get(id)?.free()
    docs.delete(id)
  },
}

self.onmessage = async (e: MessageEvent<{ seq: number; op: string; args: never[] }>) => {
  const { seq, op, args } = e.data
  try {
    await ready
    const result = await ops[op](...args)
    const transfer = result instanceof ArrayBuffer ? [result] : []
    self.postMessage({ seq, ok: true, result }, { transfer })
  } catch (err) {
    self.postMessage({ seq, ok: false, error: err instanceof Error ? err.message : String(err) })
  }
}
