import { describe, expect, it } from "vitest";
import {
  STRIDE,
  cumulativeTimes,
  formatClock,
  heightAt,
  makeGrid,
  parseTimeline,
  partAt,
  positionAt,
  profile,
  shade,
  stampRange,
  stampSegment,
  vertexAt,
  type TimelineMeta,
  type ToolShape,
} from "./sim";

const flat: ToolShape = { diameter: 2, tip: 2, corner: 0, taperDeg: 0 };
const vee: ToolShape = { diameter: 3.175, tip: 0.3, corner: 0, taperDeg: 15 };
const ball: ToolShape = { diameter: 1.5, tip: 0, corner: 0.75, taperDeg: 0 };

function path(points: [number, number, number, number, number][]): Float32Array {
  return new Float32Array(points.flat());
}

function encode(meta: TimelineMeta, data: Float32Array): ArrayBuffer {
  const json = new TextEncoder().encode(JSON.stringify(meta));
  const offset = Math.ceil((4 + json.length) / 4) * 4;
  const buf = new ArrayBuffer(offset + data.byteLength);
  new DataView(buf).setUint32(0, json.length, true);
  new Uint8Array(buf, 4, json.length).set(json);
  new Float32Array(buf, offset).set(data);
  return buf;
}

describe("Zeitleiste", () => {
  const data = path([
    [0, 0, 5, NaN, 1],
    [10, 0, 5, -10, 1],
    [10, 0, -1, 0.1, 0],
    [20, 0, -1, 0.2, 0],
  ]);

  it("liest Kopf und Daten", () => {
    const meta: TimelineMeta = { parts: [{ name: "a", start: 0, end: 4 }], tools: [], bounds: null, material: true, stockHeight: 5, zmin: -1, vertices: 4 };
    const tl = parseTimeline(encode(meta, data));
    expect(tl.meta.parts[0].name).toBe("a");
    expect(Array.from(tl.data.subarray(STRIDE, STRIDE * 2))).toEqual([10, 0, 5, -10, 1]);
  });

  it("rechnet Zeit mit Eilgang-Tempo und findet die Position", () => {
    const t = cumulativeTimes(data, 1000);
    expect(t[0]).toBe(0);
    expect(t[1]).toBeCloseTo(0.01);
    expect(t[3]).toBeCloseTo(0.31);
    expect(vertexAt(t, 0)).toBe(0);
    expect(vertexAt(t, 0.2)).toBe(2);
    expect(vertexAt(t, 99)).toBe(3);
    const p = positionAt(data, t, 0.21);
    expect(p[0]).toBeCloseTo(15, 3);
    expect(p[2]).toBeCloseTo(-1);
    expect(partAt([{ name: "a", start: 0, end: 2 }, { name: "b", start: 2, end: 4 }], 3)).toBe(1);
  });

  it("zeigt Uhrzeit", () => {
    expect(formatClock(0.5)).toBe("0:30");
    expect(formatClock(75.25)).toBe("1:15:15");
    expect(formatClock(NaN)).toBe("–");
  });
});

describe("Werkzeugformen", () => {
  it("Schaft, Stichel und Kugel", () => {
    expect(heightAt(flat, 0.9)).toBe(0);
    expect(heightAt(flat, 1.1)).toBe(Infinity);
    expect(heightAt(vee, 0.1)).toBe(0);
    expect(heightAt(vee, 0.15 + Math.tan((15 * Math.PI) / 180))).toBeCloseTo(1);
    expect(heightAt(ball, 0)).toBe(0);
    expect(heightAt(ball, 0.75)).toBeCloseTo(0.75);
    expect(profile(flat).flat).toBe(true);
    expect(profile(vee).flat).toBe(false);
  });
});

