import { describe, expect, it } from "vitest";
import { currentIndex, nextIndex, stepPoints, toDraft, validateDraft } from "./schedule";

const entries = [
  { time: "06:00", brightness: 70 },
  { time: "18:00", brightness: 80 },
  { time: "23:00", brightness: 0 },
];
const min = (h: number, m = 0) => h * 60 + m;

describe("validateDraft", () => {
  it("valid rows are converted to entries", () => {
    const r = validateDraft([{ time: "06:00", brightness: " 50 " }]);
    expect(r.errors).toEqual({});
    expect(r.entries).toEqual([{ time: "06:00", brightness: 50 }]);
  });

  it.each(["", "abc", "150", "7.5", "-1", "1e2"])("rejects brightness %j", (b) => {
    const r = validateDraft([{ time: "06:00", brightness: b }]);
    expect(r.entries).toBeNull();
    expect(r.errors[0]).toBeDefined();
  });

  it("rejects empty time", () => {
    expect(validateDraft([{ time: "", brightness: "50" }]).errors[0]).toBeDefined();
  });

  it("marks the later duplicate row", () => {
    const r = validateDraft([
      { time: "06:00", brightness: "50" },
      { time: "06:00", brightness: "60" },
    ]);
    expect(Object.keys(r.errors)).toEqual(["1"]);
    expect(r.entries).toBeNull();
  });

  it("accepts an empty schedule", () => {
    expect(validateDraft([])).toEqual({ entries: [], errors: {} });
  });

  it("toDraft round-trips", () => {
    expect(validateDraft(toDraft(entries)).entries).toEqual(entries);
  });
});

describe("currentIndex / nextIndex", () => {
  it("picks the latest entry not after now", () => {
    expect(currentIndex(entries, min(19))).toBe(1);
    expect(currentIndex(entries, min(18))).toBe(1);
  });

  it("before the first entry uses the previous day's last entry", () => {
    expect(currentIndex(entries, min(2))).toBe(2);
  });

  it("next wraps to the first entry", () => {
    expect(nextIndex(entries, min(19))).toBe(2);
    expect(nextIndex(entries, min(23, 30))).toBe(0);
  });

  it("empty schedule has no current or next", () => {
    expect(currentIndex([], min(12))).toBeNull();
    expect(nextIndex([], min(12))).toBeNull();
  });
});

describe("stepPoints", () => {
  it("draws a step line from 0:00 to 24:00 starting at the previous day's level", () => {
    expect(stepPoints(entries)).toEqual([
      { minute: 0, brightness: 0 },
      { minute: 360, brightness: 0 },
      { minute: 360, brightness: 70 },
      { minute: 1080, brightness: 70 },
      { minute: 1080, brightness: 80 },
      { minute: 1380, brightness: 80 },
      { minute: 1380, brightness: 0 },
      { minute: 1440, brightness: 0 },
    ]);
  });

  it("single entry is a flat line all day", () => {
    const pts = stepPoints([{ time: "12:00", brightness: 40 }]);
    expect(pts[0]).toEqual({ minute: 0, brightness: 40 });
    expect(pts[pts.length - 1]).toEqual({ minute: 1440, brightness: 40 });
    expect(pts.every((p) => p.brightness === 40)).toBe(true);
  });

  it("empty schedule has no points", () => {
    expect(stepPoints([])).toEqual([]);
  });
});
