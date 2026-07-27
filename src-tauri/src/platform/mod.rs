#![cfg_attr(all(test, not(target_os = "windows")), allow(dead_code))]

#[cfg(any(target_os = "windows", test))]
use std::path::PathBuf;
#[cfg(any(target_os = "windows", test))]
use std::time::SystemTime;

#[cfg(not(target_os = "windows"))]
mod noop;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(target_os = "windows"))]
pub use noop::start_event_probe;
#[cfg(target_os = "windows")]
pub use windows::start_event_probe;

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DesktopEventKind {
    ForegroundChanged,
    SessionLocked,
    SessionUnlocked,
    Suspended,
    Resumed,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessIdentity {
    pub process_id: u32,
    pub executable: PathBuf,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationFailure {
    ForegroundWindowUnavailable,
    ProcessIdUnavailable,
    ProcessOpenFailed(u32),
    ExecutablePathUnavailable(u32),
}

#[cfg(any(target_os = "windows", test))]
#[derive(Clone, Debug)]
pub struct DesktopEvent {
    pub observed_at: SystemTime,
    pub kind: DesktopEventKind,
    pub process: Option<ProcessIdentity>,
    pub failure: Option<ObservationFailure>,
}

#[cfg(any(target_os = "windows", test))]
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
    use super::{DesktopEvent, DesktopEventKind, ObservationFailure};

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