describe("Materialabtrag", () => {
  it("Schaftfräser fräst eine Nut in voller Breite und Tiefe", () => {
    const g = makeGrid([0, -1, 10, 1], 200_000);
    stampSegment(g, profile(flat), 0, 0, -1, 10, 0, -1);
    const at = (x: number, y: number) => g.z[Math.floor((y - g.y0) / g.cell) * g.w + Math.floor((x - g.x0) / g.cell)];
    expect(at(5, 0)).toBeCloseTo(-1);
    expect(at(5, 0.9)).toBeCloseTo(-1);
    expect(at(5, 1.1)).toBe(0);
    expect(at(-0.9, 0)).toBeCloseTo(-1);
    expect(at(11.1, 0)).toBe(0);
  });

  it("Stichel macht eine V-Nut", () => {
    const g = makeGrid([0, -2, 10, 2], 200_000);
    stampSegment(g, profile(vee), 0, 0, -1, 10, 0, -1);
    const at = (y: number) => g.z[Math.floor((y - g.y0) / g.cell) * g.w + Math.floor((5 - g.x0) / g.cell)];
    expect(at(0)).toBeCloseTo(-1, 1);
    const half = 0.15 + Math.tan((15 * Math.PI) / 180);
    expect(at(half * 0.6)).toBeLessThan(-0.2);
    expect(at(half + 0.1)).toBe(0);
  });

  it("Eilgänge und Bewegungen über dem Material tragen nichts ab", () => {
    const data = path([
      [0, 0, 5, NaN, 1],
      [5, 0, -1, -10, 1],
      [10, 0, 1, -10, 1],
      [10, 5, 1, 0.1, 0],
    ]);
    const g = makeGrid([0, 0, 10, 5], 50_000);
    stampRange(g, [profile(flat)], data, 0, 4);
    expect(g.z.every((v) => v === 0)).toBe(true);
  });

  it("schneller Weg für gleiche Höhe liefert dasselbe wie die genaue Rechnung", () => {
    const pts: [number, number, number, number, number][] = [[0, 0, 5, NaN, 0]];
    let x = 0;
    let y = 0;
    let seed = 7;
    const rnd = () => ((seed = (seed * 16807) % 2147483647) / 2147483647) * 2 - 1;
    for (let i = 0; i < 1500; i++) {
      x = Math.max(-5, Math.min(5, x + rnd() * 0.4));
      y = Math.max(-5, Math.min(5, y + rnd() * 0.4));
      const z = i < 500 ? -0.5 : i < 1000 ? -1.25 : -1.25 + (i - 1000) * -0.001;
      pts.push([x, y, z, 0.01, i % 97 === 0 ? 1 : 0]);
    }
    const data = path(pts);
    const fast = makeGrid([-6, -6, 6, 6], 120_000);
    const exact = makeGrid([-6, -6, 6, 6], 120_000);
    const prof = profile(flat);
    stampRange(fast, [prof], data, 0, 700);
    stampRange(fast, [prof], data, 700, pts.length);
    for (let i = 1; i < pts.length; i++) {
      const o = i * STRIDE;
      if (Number.isNaN(data[o + 3]) || data[o + 4] % 2 === 1) continue;
      stampSegment(exact, prof, data[o - 5], data[o - 4], data[o - 3], data[o], data[o + 1], data[o + 2]);
    }
    let diff = 0;
    for (let i = 0; i < fast.z.length; i++) if (Math.abs(fast.z[i] - exact.z[i]) > 1e-6) diff++;
    expect(diff).toBe(0);
    expect(Math.min(...fast.z)).toBeLessThan(-1.25);
  });

  it("Durchbruch wird durchsichtig, Material bleibt sichtbar", () => {
    const g = makeGrid([0, 0, 4, 4], 10_000);
    g.z.fill(0);
    g.z[0] = -5;
    g.z[1] = -2;
    const rgba = new Uint8ClampedArray(g.w * g.h * 4);
    shade(g, rgba, 5, 5);
    const o = (g.h - 1) * g.w * 4;
    expect(rgba[o + 3]).toBe(0);
    expect(rgba[o + 7]).toBe(255);
    expect(rgba[3]).toBe(255);
  });
});
