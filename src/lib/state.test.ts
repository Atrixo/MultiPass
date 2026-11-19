import { describe, expect, it } from "vitest";
import type { FileInfo } from "./api";
import {
  clampSafeZ,
  effectiveMap,
  findInventory,
  forgetTool,
  initialState,
  inventoryConflict,
  mergeDefaults,
  reducer,
  rememberTool,
  sequence,
  type State,
} from "./state";

function info(id: number, desc = "Fraeser", tools = [1]): FileInfo {
  return {
    id,
    name: `${id}.cnc`,
    path: `/x/${id}.cnc`,
    tools: tools.map((n) => ({ number: n, description: `${desc} ${n}` })),
    zmin: -1,
    zorigin: "Stock Top",
    stockTop: true,
    units: "mm",
    incremental: false,
    lines: 10,
  };
}

function withFiles(...ids: number[]): State {
  return reducer(initialState, { type: "add", infos: ids.map((id) => info(id)) });
}

const order = (s: State) => s.entries.map((e) => e.id);

describe("Dateiliste", () => {
  it("haengt an und waehlt die erste Datei", () => {
    const s = withFiles(1, 2);
    expect(order(s)).toEqual([1, 2]);
    expect(s.selected).toBe(1);
    expect(reducer(s, { type: "add", infos: [info(3)] }).selected).toBe(1);
  });

  it("verschiebt mit Knoepfen und Ziehen, Grenzen bleiben stabil", () => {
    let s = withFiles(1, 2, 3);
    s = reducer(s, { type: "move", id: 3, delta: -1 });
    expect(order(s)).toEqual([1, 3, 2]);
    s = reducer(s, { type: "move", id: 1, delta: -1 });
    expect(order(s)).toEqual([1, 3, 2]);
    s = reducer(s, { type: "move", id: 2, delta: 1 });
    expect(order(s)).toEqual([1, 3, 2]);
    s = reducer(s, { type: "reorder", activeId: 2, overId: 1 });
    expect(order(s)).toEqual([2, 1, 3]);
  });

  it("entfernt und waehlt den Nachbarn", () => {
    let s = withFiles(1, 2, 3);
    s = reducer(s, { type: "select", id: 3 });
    s = reducer(s, { type: "remove", id: 3 });
    expect(s.selected).toBe(2);
    s = reducer(s, { type: "remove", id: 1 });
    expect(s.selected).toBe(2);
    s = reducer(s, { type: "clear" });
    expect(s.entries).toEqual([]);
    expect(s.selected).toBeNull();
  });
});

describe("Werkzeugnummern", () => {
  it("manuelle Nummern ueberschreiben die Automatik bis zum Zuruecksetzen", () => {
    let s = withFiles(1, 2);
    s = reducer(s, { type: "setAuto", ids: [1, 2], maps: [{ "1": 1 }, { "1": 2 }] });
    s = reducer(s, { type: "setTool", id: 2, orig: 1, value: 130 });
    expect(effectiveMap(s.entries[1])).toEqual({ "1": 99 });
    s = reducer(s, { type: "setAuto", ids: [2, 1], maps: [{ "1": 1 }, { "1": 2 }] });
    expect(effectiveMap(s.entries[0])).toEqual({ "1": 2 });
    expect(effectiveMap(s.entries[1])).toEqual({ "1": 99 });
    s = reducer(s, { type: "resetTools" });
    expect(effectiveMap(s.entries[1])).toEqual({ "1": 1 });
  });

  it("Ablauf zeigt jeden Wechsel und markiert gleiches Werkzeug", () => {
    let s = reducer(initialState, { type: "add", infos: [info(1, "A", [1, 2]), info(2)] });
    s = reducer(s, { type: "setAuto", ids: [1, 2], maps: [{ "1": 13, "2": 10 }, { "1": 10 }] });
    const steps = sequence(s.entries);
    expect(steps.map((x) => [x.tool, x.position, x.sameAsBefore])).toEqual([
      [13, 1, false],
      [10, 1, false],
      [10, 2, true],
    ]);
  });
});

describe("Sicherheitshoehe", () => {
  it("bleibt im erlaubten Raster", () => {
    expect(clampSafeZ(0)).toBe(0.5);
    expect(clampSafeZ(25)).toBe(20);
    expect(clampSafeZ(1.26)).toBe(1.5);
    let s = reducer(initialState, { type: "setSafeZ", value: 0.5 - 0.5 });
    expect(s.safeZ).toBe(0.5);
    s = reducer(s, { type: "setSafeZ", value: s.safeZ + 0.5 });
    expect(s.safeZ).toBe(1);
  });
});

describe("Inventar", () => {
  const inv = [
    { description: "Spiral O Metal 3.175*12mm Makera", number: 13 },
    { description: "Spiral O Metal 1*3mm Makera", number: 9 },
  ];

  it("findet Werkzeuge unabhaengig von Leerraum und Gross/Klein", () => {
    expect(findInventory(inv, "spiral o  metal 3.175*12mm MAKERA")?.number).toBe(13);
    expect(findInventory(inv, "Spiral O Metal 2mm")).toBeUndefined();
  });

  it("merken ersetzt gleiche Beschreibung und gleiche Nummer", () => {
    let x = rememberTool(inv, "Spiral O Metal 1*3mm Makera", 10);
    expect(x).toEqual([
      { description: "Spiral O Metal 1*3mm Makera", number: 10 },
      { description: "Spiral O Metal 3.175*12mm Makera", number: 13 },
    ]);
    expect(inventoryConflict(x, "Neu", 13)?.number).toBe(13);
    expect(inventoryConflict(x, "spiral o metal 3.175*12mm makera", 13)).toBeUndefined();
    x = rememberTool(x, "Neu", 13);
    expect(x.map((e) => e.description)).toEqual(["Spiral O Metal 1*3mm Makera", "Neu"]);
    x = forgetTool(x, "neu");
    expect(x).toHaveLength(1);
    expect(mergeDefaults(x, inv)).toHaveLength(2);
  });
});
