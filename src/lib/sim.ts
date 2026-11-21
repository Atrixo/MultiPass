export const STRIDE = 5;

export interface ToolShape {
  diameter: number;
  tip: number;
  corner: number;
  taperDeg: number;
}

export interface SimTool {
  part: number;
  tool: number | null;
  shape: ToolShape;
  known: boolean;
}

export interface SimPart {
  name: string;
  start: number;
  end: number;
}

export interface TimelineMeta {
  parts: SimPart[];
  tools: SimTool[];
  bounds: [number, number, number, number] | null;
  material: boolean;
  stockHeight: number | null;
  zmin: number;
  vertices: number;
}

export interface Timeline {
  meta: TimelineMeta;
  data: Float32Array;
}

export function parseTimeline(buf: ArrayBuffer): Timeline {
  const len = new DataView(buf).getUint32(0, true);
  const meta = JSON.parse(new TextDecoder().decode(new Uint8Array(buf, 4, len))) as TimelineMeta;
  const offset = Math.ceil((4 + len) / 4) * 4;
  return { meta, data: new Float32Array(buf, offset, meta.vertices * STRIDE) };
}

export function isJump(data: Float32Array, i: number): boolean {
  return Number.isNaN(data[i * STRIDE + 3]);
}

export function isRapid(data: Float32Array, i: number): boolean {
  return data[i * STRIDE + 4] % 2 === 1;
}

export function slotOf(data: Float32Array, i: number): number {
  return Math.floor(data[i * STRIDE + 4] / 2);
}

export function cumulativeTimes(data: Float32Array, rapidSpeed: number): Float64Array {
  const n = data.length / STRIDE;
  const out = new Float64Array(n);
  let t = 0;
  for (let i = 0; i < n; i++) {
    const d = data[i * STRIDE + 3];
    if (!Number.isNaN(d)) t += d >= 0 ? d : -d / rapidSpeed;
    out[i] = t;
  }
  return out;
}

export function vertexAt(times: Float64Array, t: number): number {
  let lo = 0;
  let hi = times.length - 1;
  if (hi < 0) return 0;
  if (t >= times[hi]) return hi;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (times[mid] <= t) lo = mid + 1;
    else hi = mid;
  }
  return Math.max(0, lo - 1);
}

export function positionAt(data: Float32Array, times: Float64Array, t: number): [number, number, number, number] {
  const i = vertexAt(times, t);
  const j = i + 1;
  const o = i * STRIDE;
  if (j >= times.length || isJump(data, j) || times[j] <= times[i]) return [data[o], data[o + 1], data[o + 2], i];
  const f = Math.min(1, Math.max(0, (t - times[i]) / (times[j] - times[i])));
  const p = j * STRIDE;
  return [data[o] + (data[p] - data[o]) * f, data[o + 1] + (data[p + 1] - data[o + 1]) * f, data[o + 2] + (data[p + 2] - data[o + 2]) * f, i];
}

const TABLE = 96;

export interface Profile {
  radius: number;
  flat: boolean;
  table: Float32Array;
}

export function heightAt(s: ToolShape, d: number): number {
  const r = s.diameter / 2;
  if (d > r) return Infinity;
  if (s.taperDeg > 0) {
    const tip = Math.min(s.tip / 2, r);
    return d <= tip ? 0 : (d - tip) / Math.tan((s.taperDeg * Math.PI) / 180);
  }
  const c = Math.min(Math.max(s.corner, 0), r);
  const flat = r - c;
  if (d <= flat || c <= 0) return 0;
  const e = d - flat;
  return c - Math.sqrt(Math.max(0, c * c - e * e));
}

export function profile(s: ToolShape): Profile {
  const radius = Math.max(s.diameter / 2, 1e-4);
  const table = new Float32Array(TABLE + 1);
  for (let k = 0; k <= TABLE; k++) table[k] = heightAt(s, (k / TABLE) * radius);
  const flat = table.every((v) => v === 0);
  return { radius, flat, table };
}

function reach(p: Profile, depth: number): number {
  if (p.flat) return p.radius;
  let k = 0;
  while (k < TABLE && p.table[k + 1] < depth) k++;
  return Math.min(p.radius, ((k + 1) / TABLE) * p.radius);
}

export interface Grid {
  x0: number;
  y0: number;
  cell: number;
  w: number;
  h: number;
  z: Float32Array;
}

export const MARGIN_MM = 2;

