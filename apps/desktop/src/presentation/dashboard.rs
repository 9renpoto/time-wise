//! Leptos components for the desktop usage dashboard.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;

use crate::application::usage_dashboard::{
    format_axis_duration, format_date_label, format_usage_duration, icon_data_url,
    shift_local_date, usage_bar_height,
};
use crate::domain::usage_summary::{AppUsageTotal, DailyUsageSummary, WeeklyUsageSummary};
use crate::infrastructure::tauri_adapter::{
    fetch_measurement_health, load_daily_usage_summary, load_weekly_usage_summary,
    MeasurementHealth, MeasurementHealthStatus,
};

const USAGE_REFRESH_MILLIS: i32 = 30_000;
const WEEKDAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsagePeriod {
    Day,
    Week,
}

#[derive(Debug, Clone)]
enum UsageData {
    Daily(DailyUsageSummary),
    Weekly(WeeklyUsageSummary),
}

impl UsageData {
    fn total_duration_ms(&self) -> u64 {
        match self {
            Self::Daily(summary) => summary.total_duration_ms,
            Self::Weekly(summary) => summary.total_duration_ms,
        }
    }

    fn applications(&self) -> &[AppUsageTotal] {
        match self {
            Self::Daily(summary) => &summary.applications,
            Self::Weekly(summary) => &summary.applications,
        }
    }

    fn chart_bars(&self) -> Vec<ChartBar> {
        match self {
            Self::Daily(summary) => summary
                .hourly_usage
                .iter()
                .map(|usage| ChartBar {
                    label: match usage.hour {
                        0 => "12a".to_string(),
                        3 | 6 | 9 => format!("{}a", usage.hour),
                        12 => "12p".to_string(),
                        15 | 18 | 21 => format!("{}p", usage.hour - 12),
                        _ => String::new(),
                    },
                    duration_ms: usage.duration_ms,
                })
                .collect(),
            Self::Weekly(summary) => summary
                .daily_usage
                .iter()
                .enumerate()
                .map(|(index, usage)| ChartBar {
                    label: WEEKDAY_LABELS.get(index).unwrap_or(&"").to_string(),
                    duration_ms: usage.duration_ms,
                })
                .collect(),
        }
    }

    fn period_label(&self, today: &str) -> String {
        match self {
            Self::Daily(summary) if summary.local_date == today => "Today".to_string(),
            Self::Daily(summary) => format_date_label(&summary.local_date),
            Self::Weekly(summary) => format!(
                "{} – {}",
                format_date_label(&summary.week_start_local_date),
                format_date_label(&summary.week_end_local_date)
            ),
        }
    }
}

#[derive(Debug, Clone)]
struct ChartBar {
    label: String,
    duration_ms: u64,
}

fn today_local_date() -> String {
    let now = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        now.get_full_year(),
        now.get_month() + 1,
        now.get_date()
    )
}

fn app_initial(display_name: &str) -> String {
    display_name
        .chars()
        .find(|character| character.is_alphanumeric())
        .map(|character| character.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_string())
}

