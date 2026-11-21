import { invoke } from "@tauri-apps/api/core";

export type ToolMap = Record<string, number>;

export interface ToolInfo {
  number: number;
  description: string;
}

export interface InventoryEntry {
  description: string;
  number: number;
}

export interface FileInfo {
  id: number;
  name: string;
  path: string;
  tools: ToolInfo[];
  zmin: number | null;
  zorigin: string | null;
  stockTop: boolean;
  units: "mm" | "inch";
  incremental: boolean;
  lines: number;
}

export type NcError =
  | { code: "io"; file: string; message: string }
  | { code: "noToolChange"; file: string }
  | { code: "noProgramEnd"; file: string }
  | { code: "structure"; file: string }
  | { code: "badNumber"; file: string; line: number }
  | { code: "alreadyMerged"; file: string }
  | { code: "noFiles" }
  | { code: "mixedUnits" };

export type Warning =
  | { code: "zOriginDiffers"; origins: string[] }
  | { code: "offsetsDiffer" }
  | { code: "machinesDiffer"; models: string[] }
  | { code: "rapidOffIncremental" }
  | { code: "rapidOffZOrigin"; files: string[] }
  | { code: "rapidOffNonAscii"; file: string; line: number };

export interface CommandError {
  error: NcError | null;
  message: string;
}

export interface LoadError {
  name: string;
  path: string;
  error: NcError;
  message: string;
}

export interface LoadOutcome {
  files: FileInfo[];
  errors: LoadError[];
}

export interface LoadProgress {
  done: number;
  total: number;
  name: string;
}

export interface RapidStats {
  converted: number;
  distanceMm: number;
  feedTimeMin: number;
}

export interface Prepared {
  warnings: Warning[];
  warningTexts: string[];
  rapid: RapidStats | null;
  suggestedDir: string | null;
  suggestedName: string;
  lines: number;
}

export interface Saved {
  path: string;
  dir: string | null;
  bytes: number;
}

export interface Estimate {
  cutMm: number;
  cutMin: number;
  rapidMm: number;
  moves: number;
  unknownFeedMoves: number;
}

export interface FileEstimate {
  id: number;
  plain: Estimate;
  fast: Estimate | null;
}

export interface EstimateOutcome {
  rapidAllowed: boolean;
  files: FileEstimate[];
}

export interface Polyline {
  rapid: boolean;
  tool: number | null;
  points: number[];
}

export interface Preview {
  polylines: Polyline[];
  bounds: [number, number, number, number] | null;
  points: number;
}

export interface FilePreview {
  id: number;
  preview: Preview;
}

export function totalMinutes(e: Estimate, rapidSpeed: number): number {
  return e.cutMin + (rapidSpeed > 0 ? e.rapidMm / rapidSpeed : 0);
}

export const NC_EXTENSIONS = ["cnc", "nc", "gcode", "ngc", "tap", "txt"];

export const api = {
  loadFiles: (paths: string[]) => invoke<LoadOutcome>("load_files", { paths }),
  forgetFiles: (ids: number[]) => invoke<void>("forget_files", { ids }),
  autoTools: (ids: number[], inventory: InventoryEntry[]) => invoke<ToolMap[]>("auto_tools", { ids, inventory }),
  defaultInventory: () => invoke<InventoryEntry[]>("default_inventory"),
  prepareMerge: (ids: number[], maps: ToolMap[], restore: boolean, safeZ: number, stamp: string) =>
    invoke<Prepared>("prepare_merge", { ids, maps, restore, safeZ, stamp }),
  saveMerge: (path: string) => invoke<Saved>("save_merge", { path }),
  initialFiles: () => invoke<string[]>("initial_files"),
  estimate: (ids: number[], safeZ: number) => invoke<EstimateOutcome>("estimate", { ids, safeZ }),
  preview: (ids: number[], restore: boolean, safeZ: number) => invoke<FilePreview[]>("preview", { ids, restore, safeZ }),
  timeline: (ids: number[], restore: boolean, safeZ: number) => invoke<ArrayBuffer>("timeline", { ids, restore, safeZ }),
};

export function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "message" in e;
}

export function stamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
