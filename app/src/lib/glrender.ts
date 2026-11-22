// Renderer WebGL2 dla warstw GML: geometria trafia na kartę graficzną raz, przy zmianie widoku
// zmienia się tylko macierz przekształcenia. Linie (grube, przerywane, z obwódką) i symbole punktów
// są rysowane instancjami; wypełnienia jako trójkąty z triangulacji earcut.
import earcut from 'earcut'
import type { FrameState } from 'ol/Map'
import { parts, type Decoded } from './gmlbuf'
import type { ClassStyle, Rgba } from './gmlstyle'

const LINE_VS = `#version 300 es
in vec2 a_p0;
in vec2 a_p1;
in float a_dist;
uniform mat3 u_m;
uniform vec2 u_size;
uniform float u_width;
uniform float u_ppm;
out float v_dist;
out float v_across;
void main() {
  vec2 p0 = (u_m * vec3(a_p0, 1.0)).xy;
  vec2 p1 = (u_m * vec3(a_p1, 1.0)).xy;
  vec2 d = p1 - p0;
  float len = length(d);
  vec2 dir = len > 1e-6 ? d / len : vec2(1.0, 0.0);
  vec2 nrm = vec2(-dir.y, dir.x);
  float t = float(gl_VertexID & 1);
  float side = gl_VertexID < 2 ? -1.0 : 1.0;
  float hw = u_width * 0.5 + 1.0;
  // Przedłużenie o pół grubości daje kwadratowe zakończenia i domknięte załamania.
  float ext = min(hw, len * 0.5 + hw);
  vec2 pos = mix(p0, p1, t) + dir * (t * 2.0 - 1.0) * ext + nrm * side * hw;
  v_dist = a_dist * u_ppm + t * len + (t * 2.0 - 1.0) * ext;
  v_across = side * hw;
  gl_Position = vec4(pos.x / u_size.x * 2.0 - 1.0, 1.0 - pos.y / u_size.y * 2.0, 0.0, 1.0);
}`

const LINE_FS = `#version 300 es
precision highp float;
uniform vec4 u_color;
uniform vec4 u_dash;
uniform float u_width;
in float v_dist;
in float v_across;
out vec4 o;
void main() {
  if (u_dash.x > 0.0) {
    float t = mod(v_dist, u_dash.x + u_dash.y + u_dash.z + u_dash.w);
    bool on = t < u_dash.x || (t >= u_dash.x + u_dash.y && t < u_dash.x + u_dash.y + u_dash.z);
    if (!on) discard;
  }
  float a = clamp(u_width * 0.5 + 0.5 - abs(v_across), 0.0, 1.0) * u_color.a;
  if (a <= 0.0) discard;
  o = vec4(u_color.rgb * a, a);
}`

const POINT_VS = `#version 300 es
in vec2 a_c;
uniform mat3 u_m;
uniform vec2 u_size;
uniform float u_extent;
out vec2 v_off;
void main() {
  vec2 c = (u_m * vec3(a_c, 1.0)).xy;
  vec2 corner = vec2(float(gl_VertexID & 1), float(gl_VertexID >> 1)) * 2.0 - 1.0;
  v_off = corner * u_extent;
  vec2 pos = c + v_off;
  gl_Position = vec4(pos.x / u_size.x * 2.0 - 1.0, 1.0 - pos.y / u_size.y * 2.0, 0.0, 1.0);
}`

