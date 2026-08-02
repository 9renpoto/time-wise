use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local};

use crate::platform::{DesktopEvent, DesktopEventKind};
use crate::usage_history::{AppMetadata, NewUsageSession, UsageHistoryStore, UsageSubject};

#[cfg(target_os = "windows")]
pub const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq, Eq)]
enum OwnedSubject {
    Identified(AppMetadata),
    Unclassified,
}

#[derive(Clone, Debug)]
struct ActiveSession {
    subject: OwnedSubject,
    started_at_utc_ms: u64,
    measured_timezone: String,
    measured_local_date: String,
}

#[derive(Default)]
struct RecorderState {
    active: Option<ActiveSession>,
    interrupted: bool,
    last_event_at_ms: Option<u64>,
}

pub struct UsageRecorder {
    store: Arc<UsageHistoryStore>,
    own_executable: PathBuf,
    state: Mutex<RecorderState>,
}

impl UsageRecorder {
    #[must_use]
    pub fn new(store: Arc<UsageHistoryStore>, own_executable: PathBuf) -> Self {
        Self {
            store,
            own_executable,
            state: Mutex::new(RecorderState::default()),
        }
    }

    pub fn handle_event(&self, event: DesktopEvent) -> Result<(), String> {
        let at_ms = system_time_to_ms(event.observed_at)?;
        let mut state = self.lock_state()?;
        if state
            .last_event_at_ms
            .is_some_and(|last_event_at_ms| at_ms < last_event_at_ms)
        {
            return Ok(());
        }
        let result = match event.kind {
            DesktopEventKind::ForegroundChanged => {
                if state.interrupted {
                    Ok(())
                } else {
                    let subject = event
                        .process
                        .as_ref()
                        .map(|process| process.executable.as_path())
                        .and_then(|path| self.subject_for_executable(path));
                    let subject = match (subject, event.failure) {
                        (Some(subject), _) => Some(subject),
                        (None, Some(_)) => Some(OwnedSubject::Unclassified),
                        (None, None) => None,
                    };
                    self.switch_subject(&mut state, subject, at_ms)
                }
            }
            DesktopEventKind::SessionLocked | DesktopEventKind::Suspended => {
                self.finish_active(&mut state, at_ms, interruption_reason(&event.kind))?;
                state.interrupted = true;
                Ok(())
            }
            DesktopEventKind::SessionUnlocked | DesktopEventKind::Resumed => {
                state.interrupted = false;
                Ok(())
            }
        };
        if result.is_ok() {
            state.last_event_at_ms = Some(at_ms);
        }
        result
    }

    pub fn checkpoint(&self, at: SystemTime) -> Result<(), String> {
        let at_ms = system_time_to_ms(at)?;
        let mut state = self.lock_state()?;
        let Some(active) = state.active.clone() else {
            return Ok(());
        };
        if at_ms <= active.started_at_utc_ms {
            return Ok(());
        }
        self.persist(&active, at_ms, "checkpoint")?;
        state.active = Some(new_active(active.subject, at));
        Ok(())
    }

    pub fn stop(&self, at: SystemTime) -> Result<(), String> {
        let at_ms = system_time_to_ms(at)?;
        let mut state = self.lock_state()?;
        self.finish_active(&mut state, at_ms, "measurement_stopped")
    }

    fn switch_subject(
        &self,
        state: &mut RecorderState,
        subject: Option<OwnedSubject>,
        at_ms: u64,
    ) -> Result<(), String> {
        if state.active.as_ref().map(|active| &active.subject) == subject.as_ref() {
            return Ok(());
        }
        self.finish_active(state, at_ms, "focus_changed")?;
        if let Some(subject) = subject {
            state.active = Some(new_active(subject, system_time_from_ms(at_ms)));
        }
        Ok(())
    }

