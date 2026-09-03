#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod noop;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::start_event_probe;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub use noop::start_event_probe;
#[cfg(target_os = "windows")]
pub use windows::start_event_probe;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopEventKind {
    ForegroundChanged,
    SessionLocked,
    SessionUnlocked,
    Suspended,
    Resumed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessIdentity {
    pub process_id: u32,
    pub executable: PathBuf,
    pub package_family_name: Option<String>,
    pub application_user_model_id: Option<String>,
    pub bundle_identifier: Option<String>,
    pub product_name: Option<String>,
    pub company_name: Option<String>,
    pub icon_png: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationFailure {
    ForegroundWindowUnavailable,
    ProcessIdUnavailable,
    ProcessOpenFailed(u32),
    ExecutablePathUnavailable(u32),
}

#[derive(Clone, Debug)]
pub struct DesktopEvent {
    pub observed_at: SystemTime,
    pub kind: DesktopEventKind,
    pub process: Option<ProcessIdentity>,
    pub failure: Option<ObservationFailure>,
}

impl DesktopEvent {
    #[must_use]
    pub fn lifecycle(kind: DesktopEventKind) -> Self {
        Self {
            observed_at: SystemTime::now(),
            kind,
            process: None,
            failure: None,
        }
    }

    #[must_use]
    pub fn foreground(
        process: Option<ProcessIdentity>,
        failure: Option<ObservationFailure>,
    ) -> Self {
        Self {
            observed_at: SystemTime::now(),
            kind: DesktopEventKind::ForegroundChanged,
            process,
            failure,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_events_have_no_process_details() {
        let event = DesktopEvent::lifecycle(DesktopEventKind::SessionLocked);
        assert_eq!(event.kind, DesktopEventKind::SessionLocked);
        assert!(event.process.is_none());
        assert!(event.failure.is_none());
    }

    #[test]
    fn unidentified_foreground_event_preserves_failure() {
        let event = DesktopEvent::foreground(None, Some(ObservationFailure::ProcessOpenFailed(5)));
        assert_eq!(event.kind, DesktopEventKind::ForegroundChanged);
        assert_eq!(
            event.failure,
            Some(ObservationFailure::ProcessOpenFailed(5))
        );
    }
}
