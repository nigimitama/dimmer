pub mod commands;
pub mod monitor;
pub mod schedule;
pub mod scheduler;
pub mod settings;
pub mod state;
pub mod tray;

use monitor::win32::WinBackend;
use scheduler::SchedulerState;
use settings::SettingsStore;
use state::AppState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

pub const AUTOSTART_FLAG: &str = "--autostart";

/// 設定と OS の自動起動の登録を一致させる。
/// 有効なら毎回登録し直し、実行ファイルの場所が変わっても（dev ビルド → インストール版など）正しいパスになるようにする
fn sync_autostart(app: &AppHandle, enabled: bool) {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else if launcher.is_enabled().unwrap_or(false) {
        launcher.disable()
    } else {
        Ok(())
    };
    if let Err(e) = result {
        log::warn!("failed to sync autostart: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance は最初に登録する必要がある
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main_window(app)))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![AUTOSTART_FLAG])))
        .invoke_handler(tauri::generate_handler![
            commands::get_monitors,
            commands::set_brightness_all,
            commands::set_brightness,
            commands::get_settings,
            commands::save_schedule,
            commands::reset_schedule,
            commands::set_autostart,
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let (settings, outcome) = SettingsStore::open(path.clone())?;
            log::info!("settings {:?}: {}", outcome, path.display());
            sync_autostart(app.handle(), settings.get().autostart);
            app.manage(AppState {
                backend: Arc::new(WinBackend::new()),
                settings,
                scheduler: Mutex::new(SchedulerState::default()),
            });
            scheduler::spawn(app.handle().clone());
            tray::create(app.handle())?;
            if !std::env::args().any(|arg| arg == AUTOSTART_FLAG) {
                tray::show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // × ボタンでは終了せず、トレイに隠す
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::ThemeChanged(theme) => tray::set_theme(window.app_handle(), *theme),
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
