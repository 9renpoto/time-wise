use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const MILLIS_PER_MINUTE: u64 = 60_000;
const MINUTES_PER_HOUR: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalDate {
    year: i32,
    month: u8,
    day: u8,
}

/// Formats dashboard durations at minute precision.
pub fn format_usage_duration(duration_ms: u64) -> String {
    let total_minutes = duration_ms / MILLIS_PER_MINUTE;
    let hours = total_minutes / MINUTES_PER_HOUR;
    let minutes = total_minutes % MINUTES_PER_HOUR;
    if hours == 0 {
        format!("{minutes}m")
    } else if minutes == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {minutes}m")
    }
}

/// Formats a compact value for a chart's vertical scale.
pub fn format_axis_duration(duration_ms: u64) -> String {
    let total_minutes = duration_ms / MILLIS_PER_MINUTE;
    if total_minutes < MINUTES_PER_HOUR {
        format!("{total_minutes}m")
    } else {
        let hours = total_minutes as f64 / MINUTES_PER_HOUR as f64;
        format!("{hours:.1}h")
    }
}

/// Calculates a chart bar's height relative to the largest bucket.
pub fn usage_bar_height(duration_ms: u64, maximum_ms: u64) -> String {
    if duration_ms == 0 || maximum_ms == 0 {
        return "height:0%".to_string();
    }
    let percentage = (duration_ms as f64 / maximum_ms as f64 * 100.0).max(3.0);
    format!("height:{percentage:.0}%")
}

/// Moves an ISO local date without consulting the current system time zone.
pub fn shift_local_date(value: &str, days: i32) -> Option<String> {
    let mut date = parse_local_date(value)?;
    let direction = days.signum();
    for _ in 0..days.unsigned_abs() {
        if direction > 0 {
            increment_date(&mut date);
        } else if direction < 0 {
            decrement_date(&mut date);
        }
    }
    Some(format_local_date(date))
}

/// Formats an ISO date for a compact dashboard heading.
pub fn format_date_label(value: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    parse_local_date(value)
        .map(|date| {
            format!(
                "{} {}, {}",
                MONTHS[usize::from(date.month - 1)],
                date.day,
                date.year
            )
        })
        .unwrap_or_else(|| value.to_string())
}

/// Encodes persisted PNG bytes for an HTML image source.
pub fn icon_data_url(icon_png: &[u8]) -> String {
    format!("data:image/png;base64,{}", STANDARD.encode(icon_png))
}

fn parse_local_date(value: &str) -> Option<LocalDate> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) {
        return None;
    }
    let date = LocalDate { year, month, day };
    (day > 0 && day <= days_in_month(date)).then_some(date)
}

fn format_local_date(date: LocalDate) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn increment_date(date: &mut LocalDate) {
    if date.day < days_in_month(*date) {
        date.day += 1;
    } else if date.month < 12 {
        date.month += 1;
        date.day = 1;
    } else {
        date.year += 1;
        date.month = 1;
        date.day = 1;
    }
}

fn decrement_date(date: &mut LocalDate) {
    if date.day > 1 {
        date.day -= 1;
    } else if date.month > 1 {
        date.month -= 1;
        date.day = days_in_month(*date);
    } else {
        date.year -= 1;
        date.month = 12;
        date.day = 31;
    }
}

fn days_in_month(date: LocalDate) -> u8 {
    match date.month {
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(date.year) => 29,
        2 => 28,
        _ => 31,
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_usage_at_minute_precision() {
        assert_eq!(format_usage_duration(59_999), "0m");
        assert_eq!(format_usage_duration(42 * MILLIS_PER_MINUTE), "42m");
        assert_eq!(format_usage_duration(125 * MILLIS_PER_MINUTE), "2h 5m");
    }

    #[test]
    fn shifts_dates_across_months_years_and_leap_days() {
        assert_eq!(
            shift_local_date("2026-08-01", -1).as_deref(),
            Some("2026-07-31")
        );
        assert_eq!(
            shift_local_date("2026-12-31", 1).as_deref(),
            Some("2027-01-01")
        );
        assert_eq!(
            shift_local_date("2024-02-28", 1).as_deref(),
            Some("2024-02-29")
        );
        assert_eq!(
            shift_local_date("2024-02-29", 1).as_deref(),
            Some("2024-03-01")
        );
    }

    #[test]
    fn rejects_invalid_local_dates() {
        assert_eq!(shift_local_date("2026-02-29", 1), None);
        assert_eq!(shift_local_date("not-a-date", 1), None);
    }

    #[test]
    fn chart_height_handles_empty_and_small_values() {
        assert_eq!(usage_bar_height(0, 10), "height:0%");
        assert_eq!(usage_bar_height(1, 100), "height:3%");
        assert_eq!(usage_bar_height(100, 100), "height:100%");
    }

    #[test]
    fn encodes_png_as_data_url() {
        assert_eq!(
            icon_data_url(&[137, 80, 78, 71]),
            "data:image/png;base64,iVBORw=="
        );
    }
}
