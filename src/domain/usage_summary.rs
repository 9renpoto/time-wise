use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppUsageTotal {
    pub stable_key: Option<String>,
    pub display_name: String,
    pub icon_png: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HourlyUsageTotal {
    pub hour: u8,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageTotal {
    pub local_date: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageSummary {
    pub local_date: String,
    pub total_duration_ms: u64,
    pub applications: Vec<AppUsageTotal>,
    pub hourly_usage: Vec<HourlyUsageTotal>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyUsageSummary {
    pub week_start_local_date: String,
    pub week_end_local_date: String,
    pub total_duration_ms: u64,
    pub applications: Vec<AppUsageTotal>,
    pub daily_usage: Vec<DailyUsageTotal>,
}
