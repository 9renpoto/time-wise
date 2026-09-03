use std::path::PathBuf;
use std::sync::mpsc;

use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass};
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSBitmapImageRepPropertyKey, NSImage,
    NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
    NSWorkspaceDidActivateApplicationNotification, NSWorkspaceDidWakeNotification,
    NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{
    ns_string, NSDictionary, NSDistributedNotificationCenter, NSNotification, NSNotificationCenter,
    NSObject,
};

use super::{DesktopEvent, DesktopEventKind, ObservationFailure, ProcessIdentity};

pub struct EventProbe {
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

        #[unsafe(method(applicationActivated:))]
        fn application_activated(&self, notification: &NSNotification) {
            let event = autoreleasepool(|_| {
                let application = application_from_notification(notification);
                application
                    .as_deref()
                    .map_or_else(observe_missing_application, observe_application)
            });
            let _ = self.ivars().sender.send(event);
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
        workspace_center.addObserver_selector_name_object(
            &lifecycle_observer,
            sel!(applicationActivated:),
            Some(NSWorkspaceDidActivateApplicationNotification),
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

    let _ = sender.send(autoreleasepool(|_| observe_frontmost_application()));

    Ok((
        EventProbe {
            lifecycle_observer,
            workspace_center,
            distributed_center,
        },
        receiver,
    ))
}

fn observe_missing_application() -> DesktopEvent {
    DesktopEvent::foreground(None, Some(ObservationFailure::ForegroundWindowUnavailable))
}

fn observe_frontmost_application() -> DesktopEvent {
    let application = unsafe { NSWorkspace::sharedWorkspace().frontmostApplication() };
    application
        .as_deref()
        .map_or_else(observe_missing_application, observe_application)
}

fn application_from_notification(
    notification: &NSNotification,
) -> Option<Retained<NSRunningApplication>> {
    // SAFETY: Workspace activation notifications are created by AppKit, and
    // NSWorkspaceApplicationKey is an AppKit-owned NSString constant. The
    // retrieved object is type-checked below before it is used.
    let application = unsafe {
        notification
            .userInfo()
            .and_then(|user_info| user_info.objectForKey(NSWorkspaceApplicationKey))
    };
    application.and_then(|object| object.downcast::<NSRunningApplication>().ok())
}

fn observe_application(application: &NSRunningApplication) -> DesktopEvent {
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
    let icon_png = unsafe { application.icon() }.as_deref().and_then(image_png);

    DesktopEvent::foreground(
        Some(ProcessIdentity {
            process_id: process_id as u32,
            executable,
            bundle_identifier,
            product_name,
            icon_png,
            ..ProcessIdentity::default()
        }),
        None,
    )
}

fn image_png(image: &NSImage) -> Option<Vec<u8>> {
    // NSRunningApplication provides an NSImage whose backing representation may
    // vary by application. TIFF is AppKit's common interchange representation;
    // NSBitmapImageRep then produces bytes that browsers can display directly.
    let tiff = unsafe { image.TIFFRepresentation() }?;
    let bitmap = unsafe { NSBitmapImageRep::imageRepWithData(&tiff) }?;
    let properties = NSDictionary::<NSBitmapImageRepPropertyKey, AnyObject>::new();
    let png = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)
    }?;
    (!png.is_empty()).then(|| png.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use objc2::{AnyThread, ClassType};
    use objc2_foundation::NSData;

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

    #[test]
    fn image_png_encodes_an_appkit_image() {
        autoreleasepool(|_| {
            let source = NSData::with_bytes(include_bytes!("../../icons/32x32.png"));
            let image = NSImage::initWithData(NSImage::alloc(), &source).expect("valid PNG");

            let encoded = image_png(&image).expect("AppKit should encode the image");

            assert!(encoded.starts_with(b"\x89PNG\r\n\x1a\n"));
        });
    }

    #[test]
    fn invalid_image_data_does_not_produce_png() {
        autoreleasepool(|_| {
            let source = NSData::with_bytes(b"not an image");
            assert!(NSImage::initWithData(NSImage::alloc(), &source).is_none());
        });
    }

    #[test]
    fn activation_without_application_is_reported_as_unavailable() {
        autoreleasepool(|_| {
            let notification = unsafe {
                NSNotification::initWithName_object_userInfo(
                    NSNotification::alloc(),
                    NSWorkspaceDidActivateApplicationNotification,
                    None,
                    None,
                )
            };
            assert!(application_from_notification(&notification).is_none());
            assert_eq!(
                observe_missing_application().failure,
                Some(ObservationFailure::ForegroundWindowUnavailable)
            );
        });
    }

    #[test]
    fn activation_notification_returns_the_running_application() {
        autoreleasepool(|_| {
            // SAFETY: AppKit owns the current running application and the
            // workspace application key. NSDictionary retains both objects.
            let (application, user_info) = unsafe {
                let application = NSRunningApplication::currentApplication();
                let user_info: Retained<NSDictionary> = msg_send![
                    NSDictionary::<AnyObject, AnyObject>::class(),
                    dictionaryWithObject: application.as_super().as_super(),
                    forKey: NSWorkspaceApplicationKey
                ];
                (application, user_info)
            };
            // SAFETY: The notification name and userInfo dictionary are valid
            // AppKit/Foundation objects retained for the notification lifetime.
            let notification = unsafe {
                NSNotification::initWithName_object_userInfo(
                    NSNotification::alloc(),
                    NSWorkspaceDidActivateApplicationNotification,
                    None,
                    Some(&user_info),
                )
            };

            let observed = application_from_notification(&notification)
                .expect("workspace application should be present");

            assert_eq!(observed, application);
        });
    }
}
