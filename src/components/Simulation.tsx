import {
  Loader2,
  Maximize2,
  Pause,
  Play,
  Scan,
  SkipBack,
  SkipForward,
  StepBack,
  StepForward,
  TriangleAlert,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { api, isCommandError } from "../lib/api";
import { toolColor } from "../lib/format";
import { useI18n } from "../lib/i18n";
import {
  cumulativeTimes,
  formatClock,
  isJump,
  isRapid,
  partAt,
  parseTimeline,
  positionAt,
  slotOf,
  STRIDE,
  vertexAt,
  type Timeline,
} from "../lib/sim";
import type { FromWorker, ToWorker } from "../lib/sim.worker";
import { toolNumber, type Entry } from "../lib/state";
import { VIEW_FIT, ZOOM_STEP, apply, pan, zoomAt, type View } from "../lib/view";
import { cx, Toggle } from "./ui";

interface Props {
  entries: Entry[];
  restore: boolean;
  safeZ: number;
  rapidSpeed: number;
  onExpand?: () => void;
  header?: ReactNode;
}

export const SPEEDS = [10, 100, 500, 2000];
const TRAIL = 800;
const MAX_CELLS = 600_000;

let cached: { key: string; promise: Promise<Timeline> } | null = null;

function loadTimeline(ids: number[], restore: boolean, safeZ: number): Promise<Timeline> {
  const key = `${ids.join(",")}|${restore}|${safeZ}`;
  if (cached?.key !== key) {
    const promise = api.timeline(ids, restore, safeZ).then(parseTimeline);
    promise.catch(() => {
      if (cached?.promise === promise) cached = null;
    });
    cached = { key, promise };
  }
  return cached.promise;
}

interface GridInfo {
  x0: number;
  y0: number;
  cell: number;
  w: number;
  h: number;
}

function IconBtn({ label, onClick, disabled, children, primary }: { label: string; onClick: () => void; disabled?: boolean; children: ReactNode; primary?: boolean }) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      aria-label={label}
      title={label}
      className={cx(
        "inline-flex size-11 items-center justify-center rounded-xl border disabled:opacity-40",
        primary ? "border-primary bg-primary text-primary-fg hover:bg-primary-hover" : "border-line bg-surface hover:bg-surface-2",
      )}
    >
      {children}
    </button>
  );
}

