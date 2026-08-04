use std::ffi::{c_void, OsStr};
use std::io::Cursor;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, APPMODEL_ERROR_NO_PACKAGE, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS,
    HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, FILE_FLAGS_AND_ATTRIBUTES,
};
use windows::Win32::Storage::Packaging::Appx::{GetApplicationUserModelId, GetPackageFamilyName};
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
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW, DrawIconEx,
    GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, PostThreadMessageW, RegisterClassW,
    TranslateMessage, UnregisterClassW, DEVICE_NOTIFY_WINDOW_HANDLE, DI_NORMAL,
    EVENT_SYSTEM_FOREGROUND, MSG, PBT_APMRESUMEAUTOMATIC, PBT_APMRESUMESUSPEND, PBT_APMSUSPEND,
    WINDOW_EX_STYLE, WINDOW_STYLE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    WM_POWERBROADCAST, WM_QUIT, WM_WTSSESSION_CHANGE, WNDCLASSW, WTS_SESSION_LOCK,
    WTS_SESSION_UNLOCK,
};

use super::{DesktopEvent, DesktopEventKind, ObservationFailure, ProcessIdentity};

const MAX_EXECUTABLE_PATH: usize = 32_768;
const APP_ICON_SIZE: u32 = 32;

static EVENT_SENDER: OnceLock<Mutex<Option<mpsc::Sender<DesktopEvent>>>> = OnceLock::new();

#[derive(Debug)]
pub struct EventProbe {
    observer_thread_id: u32,
    observer: Option<JoinHandle<()>>,
}

pub fn start_event_probe() -> Result<(EventProbe, mpsc::Receiver<DesktopEvent>), String> {
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

    Ok((
        EventProbe {
            observer_thread_id,
            observer: Some(observer),
        },
        event_receiver,
    ))
}

impl Drop for EventProbe {
    fn drop(&mut self) {
        // SAFETY: observer_thread_id belongs to the message-loop thread created by this probe.
        let _ =
            unsafe { PostThreadMessageW(self.observer_thread_id, WM_QUIT, WPARAM(0), LPARAM(0)) };
        if let Some(observer) = self.observer.take() {
            let _ = observer.join();
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

    match process_identity(process_id) {
        Ok(process) => DesktopEvent::foreground(Some(process), None),
        Err(failure) => DesktopEvent::foreground(None, Some(failure)),
    }
}

fn process_identity(process_id: u32) -> Result<ProcessIdentity, ObservationFailure> {
    // SAFETY: process_id is supplied by Windows for the foreground HWND.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id) }
        .map_err(|err| ObservationFailure::ProcessOpenFailed(err.code().0 as u32))?;

    let result = process_identity_from_handle(process_id, process);
    // SAFETY: process was returned by OpenProcess and is closed exactly once.
    let _ = unsafe { CloseHandle(process) };
    result
}

fn process_identity_from_handle(
    process_id: u32,
    process: HANDLE,
) -> Result<ProcessIdentity, ObservationFailure> {
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
    result.map_err(|err| ObservationFailure::ExecutablePathUnavailable(err.code().0 as u32))?;
    buffer.truncate(length as usize);
    let executable = PathBuf::from(String::from_utf16_lossy(&buffer));
    let package_family_name = query_app_model_value(process, GetPackageFamilyName);
    let application_user_model_id = query_app_model_value(process, GetApplicationUserModelId);
    let (product_name, company_name) = file_version_metadata(&executable);
    let icon_png = executable_icon_png(&executable);

    Ok(ProcessIdentity {
        process_id,
        executable,
        package_family_name,
        application_user_model_id,
        product_name,
        company_name,
        icon_png,
    })
}

type AppModelQuery = unsafe fn(HANDLE, *mut u32, Option<PWSTR>) -> WIN32_ERROR;

fn query_app_model_value(process: HANDLE, query: AppModelQuery) -> Option<String> {
    let mut length = 0;
    // SAFETY: the first call requests the required UTF-16 buffer length only.
    let status = unsafe { query(process, &mut length, None) };
    if status == APPMODEL_ERROR_NO_PACKAGE {
        return None;
    }
    if status != ERROR_INSUFFICIENT_BUFFER && status != ERROR_SUCCESS {
        return None;
    }
    if length == 0 {
        return None;
    }

    let mut buffer = vec![0u16; length as usize];
    // SAFETY: buffer contains length writable UTF-16 code units as requested above.
    let status = unsafe { query(process, &mut length, Some(PWSTR(buffer.as_mut_ptr()))) };
    if status != ERROR_SUCCESS {
        return None;
    }
    buffer.truncate(length as usize);
    if buffer.last() == Some(&0) {
        buffer.pop();
    }
    let value = String::from_utf16_lossy(&buffer);
    (!value.trim().is_empty()).then_some(value)
}

fn file_version_metadata(executable: &Path) -> (Option<String>, Option<String>) {
    let wide_path = wide(executable.as_os_str());
    // SAFETY: wide_path is a null-terminated UTF-16 path valid for this call.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(wide_path.as_ptr()), None) };
    if size == 0 {
        return (None, None);
    }

    let mut data = vec![0u8; size as usize];
    // SAFETY: data provides exactly size writable bytes and wide_path remains alive.
    if unsafe {
        GetFileVersionInfoW(
            PCWSTR(wide_path.as_ptr()),
            None,
            size,
            data.as_mut_ptr().cast(),
        )
    }
    .is_err()
    {
        return (None, None);
    }

    let (language, code_page) = version_translation(&data).unwrap_or((0x0409, 0x04b0));
    (
        version_string(&data, language, code_page, "ProductName"),
        version_string(&data, language, code_page, "CompanyName"),
    )
}

fn version_translation(data: &[u8]) -> Option<(u16, u16)> {
    let query = wide(OsStr::new(r"\VarFileInfo\Translation"));
    let mut value = std::ptr::null_mut::<c_void>();
    let mut length = 0;
    // SAFETY: data is the version-info block returned by Windows; value and length are outputs.
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query.as_ptr()),
            &mut value,
            &mut length,
        )
    };
    if !found.as_bool() || value.is_null() || length < 4 {
        return None;
    }
    // SAFETY: VerQueryValueW returned at least four bytes containing two WORD values.
    let words = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), 2) };
    Some((words[0], words[1]))
}

