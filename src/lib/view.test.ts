import { describe, expect, it } from "vitest";
import { VIEW_FIT, ZOOM_MAX, apply, pan, zoomAt } from "./view";

describe("Toolpath preview zoom", () => {
  it("the point under the mouse stays put while zooming", () => {
    const v = zoomAt(VIEW_FIT, 2, 100, 50);
    expect(apply(v, 100, 50)).toEqual([100, 50]);
    const w = zoomAt(v, 3, 30, 70);
    const [x, y] = apply(w, ...([(30 - v.tx) / v.k, (70 - v.ty) / v.k] as [number, number]));
    expect(x).toBeCloseTo(30);
    expect(y).toBeCloseTo(70);
  });

  it("limits: not smaller than fit, not larger than maximum", () => {
    expect(zoomAt(VIEW_FIT, 0.5, 10, 10)).toEqual(VIEW_FIT);
    expect(zoomAt(zoomAt(VIEW_FIT, 4, 0, 0), 0.1, 5, 5)).toEqual(VIEW_FIT);
    expect(zoomAt({ k: ZOOM_MAX, tx: 0, ty: 0 }, 2, 0, 0).k).toBe(ZOOM_MAX);
  });

  it("panning only when zoomed in", () => {
    expect(pan(VIEW_FIT, 10, 10)).toEqual(VIEW_FIT);
    const v = pan(zoomAt(VIEW_FIT, 2, 0, 0), 10, -5);
    expect([v.tx, v.ty]).toEqual([10, -5]);
  });
});
