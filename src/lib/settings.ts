import { LazyStore } from "@tauri-apps/plugin-store";
import type { InventoryEntry } from "./api";

export type Lang = "de" | "en";
export type ThemeMode = "system" | "light" | "dark";

export interface Settings {
  lang?: Lang;
  theme?: ThemeMode;
  restore?: boolean;
  safeZ?: number;
  lastDir?: string;
  inventory?: InventoryEntry[];
  rapidSpeed?: number;
}

type Key = keyof Settings;
const KEYS: Key[] = ["lang", "theme", "restore", "safeZ", "lastDir", "inventory", "rapidSpeed"];
const LOCAL_KEY = "multipass.settings";

function useLocal(): boolean {
  return !("__TAURI_INTERNALS__" in window) || "__MULTIPASS_MOCK__" in window;
}

let store: LazyStore | null = null;
function tauriStore(): LazyStore {
  store ??= new LazyStore("settings.json", { autoSave: 300 });
  return store;
}

function readLocal(): Settings {
  try {
    return JSON.parse(localStorage.getItem(LOCAL_KEY) ?? "{}") as Settings;
  } catch {
    return {};
  }
}

export async function loadSettings(): Promise<Settings> {
  if (useLocal()) return readLocal();
  try {
    const s = tauriStore();
    const out: Settings = {};
    for (const k of KEYS) {
      const v = await s.get(k);
      if (v !== undefined && v !== null) (out as Record<string, unknown>)[k] = v;
    }
    return out;
  } catch {
    return {};
  }
}

export function saveSetting<K extends Key>(key: K, value: Settings[K]): void {
  if (useLocal()) {
    try {
      localStorage.setItem(LOCAL_KEY, JSON.stringify({ ...readLocal(), [key]: value }));
    } catch {
      return;
    }
    return;
  }
  tauriStore()
    .set(key, value)
    .catch(() => undefined);
}

export function dirOf(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}
