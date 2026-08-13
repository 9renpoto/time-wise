use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use objc2::rc::{autoreleasepool, Retained};
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass};
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{
    ns_string, NSDistributedNotificationCenter, NSNotification, NSNotificationCenter, NSObject,
};

use super::{DesktopEvent, DesktopEventKind, ObservationFailure, ProcessIdentity};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub struct EventProbe {
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    lifecycle_observer: Retained<LifecycleObserver>,
    workspace_center: Retained<NSNotificationCenter>,
    distributed_center: Retained<NSDistributedNotificationCenter>,
}

impl Drop for EventProbe {
    fn drop(&mut self) {
        // SAFETY: The observer is still retained by this probe and is removed
        // from the same notification centers it was registered with.
        unsafe {
            self.workspace_center
                .removeObserver(&self.lifecycle_observer);
            self.distributed_center.removeObserver_name_object(
                &self.lifecycle_observer,
                None,
                None,
            );
        }
        self.stopped.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct LifecycleObserverIvars {
    sender: mpsc::Sender<DesktopEvent>,
}

#[derive(Clone, Copy)]
enum LifecycleSignal {
    ScreenLocked,
    ScreenUnlocked,
    SystemWillSleep,
    SystemDidWake,
}

fn lifecycle_event_kind(signal: LifecycleSignal) -> DesktopEventKind {
    match signal {
        LifecycleSignal::ScreenLocked => DesktopEventKind::SessionLocked,
        LifecycleSignal::ScreenUnlocked => DesktopEventKind::SessionUnlocked,
        LifecycleSignal::SystemWillSleep => DesktopEventKind::Suspended,
        LifecycleSignal::SystemDidWake => DesktopEventKind::Resumed,
    }
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements. Notification callbacks
    // only read the thread-safe channel sender stored in the instance variables.
    #[unsafe(super(NSObject))]
    #[name = "TimeWiseLifecycleObserver"]
    #[ivars = LifecycleObserverIvars]
    struct LifecycleObserver;

    impl LifecycleObserver {
        #[unsafe(method(screenLocked:))]
        fn screen_locked(&self, _notification: &NSNotification) {
            self.emit(LifecycleSignal::ScreenLocked);
        }

        #[unsafe(method(screenUnlocked:))]
        fn screen_unlocked(&self, _notification: &NSNotification) {
            self.emit(LifecycleSignal::ScreenUnlocked);
        }

        #[unsafe(method(systemWillSleep:))]
        fn system_will_sleep(&self, _notification: &NSNotification) {
            self.emit(LifecycleSignal::SystemWillSleep);
        }

        #[unsafe(method(systemDidWake:))]
        fn system_did_wake(&self, _notification: &NSNotification) {
            self.emit(LifecycleSignal::SystemDidWake);
        }
    }
);

impl LifecycleObserver {
    fn new(sender: mpsc::Sender<DesktopEvent>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(LifecycleObserverIvars { sender });
        // SAFETY: `this` was allocated as LifecycleObserver and its Rust ivars
        // have been initialized before calling NSObject's initializer.
        unsafe { msg_send![super(this), init] }
    }

    fn emit(&self, signal: LifecycleSignal) {
        let event = DesktopEvent::lifecycle(lifecycle_event_kind(signal));
        let _ = self.ivars().sender.send(event);
    }
}

pub fn start_event_probe() -> Result<(EventProbe, mpsc::Receiver<DesktopEvent>), String> {
    let (sender, receiver) = mpsc::channel();
    let lifecycle_observer = LifecycleObserver::new(sender.clone());
    // SAFETY: The observer implements every registered selector with the
    // expected single-notification argument. The probe retains the observer
    // until it removes all registrations during drop.
    let (workspace_center, distributed_center) = unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let workspace_center = workspace.notificationCenter();
        workspace_center.addObserver_selector_name_object(
            &lifecycle_observer,
            sel!(systemWillSleep:),
            Some(NSWorkspaceWillSleepNotification),
            None,
        );
        workspace_center.addObserver_selector_name_object(
            &lifecycle_observer,
            sel!(systemDidWake:),
            Some(NSWorkspaceDidWakeNotification),
            None,
        );

        let distributed_center = NSDistributedNotificationCenter::defaultCenter();
        distributed_center.addObserver_selector_name_object(
            &lifecycle_observer,
            sel!(screenLocked:),
            Some(ns_string!("com.apple.screenIsLocked")),
            None,
        );
        distributed_center.addObserver_selector_name_object(
            &lifecycle_observer,
            sel!(screenUnlocked:),
            Some(ns_string!("com.apple.screenIsUnlocked")),
            None,
        );
        (workspace_center, distributed_center)
    };

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
            lifecycle_observer,
            workspace_center,
            distributed_center,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_signals_map_to_desktop_events() {
        for (signal, expected) in [
            (
                LifecycleSignal::ScreenLocked,
                DesktopEventKind::SessionLocked,
            ),
            (
                LifecycleSignal::ScreenUnlocked,
                DesktopEventKind::SessionUnlocked,
            ),
            (
                LifecycleSignal::SystemWillSleep,
                DesktopEventKind::Suspended,
            ),
            (LifecycleSignal::SystemDidWake, DesktopEventKind::Resumed),
        ] {
            assert_eq!(lifecycle_event_kind(signal), expected);
        }
    }
}
