//! React から呼ぶ Tauri command

use crate::monitor::{apply_all, apply_one, read_all, ApplyResult, MonitorInfo};
use crate::schedule::{default_schedule, format_errors, normalize, ScheduleEntry};
use crate::settings::Settings;
use crate::state::AppState;
use chrono::Local;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

/// DDC 通信は 1台あたり数十ms かかるので、UI スレッドを塞がないよう別スレッドで実行する
async fn blocking<T, F>(app: AppHandle, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> T + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_monitors(app: AppHandle) -> Result<Vec<MonitorInfo>, String> {
    blocking(app, |s| read_all(s.backend.as_ref())).await
}

#[tauri::command]
pub async fn set_brightness_all(app: AppHandle, value: u8) -> Result<ApplyResult, String> {
    blocking(app, move |s| apply_all(s.backend.as_ref(), value.min(100))).await
}

#[tauri::command]
pub async fn set_brightness(app: AppHandle, id: String, value: u8) -> Result<ApplyResult, String> {
    blocking(app, move |s| apply_one(s.backend.as_ref(), &id, value.min(100))).await
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

fn store_schedule(state: &AppState, entries: Vec<ScheduleEntry>) -> Result<Settings, String> {
    let schedule = normalize(&entries).map_err(|errors| format_errors(&errors))?;
    let settings = state
        .settings
        .update(|s| s.schedule = schedule)
        .map_err(|e| format!("設定を保存できません: {e}"))?;
    state
        .scheduler
        .lock()
        .unwrap()
        .mark_current_applied(&settings.schedule, Local::now().naive_local());
    Ok(settings)
}

// save_schedule と reset_schedule は DDC を伴わないので同期 command にする。
// UI スレッドで順番に処理されるため、保存の順序が入れ替わらない
#[tauri::command]
pub fn save_schedule(state: State<'_, AppState>, entries: Vec<ScheduleEntry>) -> Result<Settings, String> {
    store_schedule(&state, entries)
}

#[tauri::command]
pub fn reset_schedule(state: State<'_, AppState>) -> Result<Settings, String> {
    store_schedule(&state, default_schedule())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<Settings, String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("自動起動の設定を変更できません: {e}"))?;
    state
        .settings
        .update(|s| s.autostart = enabled)
        .map_err(|e| format!("設定を保存できません: {e}"))
}