fn version_string(data: &[u8], language: u16, code_page: u16, name: &str) -> Option<String> {
    let sub_block = format!(r"\StringFileInfo\{language:04x}{code_page:04x}\{name}");
    let query = wide(OsStr::new(&sub_block));
    let mut value = std::ptr::null_mut::<c_void>();
    let mut length = 0;
    // SAFETY: data is the version-info block returned by Windows; value and length are outputs.
    let found = unsafe {
        VerQueryValueW(
            data.as_ptr().cast(),
            PCWSTR(query.as_ptr()),
            &mut value,
            &mut length,
        )
    };
    if !found.as_bool() || value.is_null() || length == 0 {
        return None;
    }
    // SAFETY: Windows reports length in UTF-16 code units for version string values.
    let units = unsafe { std::slice::from_raw_parts(value.cast::<u16>(), length as usize) };
    let value = String::from_utf16_lossy(units)
        .trim_end_matches('\0')
        .trim()
        .to_string();
    (!value.is_empty()).then_some(value)
}

fn executable_icon_png(executable: &Path) -> Option<Vec<u8>> {
    let wide_path = wide(executable.as_os_str());
    let mut file_info = SHFILEINFOW::default();
    // SAFETY: wide_path is a null-terminated path and file_info is writable for its full size.
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide_path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut file_info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if result == 0 || file_info.hIcon.0.is_null() {
        return None;
    }

    let png = icon_to_png(file_info.hIcon);
    // SAFETY: SHGetFileInfoW returned ownership of this icon handle to the caller.
    let _ = unsafe { DestroyIcon(file_info.hIcon) };
    png
}

fn icon_to_png(icon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<Vec<u8>> {
    // SAFETY: a memory DC has no lifetime dependency on a window.
    let dc = unsafe { CreateCompatibleDC(None) };
    if dc.0.is_null() {
        return None;
    }

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: APP_ICON_SIZE as i32,
            // A negative height creates a top-down DIB, matching PNG row order.
            biHeight: -(APP_ICON_SIZE as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };
    let mut pixels = std::ptr::null_mut::<c_void>();
    // SAFETY: bitmap_info is initialized and pixels receives the DIB allocation address.
    let bitmap = match unsafe {
        CreateDIBSection(Some(dc), &bitmap_info, DIB_RGB_COLORS, &mut pixels, None, 0)
    } {
        Ok(bitmap) => bitmap,
        Err(_) => {
            // SAFETY: dc was created above and has no selected resources to release first.
            let _ = unsafe { DeleteDC(dc) };
            return None;
        }
    };
    // SAFETY: bitmap is a valid GDI bitmap and dc is a compatible memory DC.
    let previous = unsafe { SelectObject(dc, HGDIOBJ(bitmap.0)) };
    // SAFETY: dc owns a selected 32-bit DIB and icon is a valid shell icon handle.
    let drawn = unsafe {
        DrawIconEx(
            dc,
            0,
            0,
            icon,
            APP_ICON_SIZE as i32,
            APP_ICON_SIZE as i32,
            0,
            None,
            DI_NORMAL,
        )
    }
    .is_ok();

    let rgba = if drawn && !pixels.is_null() {
        let byte_count = (APP_ICON_SIZE * APP_ICON_SIZE * 4) as usize;
        // SAFETY: CreateDIBSection allocated byte_count bytes for the configured 32-bit DIB.
        let bgra = unsafe { std::slice::from_raw_parts(pixels.cast::<u8>(), byte_count) };
        let mut rgba = bgra.to_vec();
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        if rgba.chunks_exact(4).all(|pixel| pixel[3] == 0) {
            for pixel in rgba.chunks_exact_mut(4) {
                if pixel[..3] != [0, 0, 0] {
                    pixel[3] = u8::MAX;
                }
            }
        }
        Some(rgba)
    } else {
        None
    };

    // SAFETY: restore the prior object before deleting the DIB and memory DC.
    unsafe {
        if !previous.0.is_null() {
            SelectObject(dc, previous);
        }
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
    }

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(APP_ICON_SIZE, APP_ICON_SIZE, rgba?)?;
    let mut output = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut output, ImageFormat::Png)
        .ok()?;
    Some(output.into_inner())
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
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