const POINT_FS = `#version 300 es
precision highp float;
uniform float u_radius;
uniform float u_stroke;
uniform float u_dot;
uniform vec4 u_strokeColor;
uniform vec4 u_fill;
uniform vec4 u_dotColor;
in vec2 v_off;
out vec4 o;
vec4 over(vec4 top, vec4 base) { return top + base * (1.0 - top.a); }
void main() {
  float d = length(v_off);
  vec4 c = vec4(0.0);
  float inside = clamp(u_radius - u_stroke * 0.5 - d + 0.5, 0.0, 1.0);
  c = over(vec4(u_fill.rgb * u_fill.a, u_fill.a) * inside, c);
  float ring = clamp(u_stroke * 0.5 + 0.5 - abs(d - u_radius), 0.0, 1.0);
  c = over(vec4(u_strokeColor.rgb * u_strokeColor.a, u_strokeColor.a) * ring, c);
  if (u_dot > 0.0) {
    float dot_ = clamp(u_dot - d + 0.5, 0.0, 1.0);
    c = over(vec4(u_dotColor.rgb * u_dotColor.a, u_dotColor.a) * dot_, c);
  }
  if (c.a <= 0.0) discard;
  o = c;
}`

const FILL_VS = `#version 300 es
in vec2 a_p;
uniform mat3 u_m;
uniform vec2 u_size;
void main() {
  vec2 pos = (u_m * vec3(a_p, 1.0)).xy;
  gl_Position = vec4(pos.x / u_size.x * 2.0 - 1.0, 1.0 - pos.y / u_size.y * 2.0, 0.0, 1.0);
}`

const FILL_FS = `#version 300 es
precision highp float;
uniform vec4 u_color;
out vec4 o;
void main() { o = vec4(u_color.rgb * u_color.a, u_color.a); }`

/** Obiekty jednej klasy i wariantu: zakresy w buforach i styl. */
interface Group {
  cls: string
  style: ClassStyle
  line: [number, number]
  fill: [number, number]
  point: [number, number]
}

interface GpuLayer {
  origin: [number, number]
  groups: Group[]
  lineBuf: WebGLBuffer
  fillBuf: WebGLBuffer
  fillIdx: WebGLBuffer
  pointBuf: WebGLBuffer
  lineVao: WebGLVertexArrayObject
  fillVao: WebGLVertexArrayObject
  pointVao: WebGLVertexArrayObject
  visible: boolean
  hidden: Set<string>
}

/** Obiekty jednej klasy o wspólnym stylu. */
export interface GpuGroup {
  cls: string
  style: ClassStyle
  list: number[]
}

const WHITE: Rgba = [1, 1, 1, 1]
const LINE_STRIDE = 5

function compile(gl: WebGL2RenderingContext, vs: string, fs: string) {
  const prog = gl.createProgram()!
  for (const [type, src] of [
    [gl.VERTEX_SHADER, vs],
    [gl.FRAGMENT_SHADER, fs],
  ] as const) {
    const sh = gl.createShader(type)!
    gl.shaderSource(sh, src)
    gl.compileShader(sh)
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh) ?? 'shader')
    gl.attachShader(prog, sh)
  }
  gl.linkProgram(prog)
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog) ?? 'program')
  const uniforms = new Map<string, WebGLUniformLocation | null>()
  return {
    prog,
    attr: (n: string) => gl.getAttribLocation(prog, n),
    u: (n: string) => {
      if (!uniforms.has(n)) uniforms.set(n, gl.getUniformLocation(prog, n))
      return uniforms.get(n)!
    },
  }
}

export class GmlGpu {
  readonly canvas = document.createElement('canvas')
  private gl: WebGL2RenderingContext
  private line: ReturnType<typeof compile>
  private point: ReturnType<typeof compile>
  private fill: ReturnType<typeof compile>
  private layers = new Map<string, GpuLayer>()
  private order: string[] = []
  /** Informacja o karcie graficznej (do diagnostyki). */
  readonly renderer: string

