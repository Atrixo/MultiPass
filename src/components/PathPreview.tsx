import { Eye, Focus, Loader2, Maximize2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { FilePreview } from "../lib/api";
import { toolColor } from "../lib/format";
import { useI18n } from "../lib/i18n";
import { toolNumber, type Entry } from "../lib/state";
import { cx } from "./ui";

interface Props {
  entries: Entry[];
  previews: FilePreview[] | null;
  loading: boolean;
  selected: number | null;
  onExpand?: () => void;
}

interface Layer {
  id: number;
  color: string;
  previewTool: (orig: number | null) => number;
  data: FilePreview["preview"];
}

function Toggle({ on, onClick, icon, label }: { on: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={onClick}
      className={cx(
        "inline-flex min-h-11 items-center gap-2 rounded-xl border px-3 text-sm font-medium transition-colors",
        on ? "border-primary bg-primary-soft text-primary" : "border-line bg-surface text-muted hover:bg-surface-2",
      )}
    >
      {icon}
      {label}
    </button>
  );
}

export function PathPreview({ entries, previews, loading, selected, onExpand }: Props) {
  const { t } = useI18n();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const [onlySelected, setOnlySelected] = useState(false);
  const [showRapids, setShowRapids] = useState(true);
  const [size, setSize] = useState({ w: 0, h: 0 });

  useEffect(() => {
    const el = boxRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([e]) => setSize({ w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const allTools = useMemo(() => entries.flatMap((e) => e.info.tools.map((x) => toolNumber(e, x.number))), [entries]);

  const layers = useMemo<Layer[]>(() => {
    if (!previews) return [];
    const byId = new Map(previews.map((p) => [p.id, p]));
    return entries
      .filter((e) => byId.has(e.id) && (!onlySelected || e.id === selected))
      .map((e) => {
        const first = e.info.tools[0]?.number ?? 1;
        const previewTool = (orig: number | null) => toolNumber(e, orig ?? first);
        return { id: e.id, color: toolColor(previewTool(null), allTools), previewTool, data: byId.get(e.id)!.preview };
      });
  }, [previews, entries, onlySelected, selected, allTools]);

  const legend = useMemo(() => {
    const seen = new Map<number, string>();
    for (const e of entries)
      for (const x of e.info.tools) {
        const n = toolNumber(e, x.number);
        if (!seen.has(n)) seen.set(n, x.description);
      }
    return [...seen.entries()].sort((a, b) => a[0] - b[0]);
  }, [entries]);

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
    const bounds = layers.map((l) => l.data.bounds).filter((b): b is [number, number, number, number] => b !== null);
    if (bounds.length === 0) return;
    let [x0, y0, x1, y1] = bounds[0];
    for (const b of bounds) {
      x0 = Math.min(x0, b[0]);
      y0 = Math.min(y0, b[1]);
      x1 = Math.max(x1, b[2]);
      y1 = Math.max(y1, b[3]);
    }
    const pad = 18;
    const scale = Math.min((size.w - 2 * pad) / Math.max(x1 - x0, 1e-6), (size.h - 2 * pad) / Math.max(y1 - y0, 1e-6));
    const ox = pad + (size.w - 2 * pad - (x1 - x0) * scale) / 2;
    const oy = pad + (size.h - 2 * pad - (y1 - y0) * scale) / 2;
    const px = (x: number) => ox + (x - x0) * scale;
    const py = (y: number) => size.h - (oy + (y - y0) * scale);
    const muted = getComputedStyle(document.documentElement).getPropertyValue("--muted").trim() || "#888";

    if (x0 <= 0 && x1 >= 0 && y0 <= 0 && y1 >= 0) {
      g.strokeStyle = muted;
      g.globalAlpha = 0.6;
      g.lineWidth = 1;
      g.setLineDash([]);
      g.beginPath();
      g.moveTo(px(0) - 10, py(0));
      g.lineTo(px(0) + 10, py(0));
      g.moveTo(px(0), py(0) - 10);
      g.lineTo(px(0), py(0) + 10);
      g.stroke();
    }

    const draw = (rapid: boolean) => {
      for (const layer of layers) {
        for (const pl of layer.data.polylines) {
          if (pl.rapid !== rapid) continue;
          const pts = pl.points;
          g.strokeStyle = toolColor(layer.previewTool(pl.tool), allTools);
          g.globalAlpha = rapid ? 0.55 : onlySelected || layer.id === selected || selected === null ? 0.95 : 0.6;
          g.lineWidth = rapid ? 1 : 1.3;
          g.setLineDash(rapid ? [5, 4] : []);
          g.beginPath();
          g.moveTo(px(pts[0]), py(pts[1]));
          for (let i = 2; i < pts.length; i += 2) g.lineTo(px(pts[i]), py(pts[i + 1]));
          g.stroke();
        }
      }
    };
    draw(false);
    if (showRapids) draw(true);
    g.globalAlpha = 1;
    g.setLineDash([]);
  }, [layers, size, showRapids, onlySelected, selected, allTools]);

  const empty = !loading && previews !== null && layers.every((l) => l.data.polylines.length === 0);

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-2 px-4 py-3">
      <div className="flex flex-wrap items-center gap-2">
        <Toggle on={showRapids} onClick={() => setShowRapids((v) => !v)} icon={<Eye className="size-4" />} label={t.previewRapids} />
        <Toggle
          on={onlySelected}
          onClick={() => setOnlySelected((v) => !v)}
          icon={<Focus className="size-4" />}
          label={t.previewOnlySelected}
        />
        <span className="ml-auto text-sm text-muted">{t.previewDashed}</span>
        {onExpand && <Toggle on={false} onClick={onExpand} icon={<Maximize2 className="size-4" />} label={t.previewExpand} />}
      </div>
      <div ref={boxRef} className="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-line bg-surface-2">
        <canvas ref={canvasRef} className="absolute inset-0 size-full" />
        {(loading || empty) && (
          <div className="absolute inset-0 flex items-center justify-center gap-2 text-muted">
            {loading && <Loader2 className="size-5 animate-spin" />}
            {loading ? t.previewLoading : t.previewEmpty}
          </div>
        )}
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        {legend.map(([n, d]) => (
          <span key={n} className="inline-flex items-center gap-2 text-sm">
            <span className="inline-block h-1.5 w-5 rounded-full" style={{ background: toolColor(n, allTools) }} />
            <span className="font-semibold">T{n}</span>
            <span className="text-muted">{d}</span>
          </span>
        ))}
      </div>
    </div>
  );
}
