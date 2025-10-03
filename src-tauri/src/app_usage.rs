use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

const STALE_ENTRY_GRACE: Duration = Duration::from_secs(5 * 60);

/// Interval used for polling running applications.
pub const APP_USAGE_POLL_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct AppUsageRecorder {
    inner: Arc<Mutex<AppUsageInner>>,
}

impl Default for AppUsageRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppUsageRecorder {
    #[must_use]
    pub fn new() -> Self {
        let refresh = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
        let system = System::new_with_specifics(refresh);
        Self {
            inner: Arc::new(Mutex::new(AppUsageInner::new(system))),
        }
    }

    pub fn record_current_processes(&self) -> Result<(), String> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| "app usage recorder mutex poisoned".to_string())?;
        guard.refresh_system();
        let snapshot = guard.collect_snapshot();
        let instant_now = Instant::now();
        let system_now = SystemTime::now();
        guard.apply_snapshot(&snapshot, instant_now, system_now);
        Ok(())
    }

    pub fn records(&self) -> Vec<AppUsageRecord> {
        self.records_internal(Instant::now(), SystemTime::now())
    }

    fn records_internal(
        &self,
        instant_now: Instant,
        system_now: SystemTime,
    ) -> Vec<AppUsageRecord> {
        let guard = match self.inner.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };

        let mut records: Vec<_> = guard
            .entries
            .values()
            .map(|entry| entry.to_record(instant_now, system_now))
            .filter(|record| record.total_active_ms > 0 || record.active)
            .collect();
        records.sort_by(|a, b| b.total_active_ms.cmp(&a.total_active_ms));
        records
    }

    #[cfg(test)]
    fn record_mock_snapshot(
        &self,
        snapshot: Vec<ProcessSnapshot>,
        instant_now: Instant,
        system_now: SystemTime,
    ) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.apply_snapshot(&snapshot, instant_now, system_now);
        }
    }

    #[cfg(test)]
    fn records_at(&self, instant_now: Instant, system_now: SystemTime) -> Vec<AppUsageRecord> {
        self.records_internal(instant_now, system_now)
    }
}

struct AppUsageInner {
    system: System,
    entries: HashMap<AppIdentity, AppUsageEntry>,
}

impl AppUsageInner {
    fn new(system: System) -> Self {
        Self {
            system,
            entries: HashMap::new(),
        }
    }

    fn refresh_system(&mut self) {
        self.system.refresh_processes();
    }

    fn collect_snapshot(&self) -> Vec<ProcessSnapshot> {
        self.system
            .processes()
            .values()
            .filter_map(ProcessSnapshot::from_process)
            .collect()
    }

