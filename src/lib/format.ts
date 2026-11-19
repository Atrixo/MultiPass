export function formatDuration(minutes: number): string {
  if (!Number.isFinite(minutes) || minutes < 0) return "–";
  if (minutes < 1) return "< 1 min";
  const total = Math.round(minutes);
  const h = Math.floor(total / 60);
  const m = total % 60;
  if (h === 0) return `${m} min`;
  return `${h} h ${String(m).padStart(2, "0")} min`;
}

export const RAPID_SPEED_MIN = 1000;
export const RAPID_SPEED_MAX = 15000;
export const RAPID_SPEED_STEP = 500;
export const RAPID_SPEED_DEFAULT = 3000;

export function clampRapidSpeed(v: number): number {
  const r = Math.round(v / RAPID_SPEED_STEP) * RAPID_SPEED_STEP;
  return Math.min(RAPID_SPEED_MAX, Math.max(RAPID_SPEED_MIN, r));
}

const PALETTE = ["#0e7490", "#d97706", "#7c3aed", "#16a34a", "#dc2626", "#2563eb", "#db2777", "#65a30d", "#0891b2", "#9333ea"];

export function toolColor(tool: number, allTools: number[]): string {
  const sorted = [...new Set(allTools)].sort((a, b) => a - b);
  const i = sorted.indexOf(tool);
  return PALETTE[(i < 0 ? tool : i) % PALETTE.length];
}
