use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// トレイメニューの Quit 項目で使用する ID
pub const TRAY_QUIT_ID: &str = "quit";

/// 現在の可視状態から次の可視状態を決定（トグル）
pub fn toggled_visible(current: bool) -> bool {
    !current
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("Time Wise")
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == TRAY_QUIT_ID {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let now_visible = window.is_visible().unwrap_or(false);
                            if toggled_visible(now_visible) {
                                let _ = window.show();
                                let _ = window.set_focus();
                            } else {
                                let _ = window.hide();
                            }
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
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
}