    fn finish_active(
        &self,
        state: &mut RecorderState,
        at_ms: u64,
        reason: &str,
    ) -> Result<(), String> {
        let Some(active) = state.active.as_ref() else {
            return Ok(());
        };
        self.persist(active, at_ms.max(active.started_at_utc_ms), reason)?;
        state.active = None;
        Ok(())
    }

    fn persist(
        &self,
        active: &ActiveSession,
        ended_at_ms: u64,
        reason: &str,
    ) -> Result<(), String> {
        let subject = match &active.subject {
            OwnedSubject::Identified(metadata) => UsageSubject::Identified(metadata),
            OwnedSubject::Unclassified => UsageSubject::Unclassified,
        };
        self.store
            .record_session(&NewUsageSession {
                subject,
                started_at_utc_ms: active.started_at_utc_ms,
                ended_at_utc_ms: ended_at_ms,
                measured_timezone: &active.measured_timezone,
                measured_local_date: &active.measured_local_date,
                end_reason: reason,
            })
            .map(|_| ())
    }

    fn subject_for_executable(&self, executable: &Path) -> Option<OwnedSubject> {
        if same_executable(executable, &self.own_executable) {
            return None;
        }
        let executable_text = executable.to_string_lossy().to_string();
        let display_name = executable
            .file_stem()
            .map(|name| name.to_string_lossy().to_string())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Unclassified".to_string());
        Some(OwnedSubject::Identified(AppMetadata {
            stable_key: format!("executable:{}", executable_text.to_lowercase()),
            display_name,
            executable: Some(executable_text),
        }))
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, RecorderState>, String> {
        self.state
            .lock()
            .map_err(|_| "usage recorder mutex poisoned".to_string())
    }
}

fn new_active(subject: OwnedSubject, at: SystemTime) -> ActiveSession {
    let local: DateTime<Local> = at.into();
    ActiveSession {
        subject,
        started_at_utc_ms: system_time_to_ms(at).unwrap_or_default(),
        measured_timezone: local.offset().to_string(),
        measured_local_date: local.format("%Y-%m-%d").to_string(),
    }
}

fn interruption_reason(kind: &DesktopEventKind) -> &'static str {
    match kind {
        DesktopEventKind::SessionLocked => "session_locked",
        DesktopEventKind::Suspended => "system_suspended",
        _ => unreachable!("only interruption events are accepted"),
    }
}

fn same_executable(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn system_time_to_ms(time: SystemTime) -> Result<u64, String> {
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "event timestamp predates Unix epoch".to_string())?
        .as_millis();
    u64::try_from(millis).map_err(|_| "event timestamp exceeds u64".to_string())
}