export function makeGrid(bounds: [number, number, number, number], maxCells = 600_000): Grid {
  const x0 = bounds[0] - MARGIN_MM;
  const y0 = bounds[1] - MARGIN_MM;
  const sx = bounds[2] - bounds[0] + 2 * MARGIN_MM;
  const sy = bounds[3] - bounds[1] + 2 * MARGIN_MM;
  const cell = Math.max(0.01, Math.sqrt((sx * sy) / maxCells));
  const w = Math.max(1, Math.ceil(sx / cell));
  const h = Math.max(1, Math.ceil(sy / cell));
  return { x0, y0, cell, w, h, z: new Float32Array(w * h) };
}

export function stampSegment(g: Grid, p: Profile, ax: number, ay: number, az: number, bx: number, by: number, bz: number): void {
  const low = Math.min(az, bz);
  if (low >= 0) return;
  const r = reach(p, -low);
  const r2 = r * r;
  const { x0, y0, cell, w, h, z } = g;
  const ix0 = Math.max(0, Math.floor((Math.min(ax, bx) - r - x0) / cell));
  const ix1 = Math.min(w - 1, Math.floor((Math.max(ax, bx) + r - x0) / cell));
  const iy0 = Math.max(0, Math.floor((Math.min(ay, by) - r - y0) / cell));
  const iy1 = Math.min(h - 1, Math.floor((Math.max(ay, by) + r - y0) / cell));
  if (ix0 > ix1 || iy0 > iy1) return;
  const dx = bx - ax;
  const dy = by - ay;
  const dz = bz - az;
  const l2 = dx * dx + dy * dy;
  const inv = l2 > 1e-12 ? 1 / l2 : 0;
  const scale = TABLE / p.radius;
  const table = p.table;
  for (let iy = iy0; iy <= iy1; iy++) {
    const cy = y0 + (iy + 0.5) * cell - ay;
    const row = iy * w;
    for (let ix = ix0; ix <= ix1; ix++) {
      const cx = x0 + (ix + 0.5) * cell - ax;
      let t = (cx * dx + cy * dy) * inv;
      t = t < 0 ? 0 : t > 1 ? 1 : t;
      const ex = cx - t * dx;
      const ey = cy - t * dy;
      const d2 = ex * ex + ey * ey;
      if (d2 > r2) continue;
      let v = az + t * dz;
      if (!p.flat) {
        const k = Math.sqrt(d2) * scale;
        const k0 = k | 0;
        v += k0 >= TABLE ? table[TABLE] : table[k0] + (table[k0 + 1] - table[k0]) * (k - k0);
      }
      const i = row + ix;
      if (v < z[i]) z[i] = v;
    }
  }
}

function rowSpan(yc: number, r: number, ax: number, ay: number, bx: number, by: number, nx: number, ny: number, out: Float64Array): boolean {
  let lo = Infinity;
  let hi = -Infinity;
  const da = yc - ay;
  if (da * da <= r * r) {
    const q = Math.sqrt(r * r - da * da);
    lo = ax - q;
    hi = ax + q;
  }
  const db = yc - by;
  if (db * db <= r * r) {
    const q = Math.sqrt(r * r - db * db);
    if (bx - q < lo) lo = bx - q;
    if (bx + q > hi) hi = bx + q;
  }
  if (nx !== 0 || ny !== 0) {
    const px = [ax + nx, bx + nx, bx - nx, ax - nx];
    const py = [ay + ny, by + ny, by - ny, ay - ny];
    for (let e = 0; e < 4; e++) {
      const x1 = px[e];
      const y1 = py[e];
      const x2 = px[(e + 1) & 3];
      const y2 = py[(e + 1) & 3];
      if ((yc < y1 && yc < y2) || (yc > y1 && yc > y2) || y1 === y2) continue;
      const x = x1 + ((yc - y1) / (y2 - y1)) * (x2 - x1);
      if (x < lo) lo = x;
      if (x > hi) hi = x;
    }
  }
  out[0] = lo;
  out[1] = hi;
  return lo <= hi;
}

const span = new Float64Array(2);

