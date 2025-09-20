use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, PhysicalPosition, Position, WebviewWindow, Window,
};

#[cfg(not(target_os = "linux"))]
use tauri::tray::TrayIconEvent;

trait WindowLike {
    fn hide_window(&self);
    fn set_always_on_top_window(&self, enable: bool);
}

impl WindowLike for WebviewWindow {
    fn hide_window(&self) {
        let _ = self.hide();
    }

    fn set_always_on_top_window(&self, enable: bool) {
        let _ = self.set_always_on_top(enable);
    }
}

impl WindowLike for Window {
    fn hide_window(&self) {
        let _ = self.hide();
    }

    fn set_always_on_top_window(&self, enable: bool) {
        let _ = self.set_always_on_top(enable);
    }
}

/// トレイメニューの Quit 項目で使用する ID
pub const TRAY_QUIT_ID: &str = "quit";
/// トレイメニューのダッシュボード表示用 ID
pub const TRAY_OPEN_ID: &str = "toggle";

struct UsageWindowState {
    visible: AtomicBool,
}

impl Default for UsageWindowState {
    fn default() -> Self {
        Self {
            visible: AtomicBool::new(false),
        }
    }
}

/// 現在の可視状態から次の可視状態を決定（トグル）
pub fn toggled_visible(current: bool) -> bool {
    !current
}

fn show_usage_window(window: &WebviewWindow, usage_state: &UsageWindowState) {
    usage_state.visible.store(true, Ordering::SeqCst);

    #[cfg(target_os = "linux")]
    {
        if let (Ok(size), Ok(Some(monitor))) = (window.outer_size(), window.current_monitor()) {
            let monitor_size = monitor.size();
            let x = monitor_size.width as i32 - size.width as i32 - 24;
            let y = 32;
            let _ = window.set_position(Position::Physical(PhysicalPosition { x, y }));
        }
    }

    let _ = window.set_always_on_top(true);
    let _ = window.show();
    let _ = window.set_focus();
}

fn hide_usage_window<W>(window: &W, usage_state: &UsageWindowState)
where
    W: WindowLike,
{
    usage_state.visible.store(false, Ordering::SeqCst);

    window.set_always_on_top_window(false);
    window.hide_window();
}

fn toggle_main_window(app: &tauri::AppHandle) {
    let usage_state = app.state::<UsageWindowState>();
    if let Some(window) = app.get_webview_window("main") {
        let currently_visible = usage_state.visible.load(Ordering::SeqCst);
        if toggled_visible(currently_visible) {
            show_usage_window(&window, &usage_state);
        } else {
            hide_usage_window(&window, &usage_state);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(UsageWindowState::default());

            // 明示的にトレイアイコンを設定（macOS では必須）。
            let tray_icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("failed to load tray icon");
            let toggle_item =
                MenuItem::with_id(app, TRAY_OPEN_ID, "Open Usage", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;
            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("Time Wise")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    TRAY_QUIT_ID => app.exit(0),
                    TRAY_OPEN_ID => toggle_main_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    #[cfg(target_os = "linux")]
                    {
                        let _ = (tray, event);
                    }

                    #[cfg(not(target_os = "linux"))]
                    {
                        if let TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            position,
                            rect,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            let usage_state = app.state::<UsageWindowState>();
                            if let Some(window) = app.get_webview_window("main") {
                                if toggled_visible(usage_state.visible.load(Ordering::SeqCst)) {
                                    #[cfg(not(target_os = "macos"))]
                                    {
                                        if let Ok(size) = window.outer_size() {
                                            let monitor_height = window
                                                .current_monitor()
                                                .ok()
                                                .flatten()
                                                .map(|monitor| monitor.size().height as f64)
                                                .unwrap_or(position.y * 2.0);
                                            let x = position.x - (size.width as f64 / 2.0);
                                            let y = if position.y > monitor_height / 2.0 {
                                                position.y - size.height as f64 - 12.0
                                            } else {
                                                position.y + rect.size.height + 12.0
                                            };
                                            let _ = window.set_position(Position::Physical(
                                                PhysicalPosition {
                                                    x: x.round() as i32,
                                                    y: y.round() as i32,
                                                },
                                            ));
                                        }
                                    }

                                    show_usage_window(&window, &usage_state);
                                } else {
                                    hide_usage_window(&window, &usage_state);
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    let _ = window.set_skip_taskbar(true);
                }

                #[cfg(not(target_os = "macos"))]
                {
                    let _ = window.set_skip_taskbar(false);
                    let _ = window.hide();
                }

                app.state::<UsageWindowState>()
                    .visible
                    .store(false, Ordering::SeqCst);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let usage_state = window.app_handle().state::<UsageWindowState>();
                    hide_usage_window(window, &usage_state);
                }
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_visible_should_invert() {
        assert!(toggled_visible(false));
        assert!(!toggled_visible(true));
    }

    #[test]
    fn tray_quit_id_constant() {
        assert_eq!(TRAY_QUIT_ID, "quit");
    }

    #[test]
    fn tray_open_id_constant() {
        assert_eq!(TRAY_OPEN_ID, "toggle");
    }
}
