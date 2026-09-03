//! Dashboard-oriented daily and weekly usage aggregation.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Days, NaiveDate};
use serde::Serialize;
use tauri::State;

use crate::usage_history::{StoredUsageSession, UsageHistoryStore};

const HOURS_PER_DAY: usize = 24;
const DAYS_PER_WEEK: u64 = 7;
const MILLIS_PER_MINUTE: i128 = 60_000;
const MILLIS_PER_HOUR: i128 = 60 * MILLIS_PER_MINUTE;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUsageTotal {
    pub stable_key: Option<String>,
    pub display_name: String,
    pub icon_png: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourlyUsageTotal {
    pub hour: u8,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageTotal {
    pub local_date: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageSummary {
    pub local_date: String,
    pub total_duration_ms: u64,
    pub applications: Vec<AppUsageTotal>,
    pub hourly_usage: Vec<HourlyUsageTotal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyUsageSummary {
    pub week_start_local_date: String,
    pub week_end_local_date: String,
    pub total_duration_ms: u64,
    pub applications: Vec<AppUsageTotal>,
    pub daily_usage: Vec<DailyUsageTotal>,
}

pub fn daily_summary(
    store: &UsageHistoryStore,
    local_date: &str,
) -> Result<DailyUsageSummary, String> {
    parse_local_date(local_date)?;
    let sessions = store.sessions_for_local_date(local_date)?;
    Ok(DailyUsageSummary {
        local_date: local_date.to_string(),
        total_duration_ms: total_duration(&sessions),
        applications: application_totals(&sessions),
        hourly_usage: hourly_totals(&sessions),
    })
}

pub fn weekly_summary(
    store: &UsageHistoryStore,
    local_date: &str,
) -> Result<WeeklyUsageSummary, String> {
    let selected_date = parse_local_date(local_date)?;
    let days_from_monday = u64::from(selected_date.weekday().num_days_from_monday());
    let week_start = selected_date
        .checked_sub_days(Days::new(days_from_monday))
        .ok_or_else(|| "weekly usage start date is outside the supported range".to_string())?;
    let week_end = week_start
        .checked_add_days(Days::new(DAYS_PER_WEEK - 1))
        .ok_or_else(|| "weekly usage end date is outside the supported range".to_string())?;
    let start = week_start.format("%F").to_string();
    let end = week_end.format("%F").to_string();
    let sessions = store.sessions_for_local_date_range(&start, &end)?;

    let mut daily_usage = Vec::with_capacity(DAYS_PER_WEEK as usize);
    for offset in 0..DAYS_PER_WEEK {
        let date = week_start
            .checked_add_days(Days::new(offset))
            .ok_or_else(|| "weekly usage date is outside the supported range".to_string())?;
        let date = date.format("%F").to_string();
        let duration_ms = sessions
            .iter()
            .filter(|session| session.measured_local_date == date)
            .fold(0u64, |total, session| {
                total.saturating_add(session_duration(session))
            });
        daily_usage.push(DailyUsageTotal {
            local_date: date,
            duration_ms,
        });
    }

    Ok(WeeklyUsageSummary {
        week_start_local_date: start,
        week_end_local_date: end,
        total_duration_ms: total_duration(&sessions),
        applications: application_totals(&sessions),
        daily_usage,
    })
}

#[tauri::command]
pub fn fetch_daily_usage_summary(
    state: State<'_, Arc<UsageHistoryStore>>,
    local_date: String,
) -> Result<DailyUsageSummary, String> {
    daily_summary(&state, &local_date)
}

#[tauri::command]
pub fn fetch_weekly_usage_summary(
    state: State<'_, Arc<UsageHistoryStore>>,
    local_date: String,
) -> Result<WeeklyUsageSummary, String> {
    weekly_summary(&state, &local_date)
}

fn parse_local_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("invalid local usage date: {value}"))
}

fn total_duration(sessions: &[StoredUsageSession]) -> u64 {
    sessions.iter().fold(0u64, |total, session| {
        total.saturating_add(session_duration(session))
    })
}

fn session_duration(session: &StoredUsageSession) -> u64 {
    session
        .ended_at_utc_ms
        .saturating_sub(session.started_at_utc_ms)
}

fn application_totals(sessions: &[StoredUsageSession]) -> Vec<AppUsageTotal> {
    let mut applications = Vec::<AppUsageTotal>::new();
    let mut indices = HashMap::<String, usize>::new();

    for session in sessions {
        let key = session
            .stable_key
            .clone()
            .unwrap_or_else(|| "\0unclassified".to_string());
        let index = *indices.entry(key).or_insert_with(|| {
            applications.push(AppUsageTotal {
                stable_key: session.stable_key.clone(),
                display_name: session
                    .display_name
                    .clone()
                    .unwrap_or_else(|| "Unclassified".to_string()),
                icon_png: session.icon_png.clone(),
                duration_ms: 0,
            });
            applications.len() - 1
        });
        applications[index].duration_ms = applications[index]
            .duration_ms
            .saturating_add(session_duration(session));
        if applications[index].icon_png.is_none() {
            applications[index].icon_png.clone_from(&session.icon_png);
        }
    }

    applications.sort_by(|left, right| {
        right
            .duration_ms
            .cmp(&left.duration_ms)
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    applications
}

fn hourly_totals(sessions: &[StoredUsageSession]) -> Vec<HourlyUsageTotal> {
    let mut totals = [0u64; HOURS_PER_DAY];
    for session in sessions {
        add_session_to_hourly_totals(session, &mut totals);
    }
    totals
        .into_iter()
        .enumerate()
        .map(|(hour, duration_ms)| HourlyUsageTotal {
            hour: hour as u8,
            duration_ms,
        })
        .collect()
}

fn add_session_to_hourly_totals(session: &StoredUsageSession, totals: &mut [u64; HOURS_PER_DAY]) {
    let offset_ms = parse_utc_offset_ms(&session.measured_timezone).unwrap_or_default();
    let mut cursor = i128::from(session.started_at_utc_ms);
    let end = i128::from(session.ended_at_utc_ms);

    while cursor < end {
        let local_cursor = cursor + offset_ms;
        let local_hour = local_cursor.div_euclid(MILLIS_PER_HOUR);
        let hour = local_hour.rem_euclid(HOURS_PER_DAY as i128) as usize;
        let next_hour_utc = (local_hour + 1) * MILLIS_PER_HOUR - offset_ms;
        let segment_end = end.min(next_hour_utc);
        let duration = u64::try_from(segment_end - cursor).unwrap_or(u64::MAX);
        totals[hour] = totals[hour].saturating_add(duration);
        cursor = segment_end;
    }
}

fn parse_utc_offset_ms(value: &str) -> Option<i128> {
    if matches!(value, "UTC" | "Z" | "+00:00" | "-00:00") {
        return Some(0);
    }
    let (sign, remainder) = match value.as_bytes().first()? {
        b'+' => (1i128, &value[1..]),
        b'-' => (-1i128, &value[1..]),
        _ => return None,
    };
    let (hours, minutes) = remainder.split_once(':')?;
    let hours = hours.parse::<u8>().ok()?;
    let minutes = minutes.parse::<u8>().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (i128::from(hours) * 60 + i128::from(minutes)) * MILLIS_PER_MINUTE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage_history::{AppMetadata, NewUsageSession, UsageSubject};
    use chrono::{TimeZone, Utc};

    fn store() -> (tempfile::TempDir, UsageHistoryStore) {
        let directory = tempfile::tempdir().unwrap();
        let store =
            UsageHistoryStore::with_storage_path(directory.path().join("usage.sqlite")).unwrap();
        (directory, store)
    }

    fn timestamp(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> u64 {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
            .single()
            .unwrap()
            .timestamp_millis() as u64
    }

    fn record(
        store: &UsageHistoryStore,
        app: Option<&AppMetadata>,
        start: u64,
        end: u64,
        offset: &str,
        local_date: &str,
    ) {
        store
            .record_session(&NewUsageSession {
                subject: app
                    .map(UsageSubject::Identified)
                    .unwrap_or(UsageSubject::Unclassified),
                started_at_utc_ms: start,
                ended_at_utc_ms: end,
                measured_timezone: offset,
                measured_local_date: local_date,
                end_reason: "focus_changed",
            })
            .unwrap();
    }

    fn app(key: &str, name: &str) -> AppMetadata {
        AppMetadata {
            stable_key: key.to_string(),
            display_name: name.to_string(),
            executable: None,
            icon_source: None,
            icon_png: None,
        }
    }

    #[test]
    fn daily_summary_includes_ranked_apps_unclassified_and_hourly_usage() {
        let (_directory, store) = store();
        let editor = app("product:editor", "Editor");
        let browser = app("product:browser", "Browser");
        record(
            &store,
            Some(&editor),
            timestamp(2026, 8, 3, 0, 30),
            timestamp(2026, 8, 3, 1, 30),
            "+09:00",
            "2026-08-03",
        );
        record(
            &store,
            Some(&browser),
            timestamp(2026, 8, 3, 2, 0),
            timestamp(2026, 8, 3, 2, 15),
            "+09:00",
            "2026-08-03",
        );
        record(
            &store,
            None,
            timestamp(2026, 8, 3, 3, 0),
            timestamp(2026, 8, 3, 3, 5),
            "+09:00",
            "2026-08-03",
        );

        let summary = daily_summary(&store, "2026-08-03").unwrap();
        assert_eq!(summary.total_duration_ms, 80 * 60_000);
        assert_eq!(summary.applications.len(), 3);
        assert_eq!(summary.applications[0].display_name, "Editor");
        assert_eq!(summary.applications[1].display_name, "Browser");
        assert_eq!(summary.applications[2].display_name, "Unclassified");
        assert_eq!(summary.hourly_usage[9].duration_ms, 30 * 60_000);
        assert_eq!(summary.hourly_usage[10].duration_ms, 30 * 60_000);
        assert_eq!(summary.hourly_usage.len(), HOURS_PER_DAY);
    }

    #[test]
    fn weekly_summary_starts_on_monday_and_includes_empty_days() {
        let (_directory, store) = store();
        let editor = app("product:editor", "Editor");
        record(&store, Some(&editor), 0, 10_000, "UTC", "2026-08-03");
        record(&store, Some(&editor), 20_000, 50_000, "UTC", "2026-08-09");

        let summary = weekly_summary(&store, "2026-08-05").unwrap();
        assert_eq!(summary.week_start_local_date, "2026-08-03");
        assert_eq!(summary.week_end_local_date, "2026-08-09");
        assert_eq!(summary.total_duration_ms, 40_000);
        assert_eq!(summary.daily_usage.len(), DAYS_PER_WEEK as usize);
        assert_eq!(summary.daily_usage[0].duration_ms, 10_000);
        assert_eq!(summary.daily_usage[1].duration_ms, 0);
        assert_eq!(summary.daily_usage[6].duration_ms, 30_000);
        assert_eq!(summary.applications[0].duration_ms, 40_000);
    }

    #[test]
    fn session_crossing_midnight_keeps_its_measured_date() {
        let (_directory, store) = store();
        let editor = app("product:editor", "Editor");
        record(
            &store,
            Some(&editor),
            timestamp(2026, 8, 3, 14, 55),
            timestamp(2026, 8, 3, 15, 5),
            "+09:00",
            "2026-08-03",
        );

        let measured_day = daily_summary(&store, "2026-08-03").unwrap();
        let following_day = daily_summary(&store, "2026-08-04").unwrap();
        assert_eq!(measured_day.total_duration_ms, 10 * 60_000);
        assert_eq!(measured_day.hourly_usage[23].duration_ms, 5 * 60_000);
        assert_eq!(measured_day.hourly_usage[0].duration_ms, 5 * 60_000);
        assert_eq!(following_day.total_duration_ms, 0);
    }

    #[test]
    fn measured_dates_and_offsets_survive_timezone_and_dst_changes() {
        let (_directory, store) = store();
        let editor = app("product:editor", "Editor");
        let shared_start = timestamp(2026, 11, 1, 5, 30);
        record(
            &store,
            Some(&editor),
            shared_start,
            shared_start + 15 * 60_000,
            "-04:00",
            "2026-11-01",
        );
        record(
            &store,
            Some(&editor),
            shared_start + 60 * 60_000,
            shared_start + 75 * 60_000,
            "-05:00",
            "2026-11-01",
        );
        record(
            &store,
            Some(&editor),
            shared_start,
            shared_start + 10 * 60_000,
            "+09:00",
            "2026-11-02",
        );

        let before_change = daily_summary(&store, "2026-11-01").unwrap();
        let after_change = daily_summary(&store, "2026-11-02").unwrap();
        assert_eq!(before_change.total_duration_ms, 30 * 60_000);
        assert_eq!(before_change.hourly_usage[1].duration_ms, 30 * 60_000);
        assert_eq!(after_change.total_duration_ms, 10 * 60_000);
        assert_eq!(after_change.hourly_usage[14].duration_ms, 10 * 60_000);
    }

    #[test]
    fn rejects_invalid_dates_and_reversed_ranges() {
        let (_directory, store) = store();
        assert_eq!(
            daily_summary(&store, "2026-02-30").unwrap_err(),
            "invalid local usage date: 2026-02-30"
        );
        assert_eq!(
            store
                .sessions_for_local_date_range("2026-08-04", "2026-08-03")
                .unwrap_err(),
            "usage history date range ends before it starts"
        );
    }
}
