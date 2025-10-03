#![allow(clippy::module_name_repetitions)]

use js_sys::Date;
use wasm_bindgen::JsValue;

use crate::domain::startup_record::StartupRecord;
use crate::presentation::models::{CategorySummary, ChartPoint, StartupTile};

/// Builds the chart points from the latest samples.
pub fn compute_chart_points(records: &[StartupRecord]) -> Vec<ChartPoint> {
    let mut points: Vec<ChartPoint> = records
        .iter()
        .take(5)
        .map(|record| ChartPoint {
            label: format_time_of_day(record.recorded_at_ms),
            duration_ms: record.duration_ms,
        })
        .collect();

    points.reverse();

    while points.len() < 5 {
        points.insert(
            0,
            ChartPoint {
                label: "-".to_string(),
                duration_ms: 0,
            },
        );
    }

    points
}

/// Summarizes runs into fast, steady, slow buckets.
pub fn compute_category_summary(records: &[StartupRecord]) -> Vec<CategorySummary> {
    let mut fast: (u64, usize) = (0, 0);
    let mut steady: (u64, usize) = (0, 0);
    let mut slow: (u64, usize) = (0, 0);

    for record in records {
        match record.duration_ms {
            0..=500 => {
                fast.0 += record.duration_ms;
                fast.1 += 1;
            }
            501..=1_500 => {
                steady.0 += record.duration_ms;
                steady.1 += 1;
            }
            _ => {
                slow.0 += record.duration_ms;
                slow.1 += 1;
            }
        }
    }

    vec![
        CategorySummary {
            name: "Fast starts (<0.5s)",
            class_names: "app__category-name app__category-name--social",
            summary: summarize_bucket(fast.0, fast.1),
        },
        CategorySummary {
            name: "Steady starts (0.5–1.5s)",
            class_names: "app__category-name app__category-name--utilities",
            summary: summarize_bucket(steady.0, steady.1),
        },
        CategorySummary {
            name: "Slow starts (>1.5s)",
            class_names: "app__category-name app__category-name--health",
            summary: summarize_bucket(slow.0, slow.1),
        },
    ]
}

/// Formats the bucket label with average duration.
fn summarize_bucket(total_ms: u64, count: usize) -> String {
    if count == 0 {
        "No runs yet".to_string()
    } else {
        let average = total_ms / count as u64;
        let runs_label = if count == 1 { "run" } else { "runs" };
        format!(
            "{} avg · {} {}",
            format_duration(average),
            count,
            runs_label
        )
    }
}

/// Builds the tile grid from the latest runs.
pub fn compute_tiles(records: &[StartupRecord]) -> Vec<StartupTile> {
    records
        .iter()
        .take(6)
        .map(|record| StartupTile {
            icon: duration_icon(record.duration_ms),
            label: format_time_of_day(record.recorded_at_ms),
            duration: format_duration(record.duration_ms),
        })
        .collect()
}

/// Chooses an icon matching the duration bucket.
fn duration_icon(duration_ms: u64) -> &'static str {
    match duration_ms {
        0..=500 => "⚡",
        501..=1_500 => "🚀",
        _ => "🐢",
    }
}

/// Formats the total startup duration for the header.
pub fn format_total_duration(total_ms: u64) -> String {
    if total_ms == 0 {
        return "0 ms".to_string();
    }
    if total_ms >= 3_600_000 {
        format!("{:.1} h", total_ms as f64 / 3_600_000.0)
    } else if total_ms >= 60_000 {
        format!("{:.1} m", total_ms as f64 / 60_000.0)
    } else if total_ms >= 1_000 {
        format!("{:.1} s", total_ms as f64 / 1_000.0)
    } else {
        format!("{total_ms} ms")
    }
}

/// Compact human-readable duration.
pub fn format_duration_compact(ms: u64) -> String {
    if ms == 0 {
        "0".to_string()
    } else if ms >= 60_000 {
        format!("{:.1} m", ms as f64 / 60_000.0)
    } else if ms >= 1_000 {
        format!("{:.1} s", ms as f64 / 1_000.0)
    } else {
        format!("{ms} ms")
    }
}

pub fn format_duration(ms: u64) -> String {
    if ms >= 1_000 {
        format!("{:.2} s", ms as f64 / 1_000.0)
    } else {
        format!("{ms} ms")
    }
}

/// Formats the timestamp into a locale-aware date string.
pub fn format_timestamp(ms: u64) -> String {
    let date = Date::new(&JsValue::from_f64(ms as f64));
    Date::to_locale_string(&date, "default", &JsValue::UNDEFINED).into()
}

/// Formats the timestamp into a locale-aware time string.
fn format_time_of_day(ms: u64) -> String {
    let date = Date::new(&JsValue::from_f64(ms as f64));
    Date::to_locale_time_string(&date, "default").into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::startup_record::StartupRecord;

    #[test]
    fn summarize_bucket_reports_no_runs_when_empty() {
        assert_eq!(summarize_bucket(0, 0), "No runs yet");
    }

    #[test]
    fn summarize_bucket_formats_average_with_pluralization() {
        let summary = summarize_bucket(1_500, 3);
        assert_eq!(summary, "500 ms avg · 3 runs");

        let single_run = summarize_bucket(2_000, 1);
        assert_eq!(single_run, "2.00 s avg · 1 run");
    }

    #[test]
    fn compute_category_summary_groups_records_into_buckets() {
        let records = vec![
            StartupRecord {
                recorded_at_ms: 10,
                duration_ms: 300,
                launcher: "test".to_string(),
            },
            StartupRecord {
                recorded_at_ms: 20,
                duration_ms: 800,
                launcher: "test".to_string(),
            },
            StartupRecord {
                recorded_at_ms: 30,
                duration_ms: 2_200,
                launcher: "test".to_string(),
            },
        ];

        let summary = compute_category_summary(&records);

        assert_eq!(summary[0].name, "Fast starts (<0.5s)");
        assert_eq!(summary[0].summary, "300 ms avg · 1 run");

        assert_eq!(summary[1].name, "Steady starts (0.5–1.5s)");
        assert_eq!(summary[1].summary, "800 ms avg · 1 run");

        assert_eq!(summary[2].name, "Slow starts (>1.5s)");
        assert_eq!(summary[2].summary, "2.20 s avg · 1 run");
    }

    #[test]
    fn duration_icon_matches_duration_bucket() {
        assert_eq!(duration_icon(100), "⚡");
        assert_eq!(duration_icon(1_000), "🚀");
        assert_eq!(duration_icon(5_000), "🐢");
    }

    #[test]
    fn format_total_duration_ranges_are_human_readable() {
        assert_eq!(format_total_duration(0), "0 ms");
        assert_eq!(format_total_duration(900), "900 ms");
        assert_eq!(format_total_duration(1_500), "1.5 s");
        assert_eq!(format_total_duration(90_000), "1.5 m");
        assert_eq!(format_total_duration(7_200_000), "2.0 h");
    }

    #[test]
    fn format_duration_compact_scales_units() {
        assert_eq!(format_duration_compact(0), "0");
        assert_eq!(format_duration_compact(750), "750 ms");
        assert_eq!(format_duration_compact(1_200), "1.2 s");
        assert_eq!(format_duration_compact(120_000), "2.0 m");
    }

    #[test]
    fn format_duration_retains_precision_for_seconds() {
        assert_eq!(format_duration(500), "500 ms");
        assert_eq!(format_duration(2_345), "2.35 s");
    }
}
