#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use std::path::PathBuf;
use std::time::SystemTime;

#[cfg(not(target_os = "windows"))]
mod noop;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessIdentity {
    pub process_id: u32,
    pub executable: PathBuf,
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
