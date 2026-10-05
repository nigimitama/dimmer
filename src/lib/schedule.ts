import type { ScheduleEntry } from "../api";

/** A row being edited. Brightness is also kept as a string to preserve in-progress input */
export type DraftEntry = { time: string; brightness: string };
export type RowErrors = Record<number, string>;
export type Point = { minute: number; brightness: number };

const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;
const INT_RE = /^\d{1,3}$/;

export function validateDraft(rows: DraftEntry[]): { entries: ScheduleEntry[] | null; errors: RowErrors } {
  const errors: RowErrors = {};
  const seen = new Set<string>();
  rows.forEach((row, i) => {
    const brightness = row.brightness.trim();
    if (!TIME_RE.test(row.time)) {
      errors[i] = "Enter a time";
    } else if (!INT_RE.test(brightness) || Number(brightness) > 100) {
      errors[i] = "Brightness must be an integer from 0 to 100";
    } else if (seen.has(row.time)) {
      errors[i] = `${row.time} is duplicated`;
    } else {
      seen.add(row.time);
    }
  });
  if (Object.keys(errors).length > 0) return { entries: null, errors };
  return { entries: rows.map((r) => ({ time: r.time, brightness: Number(r.brightness.trim()) })), errors };
}

export const toDraft = (entries: ScheduleEntry[]): DraftEntry[] =>
  entries.map((e) => ({ time: e.time, brightness: String(e.brightness) }));

export const minutesOf = (time: string): number => Number(time.slice(0, 2)) * 60 + Number(time.slice(3, 5));

/** Index of the entry for the current slot, or the previous day's last entry if none of today's entries has started yet */
export function currentIndex(entries: ScheduleEntry[], nowMinutes: number): number | null {
  if (entries.length === 0) return null;
  let index = entries.length - 1;
  entries.forEach((e, i) => {
    if (minutesOf(e.time) <= nowMinutes) index = i;
  });
  return index;
}

export function nextIndex(entries: ScheduleEntry[], nowMinutes: number): number | null {
  if (entries.length === 0) return null;
  const i = entries.findIndex((e) => minutesOf(e.time) > nowMinutes);
  return i === -1 ? 0 : i;
}

/** Vertices of the 0:00–24:00 step graph. The value at 0:00 is the previous day's last entry */
export function stepPoints(entries: ScheduleEntry[]): Point[] {
  if (entries.length === 0) return [];
  let level = entries[entries.length - 1].brightness;
  const points: Point[] = [{ minute: 0, brightness: level }];
  for (const e of entries) {
    const minute = minutesOf(e.time);
    points.push({ minute, brightness: level }, { minute, brightness: e.brightness });
    level = e.brightness;
  }
  points.push({ minute: 1440, brightness: level });
  return points;
}
