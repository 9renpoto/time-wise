use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StartupRecord {
    pub recorded_at_ms: u64,
    pub duration_ms: u64,
    pub launcher: String,
}
