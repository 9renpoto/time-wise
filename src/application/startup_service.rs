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