#[component]
/// Main dashboard for daily and weekly application usage.
pub fn Dashboard() -> impl IntoView {
    let today = StoredValue::new(today_local_date());
    let (selected_date, set_selected_date) = signal(today.get_value());
    let (period, set_period) = signal(UsagePeriod::Day);
    let (usage_data, set_usage_data) = signal(None::<UsageData>);
    let (loading, set_loading) = signal(true);
    let (load_error, set_load_error) = signal(None::<String>);
    let (measurement_health, set_measurement_health) = signal(None::<MeasurementHealth>);
    let (refresh_tick, set_refresh_tick) = signal(0u64);
    let request_id = StoredValue::new(0u64);

    if let Some(win) = window() {
        let callback = Closure::wrap(Box::new(move || {
            set_refresh_tick.update(|tick| *tick = tick.wrapping_add(1));
        }) as Box<dyn FnMut()>);
        let _ = win.set_interval_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            USAGE_REFRESH_MILLIS,
        );
        callback.forget();
    }

    Effect::new(move |_| {
        let selected_period = period.get();
        let date = selected_date.get();
        let _ = refresh_tick.get();
        let next_request_id = request_id.get_value().wrapping_add(1);
        request_id.set_value(next_request_id);
        set_loading.set(true);
        set_load_error.set(None);

        spawn_local(async move {
            match fetch_measurement_health().await {
                Ok(health) => set_measurement_health.set(Some(health)),
                Err(_) => set_measurement_health.set(Some(MeasurementHealth {
                    status: MeasurementHealthStatus::EventSubscriptionFailed,
                    latest_diagnostic: None,
                })),
            }
            let result = match selected_period {
                UsagePeriod::Day => load_daily_usage_summary(&date).await.map(UsageData::Daily),
                UsagePeriod::Week => load_weekly_usage_summary(&date)
                    .await
                    .map(UsageData::Weekly),
            };
            if request_id.get_value() != next_request_id {
                return;
            }
            match result {
                Ok(summary) => set_usage_data.set(Some(summary)),
                Err(error) => {
                    set_usage_data.set(None);
                    set_load_error.set(Some(error));
                }
            }
            set_loading.set(false);
        });
    });

    let total_duration = Signal::derive(move || {
        usage_data.with(|data| {
            data.as_ref()
                .map(|summary| format_usage_duration(summary.total_duration_ms()))
                .unwrap_or_else(|| "—".to_string())
        })
    });
    let application_count = Signal::derive(move || {
        usage_data.with(|data| {
            data.as_ref()
                .map(|summary| summary.applications().len())
                .unwrap_or_default()
        })
    });
    let period_label = Signal::derive(move || {
        usage_data.with(|data| {
            data.as_ref()
                .map(|summary| summary.period_label(&today.get_value()))
                .unwrap_or_else(|| format_date_label(&selected_date.get()))
        })
    });
    let maximum_bucket = Signal::derive(move || {
        usage_data.with(|data| {
            data.as_ref()
                .and_then(|summary| {
                    summary
                        .chart_bars()
                        .into_iter()
                        .map(|bar| bar.duration_ms)
                        .max()
                })
                .unwrap_or_default()
        })
    });
    let can_move_forward = Signal::derive(move || selected_date.get() < today.get_value());

    view! {
        <main class="dashboard">
            <header class="dashboard__header">
                <div>
                    <span class="dashboard__eyebrow">"TIME WISE"</span>
                    <h1 class="dashboard__title">"Usage"</h1>
                </div>
                <div class="dashboard__period-toggle" aria-label="Usage period">
                    <button
                        class:dashboard__period-button=true
                        class:dashboard__period-button--active=move || period.get() == UsagePeriod::Day
                        on:click=move |_| set_period.set(UsagePeriod::Day)
                    >"Day"</button>
                    <button
                        class:dashboard__period-button=true
                        class:dashboard__period-button--active=move || period.get() == UsagePeriod::Week
                        on:click=move |_| set_period.set(UsagePeriod::Week)
                    >"Week"</button>
                </div>
            </header>

            <nav class="dashboard__date-navigation" aria-label="Date navigation">
                <button
                    class="dashboard__date-button"
                    aria-label="Previous period"
                    on:click=move |_| {
                        let days = if period.get_untracked() == UsagePeriod::Day { -1 } else { -7 };
                        if let Some(date) = shift_local_date(&selected_date.get_untracked(), days) {
                            set_selected_date.set(date);
                        }
                    }
                >"‹"</button>
                <div class="dashboard__date-copy">
                    <strong>{move || period_label.get()}</strong>
                    <span>{move || match period.get() {
                        UsagePeriod::Day => "Daily activity",
                        UsagePeriod::Week => "Weekly activity",
                    }}</span>
                </div>
                <button
                    class="dashboard__date-button"
                    aria-label="Next period"
                    disabled=move || !can_move_forward.get()
                    on:click=move |_| {
                        let days = if period.get_untracked() == UsagePeriod::Day { 1 } else { 7 };
                        if let Some(mut date) = shift_local_date(&selected_date.get_untracked(), days) {
                            if date > today.get_value() {
                                date = today.get_value();
                            }
                            set_selected_date.set(date);
                        }
                    }
                >"›"</button>
            </nav>

            <Show when=move || measurement_health.get().is_some_and(|health| health.status != MeasurementHealthStatus::Healthy)>
                <section class="dashboard__state dashboard__state--warning" role="status">
                    <span class="dashboard__state-icon">"!"</span>
                    <div>
                        <strong>{move || measurement_health.get().map(|health| match health.status {
                            MeasurementHealthStatus::Healthy => "Measurement active",
                            MeasurementHealthStatus::EventSubscriptionFailed => "Measurement unavailable",
                            MeasurementHealthStatus::ObservationDegraded => "Some activity is unclassified",
                            MeasurementHealthStatus::PersistenceFailed => "Activity could not be saved",
                        }).unwrap_or("Measurement unavailable")}</strong>
                        <p>{move || measurement_health.get().map(|health| match health.status {
                            MeasurementHealthStatus::Healthy => "Time Wise is recording activity.",
                            MeasurementHealthStatus::EventSubscriptionFailed => "Time Wise lost access to desktop activity events. Restart the app to retry.",
                            MeasurementHealthStatus::ObservationDegraded => "Time Wise could not identify the focused application. The time is kept as unclassified activity.",
                            MeasurementHealthStatus::PersistenceFailed => "Time Wise will retry saving when the next activity checkpoint is recorded.",
                        }).unwrap_or("Time Wise could not determine the current measurement state.")}</p>
                    </div>
                </section>
            </Show>

            <Show when=move || load_error.get().is_some()>
                <section class="dashboard__state dashboard__state--error" role="alert">
                    <span class="dashboard__state-icon">"!"</span>
                    <div>
                        <strong>"Usage data unavailable"</strong>
                        <p>"Time Wise couldn't load the recorded activity. Measurement may be temporarily unavailable."</p>
                    </div>
                    <button on:click=move |_| set_refresh_tick.update(|tick| *tick = tick.wrapping_add(1))>
                        "Try again"
                    </button>
                </section>
            </Show>

            <Show when=move || load_error.get().is_none()>
                <section class="dashboard__overview" aria-busy=move || loading.get().to_string()>
                    <div class="dashboard__total-card">
                        <span class="dashboard__metric-label">"Total usage"</span>
                        <strong class="dashboard__total-value">{move || total_duration.get()}</strong>
                        <span class="dashboard__metric-detail">{move || {
                            let count = application_count.get();
                            match count {
                                0 => "No applications recorded".to_string(),
                                1 => "1 application".to_string(),
                                _ => format!("{count} applications"),
                            }
                        }}</span>
                    </div>

                    <div class="dashboard__chart-card">
                        <div class="dashboard__section-heading">
                            <div>
                                <span class="dashboard__metric-label">{move || match period.get() {
                                    UsagePeriod::Day => "Activity by hour",
                                    UsagePeriod::Week => "Activity by day",
                                }}</span>
                                <strong>{move || format_axis_duration(maximum_bucket.get())} " peak"</strong>
                            </div>
                            <span class="dashboard__live-indicator">{move || if loading.get() { "Updating…" } else { "Recorded" }}</span>
                        </div>
                        <div class="dashboard__chart">
                            <div class="dashboard__chart-grid dashboard__chart-grid--top"></div>
                            <div class="dashboard__chart-grid dashboard__chart-grid--middle"></div>
                            {move || {
                                let maximum = maximum_bucket.get();
                                usage_data.with(|data| {
                                    data.as_ref()
                                        .map(|summary| summary.chart_bars())
                                        .unwrap_or_default()
                                })
                                .into_iter()
                                .map(|bar| {
                                    let height = usage_bar_height(bar.duration_ms, maximum);
                                    let title = format_usage_duration(bar.duration_ms);
                                    view! {
                                        <div class="dashboard__chart-column" title=title>
                                            <div class="dashboard__chart-track">
                                                <div class="dashboard__chart-bar" style=height></div>
                                            </div>
                                            <span>{bar.label}</span>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()
                                .into_view()
                            }}
                        </div>
                    </div>
                </section>

                <section class="dashboard__ranking">
                    <div class="dashboard__ranking-header">
                        <div>
                            <span class="dashboard__eyebrow">"APPLICATIONS"</span>
                            <h2>"Most used"</h2>
                        </div>
                        <span>{move || application_count.get()} " total"</span>
                    </div>

                    <Show
                        when=move || !loading.get() && application_count.get() == 0
                        fallback=move || view! { <></> }
                    >
                        <div class="dashboard__state dashboard__state--empty">
                            <span class="dashboard__state-icon">"◷"</span>
                            <div>
                                <strong>"No activity yet"</strong>
                                <p>"Application usage recorded during this period will appear here."</p>
                            </div>
                        </div>
                    </Show>

                    <Show when=move || loading.get() && usage_data.get().is_none()>
                        <div class="dashboard__loading-list" aria-label="Loading usage data">
                            <span></span><span></span><span></span>
                        </div>
                    </Show>

                    <ul class="dashboard__app-list">
                        {move || {
                            usage_data.with(|data| {
                                data.as_ref()
                                    .map(|summary| summary.applications().to_vec())
                                    .unwrap_or_default()
                            })
                            .into_iter()
                            .enumerate()
                            .map(|(index, app)| {
                                let is_unclassified = app.stable_key.is_none();
                                let icon = match app.icon_png.as_deref() {
                                    Some(bytes) if !bytes.is_empty() => view! {
                                        <img src=icon_data_url(bytes) alt="" />
                                    }.into_any(),
                                    _ => view! {
                                        <span>{app_initial(&app.display_name)}</span>
                                    }.into_any(),
                                };
                                let share = usage_data.with(|data| {
                                    data.as_ref()
                                        .map(|summary| summary.total_duration_ms())
                                        .filter(|total| *total > 0)
                                        .map(|total| app.duration_ms as f64 / total as f64 * 100.0)
                                        .unwrap_or_default()
                                });
                                view! {
                                    <li class="dashboard__app-item">
                                        <span class="dashboard__app-rank">{index + 1}</span>
                                        <div
                                            class="dashboard__app-icon"
                                            class:dashboard__app-icon--unclassified=is_unclassified
                                        >{icon}</div>
                                        <div class="dashboard__app-details">
                                            <div class="dashboard__app-copy">
                                                <strong>{app.display_name}</strong>
                                                <span>{format!("{share:.0}% of total")}</span>
                                            </div>
                                            <div class="dashboard__app-progress">
                                                <span style=format!("width:{share:.1}%")></span>
                                            </div>
                                        </div>
                                        <strong class="dashboard__app-duration">{format_usage_duration(app.duration_ms)}</strong>
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()
                            .into_view()
                        }}
                    </ul>
                </section>
            </Show>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::usage_summary::{DailyUsageTotal, HourlyUsageTotal};

    #[test]
    fn daily_chart_has_24_hour_buckets() {
        let summary = UsageData::Daily(DailyUsageSummary {
            local_date: "2026-08-04".to_string(),
            total_duration_ms: 0,
            applications: Vec::new(),
            hourly_usage: (0..24)
                .map(|hour| HourlyUsageTotal {
                    hour,
                    duration_ms: u64::from(hour),
                })
                .collect(),
        });
        let bars = summary.chart_bars();
        assert_eq!(bars.len(), 24);
        assert_eq!(bars[0].label, "12a");
        assert_eq!(bars[12].label, "12p");
        assert_eq!(bars[23].label, "");
    }

    #[test]
    fn weekly_chart_uses_monday_first_labels() {
        let summary = UsageData::Weekly(WeeklyUsageSummary {
            week_start_local_date: "2026-08-03".to_string(),
            week_end_local_date: "2026-08-09".to_string(),
            total_duration_ms: 0,
            applications: Vec::new(),
            daily_usage: (3..=9)
                .map(|day| DailyUsageTotal {
                    local_date: format!("2026-08-{day:02}"),
                    duration_ms: 0,
                })
                .collect(),
        });
        let labels: Vec<_> = summary
            .chart_bars()
            .into_iter()
            .map(|bar| bar.label)
            .collect();
        assert_eq!(labels, WEEKDAY_LABELS);
    }

    #[test]
    fn unclassified_initial_is_readable() {
        assert_eq!(app_initial("Unclassified"), "U");
        assert_eq!(app_initial("***"), "?");
    }
}
