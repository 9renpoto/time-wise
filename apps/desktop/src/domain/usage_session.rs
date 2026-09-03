#![allow(dead_code)]
use super::app_identity::UsageSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptionReason {
    SessionLocked,
    SystemSuspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionEndReason {
    FocusChanged,
    Interrupted(InterruptionReason),
    MeasurementStopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedUsageSession {
    pub subject: UsageSubject,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
    pub end_reason: SessionEndReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageSession {
    subject: UsageSubject,
    started_at_ms: u64,
}

impl UsageSession {
    #[must_use]
    pub fn new(subject: UsageSubject, started_at_ms: u64) -> Self {
        Self {
            subject,
            started_at_ms,
        }
    }
    pub fn finish(
        self,
        ended_at_ms: u64,
        end_reason: SessionEndReason,
    ) -> Result<CompletedUsageSession, SessionTimeError> {
        if ended_at_ms < self.started_at_ms {
            return Err(SessionTimeError::EndBeforeStart);
        }
        Ok(CompletedUsageSession {
            subject: self.subject,
            started_at_ms: self.started_at_ms,
            ended_at_ms,
            end_reason,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionTimeError {
    EndBeforeStart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementState {
    Idle,
    Measuring(UsageSession),
    Interrupted {
        since_ms: u64,
        reason: InterruptionReason,
    },
}

impl MeasurementState {
    pub fn start(&mut self, subject: UsageSubject, at_ms: u64) {
        *self = Self::Measuring(UsageSession::new(subject, at_ms));
    }
    pub fn interrupt(
        &mut self,
        at_ms: u64,
        reason: InterruptionReason,
    ) -> Result<Option<CompletedUsageSession>, SessionTimeError> {
        let previous = std::mem::replace(
            self,
            Self::Interrupted {
                since_ms: at_ms,
                reason,
            },
        );
        match previous {
            Self::Measuring(session) => session
                .finish(at_ms, SessionEndReason::Interrupted(reason))
                .map(Some),
            _ => Ok(None),
        }
    }
    pub fn resume(&mut self) {
        if matches!(self, Self::Interrupted { .. }) {
            *self = Self::Idle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::app_identity::AppIdentity;
    fn subject() -> UsageSubject {
        UsageSubject::Identified(AppIdentity::new("product:editor").unwrap())
    }

    #[test]
    fn session_boundaries_are_deterministic() {
        let done = UsageSession::new(subject(), 1_000)
            .finish(4_000, SessionEndReason::FocusChanged)
            .unwrap();
        assert_eq!((done.started_at_ms, done.ended_at_ms), (1_000, 4_000));
    }

    #[test]
    fn end_before_start_is_rejected() {
        assert_eq!(
            UsageSession::new(UsageSubject::Unclassified, 2_000)
                .finish(1_999, SessionEndReason::MeasurementStopped),
            Err(SessionTimeError::EndBeforeStart)
        );
    }

    #[test]
    fn lock_and_suspend_interrupt_sessions() {
        for reason in [
            InterruptionReason::SessionLocked,
            InterruptionReason::SystemSuspended,
        ] {
            let mut state = MeasurementState::Idle;
            state.start(subject(), 10);
            let done = state.interrupt(20, reason).unwrap().unwrap();
            assert_eq!(done.end_reason, SessionEndReason::Interrupted(reason));
            state.resume();
            assert_eq!(state, MeasurementState::Idle);
        }
    }
}
