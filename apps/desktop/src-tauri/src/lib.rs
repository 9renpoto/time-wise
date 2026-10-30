mod app_identity;
mod platform;
pub mod secure_storage;
pub mod usage_history;
mod usage_queries;
mod usage_recorder;

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
    Manager, RunEvent, State, WebviewUrl, WebviewWindow, Window,
};
use usage_history::UsageHistoryStore;
use usage_queries::{fetch_daily_usage_summary, fetch_weekly_usage_summary};
use usage_recorder::{MeasurementHealth, UsageRecorder, CHECKPOINT_INTERVAL_SECONDS};

#[cfg(not(target_os = "linux"))]
use tauri::tray::TrayIconEvent;
#[cfg(target_os = "linux")]
use tauri::{PhysicalPosition, Position as TauriPosition};
use tauri_plugin_autostart::AutoLaunchManager;
#[cfg(target_os = "macos")]
use tauri_plugin_autostart::MacosLauncher;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_positioner::{Position, WindowExt};

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
/// 設定画面表示用 ID
pub const TRAY_SETTINGS_ID: &str = "settings";
const AUTOSTART_SETTING: &str = "autostart";
const AUTOSTART_APP_NAME: &str = "time-wise";
const ONBOARDING_COMPLETED_SETTING: &str = "onboarding_completed";

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

    // Linux tray implementations do not reliably emit click coordinates, so
    // retain the existing top-right fallback for menu-triggered display.
    #[cfg(target_os = "linux")]
    {
        if let (Ok(size), Ok(Some(monitor))) = (window.outer_size(), window.current_monitor()) {
            let monitor_size = monitor.size();
            let x = monitor_size.width as i32 - size.width as i32 - 24;
            let y = 32;
            let _ = window.set_position(TauriPosition::Physical(PhysicalPosition { x, y }));
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

fn show_settings_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("/?view=settings".into()),
    )
    .title("Time Wise Settings")
    .inner_size(520.0, 650.0)
    .resizable(false)
    .skip_taskbar(false)
    .visible(true)
    .build();
}

#[tauri::command]
async fn get_autostart_enabled(autostart: State<'_, AutoLaunchManager>) -> Result<bool, String> {
    autostart.is_enabled().map_err(|err| err.to_string())
}

#[tauri::command]
async fn set_autostart_enabled(
    autostart: State<'_, AutoLaunchManager>,
    history: State<'_, Arc<UsageHistoryStore>>,
    enabled: bool,
) -> Result<bool, String> {
    let enabled = update_autostart(&autostart, enabled)?;
    history.set_setting(AUTOSTART_SETTING, &enabled.to_string(), current_utc_ms())?;
    Ok(enabled)
}

#[tauri::command]
fn get_onboarding_completed(history: State<'_, Arc<UsageHistoryStore>>) -> Result<bool, String> {
    history
        .setting(ONBOARDING_COMPLETED_SETTING)
        .map(|value| value.as_deref() == Some("true"))
}

#[tauri::command]
fn complete_onboarding(
    autostart: State<'_, AutoLaunchManager>,
    history: State<'_, Arc<UsageHistoryStore>>,
    enable_autostart: bool,
) -> Result<(), String> {
    let enabled = update_autostart(&autostart, enable_autostart)?;
    let now = current_utc_ms();
    history.set_setting(AUTOSTART_SETTING, &enabled.to_string(), now)?;
    history.set_setting(ONBOARDING_COMPLETED_SETTING, "true", now)
}

#[tauri::command]
fn delete_all_usage_history(recorder: State<'_, Arc<UsageRecorder>>) -> Result<(), String> {
    recorder.delete_history(SystemTime::now())
}

#[tauri::command]
fn fetch_measurement_health(recorder: State<'_, Arc<UsageRecorder>>) -> MeasurementHealth {
    recorder.health()
}

trait AutostartControl {
    fn enable_autostart(&self) -> Result<(), String>;
    fn disable_autostart(&self) -> Result<(), String>;
    fn autostart_is_enabled(&self) -> Result<bool, String>;
}

impl AutostartControl for AutoLaunchManager {
    fn enable_autostart(&self) -> Result<(), String> {
        self.enable().map_err(|err| err.to_string())
    }

    fn disable_autostart(&self) -> Result<(), String> {
        self.disable().map_err(|err| err.to_string())
    }

    fn autostart_is_enabled(&self) -> Result<bool, String> {
        self.is_enabled().map_err(|err| err.to_string())
    }
}

fn update_autostart(autostart: &AutoLaunchManager, enabled: bool) -> Result<bool, String> {
    update_autostart_for_mode(autostart, enabled, tauri::is_dev())
}

fn update_autostart_for_mode(
    autostart: &impl AutostartControl,
    enabled: bool,
    dev_mode: bool,
) -> Result<bool, String> {
    // A Tauri dev binary loads its UI from devUrl. Registering that binary with
    // the OS leaves a blank window after login because the dev server is absent.
    let enabled = enabled && !dev_mode;
    if enabled {
        autostart.enable_autostart()?;
    } else {
        autostart.disable_autostart()?;
    }

    autostart.autostart_is_enabled()
}

