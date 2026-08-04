use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;

use super::{DesktopEvent, ObservationFailure, ProcessIdentity};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub struct EventProbe {
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for EventProbe {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn start_event_probe() -> Result<(EventProbe, mpsc::Receiver<DesktopEvent>), String> {
    let (sender, receiver) = mpsc::channel();
    let stopped = Arc::new(AtomicBool::new(false));
    let thread_stopped = stopped.clone();
    let thread = thread::Builder::new()
        .name("macos-foreground-probe".to_string())
        .spawn(move || {
            let mut last_process_id = None;
            while !thread_stopped.load(Ordering::Acquire) {
                let event = autoreleasepool(|_| observe_frontmost_application());
                let process_id = event.process.as_ref().map(|process| process.process_id);
                if process_id != last_process_id {
                    last_process_id = process_id;
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                thread::sleep(POLL_INTERVAL);
            }
        })
        .map_err(|err| format!("failed to spawn macOS observer thread: {err}"))?;

    Ok((
        EventProbe {
            stopped,
            thread: Some(thread),
        },
        receiver,
    ))
}

fn observe_frontmost_application() -> DesktopEvent {
    // SAFETY: These AppKit accessors return retained immutable objects. Each call
    // is made inside an autorelease pool owned by the observer thread.
    let application = unsafe { NSWorkspace::sharedWorkspace().frontmostApplication() };
    let Some(application) = application else {
        return DesktopEvent::foreground(
            None,
            Some(ObservationFailure::ForegroundWindowUnavailable),
        );
    };

    // SAFETY: NSRunningApplication properties are immutable snapshots for the
    // running process and are copied into Rust-owned values before returning.
    let process_id = unsafe { application.processIdentifier() };
    if process_id <= 0 {
        return DesktopEvent::foreground(None, Some(ObservationFailure::ProcessIdUnavailable));
    }
    let executable = unsafe {
        application
            .executableURL()
            .and_then(|url| url.path())
            .map(|path| PathBuf::from(path.to_string()))
    };
    let Some(executable) = executable else {
        return DesktopEvent::foreground(
            None,
            Some(ObservationFailure::ExecutablePathUnavailable(
                process_id as u32,
            )),
        );
    };
    let product_name = unsafe { application.localizedName().map(|name| name.to_string()) };
    let bundle_identifier = unsafe {
        application
            .bundleIdentifier()
            .map(|identifier| identifier.to_string())
    };

    DesktopEvent::foreground(
        Some(ProcessIdentity {
            process_id: process_id as u32,
            executable,
            bundle_identifier,
            product_name,
            ..ProcessIdentity::default()
        }),
        None,
    )
}
