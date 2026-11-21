import { makeGrid, profile, shade, stampRange, type Grid, type Profile, type TimelineMeta } from "./sim";

export type ToWorker =
  | { type: "init"; meta: TimelineMeta; data: Float32Array; maxCells: number }
  | { type: "seek"; vertex: number };

export type FromWorker =
  | { type: "grid"; x0: number; y0: number; cell: number; w: number; h: number }
  | { type: "frame"; vertex: number; rgba: Uint8ClampedArray; progress: number };

const CHUNK = 15_000;
const CHECKPOINTS = 16;
const FRAME_MS = 90;

let meta: TimelineMeta | null = null;
let data: Float32Array = new Float32Array(0);
let grid: Grid | null = null;
let profiles: Profile[] = [];
let done = 0;
let target = 0;
let step = 1;
let snapshots = new Map<number, Float32Array>();
let running = false;
let lastFrame = 0;
let floor: number | null = null;
let depthRange = 1;

const post = (m: FromWorker, transfer: Transferable[] = []) => (self as unknown as Worker).postMessage(m, transfer);

function frame(): void {
  if (!grid) return;
  const rgba = new Uint8ClampedArray(grid.w * grid.h * 4);
  shade(grid, rgba, floor, depthRange);
  post({ type: "frame", vertex: done, rgba, progress: target - done > 40_000 ? done / target : 1 }, [rgba.buffer]);
  lastFrame = performance.now();
}

function rewind(to: number): void {
  if (!grid) return;
  let best = 0;
  for (const k of snapshots.keys()) if (k <= to && k > best) best = k;
  if (best === 0) grid.z.fill(0);
  else grid.z.set(snapshots.get(best)!);
  done = best;
}

function work(): void {
  if (!grid || !meta) return;
  running = true;
  const until = Math.min(target, done + CHUNK);
  while (done < until) {
    const next = Math.min(until, (Math.floor(done / step) + 1) * step);
    stampRange(grid, profiles, data, done, next);
    done = next;
    if (done % step === 0 && !snapshots.has(done) && snapshots.size < CHECKPOINTS) snapshots.set(done, grid.z.slice());
  }
  if (done >= target) {
    running = false;
    frame();
    return;
  }
  if (performance.now() - lastFrame > FRAME_MS) frame();
  setTimeout(work, 0);
}

self.onmessage = (e: MessageEvent<ToWorker>) => {
  const m = e.data;
  if (m.type === "init") {
    meta = m.meta;
    data = m.data;
    profiles = meta.tools.map((t) => profile(t.shape));
    grid = meta.bounds ? makeGrid(meta.bounds, m.maxCells) : null;
    floor = meta.stockHeight;
    depthRange = Math.max(0.1, Math.min(meta.stockHeight ?? Infinity, -meta.zmin));
    snapshots = new Map();
    step = Math.max(1, Math.ceil(meta.vertices / CHECKPOINTS));
    done = 0;
    target = 0;
    if (grid) post({ type: "grid", x0: grid.x0, y0: grid.y0, cell: grid.cell, w: grid.w, h: grid.h });
    return;
  }
  if (!grid || !meta) return;
  const v = Math.max(0, Math.min(meta.vertices, Math.floor(m.vertex)));
  if (v < done) rewind(v);
  target = v;
  if (!running) {
    if (done >= target) frame();
    else work();
  }
};