fn current_utc_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn configure_positioner<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.plugin(tauri_plugin_positioner::init())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Register this first so duplicate launches exit before initializing
    // autostart, windows, tray icons, or measurement workers.
    let builder = tauri::Builder::default().plugin(tauri_plugin_single_instance::init(
        |_app, _args, _working_directory| {},
    ));
    let builder = configure_positioner(builder)
        .plugin(tauri_plugin_opener::init())
        .plugin({
            let builder = tauri_plugin_autostart::Builder::new().app_name(AUTOSTART_APP_NAME);
            #[cfg(target_os = "macos")]
            let builder = builder.macos_launcher(MacosLauncher::LaunchAgent);
            builder.build()
        })
        .invoke_handler(tauri::generate_handler![
            complete_onboarding,
            delete_all_usage_history,
            fetch_daily_usage_summary,
            fetch_measurement_health,
            fetch_weekly_usage_summary,
            get_autostart_enabled,
            get_onboarding_completed,
            set_autostart_enabled
        ])
        .setup(|app| {
            app.manage(UsageWindowState::default());
            let onboarding_required;

            // Remove registrations created by older dev builds. Packaged builds
            // keep the user's configured automatic-launch preference.
            if tauri::is_dev() {
                if let Err(err) = update_autostart(&app.state::<AutoLaunchManager>(), false) {
                    eprintln!("failed to remove development autostart registration: {err}");
                }
            }

            let usage_history_path = app
                .path()
                .resolve("usage_history.sqlite", BaseDirectory::AppLocalData)
                .unwrap_or_else(|err| {
                    eprintln!("failed to resolve usage history path: {err}");
                    env::temp_dir().join("time-wise-usage-history.sqlite")
                });
            match UsageHistoryStore::with_storage_path(usage_history_path) {
                Ok(store) => {
                    onboarding_required = store
                        .setting(ONBOARDING_COMPLETED_SETTING)
                        .map(|value| value.as_deref() != Some("true"))
                        .unwrap_or(true);
                    let store = Arc::new(store);
                    let recorder = Arc::new(UsageRecorder::new(store.clone()));
                    app.manage(store);

                    match platform::start_event_probe() {
                        Ok((probe, event_receiver)) => {
                            let event_recorder = recorder.clone();
                            tauri::async_runtime::spawn_blocking(move || {
                                while let Ok(event) = event_receiver.recv() {
                                    event_recorder.handle_event(event);
                                }
                                event_recorder.record_subscription_ended();
                            });
                            app.manage(probe);
                        }
                        Err(err) => {
                            recorder.record_subscription_failure(err.clone());
                            eprintln!("failed to start desktop event probe: {err}");
                        }
                    }

                    let checkpoint_recorder = recorder.clone();
                    tauri::async_runtime::spawn(async move {
                        loop {
                            tokio::time::sleep(Duration::from_secs(CHECKPOINT_INTERVAL_SECONDS))
                                .await;
                            checkpoint_recorder.checkpoint(SystemTime::now());
                        }
                    });
                    app.manage(recorder);
                }
                Err(err) => {
                    return Err(std::io::Error::other(format!(
                        "failed to initialize usage history database: {err}"
                    ))
                    .into());
                }
            }

            tauri::WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::App("/?view=settings".into()),
            )
            .title("Time Wise Settings")
            .inner_size(520.0, 650.0)
            .resizable(false)
            .visible(false)
            .skip_taskbar(false)
            .build()?;

            // 明示的にトレイアイコンを設定（macOS では必須）。
            let tray_icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("failed to load tray icon");
            let usage_item =
                MenuItem::with_id(app, TRAY_OPEN_ID, "Open Usage", true, None::<&str>)?;
            let containers_label = MenuItem::new(app, "Containers", false, None::<&str>)?;
            // Placeholder desktop apps shown under Containers until runtime data is wired up.
            let desktop_app_primary =
                MenuItem::new(app, "Desktop App Aurora", false, None::<&str>)?;
            let desktop_app_secondary =
                MenuItem::new(app, "Desktop App Nimbus", false, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, TRAY_SETTINGS_ID, "Settings...", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, TRAY_QUIT_ID, "Quit", true, None::<&str>)?;
            let menu = MenuBuilder::new(app)
                .item(&usage_item)
                .separator()
                .item(&containers_label)
                .item(&desktop_app_primary)
                .item(&desktop_app_secondary)
                .separator()
                .item(&settings_item)
                .item(&quit_item)
                .build()?;
            TrayIconBuilder::new()
                .icon(tray_icon)
                .icon_as_template(true)
                .menu(&menu)
                .tooltip("Time Wise")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    TRAY_QUIT_ID => app.exit(0),
                    TRAY_OPEN_ID => toggle_main_window(app),
                    TRAY_SETTINGS_ID => show_settings_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                    #[cfg(target_os = "linux")]
                    {
                        let _ = (tray, event);
                    }

                    #[cfg(not(target_os = "linux"))]
                    {
                        if let TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            let usage_state = app.state::<UsageWindowState>();
                            if let Some(window) = app.get_webview_window("main") {
                                if toggled_visible(usage_state.visible.load(Ordering::SeqCst)) {
                                    let _ = window
                                        .as_ref()
                                        .window()
                                        .move_window_constrained(Position::TrayCenter);

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
                if onboarding_required {
                    let _ = window.set_skip_taskbar(false);
                    let _ = window.show();
                    let _ = window.set_focus();
                    app.state::<UsageWindowState>()
                        .visible
                        .store(true, Ordering::SeqCst);
                } else {
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
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                match window.label() {
                    "main" => {
                        let usage_state = window.app_handle().state::<UsageWindowState>();
                        hide_usage_window(window, &usage_state);
                        api.prevent_close();
                    }
                    "settings" => {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                    _ => {}
                }
            }
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(recorder) = app_handle.try_state::<Arc<UsageRecorder>>() {
                recorder.stop(SystemTime::now());
                let diagnostics = recorder.diagnostics();
                if diagnostics != Default::default() {
                    eprintln!("usage recorder diagnostics at exit: {diagnostics:?}");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::{atomic::Ordering, Mutex};

    #[test]
    fn positioner_handles_tray_events_after_initialization() {
        use tauri::{
            test::{mock_builder, mock_context, noop_assets},
            tray::TrayIconEvent,
            PhysicalPosition, PhysicalSize, Rect,
        };

        let app = configure_positioner(mock_builder())
            .build(mock_context(noop_assets()))
            .unwrap();
        let event = TrayIconEvent::Enter {
            id: "test".into(),
            position: PhysicalPosition::new(10.0, 20.0),
            rect: Rect {
                position: PhysicalPosition::new(10.0, 20.0).into(),
                size: PhysicalSize::new(16.0, 16.0).into(),
            },
        };
        tauri_plugin_positioner::on_tray_event(app.handle(), &event);
    }

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

    #[test]
    fn usage_window_state_defaults_to_hidden() {
        let state = UsageWindowState::default();
        assert!(!state.visible.load(Ordering::SeqCst));
    }

    #[derive(Default)]
    struct MockAutostart {
        enabled: Cell<bool>,
        enable_calls: Cell<u32>,
        disable_calls: Cell<u32>,
    }

    impl AutostartControl for MockAutostart {
        fn enable_autostart(&self) -> Result<(), String> {
            self.enable_calls.set(self.enable_calls.get() + 1);
            self.enabled.set(true);
            Ok(())
        }

        fn disable_autostart(&self) -> Result<(), String> {
            self.disable_calls.set(self.disable_calls.get() + 1);
            self.enabled.set(false);
            Ok(())
        }

        fn autostart_is_enabled(&self) -> Result<bool, String> {
            Ok(self.enabled.get())
        }
    }

    #[test]
    fn dev_mode_removes_autostart_instead_of_registering_dev_binary() {
        let autostart = MockAutostart::default();

        let enabled = update_autostart_for_mode(&autostart, true, true).unwrap();

        assert!(!enabled);
        assert_eq!(autostart.enable_calls.get(), 0);
        assert_eq!(autostart.disable_calls.get(), 1);
    }

    #[test]
    fn packaged_mode_follows_autostart_preference() {
        let autostart = MockAutostart::default();

        assert!(update_autostart_for_mode(&autostart, true, false).unwrap());
        assert_eq!(autostart.enable_calls.get(), 1);
        assert_eq!(autostart.disable_calls.get(), 0);

        assert!(!update_autostart_for_mode(&autostart, false, false).unwrap());
        assert_eq!(autostart.enable_calls.get(), 1);
        assert_eq!(autostart.disable_calls.get(), 1);
    }

    struct MockWindow {
        hide_calls: Mutex<u32>,
        always_on_top: Mutex<Vec<bool>>,
    }

    impl MockWindow {
        fn new() -> Self {
            Self {
                hide_calls: Mutex::new(0),
                always_on_top: Mutex::new(Vec::new()),
            }
        }

        fn hide_count(&self) -> u32 {
            *self.hide_calls.lock().unwrap()
        }

        fn last_always_on_top(&self) -> Option<bool> {
            self.always_on_top.lock().unwrap().last().copied()
        }
    }

    impl WindowLike for MockWindow {
        fn hide_window(&self) {
            let mut calls = self.hide_calls.lock().unwrap();
            *calls += 1;
        }

        fn set_always_on_top_window(&self, enable: bool) {
            self.always_on_top.lock().unwrap().push(enable);
        }
    }

    #[test]
    fn hide_usage_window_updates_state_and_invokes_window_actions() {
        let window = MockWindow::new();
        let usage_state = UsageWindowState::default();
        usage_state.visible.store(true, Ordering::SeqCst);

        hide_usage_window(&window, &usage_state);

        assert!(!usage_state.visible.load(Ordering::SeqCst));
        assert_eq!(window.hide_count(), 1);
        assert_eq!(window.last_always_on_top(), Some(false));
    }

    #[test]
    fn test_current_utc_ms() {
        assert!(current_utc_ms() > 0);
    }
}
