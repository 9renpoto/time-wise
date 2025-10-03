use crate::application::startup_service::format_duration;
#[cfg(target_arch = "wasm32")]
use crate::application::startup_service::format_timestamp;
use crate::domain::app_usage_record::AppUsageRecord;
use crate::presentation::models::UsageTile;

/// Builds the usage tiles shown in the dashboard from the recorder output.
pub fn compute_usage_tiles(records: &[AppUsageRecord]) -> Vec<UsageTile> {
    let mut items: Vec<_> = records.iter().collect();
    items.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| b.total_active_ms.cmp(&a.total_active_ms))
            .then_with(|| b.last_seen_at_ms.cmp(&a.last_seen_at_ms))
    });
    items
        .into_iter()
        .take(6)
        .map(|record| UsageTile {
            name: record.name.clone(),
            duration: format_duration(record.total_active_ms),
            subtitle: if record.active {
                "Active now".to_string()
            } else {
                format_last_active_label(record.last_seen_at_ms)
            },
            active: record.active,
        })
        .collect()
}

/// Counts applications that are currently marked active.
pub fn active_app_count(records: &[AppUsageRecord]) -> usize {
    records.iter().filter(|record| record.active).count()
}

/// Returns the timestamp string for the most recently observed application.
pub fn latest_usage_timestamp(records: &[AppUsageRecord]) -> Option<String> {
    records
        .iter()
        .max_by_key(|record| record.last_seen_at_ms)
        .map(|record| format_last_seen_human(record.last_seen_at_ms))
}

fn format_last_active_label(last_seen_ms: u64) -> String {
    format!("Last active {}", format_last_seen_human(last_seen_ms))
}

fn format_last_seen_human(last_seen_ms: u64) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        format_timestamp(last_seen_ms)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        format!("{} ms", last_seen_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &str, active: bool, total_ms: u64, last_seen: u64) -> AppUsageRecord {
        AppUsageRecord {
            name: name.to_string(),
            executable: None,
            total_active_ms: total_ms,
            last_seen_at_ms: last_seen,
            first_seen_at_ms: last_seen.saturating_sub(1_000),
            active,
        }
    }

    #[test]
    fn compute_usage_tiles_prioritizes_active_records() {
        let records = vec![
            record("Mail", false, 800, 20),
            record("Code", true, 1_200, 50),
            record("Music", true, 300, 40),
        ];

        let tiles = compute_usage_tiles(&records);
        assert_eq!(tiles.len(), 3);
        assert_eq!(tiles[0].name, "Code");
        assert!(tiles[0].active);
        assert_eq!(tiles[1].name, "Music");
        assert!(tiles[1].active);
        assert_eq!(tiles[2].name, "Mail");
        assert!(!tiles[2].active);
    }

    #[test]
    fn active_app_count_counts_active_entries() {
        let records = vec![
            record("Mail", false, 100, 10),
            record("Code", true, 200, 20),
        ];
        assert_eq!(active_app_count(&records), 1);
    }

    #[test]
    fn latest_usage_timestamp_returns_latest_formatted_timestamp() {
        let records = vec![
            record("Mail", false, 100, 1000),
            record("Code", true, 200, 2_000),
        ];

        let timestamp = latest_usage_timestamp(&records);
        assert!(timestamp.is_some());
    }
}
