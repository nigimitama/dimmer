//! トレイアイコンとメインウィンドウの表示切り替え

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Theme};

const TRAY_ID: &str = "main-tray";
const ICON_DARK: &[u8] = include_bytes!("../../assets/icon-dark.png");
const ICON_LIGHT: &[u8] = include_bytes!("../../assets/icon-light.png");

fn icon_for(theme: Theme) -> Image<'static> {
    let bytes = match theme {
        Theme::Dark => ICON_DARK,
        _ => ICON_LIGHT,
    };
    Image::from_bytes(bytes).expect("tray icon must be a valid PNG")
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "表示/非表示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
    let theme = app
        .get_webview_window("main")
        .and_then(|w| w.theme().ok())
        .unwrap_or(Theme::Light);

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon_for(theme))
        .tooltip("Dimmer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn set_theme(app: &AppHandle, theme: Theme) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_icon(Some(icon_for(theme)));
    }
}
