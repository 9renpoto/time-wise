//! Collects and serves startup timing metrics persisted in SQLite so the frontend can query them.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const MAX_RECORDS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Represents a single startup measurement in milliseconds.
pub struct StartupRecord {
    pub recorded_at_ms: u64,
    pub duration_ms: u64,
    pub launcher: String,
}

/// High-level manager that persists and serves startup metrics.
pub struct StartupMetrics {
    connection: Mutex<Connection>,
    recorded_once: AtomicBool,
}

impl StartupMetrics {
    /// Opens or creates the SQLite database at the provided path and runs migrations.
    pub fn with_storage_path(storage_path: PathBuf) -> Self {
        if let Some(parent) = storage_path.parent() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                eprintln!("failed to create startup metrics directory: {err}");
            }
        }

        let connection = match Connection::open(&storage_path).and_then(|connection| {
            Self::migrate(&connection)?;
            Ok(connection)
        }) {
            Ok(connection) => connection,
            Err(err) => {
                eprintln!("failed to open startup metrics database: {err}");
                let connection = Connection::open_in_memory()
                    .expect("failed to open in-memory sqlite connection");
                if let Err(migrate_err) = Self::migrate(&connection) {
                    eprintln!("failed to initialize in-memory database: {migrate_err}");
                }
                connection
            }
        };

        Self {
            connection: Mutex::new(connection),
            recorded_once: AtomicBool::new(false),
        }
    }

    /// Ensures the backing tables and indexes exist.
    fn migrate(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS startup_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                recorded_at_ms INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                launcher TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_startup_records_recorded_at
                ON startup_records(recorded_at_ms DESC);
            ",
        )?;

        Self::ensure_launcher_column(connection)
    }

    fn ensure_launcher_column(connection: &Connection) -> rusqlite::Result<()> {
        let mut statement = connection.prepare("PRAGMA table_info(startup_records)")?;
        let mut has_launcher_column = false;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        for name in columns.flatten() {
            if name == "launcher" {
                has_launcher_column = true;
                break;
            }
        }

        if !has_launcher_column {
            connection.execute("ALTER TABLE startup_records ADD COLUMN launcher TEXT", [])?;
            connection.execute(
                "UPDATE startup_records SET launcher = 'unknown' WHERE launcher IS NULL",
                [],
            )?;
        }

        Ok(())
    }

    /// Records the startup duration once per application run and trims the table to `MAX_RECORDS`.
    pub fn record_startup(
        &self,
        duration: Duration,
        launcher: String,
    ) -> Result<Option<StartupRecord>, String> {
        if self.recorded_once.swap(true, Ordering::SeqCst) {
            return Ok(None);
        }

        let duration_ms_clamped = duration.as_millis().min(i64::MAX as u128);
        let duration_ms = duration_ms_clamped as u64;
        let recorded_at_ms_clamped = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(i64::MAX as u128);
        let recorded_at_ms = recorded_at_ms_clamped as u64;

        let record = StartupRecord {
            recorded_at_ms,
            duration_ms,
            launcher: launcher.clone(),
        };

        let connection = self
            .connection
            .lock()
            .map_err(|_| "startup metrics mutex poisoned".to_string())?;

        connection
            .execute(
                "INSERT INTO startup_records (recorded_at_ms, duration_ms, launcher) VALUES (?1, ?2, ?3)",
                params![
                    recorded_at_ms_clamped as i64,
                    duration_ms_clamped as i64,
                    launcher
                ],
            )
            .map_err(|err| err.to_string())?;

        connection
            .execute(
                "DELETE FROM startup_records
                 WHERE id NOT IN (
                     SELECT id FROM startup_records
                     ORDER BY recorded_at_ms DESC
                     LIMIT ?1
                 )",
                params![MAX_RECORDS as i64],
            )
            .map_err(|err| err.to_string())?;

        Ok(Some(record))
    }

    /// Returns all available startup records ordered by most recent first.
    pub fn records(&self) -> Vec<StartupRecord> {
        let connection = match self.connection.lock() {
            Ok(connection) => connection,
            Err(_) => return Vec::new(),
        };

        let mut statement = match connection.prepare(
            "SELECT recorded_at_ms, duration_ms, launcher
             FROM startup_records
             ORDER BY recorded_at_ms DESC",
        ) {
            Ok(statement) => statement,
            Err(err) => {
                eprintln!("failed to read startup metrics: {err}");
                return Vec::new();
            }
        };

        let rows = match statement.query_map([], |row| {
            Ok(StartupRecord {
                recorded_at_ms: row.get::<_, i64>(0)?.max(0) as u64,
                duration_ms: row.get::<_, i64>(1)?.max(0) as u64,
                launcher: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "unknown".to_string()),
            })
        }) {
            Ok(rows) => rows,
            Err(err) => {
                eprintln!("failed to collect startup metrics: {err}");
                return Vec::new();
            }
        };

        rows.filter_map(Result::ok).collect()
    }
}

#[tauri::command]
/// Tauri command exposed to the frontend for retrieving startup metrics.
pub fn fetch_startup_records(state: tauri::State<'_, StartupMetrics>) -> Vec<StartupRecord> {
    state.records()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use std::time::Duration;

    #[test]
    fn records_are_trimmed_to_maximum() {
        let dir = tempfile::tempdir().unwrap();
        let storage_path = dir.path().join("records.sqlite");
        let metrics = StartupMetrics::with_storage_path(storage_path.clone());

        let seed_connection = Connection::open(&storage_path).unwrap();
        for index in 0..MAX_RECORDS + 5 {
            seed_connection
                .execute(
                    "INSERT INTO startup_records (recorded_at_ms, duration_ms, launcher) VALUES (?1, ?2, ?3)",
                    params![index as i64, 10i64, "seed"],
                )
                .unwrap();
        }

        metrics
            .record_startup(Duration::from_millis(10), "test".to_string())
            .unwrap();

        let records = metrics.records();
        assert_eq!(records.len(), MAX_RECORDS);

        let count: i64 = seed_connection
            .query_row("SELECT COUNT(*) FROM startup_records", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count as usize, MAX_RECORDS);
    }

    #[test]
    fn records_only_once_per_run() {
        let dir = tempfile::tempdir().unwrap();
        let storage_path = dir.path().join("records.sqlite");
        let metrics = StartupMetrics::with_storage_path(storage_path);

        assert!(metrics
            .record_startup(Duration::from_millis(5), "test".to_string())
            .unwrap()
            .is_some());
        assert!(metrics
            .record_startup(Duration::from_millis(5), "test".to_string())
            .unwrap()
            .is_none());
    }
}
