import type { ScheduleEntry } from "../api";

/** 編集中の行。入力途中の値を保持するため、輝度も文字列で持つ */
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
      errors[i] = "時刻を入力してください";
    } else if (!INT_RE.test(brightness) || Number(brightness) > 100) {
      errors[i] = "輝度は 0〜100 の整数で入力してください";
    } else if (seen.has(row.time)) {
      errors[i] = `${row.time} が重複しています`;
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

/** 現在のスロットのエントリの index。今日のエントリがまだなければ前日の最後のエントリ */
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

/** 0:00〜24:00 の階段グラフの頂点。0:00 時点は前日の最後のエントリの値 */
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
