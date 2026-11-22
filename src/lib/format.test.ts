import { describe, expect, it } from "vitest";
import { totalMinutes } from "./api";
import { clampRapidSpeed, formatDuration, toolColor } from "./format";

describe("Run time", () => {
  it("formats minutes and hours", () => {
    expect(formatDuration(0.4)).toBe("< 1 min");
    expect(formatDuration(12.4)).toBe("12 min");
    expect(formatDuration(65)).toBe("1 h 05 min");
    expect(formatDuration(471.2)).toBe("7 h 51 min");
    expect(formatDuration(Number.NaN)).toBe("–");
  });

  it("uses the configured rapid speed", () => {
    const e = { cutMm: 100, cutMin: 10, rapidMm: 3000, moves: 3, unknownFeedMoves: 0 };
    expect(totalMinutes(e, 3000)).toBe(11);
    expect(totalMinutes(e, 0)).toBe(10);
  });

  it("rapid speed stays on the step grid", () => {
    expect(clampRapidSpeed(200)).toBe(1000);
    expect(clampRapidSpeed(3260)).toBe(3500);
    expect(clampRapidSpeed(99999)).toBe(15000);
  });

  it("same tool number same colour, different ones differ", () => {
    const all = [13, 10, 9, 16, 13];
    expect(toolColor(13, all)).toBe(toolColor(13, [9, 10, 13, 16]));
    expect(new Set([9, 10, 13, 16].map((t) => toolColor(t, all))).size).toBe(4);
  });
});
