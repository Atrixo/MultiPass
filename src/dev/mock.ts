import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { FileInfo, Prepared, Saved } from "../lib/api";

const samples: Omit<FileInfo, "id" | "path">[] = [
  { name: "1_T13_Schruppen.cnc", tools: [{ number: 1, description: "Spiral O Metal 3.175*12mm Makera" }], zmin: -2.55, zorigin: "Stock Top", stockTop: true, units: "mm", incremental: false, lines: 101234 },
  { name: "2_T10_Rest.cnc", tools: [{ number: 1, description: "Spiral O Metal 1.5*6mm Makera" }], zmin: -2.55, zorigin: "Stock Top", stockTop: true, units: "mm", incremental: false, lines: 133000 },
  { name: "3_T9_Rest_Boeden.cnc", tools: [{ number: 1, description: "Spiral O Metal 1*3mm Makera" }], zmin: -2.6, zorigin: "Stock Top", stockTop: true, units: "mm", incremental: false, lines: 365000 },
  { name: "4_T16_Gravur.cnc", tools: [{ number: 1, description: "Single Flute Engraving Metal 30 deg*.3mm" }], zmin: -2.1, zorigin: "Stock Top", stockTop: true, units: "mm", incremental: false, lines: 234 },
  { name: "5_T13_Kontur.cnc", tools: [{ number: 1, description: "Spiral O Metal 3.175*12mm Makera" }], zmin: -5, zorigin: "Model Bottom", stockTop: false, units: "mm", incremental: false, lines: 246 },
];

let nextId = 0;
const loaded = new Map<number, FileInfo>();

export function installMock() {
  (window as unknown as Record<string, unknown>).__MULTIPASS_MOCK__ = true;
  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "initial_files":
          return [];
        case "default_inventory":
          return [
            { description: "Spiral O Metal 1*3mm Makera", number: 9 },
            { description: "Spiral O Metal 1.5*6mm Makera", number: 10 },
            { description: "Spiral O Metal 3.175*12mm Makera", number: 13 },
            { description: "Single Flute Engraving Metal 30 deg*.3mm", number: 16 },
          ];
        case "plugin:opener|reveal_item_in_dir":
          return null;
        case "plugin:dialog|open":
          return samples.map((s) => `C:/Fraesen/${s.name}`);
        case "plugin:dialog|save":
          return "C:/Fraesen/GESAMT.cnc";
        case "plugin:path|join":
          return (args.paths as string[]).join("/");
        case "load_files": {
          const files = (args.paths as string[]).map((p) => {
            const name = p.split("/").pop() ?? p;
            const s = samples.find((x) => x.name === name) ?? samples[0];
            const info: FileInfo = { ...s, id: ++nextId, path: p };
            loaded.set(info.id, info);
            return info;
          });
          return { files, errors: [{ name: "kaputt.cnc", path: "C:/Fraesen/kaputt.cnc", message: "", error: { code: "noProgramEnd", file: "kaputt.cnc" } }] };
        }
        case "forget_files":
          return null;
        case "auto_tools": {
          const inv = (args.inventory as { description: string; number: number }[]) ?? [];
          const norm = (d: string) => d.trim().split(/\s+/).join(" ").toLowerCase();
          const taken = new Set(inv.map((e) => e.number));
          const by = new Map<string, number>();
          return (args.ids as number[]).map((id) => {
            const m: Record<string, number> = {};
            for (const t of loaded.get(id)?.tools ?? []) {
              if (!by.has(t.description)) {
                let n = inv.find((e) => norm(e.description) === norm(t.description))?.number;
                if (n === undefined) {
                  n = 1;
                  while (taken.has(n)) n++;
                }
                taken.add(n);
                by.set(t.description, n);
              }
              m[String(t.number)] = by.get(t.description)!;
            }
            return m;
          });
        }
        case "prepare_merge": {
          const p: Prepared = {
            warnings: [
              { code: "zOriginDiffers", origins: ["Model Bottom", "Stock Top"] },
              { code: "rapidOffZOrigin", files: ["5_T13_Kontur.cnc"] },
            ],
            warningTexts: [],
            rapid: null,
            suggestedDir: "C:/Fraesen",
            suggestedName: "GESAMT.cnc",
            lines: 600000,
          };
          if (!args.restore) p.warnings = p.warnings.slice(0, 1);
          return p;
        }
        case "save_merge": {
          const s: Saved = { path: String(args.path), dir: "C:/Fraesen", bytes: 1234 };
          return s;
        }
      }
      return null;
    },
    { shouldMockEvents: true },
  );
}
