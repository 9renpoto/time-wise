//! SQLite persistence for application usage history.

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::secure_storage::{default_key_store, load_or_create, KeyStore, USAGE_HISTORY_KEY_ID};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMetadata {
    pub stable_key: String,
    pub display_name: String,
    pub executable: Option<String>,
    pub icon_source: Option<String>,
    pub icon_png: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageSubject<'a> {
    Identified(&'a AppMetadata),
    Unclassified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUsageSession<'a> {
    pub subject: UsageSubject<'a>,
    pub started_at_utc_ms: u64,
    pub ended_at_utc_ms: u64,
    pub measured_timezone: &'a str,
    pub measured_local_date: &'a str,
    pub end_reason: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredUsageSession {
    pub stable_key: Option<String>,
    pub display_name: Option<String>,
    pub executable: Option<String>,
    pub icon_source: Option<String>,
    pub icon_png: Option<Vec<u8>>,
    pub started_at_utc_ms: u64,
    pub ended_at_utc_ms: u64,
    pub measured_timezone: String,
    pub measured_local_date: String,
    pub end_reason: String,
}

pub struct UsageHistoryStore {
    connection: Mutex<Connection>,
}

impl UsageHistoryStore {
    pub fn with_storage_path(path: PathBuf) -> Result<Self, String> {
        Self::with_storage_path_and_key_store(path, default_key_store())
    }

    pub fn with_storage_path_and_key_store(
        path: PathBuf,
        key_store: Arc<dyn KeyStore>,
    ) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        discard_unencrypted_database(&path)?;
        let key = load_or_create(key_store.as_ref(), USAGE_HISTORY_KEY_ID)?;
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        // SQLCipher is compiled into rusqlite for every supported desktop target.
        // The key is applied through the native API before SQLite reads page 1.
        let result = unsafe {
            rusqlite::ffi::sqlite3_key(
                connection.handle(),
                key.as_ptr().cast(),
                key.len()
                    .try_into()
                    .map_err(|_| "database key is too long")?,
            )
        };
        if result != rusqlite::ffi::SQLITE_OK {
            return Err(format!(
                "failed to unlock encrypted usage database (SQLite code {result})"
            ));
        }
        connection
            .execute_batch("PRAGMA foreign_keys = ON; SELECT count(*) FROM sqlite_master;")
            .map_err(|error| error.to_string())?;
        Self::migrate(&connection).map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn migrate(connection: &Connection) -> rusqlite::Result<()> {
        connection.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_utc_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS app_identities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                stable_key TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                executable TEXT,
                icon_source TEXT,
                icon_png BLOB,
                updated_at_utc_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS usage_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                app_identity_id INTEGER,
                subject_kind TEXT NOT NULL CHECK (subject_kind IN ('identified', 'unclassified')),
                started_at_utc_ms INTEGER NOT NULL,
                ended_at_utc_ms INTEGER NOT NULL,
                measured_timezone TEXT NOT NULL,
                measured_local_date TEXT NOT NULL,
                end_reason TEXT NOT NULL,
                CHECK (ended_at_utc_ms >= started_at_utc_ms),
                CHECK (
                    (subject_kind = 'identified' AND app_identity_id IS NOT NULL)
                    OR (subject_kind = 'unclassified' AND app_identity_id IS NULL)
                ),
                FOREIGN KEY (app_identity_id) REFERENCES app_identities(id)
            );
            CREATE INDEX IF NOT EXISTS idx_usage_sessions_local_date
                ON usage_sessions(measured_local_date, started_at_utc_ms);
            CREATE INDEX IF NOT EXISTS idx_usage_sessions_app
                ON usage_sessions(app_identity_id, started_at_utc_ms);
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at_utc_ms INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO schema_migrations (version, applied_at_utc_ms)
                VALUES (1, CAST(unixepoch('subsec') * 1000 AS INTEGER));
            COMMIT;",
        )?;

        Self::ensure_app_metadata_column(connection, "icon_source", "TEXT", 2)?;
        Self::ensure_app_metadata_column(connection, "icon_png", "BLOB", 3)?;
        Ok(())
    }

    fn ensure_app_metadata_column(
        connection: &Connection,
        column: &str,
        sql_type: &str,
        version: i64,
    ) -> rusqlite::Result<()> {
        let columns = connection
            .prepare("PRAGMA table_info(app_identities)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if !columns.iter().any(|existing| existing == column) {
            connection.execute(
                &format!("ALTER TABLE app_identities ADD COLUMN {column} {sql_type}"),
                [],
            )?;
        }
        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at_utc_ms)
                VALUES (?1, CAST(unixepoch('subsec') * 1000 AS INTEGER))",
            [version],
        )?;
        Ok(())
    }

    pub fn record_session(&self, session: &NewUsageSession<'_>) -> Result<i64, String> {
        validate_session(session)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "usage history mutex poisoned".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let (kind, app_id) = match session.subject {
            UsageSubject::Identified(metadata) => (
                "identified",
                Some(Self::upsert_app(
                    &transaction,
                    metadata,
                    session.ended_at_utc_ms,
                )?),
            ),
            UsageSubject::Unclassified => ("unclassified", None),
        };
        transaction
            .execute(
                "INSERT INTO usage_sessions (
                app_identity_id, subject_kind, started_at_utc_ms, ended_at_utc_ms,
                measured_timezone, measured_local_date, end_reason
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    app_id,
                    kind,
                    sql_timestamp(session.started_at_utc_ms)?,
                    sql_timestamp(session.ended_at_utc_ms)?,
                    session.measured_timezone,
                    session.measured_local_date,
                    session.end_reason
                ],
            )
            .map_err(|error| error.to_string())?;
        let id = transaction.last_insert_rowid();
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(id)
    }

    fn upsert_app(
        transaction: &Transaction<'_>,
        metadata: &AppMetadata,
        updated_at_utc_ms: u64,
    ) -> Result<i64, String> {
        non_empty("stable key", &metadata.stable_key)?;
        non_empty("display name", &metadata.display_name)?;
        transaction
            .execute(
                "INSERT INTO app_identities (
                stable_key, display_name, executable, icon_source, icon_png, updated_at_utc_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(stable_key) DO UPDATE SET
                display_name = excluded.display_name,
                executable = excluded.executable,
                icon_source = COALESCE(excluded.icon_source, app_identities.icon_source),
                icon_png = COALESCE(excluded.icon_png, app_identities.icon_png),
                updated_at_utc_ms = excluded.updated_at_utc_ms",
                params![
                    metadata.stable_key,
                    metadata.display_name,
                    metadata.executable,
                    metadata.icon_source,
                    metadata.icon_png,
                    sql_timestamp(updated_at_utc_ms)?
                ],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .query_row(
                "SELECT id FROM app_identities WHERE stable_key = ?1",
                params![metadata.stable_key],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())
    }

    pub fn sessions_for_local_date(&self, date: &str) -> Result<Vec<StoredUsageSession>, String> {
        self.sessions_for_local_date_range(date, date)
    }

    pub fn sessions_for_local_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<StoredUsageSession>, String> {
        if end_date < start_date {
            return Err("usage history date range ends before it starts".to_string());
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "usage history mutex poisoned".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT apps.stable_key, apps.display_name, apps.executable, apps.icon_source,
                apps.icon_png,
                sessions.started_at_utc_ms, sessions.ended_at_utc_ms,
                sessions.measured_timezone, sessions.measured_local_date, sessions.end_reason
             FROM usage_sessions AS sessions
             LEFT JOIN app_identities AS apps ON apps.id = sessions.app_identity_id
             WHERE sessions.measured_local_date BETWEEN ?1 AND ?2
             ORDER BY sessions.measured_local_date, sessions.started_at_utc_ms, sessions.id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![start_date, end_date], |row| {
                Ok(StoredUsageSession {
                    stable_key: row.get(0)?,
                    display_name: row.get(1)?,
                    executable: row.get(2)?,
                    icon_source: row.get(3)?,
                    icon_png: row.get(4)?,
                    started_at_utc_ms: row.get::<_, i64>(5)?.max(0) as u64,
                    ended_at_utc_ms: row.get::<_, i64>(6)?.max(0) as u64,
                    measured_timezone: row.get(7)?,
                    measured_local_date: row.get(8)?,
                    end_reason: row.get(9)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str, at_utc_ms: u64) -> Result<(), String> {
        non_empty("setting key", key)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "usage history mutex poisoned".to_string())?;
        connection
            .execute(
                "INSERT INTO settings (key, value, updated_at_utc_ms) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value,
                updated_at_utc_ms = excluded.updated_at_utc_ms",
                params![key, value, sql_timestamp(at_utc_ms)?],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "usage history mutex poisoned".to_string())?;
        connection
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    /// Removes all measured usage while preserving application settings.
    pub fn delete_all_usage_history(&self) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "usage history mutex poisoned".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM usage_sessions", [])
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM app_identities", [])
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }
}

fn discard_unencrypted_database(path: &std::path::Path) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.starts_with(b"SQLite format 3\0") {
        std::fs::remove_file(path)
            .map_err(|error| format!("failed to discard unencrypted usage database: {error}"))?;
    }
    Ok(())
}

fn validate_session(session: &NewUsageSession<'_>) -> Result<(), String> {
    if session.ended_at_utc_ms < session.started_at_utc_ms {
        return Err("usage session ends before it starts".to_string());
    }
    non_empty("measured timezone", session.measured_timezone)?;
    non_empty("measured local date", session.measured_local_date)?;
    non_empty("end reason", session.end_reason)
}

fn non_empty(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} cannot be empty"))
    } else {
        Ok(())
    }
}

