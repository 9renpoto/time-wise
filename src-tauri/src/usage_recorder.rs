//! Converts desktop lifecycle events into durable usage sessions.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, Utc};

use crate::app_identity;
#[cfg(test)]
use crate::platform::ProcessIdentity;
use crate::platform::{DesktopEvent, DesktopEventKind, ObservationFailure};
use crate::usage_history::{AppMetadata, NewUsageSession, UsageHistoryStore, UsageSubject};

pub const CHECKPOINT_INTERVAL_SECONDS: u64 = 30;
const MAX_DIAGNOSTIC_EVENTS: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrackedSubject {
    Identified(AppMetadata),
    Unclassified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveSession {
    subject: TrackedSubject,
    started_at_utc_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecorderState {
    Idle,
    Recording(ActiveSession),
    Interrupted,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecorderDiagnostics {
    pub subscription_error: Option<String>,
    pub observation_failure: Option<ObservationFailure>,
    pub persistence_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementHealthStatus {
    Healthy,
    EventSubscriptionFailed,
    ObservationDegraded,
    PersistenceFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticEvent {
    pub occurred_at_utc_ms: u64,
    pub code: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeasurementHealth {
    pub status: MeasurementHealthStatus,
    pub latest_diagnostic: Option<DiagnosticEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionRecord {
    subject: TrackedSubject,
    started_at_utc_ms: u64,
    ended_at_utc_ms: u64,
    measured_timezone: String,
    measured_local_date: String,
    end_reason: &'static str,
}

trait SessionSink: Send + Sync {
    fn save(&self, session: &SessionRecord) -> Result<(), String>;
}

struct SqliteSessionSink {
    store: Arc<UsageHistoryStore>,
}

impl SessionSink for SqliteSessionSink {
    fn save(&self, session: &SessionRecord) -> Result<(), String> {
        let subject = match &session.subject {
            TrackedSubject::Identified(metadata) => UsageSubject::Identified(metadata),
            TrackedSubject::Unclassified => UsageSubject::Unclassified,
        };
        self.store
            .record_session(&NewUsageSession {
                subject,
                started_at_utc_ms: session.started_at_utc_ms,
                ended_at_utc_ms: session.ended_at_utc_ms,
                measured_timezone: &session.measured_timezone,
                measured_local_date: &session.measured_local_date,
                end_reason: session.end_reason,
            })
            .map(|_| ())
    }
}

pub struct UsageRecorder {
    sink: Arc<dyn SessionSink>,
    history_store: Option<Arc<UsageHistoryStore>>,
    state: Mutex<RecorderState>,
    diagnostics: Mutex<RecorderDiagnostics>,
    diagnostic_events: Mutex<VecDeque<DiagnosticEvent>>,
    last_event_at_ms: Mutex<Option<u64>>,
}

impl UsageRecorder {
    #[must_use]
    pub fn new(store: Arc<UsageHistoryStore>) -> Self {
        Self {
            sink: Arc::new(SqliteSessionSink {
                store: store.clone(),
            }),
            history_store: Some(store),
            state: Mutex::new(RecorderState::Idle),
            diagnostics: Mutex::new(RecorderDiagnostics::default()),
            diagnostic_events: Mutex::new(VecDeque::new()),
            last_event_at_ms: Mutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_sink(sink: Arc<dyn SessionSink>) -> Self {
        Self {
            sink,
            history_store: None,
            state: Mutex::new(RecorderState::Idle),
            diagnostics: Mutex::new(RecorderDiagnostics::default()),
            diagnostic_events: Mutex::new(VecDeque::new()),
            last_event_at_ms: Mutex::new(None),
        }
    }

    pub fn handle_event(&self, event: DesktopEvent) {
        let observed_at_utc_ms = system_time_to_ms(event.observed_at);
        if !self.accept_event_timestamp(observed_at_utc_ms) {
            return;
        }
        if let Ok(mut diagnostics) = self.diagnostics.lock() {
            diagnostics.subscription_error = None;
        }
        match event.kind {
            DesktopEventKind::ForegroundChanged => {
                self.record_observation_failure(event.failure);
                if event
                    .process
                    .as_ref()
                    .is_some_and(|process| process.process_id == std::process::id())
                {
                    self.exclude_focus(observed_at_utc_ms);
                    return;
                }
                let subject = event
                    .process
                    .as_ref()
                    .map(|process| TrackedSubject::Identified(app_identity::resolve(process)))
                    .unwrap_or(TrackedSubject::Unclassified);
                self.focus_changed(subject, observed_at_utc_ms);
            }
            DesktopEventKind::SessionLocked => {
                self.interrupt(observed_at_utc_ms, "session_locked");
            }
            DesktopEventKind::Suspended => {
                self.interrupt(observed_at_utc_ms, "system_suspended");
            }
            DesktopEventKind::SessionUnlocked | DesktopEventKind::Resumed => self.resume(),
        }
    }

    pub fn checkpoint(&self, at: SystemTime) {
        self.close_and_restart(system_time_to_ms(at), "checkpoint");
    }

    pub fn stop(&self, at: SystemTime) {
        let at_utc_ms = system_time_to_ms(at);
        if let Ok(mut state) = self.state.lock() {
            if let RecorderState::Recording(active) = &*state {
                self.persist(active, at_utc_ms, "measurement_stopped");
            }
            *state = RecorderState::Idle;
        }
    }

    /// Clears durable usage and restarts an active measurement at the deletion boundary.
    pub fn delete_history(&self, at: SystemTime) -> Result<(), String> {
        let Some(store) = &self.history_store else {
            return Err("usage history store unavailable".to_string());
        };
        let at_utc_ms = system_time_to_ms(at);
        let mut state = self
            .state
            .lock()
            .map_err(|_| "usage recorder state mutex poisoned".to_string())?;

        if let RecorderState::Recording(active) = &*state {
            if at_utc_ms > active.started_at_utc_ms {
                self.persist(active, at_utc_ms, "history_deleted");
                *state = RecorderState::Recording(ActiveSession {
                    subject: active.subject.clone(),
                    started_at_utc_ms: at_utc_ms,
                });
            }
        }

        store.delete_all_usage_history().inspect_err(|error| {
            if let Ok(mut diagnostics) = self.diagnostics.lock() {
                diagnostics.persistence_error = Some(error.clone());
            }
        })
    }

    pub fn record_subscription_failure(&self, error: String) {
        if let Ok(mut diagnostics) = self.diagnostics.lock() {
            diagnostics.subscription_error = Some(error);
        }
        self.record_diagnostic("event_subscription_failed");
    }

    pub fn record_subscription_ended(&self) {
        self.record_subscription_failure("desktop event subscription ended".to_string());
    }

    #[must_use]
    pub fn diagnostics(&self) -> RecorderDiagnostics {
        self.diagnostics
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| RecorderDiagnostics {
                persistence_error: Some("usage recorder diagnostics mutex poisoned".into()),
                ..RecorderDiagnostics::default()
            })
    }

    #[must_use]
    pub fn health(&self) -> MeasurementHealth {
        let diagnostics = self.diagnostics();
        let status = if diagnostics.persistence_error.is_some() {
            MeasurementHealthStatus::PersistenceFailed
        } else if diagnostics.subscription_error.is_some() {
            MeasurementHealthStatus::EventSubscriptionFailed
        } else if diagnostics.observation_failure.is_some() {
            MeasurementHealthStatus::ObservationDegraded
        } else {
            MeasurementHealthStatus::Healthy
        };
        let latest_diagnostic = self
            .diagnostic_events
            .lock()
            .ok()
            .and_then(|events| events.back().cloned());
        MeasurementHealth {
            status,
            latest_diagnostic,
        }
    }

    fn focus_changed(&self, subject: TrackedSubject, at_utc_ms: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let RecorderState::Recording(active) = &*state {
            if active.subject == subject {
                return;
            }
            self.persist(active, at_utc_ms, "focus_changed");
        }
        *state = RecorderState::Recording(ActiveSession {
            subject,
            started_at_utc_ms: at_utc_ms,
        });
    }

    fn exclude_focus(&self, at_utc_ms: u64) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let RecorderState::Recording(active) = &*state {
            self.persist(active, at_utc_ms, "focus_changed");
        }
        *state = RecorderState::Idle;
    }

    fn interrupt(&self, at_utc_ms: u64, reason: &'static str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let RecorderState::Recording(active) = &*state {
            self.persist(active, at_utc_ms, reason);
        }
        *state = RecorderState::Interrupted;
    }

    fn resume(&self) {
        if let Ok(mut state) = self.state.lock() {
            if matches!(*state, RecorderState::Interrupted) {
                *state = RecorderState::Idle;
            }
        }
    }

    fn close_and_restart(&self, at_utc_ms: u64, reason: &'static str) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if let RecorderState::Recording(active) = &*state {
            if at_utc_ms <= active.started_at_utc_ms {
                return;
            }
            self.persist(active, at_utc_ms, reason);
            *state = RecorderState::Recording(ActiveSession {
                subject: active.subject.clone(),
                started_at_utc_ms: at_utc_ms,
            });
        }
    }

    fn persist(&self, active: &ActiveSession, ended_at_utc_ms: u64, reason: &'static str) {
        if ended_at_utc_ms < active.started_at_utc_ms {
            return;
        }
        let (measured_timezone, measured_local_date) =
            local_measurement_context(active.started_at_utc_ms);
        let result = self.sink.save(&SessionRecord {
            subject: active.subject.clone(),
            started_at_utc_ms: active.started_at_utc_ms,
            ended_at_utc_ms,
            measured_timezone,
            measured_local_date,
            end_reason: reason,
        });
        if let Ok(mut diagnostics) = self.diagnostics.lock() {
            match result {
                Ok(()) => diagnostics.persistence_error = None,
                Err(error) => {
                    diagnostics.persistence_error = Some(error);
                    drop(diagnostics);
                    self.record_diagnostic("usage_history_write_failed");
                }
            }
        }
    }

    fn record_observation_failure(&self, failure: Option<ObservationFailure>) {
        if let Ok(mut diagnostics) = self.diagnostics.lock() {
            match failure {
                Some(failure) => {
                    diagnostics.observation_failure = Some(failure);
                    drop(diagnostics);
                    self.record_diagnostic("foreground_observation_failed");
                }
                None => diagnostics.observation_failure = None,
            }
        }
    }

    fn record_diagnostic(&self, code: &'static str) {
        let Ok(mut events) = self.diagnostic_events.lock() else {
            return;
        };
        if events.len() == MAX_DIAGNOSTIC_EVENTS {
            events.pop_front();
        }
        events.push_back(DiagnosticEvent {
            occurred_at_utc_ms: system_time_to_ms(SystemTime::now()),
            code,
        });
    }

    fn accept_event_timestamp(&self, at_utc_ms: u64) -> bool {
        let Ok(mut last_event_at_ms) = self.last_event_at_ms.lock() else {
            return false;
        };
        if last_event_at_ms.is_some_and(|last| at_utc_ms < last) {
            return false;
        }
        *last_event_at_ms = Some(at_utc_ms);
        true
    }
}

fn local_measurement_context(at_utc_ms: u64) -> (String, String) {
    let utc = i64::try_from(at_utc_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .unwrap_or_else(Utc::now);
    let local = utc.with_timezone(&Local);
    (
        local.format("%:z").to_string(),
        local.format("%F").to_string(),
    )
}

fn system_time_to_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    struct MemorySink {
        sessions: Mutex<Vec<SessionRecord>>,
    }

    impl SessionSink for MemorySink {
        fn save(&self, session: &SessionRecord) -> Result<(), String> {
            self.sessions.lock().unwrap().push(session.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecoveringSink {
        fail_next: AtomicBool,
    }

    impl SessionSink for RecoveringSink {
        fn save(&self, _session: &SessionRecord) -> Result<(), String> {
            if self.fail_next.swap(false, Ordering::SeqCst) {
                Err("database write failed for Secret Document.txt".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn at(milliseconds: u64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_millis(milliseconds)
    }

    fn foreground(milliseconds: u64, executable: &str) -> DesktopEvent {
        DesktopEvent {
            observed_at: at(milliseconds),
            kind: DesktopEventKind::ForegroundChanged,
            process: Some(ProcessIdentity {
                process_id: 42,
                executable: PathBuf::from(executable),
                ..ProcessIdentity::default()
            }),
            failure: None,
        }
    }

    fn recorder() -> (Arc<MemorySink>, UsageRecorder) {
        let sink = Arc::new(MemorySink::default());
        let recorder = UsageRecorder::with_sink(sink.clone());
        (sink, recorder)
    }

    #[test]
    fn focus_changes_create_sessions_immediately() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(1_000, r"C:\Apps\Editor.exe"));
        recorder.handle_event(foreground(2_500, r"C:\Apps\Browser.exe"));
        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            (sessions[0].started_at_utc_ms, sessions[0].ended_at_utc_ms),
            (1_000, 2_500)
        );
        assert_eq!(sessions[0].end_reason, "focus_changed");
    }

    #[test]
    fn time_wise_focus_is_excluded() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(1_000, r"C:\Apps\Editor.exe"));
        recorder.handle_event(DesktopEvent {
            observed_at: at(2_000),
            kind: DesktopEventKind::ForegroundChanged,
            process: Some(ProcessIdentity {
                process_id: std::process::id(),
                executable: PathBuf::from(r"C:\Apps\time-wise.exe"),
                ..ProcessIdentity::default()
            }),
            failure: None,
        });
        recorder.checkpoint(at(3_000));
        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].ended_at_utc_ms, 2_000);
    }

    #[test]
    fn lock_and_suspend_close_sessions_until_fresh_focus() {
        for (kind, reason) in [
            (DesktopEventKind::SessionLocked, "session_locked"),
            (DesktopEventKind::Suspended, "system_suspended"),
        ] {
            let (sink, recorder) = recorder();
            recorder.handle_event(foreground(1_000, r"C:\Apps\Editor.exe"));
            recorder.handle_event(DesktopEvent {
                observed_at: at(2_000),
                kind,
                process: None,
                failure: None,
            });
            recorder.handle_event(DesktopEvent {
                observed_at: at(3_000),
                kind: DesktopEventKind::Resumed,
                process: None,
                failure: None,
            });
            recorder.checkpoint(at(4_000));
            assert_eq!(sink.sessions.lock().unwrap().len(), 1);
            assert_eq!(sink.sessions.lock().unwrap()[0].end_reason, reason);
            recorder.handle_event(foreground(5_000, r"C:\Apps\Editor.exe"));
            recorder.stop(at(6_000));
            assert_eq!(sink.sessions.lock().unwrap().len(), 2);
        }
    }

    #[test]
    fn checkpoints_bound_abnormal_exit_loss_to_less_than_thirty_seconds() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(0, r"C:\Apps\Editor.exe"));
        recorder.checkpoint(at(30_000));
        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].ended_at_utc_ms, 30_000);
        let simulated_crash_at_ms = 59_999;
        assert!(simulated_crash_at_ms - sessions[0].ended_at_utc_ms < 30_000);
    }

    #[test]
    fn normal_stop_finalizes_the_active_session() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(10, r"C:\Apps\Editor.exe"));
        recorder.stop(at(20));
        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].end_reason, "measurement_stopped");
    }

    #[test]
    fn deleting_history_restarts_an_active_session_at_the_deletion_boundary() {
        let directory = tempfile::tempdir().unwrap();
        let store = Arc::new(
            UsageHistoryStore::with_storage_path(directory.path().join("usage.sqlite")).unwrap(),
        );
        let recorder = UsageRecorder::new(store.clone());
        recorder.handle_event(foreground(1_000, r"C:\Apps\Editor.exe"));

        recorder.delete_history(at(2_000)).unwrap();
        recorder.checkpoint(at(3_000));

        let (_, measured_date) = local_measurement_context(2_000);
        let sessions = store.sessions_for_local_date(&measured_date).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].started_at_utc_ms, 2_000);
        assert_eq!(sessions[0].ended_at_utc_ms, 3_000);
        assert_eq!(sessions[0].end_reason, "checkpoint");
    }

    #[test]
    fn observation_and_subscription_failures_are_diagnostic() {
        let (_sink, recorder) = recorder();
        recorder.record_subscription_failure("hook failed".into());
        recorder.handle_event(DesktopEvent {
            observed_at: at(10),
            kind: DesktopEventKind::ForegroundChanged,
            process: None,
            failure: Some(ObservationFailure::ProcessOpenFailed(5)),
        });
        let diagnostics = recorder.diagnostics();
        assert_eq!(diagnostics.subscription_error, None);
        assert_eq!(
            diagnostics.observation_failure,
            Some(ObservationFailure::ProcessOpenFailed(5))
        );
        assert_eq!(
            recorder.health().status,
            MeasurementHealthStatus::ObservationDegraded
        );

        recorder.handle_event(foreground(20, r"C:\Apps\Editor.exe"));
        assert_eq!(recorder.health().status, MeasurementHealthStatus::Healthy);
    }

    #[test]
    fn persistence_health_recovers_after_a_successful_checkpoint() {
        let sink = Arc::new(RecoveringSink::default());
        sink.fail_next.store(true, Ordering::SeqCst);
        let recorder = UsageRecorder::with_sink(sink);
        recorder.handle_event(foreground(1_000, r"C:\Apps\Editor.exe"));
        recorder.checkpoint(at(2_000));
        assert_eq!(
            recorder.health().status,
            MeasurementHealthStatus::PersistenceFailed
        );

        recorder.checkpoint(at(3_000));
        assert_eq!(recorder.health().status, MeasurementHealthStatus::Healthy);
    }

    #[test]
    fn public_diagnostics_never_include_private_error_details() {
        let (_sink, recorder) = recorder();
        recorder.record_subscription_failure(
            "window title: Secret Document; https://private.example".to_string(),
        );

        let serialized = serde_json::to_string(&recorder.health()).unwrap();
        assert!(serialized.contains("event_subscription_failed"));
        assert!(!serialized.contains("Secret Document"));
        assert!(!serialized.contains("private.example"));
    }

    #[test]
    fn stale_events_do_not_rewind_the_active_subject() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(10_000, r"C:\Apps\Editor.exe"));
        recorder.handle_event(foreground(20_000, r"C:\Apps\Browser.exe"));
        recorder.handle_event(foreground(15_000, r"C:\Apps\Terminal.exe"));
        recorder.stop(at(30_000));

        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(
            sessions[1].subject,
            TrackedSubject::Identified(app_identity::resolve(&ProcessIdentity {
                process_id: 42,
                executable: PathBuf::from(r"C:\Apps\Browser.exe"),
                ..ProcessIdentity::default()
            }))
        );
        assert_eq!(sessions[1].ended_at_utc_ms, 30_000);
    }

    #[test]
    fn checkpoint_at_or_before_active_boundary_is_ignored() {
        let (sink, recorder) = recorder();
        recorder.handle_event(foreground(10_000, r"C:\Apps\Editor.exe"));
        recorder.checkpoint(at(10_000));
        recorder.checkpoint(at(9_000));
        recorder.stop(at(11_000));

        let sessions = sink.sessions.lock().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            (sessions[0].started_at_utc_ms, sessions[0].ended_at_utc_ms),
            (10_000, 11_000)
        );
    }
}