export function Simulation({ entries, restore, safeZ, rapidSpeed, onExpand, header }: Props) {
  const { t } = useI18n();
  const ids = useMemo(() => entries.map((e) => e.id), [entries]);
  const idKey = ids.join(",");
  const [tl, setTl] = useState<Timeline | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [simT, setSimT] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [speed, setSpeed] = useState(SPEEDS[1]);
  const [stopAtChange, setStopAtChange] = useState(true);
  const [notice, setNotice] = useState<string | null>(null);
  const [grid, setGrid] = useState<GridInfo | null>(null);
  const [frameNo, setFrameNo] = useState(0);
  const [progress, setProgress] = useState(1);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [view, setView] = useState<View>(VIEW_FIT);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLCanvasElement | null>(null);
  const workerRef = useRef<Worker | null>(null);
  const flow = useRef({ busy: false, wanted: -1, sent: -1 });
  const drag = useRef<{ x: number; y: number; id: number } | null>(null);

  useEffect(() => {
    let stale = false;
    setTl(null);
    setError(null);
    setPlaying(false);
    setNotice(null);
    if (!idKey) return;
    loadTimeline(idKey.split(",").map(Number), restore, safeZ)
      .then((v) => !stale && setTl(v))
      .catch((e) => !stale && setError(isCommandError(e) ? e.message : String(e)));
    return () => {
      stale = true;
    };
  }, [idKey, restore, safeZ]);

  const times = useMemo(() => (tl ? cumulativeTimes(tl.data, rapidSpeed) : new Float64Array(0)), [tl, rapidSpeed]);
  const total = times.length > 0 ? times[times.length - 1] : 0;

  const request = useCallback((vertex: number) => {
    const f = flow.current;
    f.wanted = vertex;
    if (f.busy || !workerRef.current || f.sent === vertex) return;
    f.busy = true;
    f.sent = vertex;
    workerRef.current.postMessage({ type: "seek", vertex } satisfies ToWorker);
  }, []);

  useEffect(() => {
    setGrid(null);
    imageRef.current = null;
    flow.current = { busy: false, wanted: -1, sent: -1 };
    if (!tl || !tl.meta.material || !tl.meta.bounds) return;
    const w = new Worker(new URL("../lib/sim.worker.ts", import.meta.url), { type: "module" });
    workerRef.current = w;
    w.onmessage = (e: MessageEvent<FromWorker>) => {
      const m = e.data;
      if (m.type === "grid") {
        const c = document.createElement("canvas");
        c.width = m.w;
        c.height = m.h;
        imageRef.current = c;
        setGrid({ x0: m.x0, y0: m.y0, cell: m.cell, w: m.w, h: m.h });
        return;
      }
      const c = imageRef.current;
      if (c) c.getContext("2d")?.putImageData(new ImageData(new Uint8ClampedArray(m.rgba.buffer as ArrayBuffer), c.width, c.height), 0, 0);
      setProgress(m.progress);
      setFrameNo((n) => n + 1);
      const f = flow.current;
      if (m.vertex === f.sent) {
        f.busy = false;
        if (f.wanted !== f.sent) {
          f.busy = true;
          f.sent = f.wanted;
          w.postMessage({ type: "seek", vertex: f.wanted } satisfies ToWorker);
        }
      }
    };
    const data = tl.data.slice();
    w.postMessage({ type: "init", meta: tl.meta, data, maxCells: MAX_CELLS } satisfies ToWorker, [data.buffer]);
    setProgress(0);
    return () => {
      w.terminate();
      if (workerRef.current === w) workerRef.current = null;
    };
  }, [tl]);

  useEffect(() => {
    if (tl) setSimT(total);
  }, [tl, total]);

  const vertex = useMemo(() => (times.length ? vertexAt(times, simT) : 0), [times, simT]);

  useEffect(() => {
    if (!tl || !grid) return;
    request(Math.min(tl.meta.vertices, vertex + 1));
  }, [tl, grid, vertex, request]);

  const boundaries = useMemo(() => (tl ? tl.meta.parts.slice(1).map((p) => times[Math.max(0, p.start - 1)] ?? 0) : []), [tl, times]);

  const toolLabel = useCallback(
    (slot: number) => {
      if (!tl) return "";
      const st = tl.meta.tools[slot];
      const e = entries[st?.part ?? 0];
      if (!st || !e) return "";
      const orig = st.tool ?? e.info.tools[0]?.number ?? 1;
      return `T${toolNumber(e, orig)}`;
    },
    [tl, entries],
  );

  const simRef = useRef(0);
  simRef.current = simT;

  useEffect(() => {
    if (!playing || !tl) return;
    let raf = 0;
    let last = performance.now();
    const loop = (now: number) => {
      const cur = simRef.current;
      let next = cur + ((now - last) / 60000) * speed;
      last = now;
      let stop = false;
      if (stopAtChange) {
        const bi = boundaries.findIndex((x) => x > cur + 1e-9 && x <= next);
        if (bi >= 0) {
          next = boundaries[bi];
          stop = true;
          const first = tl.meta.tools.findIndex((x) => x.part === bi + 1);
          setNotice(t.simToolChange(toolLabel(first < 0 ? 0 : first)));
        }
      }
      if (next >= total) {
        next = total;
        stop = true;
      }
      simRef.current = next;
      setSimT(next);
      if (stop) setPlaying(false);
      else raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [playing, tl, speed, stopAtChange, boundaries, total, t, toolLabel]);

  useEffect(() => {
    const el = boxRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setSize({ w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const c = canvasRef.current;
    if (!c) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const r = c.getBoundingClientRect();
      setView((v) => zoomAt(v, e.deltaY < 0 ? 1.25 : 1 / 1.25, e.clientX - r.left, e.clientY - r.top));
    };
    c.addEventListener("wheel", onWheel, { passive: false });
    return () => c.removeEventListener("wheel", onWheel);
  }, []);

  const allTools = useMemo(() => entries.flatMap((e) => e.info.tools.map((x) => toolNumber(e, x.number))), [entries]);

  const slotColor = useCallback(
    (slot: number) => {
      const st = tl?.meta.tools[slot];
      const e = st ? entries[st.part] : undefined;
      if (!st || !e) return "#888";
      return toolColor(toolNumber(e, st.tool ?? e.info.tools[0]?.number ?? 1), allTools);
    },
    [tl, entries, allTools],
  );

  const world = useMemo<[number, number, number, number] | null>(() => {
    if (grid) return [grid.x0, grid.y0, grid.x0 + grid.w * grid.cell, grid.y0 + grid.h * grid.cell];
    if (!tl) return null;
    if (tl.meta.bounds) return tl.meta.bounds;
    let b: [number, number, number, number] | null = null;
    for (let i = 0; i < tl.meta.vertices; i++) {
      const x = tl.data[i * STRIDE];
      const y = tl.data[i * STRIDE + 1];
      if (!b) b = [x, y, x, y];
      else b = [Math.min(b[0], x), Math.min(b[1], y), Math.max(b[2], x), Math.max(b[3], y)];
    }
    return b;
  }, [grid, tl]);

  const pos = useMemo(() => (tl && times.length ? positionAt(tl.data, times, simT) : null), [tl, times, simT]);

  useEffect(() => {
    const c = canvasRef.current;
    if (!c || size.w === 0 || size.h === 0) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(size.w * dpr);
    c.height = Math.round(size.h * dpr);
    const g = c.getContext("2d");
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, size.w, size.h);
    if (!world || !tl) return;
    const [x0, y0, x1, y1] = world;
    const pad = 18;
    const scale = Math.min((size.w - 2 * pad) / Math.max(x1 - x0, 1e-6), (size.h - 2 * pad) / Math.max(y1 - y0, 1e-6));
    const ox = pad + (size.w - 2 * pad - (x1 - x0) * scale) / 2;
    const oy = pad + (size.h - 2 * pad - (y1 - y0) * scale) / 2;
    const px = (x: number) => (ox + (x - x0) * scale) * view.k + view.tx;
    const py = (y: number) => apply(view, 0, size.h - (oy + (y - y0) * scale))[1];
    const s = scale * view.k;

    const img = imageRef.current;
    if (img && grid) {
      g.imageSmoothingEnabled = s / (1 / grid.cell) < 2;
      g.drawImage(img, px(x0), py(y1), (x1 - x0) * s, (y1 - y0) * s);
    }

    const muted = getComputedStyle(document.documentElement).getPropertyValue("--muted").trim() || "#888";
    if (x0 <= 0 && x1 >= 0 && y0 <= 0 && y1 >= 0) {
      g.strokeStyle = muted;
      g.globalAlpha = 0.7;
      g.lineWidth = 1;
      g.beginPath();
      g.moveTo(px(0) - 10, py(0));
      g.lineTo(px(0) + 10, py(0));
      g.moveTo(px(0), py(0) - 10);
      g.lineTo(px(0), py(0) + 10);
      g.stroke();
    }

    if (!pos) return;
    const [cx, cy, cz, vi] = pos;
    const d = tl.data;
    const from = Math.max(1, vi - TRAIL);
    g.lineWidth = 1.5;
    for (let i = from; i <= vi; i++) {
      if (isJump(d, i)) continue;
      const rapid = isRapid(d, i);
      g.strokeStyle = slotColor(slotOf(d, i));
      g.globalAlpha = (rapid ? 0.5 : 0.9) * (0.25 + (0.75 * (i - from)) / Math.max(1, vi - from));
      g.setLineDash(rapid ? [5, 4] : []);
      g.beginPath();
      g.moveTo(px(d[(i - 1) * STRIDE]), py(d[(i - 1) * STRIDE + 1]));
      g.lineTo(px(d[i * STRIDE]), py(d[i * STRIDE + 1]));
      g.stroke();
    }
    const next = vi + 1 < tl.meta.vertices && !isJump(d, vi + 1) ? vi + 1 : vi;
    const slot = slotOf(d, next);
    const color = slotColor(slot);
    if (next !== vi) {
      g.setLineDash(isRapid(d, next) ? [5, 4] : []);
      g.globalAlpha = 0.9;
      g.strokeStyle = color;
      g.beginPath();
      g.moveTo(px(d[vi * STRIDE]), py(d[vi * STRIDE + 1]));
      g.lineTo(px(cx), py(cy));
      g.stroke();
    }
    g.setLineDash([]);
    const r = Math.max(3, ((tl.meta.tools[slot]?.shape.diameter ?? 1) / 2) * s);
    g.globalAlpha = cz < 0 ? 0.35 : 0.12;
    g.fillStyle = color;
    g.beginPath();
    g.arc(px(cx), py(cy), r, 0, Math.PI * 2);
    g.fill();
    g.globalAlpha = 1;
    g.lineWidth = 2;
    g.strokeStyle = color;
    g.stroke();
    g.beginPath();
    g.arc(px(cx), py(cy), 2.5, 0, Math.PI * 2);
    g.fillStyle = color;
    g.fill();
  }, [size, view, world, tl, grid, pos, frameNo, slotColor]);

  const zoomCenter = (factor: number) => setView((v) => zoomAt(v, factor, size.w / 2, size.h / 2));

  const seek = (m: number) => {
    setNotice(null);
    setSimT(Math.max(0, Math.min(total, m)));
  };

  const partIndex = tl ? partAt(tl.meta.parts, vertex) : 0;
  const partStarts = useMemo(() => (tl ? [0, ...boundaries] : []), [tl, boundaries]);
  const prevPart = () => {
    const before = partStarts.filter((x) => x < simT - 1e-6);
    const target = before.length > 1 && simT - before[before.length - 1] < total * 0.002 ? before[before.length - 2] : before[before.length - 1];
    seek(target ?? 0);
  };
  const nextPart = () => seek(partStarts.find((x) => x > simT + 1e-6) ?? total);

  const togglePlay = () => {
    if (playing) {
      setPlaying(false);
      return;
    }
    setNotice(null);
    if (simT >= total - 1e-9) setSimT(0);
    setPlaying(true);
  };

  const barRef = useRef<HTMLDivElement>(null);
  const barDrag = useRef<number | null>(null);
  const seekBar = (clientX: number) => {
    const el = barRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    seek(((clientX - r.left) / Math.max(1, r.width)) * total);
  };

  const slot = tl && tl.meta.vertices > 0 ? slotOf(tl.data, Math.min(tl.meta.vertices - 1, vertex + 1)) : 0;
  const entry = tl ? entries[tl.meta.tools[slot]?.part ?? 0] : undefined;
  const toolDesc = entry && tl ? (entry.info.tools.find((x) => x.number === (tl.meta.tools[slot]?.tool ?? entry.info.tools[0]?.number))?.description ?? "") : "";
  const unknownTool = tl?.meta.tools.some((x) => !x.known) ?? false;
  const computing = !!tl?.meta.material && progress < 1;
  const empty = !!tl && tl.meta.vertices === 0;
  const compact = !!onExpand;
  const playButton = (
    <IconBtn label={playing ? t.simPause : t.simPlay} onClick={togglePlay} disabled={!tl || empty} primary>
      {playing ? <Pause className="size-5" /> : <Play className="size-5" />}
    </IconBtn>
  );
  const clock = (
    <span className="text-[15px] font-semibold tabular-nums">
      {formatClock(simT)} / {formatClock(total)}
    </span>
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 px-4 py-3">
      <div className="flex flex-wrap items-center gap-2">
        {header}
        {compact && playButton}
        {compact && clock}
        {!compact && <span className="text-sm text-muted">{t.simSpeed}</span>}
        {!compact && (
        <div className="inline-flex overflow-hidden rounded-xl border border-line">
          {SPEEDS.map((v) => (
            <button
              key={v}
              type="button"
              aria-pressed={speed === v}
              onClick={() => setSpeed(v)}
              className={cx(
                "min-h-11 min-w-14 px-2 text-sm font-semibold tabular-nums transition-colors",
                speed === v ? "bg-primary-soft text-primary" : "bg-surface text-muted hover:bg-surface-2",
              )}
            >
              {v}×
            </button>
          ))}
        </div>
        )}
        {!compact && <Toggle on={stopAtChange} onClick={() => setStopAtChange((v) => !v)} icon={<Pause className="size-4" />} label={t.simStopAtChange} />}
        <span className="ml-auto" />
        {!compact && (
          <>
            <IconBtn label={t.zoomOut} onClick={() => zoomCenter(1 / ZOOM_STEP)} disabled={view.k <= 1}>
              <ZoomOut className="size-5" />
            </IconBtn>
            <IconBtn label={t.zoomIn} onClick={() => zoomCenter(ZOOM_STEP)}>
              <ZoomIn className="size-5" />
            </IconBtn>
            <Toggle on={false} onClick={() => setView(VIEW_FIT)} icon={<Scan className="size-4" />} label={t.zoomFit} />
          </>
        )}
        {onExpand && <Toggle on={false} onClick={() => { setPlaying(false); onExpand(); }} icon={<Maximize2 className="size-4" />} label={t.previewExpand} />}
      </div>
      <div ref={boxRef} className="relative min-h-28 flex-1 overflow-hidden rounded-xl border border-line bg-surface-2">
        <canvas
          ref={canvasRef}
          className={cx("absolute inset-0 size-full touch-none", view.k > 1 && "cursor-grab active:cursor-grabbing")}
          onPointerDown={(e) => {
            if (e.button !== 0) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            drag.current = { x: e.clientX, y: e.clientY, id: e.pointerId };
          }}
          onPointerMove={(e) => {
            const d = drag.current;
            if (!d || d.id !== e.pointerId) return;
            setView((v) => pan(v, e.clientX - d.x, e.clientY - d.y));
            drag.current = { ...d, x: e.clientX, y: e.clientY };
          }}
          onPointerUp={() => (drag.current = null)}
          onPointerCancel={() => (drag.current = null)}
        />
        {view.k > 1 && (
          <span className="pointer-events-none absolute top-2 left-2 rounded-md bg-surface/80 px-2 py-0.5 text-sm font-semibold tabular-nums">
            {Math.round(view.k * 100)} %
          </span>
        )}
        {computing && (
          <span className="pointer-events-none absolute top-2 right-2 inline-flex items-center gap-2 rounded-md bg-surface/85 px-2 py-1 text-sm">
            <Loader2 className="size-4 animate-spin" />
            {t.simComputing(Math.round(progress * 100))}
          </span>
        )}
        {notice && (
          <div className="pointer-events-none absolute inset-x-0 top-3 flex justify-center px-3">
            <span className="rounded-xl border border-warn-line bg-warn-soft px-4 py-2 text-[15px] font-semibold text-ink shadow-sm">{notice}</span>
          </div>
        )}
        {(!tl || empty || error) && (
          <div className="absolute inset-0 flex items-center justify-center gap-2 px-6 text-center text-muted">
            {!tl && !error && <Loader2 className="size-5 animate-spin" />}
            {error ?? (empty ? t.simEmpty : t.simLoading)}
          </div>
        )}
      </div>
      <div
        ref={barRef}
        role="slider"
        aria-label={t.simTimeline}
        aria-valuemin={0}
        aria-valuemax={Math.round(total * 60)}
        aria-valuenow={Math.round(simT * 60)}
        className={cx("relative shrink-0 cursor-pointer touch-none rounded-lg border border-line bg-surface-2", compact ? "h-7" : "h-9")}
        onPointerDown={(e) => {
          if (e.button !== 0 || !total) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          barDrag.current = e.pointerId;
          setPlaying(false);
          seekBar(e.clientX);
        }}
        onPointerMove={(e) => barDrag.current === e.pointerId && seekBar(e.clientX)}
        onPointerUp={() => (barDrag.current = null)}
        onPointerCancel={() => (barDrag.current = null)}
      >
        {tl &&
          total > 0 &&
          tl.meta.parts.map((p, i) => {
            const a = i === 0 ? 0 : boundaries[i - 1];
            const b = i + 1 < tl.meta.parts.length ? boundaries[i] : total;
            const s0 = tl.meta.tools.findIndex((x) => x.part === i);
            return (
              <div
                key={i}
                className="absolute inset-y-1 rounded-sm border-r-2 border-surface"
                style={{ left: `${(a / total) * 100}%`, width: `${((b - a) / total) * 100}%`, background: slotColor(s0 < 0 ? 0 : s0), opacity: i === partIndex ? 0.85 : 0.45 }}
                title={p.name}
              />
            );
          })}
        {total > 0 && (
          <div className="pointer-events-none absolute inset-y-0 w-1 -translate-x-1/2 rounded-full bg-ink shadow" style={{ left: `${(simT / total) * 100}%` }} />
        )}
      </div>
      {!compact && (
        <>
      <div className="flex flex-wrap items-center gap-2">
        <IconBtn label={t.simStart} onClick={() => seek(0)} disabled={!tl}>
          <SkipBack className="size-5" />
        </IconBtn>
        <IconBtn label={t.simPrevPart} onClick={prevPart} disabled={!tl}>
          <StepBack className="size-5" />
        </IconBtn>
        {playButton}
        <IconBtn label={t.simNextPart} onClick={nextPart} disabled={!tl}>
          <StepForward className="size-5" />
        </IconBtn>
        <IconBtn label={t.simEnd} onClick={() => seek(total)} disabled={!tl}>
          <SkipForward className="size-5" />
        </IconBtn>
        {clock}
        {tl && tl.meta.parts.length > 0 && (
          <span className="min-w-0 truncate text-sm">
            <span className="font-semibold">{t.simPart(partIndex + 1, tl.meta.parts.length, tl.meta.parts[partIndex].name)}</span>
            <span className="text-muted">
              {" · "}
              <span style={{ color: slotColor(slot) }} className="font-semibold">
                {toolLabel(slot)}
              </span>{" "}
              {toolDesc}
              {pos && ` · Z ${pos[2].toFixed(2)} mm ${pos[2] < 0 ? t.simCutting : t.simInAir}`}
            </span>
          </span>
        )}
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-muted">
        {tl && !tl.meta.material && (
          <span className="inline-flex items-center gap-1.5 text-ink">
            <TriangleAlert className="size-4 text-warn" />
            {t.simNoMaterial}
          </span>
        )}
        {unknownTool && (
          <span className="inline-flex items-center gap-1.5 text-ink">
            <TriangleAlert className="size-4 text-warn" />
            {t.simUnknownTool}
          </span>
        )}
        <span>{t.simHint}</span>
      </div>
        </>
      )}
    </div>
  );
}