export function fillFlat(g: Grid, r: number, ax: number, ay: number, bx: number, by: number, z: number, skipA: boolean): void {
  if (z >= 0) return;
  const { x0, y0, cell, w, h } = g;
  const grid = g.z;
  const dx = bx - ax;
  const dy = by - ay;
  const len = Math.sqrt(dx * dx + dy * dy);
  const nx = len > 1e-12 ? (-dy / len) * r : 0;
  const ny = len > 1e-12 ? (dx / len) * r : 0;
  const iy0 = Math.max(0, Math.ceil((Math.min(ay, by) - r - y0) / cell - 0.5));
  const iy1 = Math.min(h - 1, Math.floor((Math.max(ay, by) + r - y0) / cell - 0.5));
  const r2 = r * r;
  for (let iy = iy0; iy <= iy1; iy++) {
    const yc = y0 + (iy + 0.5) * cell;
    if (!rowSpan(yc, r, ax, ay, bx, by, nx, ny, span)) continue;
    let a = Math.max(0, Math.ceil((span[0] - x0) / cell - 0.5));
    let b = Math.min(w - 1, Math.floor((span[1] - x0) / cell - 0.5));
    if (a > b) continue;
    const row = iy * w;
    let sa = 1;
    let sb = 0;
    if (skipA) {
      const da = yc - ay;
      if (da * da < r2) {
        const q = Math.sqrt(r2 - da * da);
        sa = Math.ceil((ax - q - x0) / cell - 0.5) + 1;
        sb = Math.floor((ax + q - x0) / cell - 0.5) - 1;
      }
    }
    if (sa <= sb) {
      for (let ix = a; ix < Math.min(sa, b + 1); ix++) if (z < grid[row + ix]) grid[row + ix] = z;
      a = Math.max(a, sb + 1);
    }
    for (let ix = a; ix <= b; ix++) if (z < grid[row + ix]) grid[row + ix] = z;
  }
}

export function stampRange(g: Grid, profiles: Profile[], data: Float32Array, from: number, to: number): void {
  let level = NaN;
  let levelSlot = -1;
  for (let i = Math.max(1, from); i < to; i++) {
    const o = i * STRIDE;
    if (Number.isNaN(data[o + 3]) || data[o + 4] % 2 === 1) {
      level = NaN;
      continue;
    }
    const q = o - STRIDE;
    const slot = Math.floor(data[o + 4] / 2);
    const p = profiles[slot];
    const az = data[q + 2];
    const bz = data[o + 2];
    if (p.flat && az === bz) {
      fillFlat(g, p.radius, data[q], data[q + 1], data[o], data[o + 1], az, level === az && levelSlot === slot);
      level = az;
      levelSlot = slot;
      continue;
    }
    stampSegment(g, p, data[q], data[q + 1], az, data[o], data[o + 1], bz);
    level = NaN;
  }
}

export interface Shading {
  stock: [number, number, number];
  top: [number, number, number];
  deep: [number, number, number];
}

export const METAL: Shading = { stock: [205, 211, 219], top: [168, 182, 199], deep: [58, 82, 116] };

export function shade(g: Grid, rgba: Uint8ClampedArray, floor: number | null, depthRange: number, s: Shading = METAL): void {
  const { w, h, z, cell } = g;
  const lx = -0.45;
  const ly = 0.45;
  const lz = 0.77;
  const range = Math.max(depthRange, 0.05);
  const through = floor === null ? -Infinity : -floor + 1e-3;
  for (let iy = 0; iy < h; iy++) {
    const out = (h - 1 - iy) * w;
    for (let ix = 0; ix < w; ix++) {
      const i = iy * w + ix;
      const o = (out + ix) * 4;
      const v = z[i];
      if (v <= through) {
        rgba[o + 3] = 0;
        continue;
      }
      if (v >= -1e-4) {
        rgba[o] = s.stock[0];
        rgba[o + 1] = s.stock[1];
        rgba[o + 2] = s.stock[2];
        rgba[o + 3] = 255;
        continue;
      }
      const at = (j: number) => {
        const u = z[j];
        return u <= through ? v : u;
      };
      const zl = ix > 0 ? at(i - 1) : v;
      const zr = ix < w - 1 ? at(i + 1) : v;
      const zd = iy > 0 ? at(i - w) : v;
      const zu = iy < h - 1 ? at(i + w) : v;
      const nx = -(zr - zl) / (2 * cell);
      const ny = -(zu - zd) / (2 * cell);
      const len = Math.sqrt(nx * nx + ny * ny + 1);
      const lit = 0.55 + 0.45 * Math.max(0, (nx * lx + ny * ly + lz) / len);
      const f = Math.min(1, -v / range);
      rgba[o] = (s.top[0] + (s.deep[0] - s.top[0]) * f) * lit;
      rgba[o + 1] = (s.top[1] + (s.deep[1] - s.top[1]) * f) * lit;
      rgba[o + 2] = (s.top[2] + (s.deep[2] - s.top[2]) * f) * lit;
      rgba[o + 3] = 255;
    }
  }
}

export function partAt(parts: SimPart[], vertex: number): number {
  for (let i = parts.length - 1; i >= 0; i--) if (vertex >= parts[i].start) return i;
  return 0;
}

export function formatClock(minutes: number): string {
  if (!Number.isFinite(minutes) || minutes < 0) return "–";
  const s = Math.round(minutes * 60);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const p = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${p(m)}:${p(sec)}` : `${m}:${p(sec)}`;
}