fn system_time_from_ms(ms: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ObservationFailure, ProcessIdentity};

    fn fixture() -> (tempfile::TempDir, Arc<UsageHistoryStore>, UsageRecorder) {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            UsageHistoryStore::with_storage_path(directory.path().join("usage.sqlite")).unwrap(),
        );
        let recorder = UsageRecorder::new(store.clone(), PathBuf::from("time-wise.exe"));
        (directory, store, recorder)
    }

    fn event(at_ms: u64, kind: DesktopEventKind, executable: Option<&str>) -> DesktopEvent {
        DesktopEvent {
            observed_at: system_time_from_ms(at_ms),
            kind,
            process: executable.map(|path| ProcessIdentity {
                process_id: 1,
                executable: PathBuf::from(path),
            }),
            failure: None,
        }
    }

    fn sessions(store: &UsageHistoryStore) -> Vec<crate::usage_history::StoredUsageSession> {
        let date = DateTime::<Local>::from(system_time_from_ms(1_000))
            .format("%Y-%m-%d")
            .to_string();
        store.sessions_for_local_date(&date).unwrap()
    }

    #[test]
    fn focus_changes_create_sessions() {
        let (_directory, store, recorder) = fixture();
        recorder
            .handle_event(event(
                1_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder
            .handle_event(event(
                4_000,
                DesktopEventKind::ForegroundChanged,
                Some("b.exe"),
            ))
            .unwrap();
        recorder.stop(system_time_from_ms(6_000)).unwrap();
        let saved = sessions(&store);
        assert_eq!(saved.len(), 2);
        assert_eq!(
            (saved[0].started_at_utc_ms, saved[0].ended_at_utc_ms),
            (1_000, 4_000)
        );
        assert_eq!(saved[0].end_reason, "focus_changed");
        assert_eq!(saved[1].end_reason, "measurement_stopped");
    }

    #[test]
    fn checkpoint_limits_uncommitted_time_and_continues_session() {
        let (_directory, store, recorder) = fixture();
        recorder
            .handle_event(event(
                1_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder.checkpoint(system_time_from_ms(31_000)).unwrap();
        recorder.stop(system_time_from_ms(40_000)).unwrap();
        let saved = sessions(&store);
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].end_reason, "checkpoint");
        assert_eq!(saved[1].started_at_utc_ms, 31_000);
    }

    #[test]
    fn lock_stops_measurement_until_a_post_resume_focus_event() {
        let (_directory, store, recorder) = fixture();
        recorder
            .handle_event(event(
                1_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder
            .handle_event(event(2_000, DesktopEventKind::SessionLocked, None))
            .unwrap();
        recorder
            .handle_event(event(
                3_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder
            .handle_event(event(5_000, DesktopEventKind::SessionUnlocked, None))
            .unwrap();
        recorder
            .handle_event(event(
                5_001,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder.stop(system_time_from_ms(6_000)).unwrap();
        let saved = sessions(&store);
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[0].end_reason, "session_locked");
        assert_eq!(saved[1].started_at_utc_ms, 5_001);
    }

    #[test]
    fn failures_are_recorded_as_unclassified_and_self_is_excluded() {
        let (_directory, store, recorder) = fixture();
        let mut failed = event(1_000, DesktopEventKind::ForegroundChanged, None);
        failed.failure = Some(ObservationFailure::ProcessIdUnavailable);
        recorder.handle_event(failed).unwrap();
        recorder
            .handle_event(event(
                2_000,
                DesktopEventKind::ForegroundChanged,
                Some("TIME-WISE.EXE"),
            ))
            .unwrap();
        let saved = sessions(&store);
        assert_eq!(saved.len(), 1);
        assert!(saved[0].stable_key.is_none());
    }

    #[test]
    fn checkpoint_before_active_boundary_is_ignored() {
        let (_directory, store, recorder) = fixture();
        recorder
            .handle_event(event(
                10_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();

        recorder.checkpoint(system_time_from_ms(9_000)).unwrap();
        recorder.stop(system_time_from_ms(11_000)).unwrap();

        let saved = sessions(&store);
        assert_eq!(saved.len(), 1);
        assert_eq!(
            (saved[0].started_at_utc_ms, saved[0].ended_at_utc_ms),
            (10_000, 11_000)
        );
    }

    #[test]
    fn stale_events_do_not_rewind_the_active_subject() {
        let (_directory, store, recorder) = fixture();
        recorder
            .handle_event(event(
                10_000,
                DesktopEventKind::ForegroundChanged,
                Some("a.exe"),
            ))
            .unwrap();
        recorder
            .handle_event(event(
                20_000,
                DesktopEventKind::ForegroundChanged,
                Some("b.exe"),
            ))
            .unwrap();

        recorder
            .handle_event(event(
                15_000,
                DesktopEventKind::ForegroundChanged,
                Some("c.exe"),
            ))
            .unwrap();
        recorder.stop(system_time_from_ms(30_000)).unwrap();

        let saved = sessions(&store);
        assert_eq!(saved.len(), 2);
        assert_eq!(saved[1].display_name.as_deref(), Some("b"));
        assert_eq!(saved[1].ended_at_utc_ms, 30_000);
    }
}
