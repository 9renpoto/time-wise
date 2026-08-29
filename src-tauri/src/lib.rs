mod app_identity;
mod app_usage;
mod platform;
pub mod secure_storage;
mod startup_metrics;
pub mod usage_history;
mod usage_queries;
mod usage_recorder;

use std::env;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use app_usage::{AppUsageRecord, AppUsageRecorder, APP_USAGE_POLL_INTERVAL};
use startup_metrics::{fetch_startup_records, StartupMetrics};
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

#[cfg(not(target_os = "macos"))]
use tauri::{PhysicalPosition, Position};

use sysinfo::{get_current_pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
#[cfg(not(target_os = "linux"))]
use tauri::tray::TrayIconEvent;
use tauri_plugin_autostart::AutoLaunchManager;
#[cfg(target_os = "macos")]
use tauri_plugin_autostart::MacosLauncher;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let startup_instant = Instant::now();

    let builder = tauri::Builder::default()
        // Register this first so duplicate launches exit before initializing
        // autostart, windows, tray icons, or measurement workers.
        .plugin(tauri_plugin_single_instance::init(
            |_app, _args, _working_directory| {},
        ))
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
            fetch_app_usage_records,
            fetch_daily_usage_summary,
            fetch_measurement_health,
            fetch_startup_records,
            fetch_weekly_usage_summary,
            get_autostart_enabled,
            get_onboarding_completed,
            set_autostart_enabled
        ])
        .setup(|app| {
            app.manage(UsageWindowState::default());
            let mut onboarding_required = true;

            // Remove registrations created by older dev builds. Packaged builds
            // keep the user's configured automatic-launch preference.
            if tauri::is_dev() {
                if let Err(err) = update_autostart(&app.state::<AutoLaunchManager>(), false) {
                    eprintln!("failed to remove development autostart registration: {err}");
                }
            }

            let app_usage_recorder = AppUsageRecorder::default();
            if let Err(err) = app_usage_recorder.record_current_processes() {
                eprintln!("failed to seed app usage data: {err}");
            }

            let recorder_for_task = app_usage_recorder.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(APP_USAGE_POLL_INTERVAL).await;
                    if let Err(err) = recorder_for_task.record_current_processes() {
                        eprintln!("failed to record app usage: {err}");
                    }
                }
            });

            app.manage(app_usage_recorder);

            let storage_path = app
                .path()
                .resolve("startup_times.sqlite", BaseDirectory::AppData)
                .unwrap_or_else(|err| {
                    eprintln!("failed to resolve startup metrics path: {err}");
                    env::temp_dir().join("time-wise-startup-times.sqlite")
                });
            let metrics = StartupMetrics::with_storage_path(storage_path);
            app.manage(metrics);

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
                Err(err) => eprintln!("failed to initialize usage history database: {err}"),
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
                                    #[cfg(target_os = "macos")]
                                    {
                                        let _ = (position, rect);
                                    }
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
                                            let tray_height = match rect.size {
                                                tauri::Size::Physical(size) => size.height as f64,
                                                tauri::Size::Logical(size) => size.height,
                                            };
                                            let y = if position.y > monitor_height / 2.0 {
                                                position.y - size.height as f64 - 12.0
                                            } else {
                                                position.y + tray_height + 12.0
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

    let launcher = resolve_launcher_name();

    app.run(move |app_handle, event| match event {
        RunEvent::Ready => {
            let metrics = app_handle.state::<StartupMetrics>();
            if let Err(err) = metrics.record_startup(startup_instant.elapsed(), launcher.clone()) {
                eprintln!("failed to record startup time: {err}");
            }
        }
        RunEvent::Exit => {
            if let Some(recorder) = app_handle.try_state::<Arc<UsageRecorder>>() {
                recorder.stop(SystemTime::now());
                let diagnostics = recorder.diagnostics();
                if diagnostics != Default::default() {
                    eprintln!("usage recorder diagnostics at exit: {diagnostics:?}");
                }
            }
        }
        _ => {}
    });
}

fn resolve_launcher_name() -> String {
    let refresh = RefreshKind::nothing().with_processes(ProcessRefreshKind::everything());
    let mut system = System::new_with_specifics(refresh);
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut pid = match get_current_pid() {
        Ok(pid) => pid,
        Err(_) => return "unknown".to_string(),
    };

    let mut fallback: Option<String> = None;

    for _ in 0..10 {
        let process = match system.process(pid) {
            Some(process) => process,
            None => break,
        };

        let parent_pid = match process.parent() {
            Some(parent) => parent,
            None => break,
        };

        let parent_process = match system.process(parent_pid) {
            Some(process) => process,
            None => break,
        };

        if let Some(path) = parent_process.exe() {
            if let Some(path_str) = path.to_str() {
                if let Some(app_name) = extract_app_name(path_str) {
                    return app_name;
                }
            }
        }

        if let Some(name) = process_name(parent_process) {
            fallback = Some(name);
        }

        pid = parent_pid;
    }

    fallback.unwrap_or_else(|| "unknown".to_string())
}

fn process_name(process: &sysinfo::Process) -> Option<String> {
    let name = process.name();
    if name.is_empty() {
        return None;
    }
    let name = name.to_string_lossy();
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_app_name(path: &str) -> Option<String> {
    if let Some(index) = path.find(".app/") {
        let prefix = &path[..index];
        if let Some(app_name) = prefix.rsplit('/').next() {
            if !app_name.is_empty() {
                return Some(app_name.to_string());
            }
        }
    }

    if path.ends_with(".exe") {
        return Path::new(path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string());
    }

    None
}

#[tauri::command]
async fn fetch_app_usage_records(
    state: State<'_, AppUsageRecorder>,
) -> Result<Vec<AppUsageRecord>, ()> {
    Ok(state.records())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::{atomic::Ordering, Mutex};

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
    fn test_extract_app_name() {
        assert_eq!(
            extract_app_name("/Applications/Focus.app/Contents/MacOS/Focus"),
            Some("Focus".to_string())
        );
        assert_eq!(
            extract_app_name("/Program Files/Editor/Editor.exe"),
            Some("Editor".to_string())
        );
        assert_eq!(extract_app_name("/usr/local/bin/my_tool"), None);
        assert_eq!(
            extract_app_name("/Applications/.app/Contents/MacOS/Tool"),
            None
        );
    }

    #[test]
    fn test_current_utc_ms() {
        assert!(current_utc_ms() > 0);
    }

    #[test]
    fn test_resolve_launcher_name() {
        let name = resolve_launcher_name();
        assert!(!name.is_empty());
    }
}
