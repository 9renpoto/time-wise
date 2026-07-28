use std::path::PathBuf;
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::UNIX_EPOCH;

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Power::{
    RegisterSuspendResumeNotification, UnregisterSuspendResumeNotification, HPOWERNOTIFY,
};
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows::Win32::System::Threading::{
    GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
    GetMessageW, GetWindowThreadProcessId, PostThreadMessageW, RegisterClassW, TranslateMessage,
    UnregisterClassW, DEVICE_NOTIFY_WINDOW_HANDLE, EVENT_SYSTEM_FOREGROUND, MSG,
    PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND, WINDOW_EX_STYLE, WINDOW_STYLE,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_POWERBROADCAST, WM_QUIT,
    WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
};

use super::{DesktopEvent, DesktopEventKind, ObservationFailure, ProcessIdentity};

const MAX_EXECUTABLE_PATH: usize = 32_768;

static EVENT_SENDER: OnceLock<Mutex<Option<mpsc::Sender<DesktopEvent>>>> = OnceLock::new();

#[derive(Debug)]
pub struct EventProbe {
    observer_thread_id: u32,
    observer: Option<JoinHandle<()>>,
    logger: Option<JoinHandle<()>>,
}

pub fn start_event_probe() -> Result<EventProbe, String> {
    let (event_sender, event_receiver) = mpsc::channel();
    set_event_sender(Some(event_sender.clone()));

    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let observer = thread::Builder::new()
        .name("windows-event-probe".to_string())
        .spawn(move || {
            if let Err(err) = run_observer(event_sender, &ready_sender) {
                if ready_sender.try_send(Err(err.clone())).is_err() {
                    eprintln!("Windows event probe stopped: {err}");
                }
            }
            set_event_sender(None);
        })
        .map_err(|err| format!("failed to spawn Windows observer thread: {err}"))?;

    let observer_thread_id = match ready_receiver.recv() {
        Ok(Ok(thread_id)) => thread_id,
        Ok(Err(err)) => {
            let _ = observer.join();
            return Err(err);
        }
        Err(err) => {
            let _ = observer.join();
            return Err(format!("Windows observer stopped during startup: {err}"));
        }
    };

    let logger = match thread::Builder::new()
        .name("windows-event-probe-logger".to_string())
        .spawn(move || {
            while let Ok(event) = event_receiver.recv() {
                log_event(&event);
            }
        }) {
        Ok(logger) => logger,
        Err(err) => {
            // SAFETY: observer_thread_id was reported by the running message-loop thread.
            let _ =
                unsafe { PostThreadMessageW(observer_thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
            let _ = observer.join();
            return Err(format!("failed to spawn Windows event logger: {err}"));
        }
    };

    Ok(EventProbe {
        observer_thread_id,
        observer: Some(observer),
        logger: Some(logger),
    })
}

impl Drop for EventProbe {
    fn drop(&mut self) {
        // SAFETY: observer_thread_id belongs to the message-loop thread created by this probe.
        let _ =
            unsafe { PostThreadMessageW(self.observer_thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(observer) = self.observer.take() {
            let _ = observer.join();
        }
        if let Some(logger) = self.logger.take() {
            let _ = logger.join();
        }
    }
}

fn run_observer(
    event_sender: mpsc::Sender<DesktopEvent>,
    ready_sender: &mpsc::SyncSender<Result<u32, String>>,
) -> Result<(), String> {
    // SAFETY: All Win32 resources are created and destroyed on this dedicated message-loop thread.
    unsafe {
        let instance = GetModuleHandleW(None)
            .map(|module| HINSTANCE(module.0))
            .map_err(|err| format!("GetModuleHandleW failed: {err}"))?;
        let class_name = w!("TimeWiseEventProbeWindow");
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: class_name,
            ..Default::default()
        };

        if RegisterClassW(&window_class) == 0 {
            return Err(format!(
                "RegisterClassW failed with error {}",
                GetLastError().0
            ));
        }

        let window = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Time Wise event probe"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        ) {
            Ok(window) => window,
            Err(err) => {
                let _ = UnregisterClassW(class_name, Some(instance));
                return Err(format!("CreateWindowExW failed: {err}"));
            }
        };

        let result = run_registered_observer(window, event_sender, ready_sender);
        let _ = DestroyWindow(window);
        let _ = UnregisterClassW(class_name, Some(instance));
        result
    }
}

unsafe fn run_registered_observer(
    window: HWND,
    event_sender: mpsc::Sender<DesktopEvent>,
    ready_sender: &mpsc::SyncSender<Result<u32, String>>,
) -> Result<(), String> {
    // SAFETY: window is a valid hidden top-level window owned by the current thread.
    unsafe { WTSRegisterSessionNotification(window, NOTIFY_FOR_THIS_SESSION) }
        .map_err(|err| format!("WTSRegisterSessionNotification failed: {err}"))?;

    // SAFETY: Windows interprets the HANDLE value as HWND with DEVICE_NOTIFY_WINDOW_HANDLE.
    let power_notification = match unsafe {
        RegisterSuspendResumeNotification(HANDLE(window.0), DEVICE_NOTIFY_WINDOW_HANDLE)
    } {
        Ok(notification) => notification,
        Err(err) => {
            // SAFETY: session notifications were registered for this valid window above.
            let _ = unsafe { WTSUnRegisterSessionNotification(window) };
            return Err(format!("RegisterSuspendResumeNotification failed: {err}"));
        }
    };

    // SAFETY: Callback is a static function and events are delivered to this thread's message loop.
    let foreground_hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(foreground_event_callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        )
    };

    if foreground_hook.0.is_null() {
        cleanup_registrations(window, power_notification, None);
        return Err(format!(
            "SetWinEventHook failed with error {}",
            unsafe { GetLastError() }.0
        ));
    }

    let _ = event_sender.send(observe_foreground_window());
    let thread_id = unsafe { GetCurrentThreadId() };
    if let Err(err) = ready_sender.send(Ok(thread_id)) {
        cleanup_registrations(window, power_notification, Some(foreground_hook));
        return Err(format!(
            "failed to report Windows observer readiness: {err}"
        ));
    }

    let mut message = MSG::default();
    loop {
        // SAFETY: message points to initialized storage and this thread owns the message loop.
        let status = unsafe { GetMessageW(&mut message, None, 0, 0) }.0;
        if status == -1 {
            cleanup_registrations(window, power_notification, Some(foreground_hook));
            return Err(format!(
                "GetMessageW failed with error {}",
                unsafe { GetLastError() }.0
            ));
        }
        if status == 0 {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    cleanup_registrations(window, power_notification, Some(foreground_hook));
    Ok(())
}

fn cleanup_registrations(
    window: HWND,
    power_notification: HPOWERNOTIFY,
    foreground_hook: Option<HWINEVENTHOOK>,
) {
    // SAFETY: Handles were registered by the observer thread and are released once on that thread.
    unsafe {
        if let Some(hook) = foreground_hook {
            let _ = UnhookWinEvent(hook);
        }
        let _ = UnregisterSuspendResumeNotification(power_notification);
        let _ = WTSUnRegisterSessionNotification(window);
    }
}

unsafe extern "system" fn foreground_event_callback(
    _hook: HWINEVENTHOOK,
    _event: u32,
    window: HWND,
    _object_id: i32,
    _child_id: i32,
    _event_thread_id: u32,
    _event_time_ms: u32,
) {
    emit(observe_window(window));
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_WTSSESSION_CHANGE => match wparam.0 as u32 {
            WTS_SESSION_LOCK => emit(DesktopEvent::lifecycle(DesktopEventKind::SessionLocked)),
            WTS_SESSION_UNLOCK => {
                emit(DesktopEvent::lifecycle(DesktopEventKind::SessionUnlocked));
                emit(observe_foreground_window());
            }
            _ => {}
        },
        WM_POWERBROADCAST => match wparam.0 as u32 {
            PBT_APMSUSPEND => emit(DesktopEvent::lifecycle(DesktopEventKind::Suspended)),
            PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => {
                emit(DesktopEvent::lifecycle(DesktopEventKind::Resumed));
                emit(observe_foreground_window());
            }
            _ => {}
        },
        _ => {}
    }

    // SAFETY: Unhandled messages are delegated to the default WindowProc.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn observe_foreground_window() -> DesktopEvent {
    // SAFETY: GetForegroundWindow requires no preconditions.
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return DesktopEvent::foreground(
            None,
            Some(ObservationFailure::ForegroundWindowUnavailable),
        );
    }
    observe_window(window)
}

fn observe_window(window: HWND) -> DesktopEvent {
    let mut process_id = 0;
    // SAFETY: process_id points to writable storage and window came from a Win32 event.
    unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
    if process_id == 0 {
        return DesktopEvent::foreground(None, Some(ObservationFailure::ProcessIdUnavailable));
    }

    match executable_path(process_id) {
        Ok(executable) => DesktopEvent::foreground(
            Some(ProcessIdentity {
                process_id,
                executable,
            }),
            None,
        ),
        Err(failure) => DesktopEvent::foreground(None, Some(failure)),
    }
}

fn executable_path(process_id: u32) -> Result<PathBuf, ObservationFailure> {
    // SAFETY: process_id is supplied by Windows for the foreground HWND.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
        .map_err(|err| ObservationFailure::ProcessOpenFailed(err.code().0 as u32))?;

    let mut buffer = vec![0u16; MAX_EXECUTABLE_PATH];
    let mut length = buffer.len() as u32;
    // SAFETY: buffer is writable for length UTF-16 code units and process is a valid handle.
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    };
    // SAFETY: process was returned by OpenProcess and is closed exactly once.
    let _ = unsafe { CloseHandle(process) };

    result.map_err(|err| ObservationFailure::ExecutablePathUnavailable(err.code().0 as u32))?;
    buffer.truncate(length as usize);
    Ok(PathBuf::from(String::from_utf16_lossy(&buffer)))
}

fn set_event_sender(sender: Option<mpsc::Sender<DesktopEvent>>) {
    let slot = EVENT_SENDER.get_or_init(|| Mutex::new(None));
    if let Ok(mut current) = slot.lock() {
        *current = sender;
    }
}

fn emit(event: DesktopEvent) {
    let Some(slot) = EVENT_SENDER.get() else {
        return;
    };
    if let Ok(sender) = slot.lock() {
        if let Some(sender) = sender.as_ref() {
            let _ = sender.send(event);
        }
    }
}

fn log_event(event: &DesktopEvent) {
    let observed_at_ms = event
        .observed_at
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let process_id = event.process.as_ref().map(|process| process.process_id);
    let executable = event
        .process
        .as_ref()
        .map(|process| process.executable.display().to_string());
    eprintln!(
        "[windows-event-probe] observed_at_ms={observed_at_ms} kind={:?} pid={process_id:?} executable={executable:?} failure={:?}",
        event.kind, event.failure
    );
}
