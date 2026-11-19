import type { FileInfo, ToolMap } from "./api";

export interface Entry {
  id: number;
  info: FileInfo;
  auto: ToolMap;
  manual: ToolMap | null;
}

export interface State {
  entries: Entry[];
  selected: number | null;
  restore: boolean;
  safeZ: number;
}

export type Action =
  | { type: "add"; infos: FileInfo[] }
  | { type: "remove"; id: number }
  | { type: "clear" }
  | { type: "move"; id: number; delta: number }
  | { type: "reorder"; activeId: number; overId: number }
  | { type: "select"; id: number | null }
  | { type: "setAuto"; ids: number[]; maps: ToolMap[] }
  | { type: "setTool"; id: number; orig: number; value: number }
  | { type: "resetTools" }
  | { type: "setRestore"; value: boolean }
  | { type: "setSafeZ"; value: number };

export const SAFE_Z_MIN = 0.5;
export const SAFE_Z_MAX = 20;
export const SAFE_Z_STEP = 0.5;
export const TOOL_MIN = 1;
export const TOOL_MAX = 99;

export const initialState: State = { entries: [], selected: null, restore: false, safeZ: 1.0 };

export function effectiveMap(e: Entry): ToolMap {
  return { ...e.auto, ...(e.manual ?? {}) };
}

export function toolNumber(e: Entry, orig: number): number {
  return effectiveMap(e)[String(orig)] ?? orig;
}

export function clampSafeZ(v: number): number {
  const r = Math.round(v / SAFE_Z_STEP) * SAFE_Z_STEP;
  return Math.min(SAFE_Z_MAX, Math.max(SAFE_Z_MIN, r));
}

export function clampTool(v: number): number {
  return Math.min(TOOL_MAX, Math.max(TOOL_MIN, Math.round(v)));
}

function moveItem<T>(list: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return list;
  const next = list.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

export function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "add": {
      if (action.infos.length === 0) return state;
      const added: Entry[] = action.infos.map((info) => ({ id: info.id, info, auto: {}, manual: null }));
      return { ...state, entries: [...state.entries, ...added], selected: state.selected ?? added[0].id };
    }
    case "remove": {
      const idx = state.entries.findIndex((e) => e.id === action.id);
      if (idx < 0) return state;
      const entries = state.entries.filter((e) => e.id !== action.id);
      let selected = state.selected;
      if (selected === action.id) selected = entries[Math.min(idx, entries.length - 1)]?.id ?? null;
      return { ...state, entries, selected };
    }
    case "clear":
      return { ...state, entries: [], selected: null };
    case "move": {
      const idx = state.entries.findIndex((e) => e.id === action.id);
      return { ...state, entries: moveItem(state.entries, idx, idx + action.delta) };
    }
    case "reorder": {
      const from = state.entries.findIndex((e) => e.id === action.activeId);
      const to = state.entries.findIndex((e) => e.id === action.overId);
      return { ...state, entries: moveItem(state.entries, from, to) };
    }
    case "select":
      return { ...state, selected: action.id };
    case "setAuto": {
      const byId = new Map(action.ids.map((id, i) => [id, action.maps[i]]));
      return {
        ...state,
        entries: state.entries.map((e) => (byId.has(e.id) ? { ...e, auto: byId.get(e.id) ?? {} } : e)),
      };
    }
    case "setTool":
      return {
        ...state,
        entries: state.entries.map((e) =>
          e.id === action.id ? { ...e, manual: { ...(e.manual ?? {}), [String(action.orig)]: clampTool(action.value) } } : e,
        ),
      };
    case "resetTools":
      return { ...state, entries: state.entries.map((e) => ({ ...e, manual: null })) };
    case "setRestore":
      return { ...state, restore: action.value };
    case "setSafeZ":
      return { ...state, safeZ: clampSafeZ(action.value) };
  }
}

export interface SequenceStep {
  tool: number;
  description: string;
  sameAsBefore: boolean;
  position: number;
  name: string;
}

export function sequence(entries: Entry[]): SequenceStep[] {
  const steps: SequenceStep[] = [];
  entries.forEach((e, i) => {
    for (const t of e.info.tools) {
      const tool = toolNumber(e, t.number);
      const last = steps[steps.length - 1];
      steps.push({ tool, description: t.description, sameAsBefore: last?.tool === tool, position: i + 1, name: e.info.name });
    }
  });
  return steps;
}

export function hasManual(state: State): boolean {
  return state.entries.some((e) => e.manual !== null);
}