    fn apply_snapshot(
        &mut self,
        snapshot: &[ProcessSnapshot],
        instant_now: Instant,
        system_now: SystemTime,
    ) {
        let mut observed: HashSet<AppIdentity> = HashSet::with_capacity(snapshot.len());

        for process in snapshot {
            observed.insert(process.identity.clone());
            let entry = self
                .entries
                .entry(process.identity.clone())
                .or_insert_with(|| AppUsageEntry::new(process.identity.clone(), system_now));
            entry.record_presence(instant_now, system_now);
        }

        for (identity, entry) in &mut self.entries {
            if !observed.contains(identity) {
                entry.mark_inactive(instant_now);
            }
        }

        self.entries.retain(|_, entry| {
            if entry.active {
                return true;
            }
            match system_now.duration_since(entry.last_seen) {
                Ok(elapsed) => elapsed <= STALE_ENTRY_GRACE,
                Err(_) => false,
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AppIdentity {
    name: String,
    executable: Option<PathBuf>,
}

#[derive(Debug)]
struct AppUsageEntry {
    identity: AppIdentity,
    accumulated: Duration,
    last_tick: Option<Instant>,
    first_seen: SystemTime,
    last_seen: SystemTime,
    active: bool,
}

impl AppUsageEntry {
    fn new(identity: AppIdentity, seen_at: SystemTime) -> Self {
        Self {
            identity,
            accumulated: Duration::default(),
            last_tick: None,
            first_seen: seen_at,
            last_seen: seen_at,
            active: false,
        }
    }

    fn record_presence(&mut self, instant_now: Instant, system_now: SystemTime) {
        let was_active = self.active;
        if let Some(last_tick) = self.last_tick {
            if was_active {
                let delta = instant_now.saturating_duration_since(last_tick);
                self.accumulated += delta;
            }
        }
        self.last_tick = Some(instant_now);
        self.last_seen = system_now;
        self.active = true;
    }

    fn mark_inactive(&mut self, instant_now: Instant) {
        if self.active {
            if let Some(last_tick) = self.last_tick {
                self.accumulated += instant_now.saturating_duration_since(last_tick);
            }
        }
        self.active = false;
        self.last_tick = Some(instant_now);
    }

    fn to_record(&self, instant_now: Instant, _system_now: SystemTime) -> AppUsageRecord {
        let mut total = self.accumulated;
        if self.active {
            if let Some(last_tick) = self.last_tick {
                total += instant_now.saturating_duration_since(last_tick);
            }
        }

        AppUsageRecord {
            name: self.identity.name.clone(),
            executable: self
                .identity
                .executable
                .as_ref()
                .map(|path| path.display().to_string()),
            total_active_ms: duration_to_ms(total),
            last_seen_at_ms: system_time_to_ms(self.last_seen),
            active: self.active,
            first_seen_at_ms: system_time_to_ms(self.first_seen),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsageRecord {
    pub name: String,
    pub executable: Option<String>,
    pub total_active_ms: u64,
    pub last_seen_at_ms: u64,
    pub first_seen_at_ms: u64,
    pub active: bool,
}

#[derive(Clone)]
struct ProcessSnapshot {
    identity: AppIdentity,
}

impl ProcessSnapshot {
    fn from_process(process: &sysinfo::Process) -> Option<Self> {
        if !should_track_process(process) {
            return None;
        }

        let name = process.name().trim();
        if name.is_empty() {
            return None;
        }

        let executable = executable_from_process(process);

        Some(Self {
            identity: AppIdentity {
                name: name.to_string(),
                executable,
            },
        })
    }

    #[cfg(test)]
    fn for_tests(name: &str, executable: Option<&str>) -> Self {
        Self {
            identity: AppIdentity {
                name: name.to_string(),
                executable: executable.map(PathBuf::from),
            },
        }
    }
}

fn executable_from_process(process: &sysinfo::Process) -> Option<PathBuf> {
    let path = process.exe()?;
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path.to_path_buf())
    }
}

#[cfg(target_os = "macos")]
fn should_track_process(process: &sysinfo::Process) -> bool {
    let Some(path) = process.exe() else {
        return false;
    };
    if path.as_os_str().is_empty() {
        return false;
    }
    if let Some(path_str) = path.to_str() {
        return path_str.contains(".app/") && !path_str.contains("/System/");
    }
    false
}

#[cfg(target_os = "windows")]
fn should_track_process(process: &sysinfo::Process) -> bool {
    let Some(path) = process.exe() else {
        return false;
    };
    if path.as_os_str().is_empty() {
        return false;
    }
    if let Some(path_str) = path.to_str() {
        let lower = path_str.to_ascii_lowercase();
        return lower.ends_with(".exe") && !lower.contains("\\windows\\");
    }
    false
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn should_track_process(process: &sysinfo::Process) -> bool {
    !process.name().trim().is_empty()
}

fn duration_to_ms(duration: Duration) -> u64 {
    duration
        .as_millis()
        .min(u64::MAX as u128)
        .try_into()
        .unwrap_or(u64::MAX)
}

fn system_time_to_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128)
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_usage_across_snapshots() {
        let recorder = AppUsageRecorder::new();
        let instant_start = Instant::now();
        let system_start = SystemTime::now();

        recorder.record_mock_snapshot(
            vec![ProcessSnapshot::for_tests(
                "Focus",
                Some("/Applications/Focus.app/Contents/MacOS/Focus"),
            )],
            instant_start,
            system_start,
        );

        let instant_next = instant_start + Duration::from_secs(5);
        let system_next = system_start + Duration::from_secs(5);

        recorder.record_mock_snapshot(
            vec![ProcessSnapshot::for_tests(
                "Focus",
                Some("/Applications/Focus.app/Contents/MacOS/Focus"),
            )],
            instant_next,
            system_next,
        );

        let records = recorder.records_at(instant_next, system_next);
        let record = records
            .iter()
            .find(|entry| entry.name == "Focus")
            .expect("record should exist");
        assert_eq!(record.total_active_ms, 5_000);
        assert!(record.active);

        let instant_end = instant_next + Duration::from_secs(5);
        let system_end = system_next + Duration::from_secs(5);

        recorder.record_mock_snapshot(Vec::new(), instant_end, system_end);

        let records = recorder.records_at(instant_end + Duration::from_secs(5), system_end);
        let record = records
            .iter()
            .find(|entry| entry.name == "Focus")
            .expect("record should persist");
        assert_eq!(record.total_active_ms, 10_000);
        assert!(!record.active);
    }

    #[test]
    fn records_reports_tracked_processes() {
        let recorder = AppUsageRecorder::new();
        let instant_start = Instant::now();
        let system_start = SystemTime::now();

        recorder.record_mock_snapshot(
            vec![ProcessSnapshot::for_tests(
                "Focus",
                Some("/Applications/Focus.app/Contents/MacOS/Focus"),
            )],
            instant_start,
            system_start,
        );

        std::thread::sleep(Duration::from_millis(25));

        let instant_end = Instant::now();
        let system_end = SystemTime::now();
        recorder.record_mock_snapshot(Vec::new(), instant_end, system_end);

        let records = recorder.records();
        let record = records
            .iter()
            .find(|entry| entry.name == "Focus")
            .expect("record should exist after polling");
        assert!(record.total_active_ms >= 20);
        assert!(!record.active);
    }
}