  constructor() {
    this.canvas.style.cssText = 'position:absolute;left:0;top:0;width:100%;height:100%;pointer-events:none'
    const gl = this.canvas.getContext('webgl2', { premultipliedAlpha: true, antialias: false, alpha: true })
    if (!gl) throw new Error('Brak WebGL2: karta graficzna lub przeglądarka nie obsługuje przyspieszenia.')
    this.gl = gl
    this.line = compile(gl, LINE_VS, LINE_FS)
    this.point = compile(gl, POINT_VS, POINT_FS)
    this.fill = compile(gl, FILL_VS, FILL_FS)
    const dbg = gl.getExtension('WEBGL_debug_renderer_info')
    this.renderer = dbg ? String(gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL)) : 'WebGL2'
  }

  /** Buduje bufory warstwy: grupy (klasa, wariant) w kolejności rysowania. */
  set(id: string, d: Decoded, defs: GpuGroup[], visible: boolean, hidden: string[]) {
    this.remove(id)
    const gl = this.gl
    // Środek zasięgu jako początek układu: współrzędne względne mieszczą się w float32 z milimetrami.
    let [minX, minY, maxX, maxY] = [Infinity, Infinity, -Infinity, -Infinity]
    for (let k = 0; k < d.coords.length; k += 2) {
      const [x, y] = [d.coords[k], d.coords[k + 1]]
      if (x < minX) minX = x
      if (x > maxX) maxX = x
      if (y < minY) minY = y
      if (y > maxY) maxY = y
    }
    const origin: [number, number] = d.coords.length ? [(minX + maxX) / 2, (minY + maxY) / 2] : [0, 0]

    const groupDefs = [...defs].sort((a, b) => a.style.order - b.style.order)

    // Wstępne policzenie rozmiarów buforów.
    let nSeg = 0
    let nPts = 0
    let nFillV = 0
    for (const g of groupDefs) {
      for (const i of g.list) {
        for (const p of parts(d, i)) {
          if (p.t === 'point') nPts++
          else if (p.t === 'line') nSeg += Math.max(0, p.n - 1)
          else
            for (const [, n] of p.rings) {
              nSeg += Math.max(0, n - 1)
              if (g.style.fill) nFillV += n
            }
        }
      }
    }
    const lines = new Float32Array(nSeg * LINE_STRIDE)
    const points = new Float32Array(nPts * 2)
    const fillV = new Float32Array(nFillV * 2)
    const fillI: number[] = []
    const [ox, oy] = origin
    const c = d.coords
    let [li, pi, fv] = [0, 0, 0]

    const groups: Group[] = []
    for (const g of groupDefs) {
      const start = { line: li / LINE_STRIDE, point: pi / 2, fill: fillI.length }
      const pushLine = (s: number, n: number) => {
        let dist = 0
        for (let k = s; k < s + n - 1; k++) {
          const [x0, y0, x1, y1] = [c[2 * k] - ox, c[2 * k + 1] - oy, c[2 * k + 2] - ox, c[2 * k + 3] - oy]
          lines.set([x0, y0, x1, y1, dist], li)
          li += LINE_STRIDE
          dist += Math.hypot(x1 - x0, y1 - y0)
        }
      }
      for (const i of g.list) {
        for (const p of parts(d, i)) {
          if (p.t === 'point') {
            points[pi++] = c[2 * p.c] - ox
            points[pi++] = c[2 * p.c + 1] - oy
          } else if (p.t === 'line') {
            pushLine(p.c, p.n)
          } else {
            for (const [s, n] of p.rings) pushLine(s, n)
            if (g.style.fill) {
              // Triangulacja wieloboku z otworami; ostatni wierzchołek pierścienia powtarza pierwszy.
              const flat: number[] = []
              const holes: number[] = []
              for (const [r, [s, n]] of p.rings.entries()) {
                if (r > 0) holes.push(flat.length / 2)
                for (let k = s; k < s + n - 1; k++) flat.push(c[2 * k] - ox, c[2 * k + 1] - oy)
              }
              const base = fv / 2
              fillV.set(flat, fv)
              fv += flat.length
              for (const t of earcut(flat, holes)) fillI.push(base + t)
            }
          }
        }
      }
      groups.push({
        cls: g.cls,
        style: g.style,
        line: [start.line, li / LINE_STRIDE - start.line],
        point: [start.point, pi / 2 - start.point],
        fill: [start.fill, fillI.length - start.fill],
      })
    }

    const buf = (target: number, dataArr: ArrayBufferView) => {
      const b = gl.createBuffer()!
      gl.bindBuffer(target, b)
      gl.bufferData(target, dataArr, gl.STATIC_DRAW)
      return b
    }
    const lineBuf = buf(gl.ARRAY_BUFFER, lines)
    const pointBuf = buf(gl.ARRAY_BUFFER, points)
    const fillBuf = buf(gl.ARRAY_BUFFER, fillV.subarray(0, fv))
    const fillIdx = buf(gl.ELEMENT_ARRAY_BUFFER, new Uint32Array(fillI))

    const lineVao = gl.createVertexArray()!
    const pointVao = gl.createVertexArray()!
    const fillVao = gl.createVertexArray()!
    gl.bindVertexArray(fillVao)
    gl.bindBuffer(gl.ARRAY_BUFFER, fillBuf)
    gl.enableVertexAttribArray(this.fill.attr('a_p'))
    gl.vertexAttribPointer(this.fill.attr('a_p'), 2, gl.FLOAT, false, 8, 0)
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, fillIdx)
    gl.bindVertexArray(null)

    this.layers.set(id, {
      origin,
      groups,
      lineBuf,
      fillBuf,
      fillIdx,
      pointBuf,
      lineVao,
      pointVao,
      fillVao,
      visible,
      hidden: new Set(hidden),
    })
    this.order.push(id)
  }

  setVisibility(id: string, visible: boolean, hidden: string[]) {
    const l = this.layers.get(id)
    if (!l) return
    l.visible = visible
    l.hidden = new Set(hidden)
  }

  remove(id: string) {
    const l = this.layers.get(id)
    if (!l) return
    const gl = this.gl
    for (const b of [l.lineBuf, l.fillBuf, l.fillIdx, l.pointBuf]) gl.deleteBuffer(b)
    for (const v of [l.lineVao, l.fillVao, l.pointVao]) gl.deleteVertexArray(v)
    this.layers.delete(id)
    this.order = this.order.filter((o) => o !== id)
  }

  ids() {
    return [...this.order]
  }

  /** Rysuje klatkę; wywoływane przez warstwę OpenLayers przy każdej zmianie widoku. */
  render(frame: FrameState): HTMLCanvasElement {
    const gl = this.gl
    const pr = frame.pixelRatio
    const [w, h] = [Math.round(frame.size[0] * pr), Math.round(frame.size[1] * pr)]
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w
      this.canvas.height = h
    }
    gl.viewport(0, 0, w, h)
    gl.clearColor(0, 0, 0, 0)
    gl.clear(gl.COLOR_BUFFER_BIT)
    gl.enable(gl.BLEND)
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA)

    const res = frame.viewState.resolution
    // [a, b, c, d, e, f]: x' = a·x + c·y + e, y' = b·x + d·y + f (piksele CSS).
    const t = frame.coordinateToPixelTransform
    for (const id of this.order) {
      const l = this.layers.get(id)!
      if (!l.visible) continue
      const [ox, oy] = l.origin
      const m = new Float32Array([
        t[0] * pr,
        t[1] * pr,
        0,
        t[2] * pr,
        t[3] * pr,
        0,
        (t[0] * ox + t[2] * oy + t[4]) * pr,
        (t[1] * ox + t[3] * oy + t[5]) * pr,
        1,
      ])
      const shown = l.groups.filter((g) => !l.hidden.has(g.cls) && res <= g.style.maxRes)
      this.drawFills(l, shown, m, w, h)
      this.drawLines(l, shown, m, w, h, pr, res, true)
      this.drawLines(l, shown, m, w, h, pr, res, false)
      this.drawPoints(l, shown, m, w, h, pr)
    }
    gl.bindVertexArray(null)
    return this.canvas
  }

  private drawFills(l: GpuLayer, groups: Group[], m: Float32Array, w: number, h: number) {
    const gl = this.gl
    const p = this.fill
    gl.useProgram(p.prog)
    gl.uniformMatrix3fv(p.u('u_m'), false, m)
    gl.uniform2f(p.u('u_size'), w, h)
    gl.bindVertexArray(l.fillVao)
    for (const g of groups) {
      if (!g.style.fill || g.fill[1] === 0) continue
      gl.uniform4fv(p.u('u_color'), g.style.fill)
      gl.drawElements(gl.TRIANGLES, g.fill[1], gl.UNSIGNED_INT, g.fill[0] * 4)
    }
  }

  private drawLines(
    l: GpuLayer,
    groups: Group[],
    m: Float32Array,
    w: number,
    h: number,
    pr: number,
    res: number,
    halo: boolean,
  ) {
    const gl = this.gl
    const p = this.line
    gl.useProgram(p.prog)
    gl.uniformMatrix3fv(p.u('u_m'), false, m)
    gl.uniform2f(p.u('u_size'), w, h)
    gl.uniform1f(p.u('u_ppm'), pr / res)
    gl.bindVertexArray(l.lineVao)
    gl.bindBuffer(gl.ARRAY_BUFFER, l.lineBuf)
    const [a0, a1, ad] = [p.attr('a_p0'), p.attr('a_p1'), p.attr('a_dist')]
    for (const a of [a0, a1, ad]) {
      gl.enableVertexAttribArray(a)
      gl.vertexAttribDivisor(a, 1)
    }
    for (const g of groups) {
      const s = g.style.line
      if (!s || g.line[1] === 0) continue
      const far = s.far && res > s.far.res ? s.far : undefined
      const width = far?.width ?? s.width
      const haloW = far?.halo ?? s.halo ?? 0
      if (halo && haloW <= 0) continue
      const bytes = g.line[0] * LINE_STRIDE * 4
      gl.vertexAttribPointer(a0, 2, gl.FLOAT, false, LINE_STRIDE * 4, bytes)
      gl.vertexAttribPointer(a1, 2, gl.FLOAT, false, LINE_STRIDE * 4, bytes + 8)
      gl.vertexAttribPointer(ad, 1, gl.FLOAT, false, LINE_STRIDE * 4, bytes + 16)
      gl.uniform1f(p.u('u_width'), (halo ? width + 2 * haloW : width) * pr)
      gl.uniform4fv(p.u('u_color'), halo ? WHITE : s.color)
      const dash = !halo && s.dash ? s.dash : [0, 0]
      gl.uniform4f(p.u('u_dash'), dash[0] * pr, dash[1] * pr, (dash[2] ?? 0) * pr, (dash[3] ?? 0) * pr)
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, g.line[1])
    }
  }

  private drawPoints(l: GpuLayer, groups: Group[], m: Float32Array, w: number, h: number, pr: number) {
    const gl = this.gl
    const p = this.point
    gl.useProgram(p.prog)
    gl.uniformMatrix3fv(p.u('u_m'), false, m)
    gl.uniform2f(p.u('u_size'), w, h)
    gl.bindVertexArray(l.pointVao)
    gl.bindBuffer(gl.ARRAY_BUFFER, l.pointBuf)
    const a = p.attr('a_c')
    gl.enableVertexAttribArray(a)
    gl.vertexAttribDivisor(a, 1)
    for (const g of groups) {
      const s = g.style.point
      if (!s || g.point[1] === 0) continue
      gl.vertexAttribPointer(a, 2, gl.FLOAT, false, 8, g.point[0] * 8)
      gl.uniform1f(p.u('u_radius'), s.radius * pr)
      gl.uniform1f(p.u('u_stroke'), s.strokeWidth * pr)
      gl.uniform1f(p.u('u_dot'), (s.dot ?? 0) * pr)
      gl.uniform1f(p.u('u_extent'), (s.radius + s.strokeWidth + 1.5) * pr)
      gl.uniform4fv(p.u('u_strokeColor'), s.stroke)
      gl.uniform4fv(p.u('u_fill'), s.fill ?? [0, 0, 0, 0])
      gl.uniform4fv(p.u('u_dotColor'), s.dotColor ?? [0, 0, 0, 1])
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, g.point[1])
    }
  }
}
