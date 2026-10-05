import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Monitor = { id: string; name: string; brightness: number | null };
export type ApplyFailure = { id: string; name: string; error: string };
export type ApplyResult = { attempted: number; failed: ApplyFailure[] };
export type ScheduleEntry = { time: string; brightness: number };
export type Settings = { version: number; autostart: boolean; schedule: ScheduleEntry[] };

export const api = {
  getMonitors: () => invoke<Monitor[]>("get_monitors"),
  setBrightnessAll: (value: number) => invoke<ApplyResult>("set_brightness_all", { value }),
  setBrightness: (id: string, value: number) => invoke<ApplyResult>("set_brightness", { id, value }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSchedule: (entries: ScheduleEntry[]) => invoke<Settings>("save_schedule", { entries }),
  resetSchedule: () => invoke<Settings>("reset_schedule"),
  setAutostart: (enabled: boolean) => invoke<Settings>("set_autostart", { enabled }),
  onMonitorsUpdated: (cb: (monitors: Monitor[]) => void): Promise<UnlistenFn> =>
    listen<Monitor[]>("monitors-updated", (event) => cb(event.payload)),
};
