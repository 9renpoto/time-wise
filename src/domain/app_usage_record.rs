use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AppUsageRecord {
    pub name: String,
    pub executable: Option<String>,
    pub total_active_ms: u64,
    pub last_seen_at_ms: u64,
    pub first_seen_at_ms: u64,
    pub active: bool,
}