fn sql_timestamp(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| "timestamp exceeds SQLite INTEGER range".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secure_storage::MemoryKeyStore;

    fn store() -> (tempfile::TempDir, UsageHistoryStore) {
        let directory = tempfile::tempdir().unwrap();
        let store =
            UsageHistoryStore::with_storage_path(directory.path().join("usage.sqlite")).unwrap();
        (directory, store)
    }

    #[test]
    fn encrypted_database_can_be_reopened_with_the_stored_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let key_store = Arc::new(MemoryKeyStore::new(None));
        let store =
            UsageHistoryStore::with_storage_path_and_key_store(path.clone(), key_store.clone())
                .unwrap();
        store.set_setting("secret", "known-value", 1).unwrap();
        drop(store);

        let reopened = UsageHistoryStore::with_storage_path_and_key_store(path, key_store).unwrap();
        assert_eq!(
            reopened.setting("secret").unwrap().as_deref(),
            Some("known-value")
        );
    }

    #[test]
    fn encrypted_database_rejects_a_different_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let first_key = Arc::new(MemoryKeyStore::new(Some(vec![7; 32])));
        UsageHistoryStore::with_storage_path_and_key_store(path.clone(), first_key).unwrap();

        let different_key = Arc::new(MemoryKeyStore::new(Some(vec![8; 32])));
        let result = UsageHistoryStore::with_storage_path_and_key_store(path, different_key);
        assert!(result.is_err());
    }

    #[test]
    fn credential_store_failure_does_not_create_a_database() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let result = UsageHistoryStore::with_storage_path_and_key_store(
            path.clone(),
            Arc::new(MemoryKeyStore::failing()),
        );
        assert!(result.is_err());
        assert!(!path.exists());
    }

    #[test]
    fn existing_plaintext_database_is_discarded() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let plain = Connection::open(&path).unwrap();
        plain
            .execute_batch(
                "CREATE TABLE leaked (value TEXT); INSERT INTO leaked VALUES ('known-value');",
            )
            .unwrap();
        drop(plain);

        let store = UsageHistoryStore::with_storage_path_and_key_store(
            path.clone(),
            Arc::new(MemoryKeyStore::new(None)),
        )
        .unwrap();
        assert!(store.setting("secret").unwrap().is_none());
        let bytes = std::fs::read(path).unwrap();
        assert!(!bytes
            .windows(b"known-value".len())
            .any(|window| window == b"known-value"));
    }

    #[test]
    fn regular_sqlite_cannot_read_encrypted_content() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let store = UsageHistoryStore::with_storage_path_and_key_store(
            path.clone(),
            Arc::new(MemoryKeyStore::new(Some(vec![9; 32]))),
        )
        .unwrap();
        store.set_setting("secret", "known-value", 1).unwrap();
        drop(store);

        let regular_connection = Connection::open(path).unwrap();
        assert!(regular_connection
            .query_row("SELECT value FROM settings", [], |row| row
                .get::<_, String>(0))
            .is_err());
    }

    #[test]
    fn persists_identified_and_unclassified_sessions() {
        let (_directory, store) = store();
        let app = AppMetadata {
            stable_key: "product:editor".into(),
            display_name: "Editor".into(),
            executable: Some("editor.exe".into()),
            icon_source: Some("editor.exe".into()),
            icon_png: Some(vec![137, 80, 78, 71]),
        };
        store
            .record_session(&NewUsageSession {
                subject: UsageSubject::Identified(&app),
                started_at_utc_ms: 1_000,
                ended_at_utc_ms: 4_000,
                measured_timezone: "Asia/Tokyo",
                measured_local_date: "2026-07-31",
                end_reason: "focus_changed",
            })
            .unwrap();
        store
            .record_session(&NewUsageSession {
                subject: UsageSubject::Unclassified,
                started_at_utc_ms: 4_000,
                ended_at_utc_ms: 5_000,
                measured_timezone: "Asia/Tokyo",
                measured_local_date: "2026-07-31",
                end_reason: "measurement_stopped",
            })
            .unwrap();
        let sessions = store.sessions_for_local_date("2026-07-31").unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].stable_key.as_deref(), Some("product:editor"));
        assert_eq!(sessions[0].display_name.as_deref(), Some("Editor"));
        assert_eq!(sessions[0].icon_source.as_deref(), Some("editor.exe"));
        assert_eq!(
            sessions[0].icon_png.as_deref(),
            Some([137, 80, 78, 71].as_slice())
        );
        assert_eq!(sessions[1].stable_key, None);
    }

    #[test]
    fn updates_metadata_for_a_stable_identity() {
        let (_directory, store) = store();
        let first = AppMetadata {
            stable_key: "product:browser".into(),
            display_name: "Old Name".into(),
            executable: None,
            icon_source: None,
            icon_png: None,
        };
        let updated = AppMetadata {
            stable_key: "product:browser".into(),
            display_name: "Browser".into(),
            executable: Some("browser.exe".into()),
            icon_source: Some("browser.exe".into()),
            icon_png: Some(vec![1, 2, 3]),
        };
        for (app, start) in [(&first, 10), (&updated, 20)] {
            store
                .record_session(&NewUsageSession {
                    subject: UsageSubject::Identified(app),
                    started_at_utc_ms: start,
                    ended_at_utc_ms: start + 10,
                    measured_timezone: "UTC",
                    measured_local_date: "2026-07-31",
                    end_reason: "focus_changed",
                })
                .unwrap();
        }
        let sessions = store.sessions_for_local_date("2026-07-31").unwrap();
        assert!(sessions
            .iter()
            .all(|session| session.display_name.as_deref() == Some("Browser")));
        assert!(sessions
            .iter()
            .all(|session| session.executable.as_deref() == Some("browser.exe")));
        assert!(sessions
            .iter()
            .all(|session| session.icon_png.as_deref() == Some([1, 2, 3].as_slice())));
    }

    #[test]
    fn settings_are_upserted() {
        let (_directory, store) = store();
        store.set_setting("autostart", "false", 10).unwrap();
        store.set_setting("autostart", "true", 20).unwrap();
        assert_eq!(store.setting("autostart").unwrap().as_deref(), Some("true"));
    }

    #[test]
    fn deleting_history_preserves_settings_and_removes_identity_metadata() {
        let (_directory, store) = store();
        let app = AppMetadata {
            stable_key: "product:editor".into(),
            display_name: "Editor".into(),
            executable: Some("editor.exe".into()),
            icon_source: None,
            icon_png: None,
        };
        store
            .record_session(&NewUsageSession {
                subject: UsageSubject::Identified(&app),
                started_at_utc_ms: 1_000,
                ended_at_utc_ms: 2_000,
                measured_timezone: "UTC",
                measured_local_date: "2026-08-04",
                end_reason: "focus_changed",
            })
            .unwrap();
        store
            .set_setting("onboarding_completed", "true", 2_000)
            .unwrap();

        store.delete_all_usage_history().unwrap();

        assert!(store
            .sessions_for_local_date("2026-08-04")
            .unwrap()
            .is_empty());
        assert_eq!(
            store.setting("onboarding_completed").unwrap().as_deref(),
            Some("true")
        );
        let connection = store.connection.lock().unwrap();
        let identity_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM app_identities", [], |row| row.get(0))
            .unwrap();
        assert_eq!(identity_count, 0);
    }

    #[test]
    fn migrates_existing_identity_metadata_cache() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("usage.sqlite");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_utc_ms INTEGER NOT NULL
                );
                CREATE TABLE app_identities (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    stable_key TEXT NOT NULL UNIQUE,
                    display_name TEXT NOT NULL,
                    executable TEXT,
                    updated_at_utc_ms INTEGER NOT NULL
                );
                INSERT INTO schema_migrations VALUES (1, 0);",
            )
            .unwrap();
        drop(connection);

        let store = UsageHistoryStore::with_storage_path(path).unwrap();
        let connection = store.connection.lock().unwrap();
        let columns = connection
            .prepare("PRAGMA table_info(app_identities)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(columns.iter().any(|column| column == "icon_source"));
        assert!(columns.iter().any(|column| column == "icon_png"));
    }

    #[test]
    fn rejects_invalid_session_boundaries() {
        let (_directory, store) = store();
        let result = store.record_session(&NewUsageSession {
            subject: UsageSubject::Unclassified,
            started_at_utc_ms: 2,
            ended_at_utc_ms: 1,
            measured_timezone: "UTC",
            measured_local_date: "2026-07-31",
            end_reason: "measurement_stopped",
        });
        assert_eq!(result.unwrap_err(), "usage session ends before it starts");
    }
}
