//! Leptos component definitions that render startup metrics fetched from the Tauri backend.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{console, window};

use crate::application::startup_service::{
    compute_category_summary, compute_chart_points, compute_tiles, format_duration,
    format_duration_compact, format_timestamp, format_total_duration,
};
use crate::application::usage_service::{
    active_app_count, compute_usage_tiles, latest_usage_timestamp,
};
use crate::domain::{app_usage_record::AppUsageRecord, startup_record::StartupRecord};
use crate::infrastructure::tauri_adapter::{load_app_usage_records, load_startup_records};

const STARTUP_HISTORY_LIMIT: usize = 5;
const APP_USAGE_REFRESH_MILLIS: i32 = 15_000;

/// Returns percentage height style for chart bars.
fn bar_height(bin: u64, max_bin: u64) -> String {
    let height = if max_bin == 0 {
        0.0
    } else {
        (bin as f64 / max_bin as f64 * 100.0).max(8.0)
    };
    format!("height:{height:.0}%")
}

fn launcher_display_label(launcher: &str) -> Option<String> {
    let trimmed = launcher.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[component]
/// Main dashboard component rendering startup metrics.
pub fn Dashboard() -> impl IntoView {
    let (startup_records, set_startup_records) = signal(Vec::<StartupRecord>::new());
    let (usage_records, set_usage_records) = signal(Vec::<AppUsageRecord>::new());
    let (loaded, set_loaded) = signal(false);

    fn schedule_usage_fetch(setter: WriteSignal<Vec<AppUsageRecord>>) {
        spawn_local(async move {
            let records = load_app_usage_records().await;
            setter.set(records);
        });
    }

    schedule_usage_fetch(set_usage_records);

    if let Some(win) = window() {
        let setter = set_usage_records;
        let callback = Closure::wrap(Box::new(move || {
            schedule_usage_fetch(setter);
        }) as Box<dyn FnMut()>);

        if let Err(err) = win.set_interval_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            APP_USAGE_REFRESH_MILLIS,
        ) {
            console::error_1(&err);
        }

        callback.forget();
    }

    Effect::new(move |_| {
        if loaded.get() {
            return;
        }
        spawn_local({
            let set_startup_records = set_startup_records;
            let set_loaded = set_loaded;
            async move {
                let records = load_startup_records().await;
                set_startup_records.set(records);
                set_loaded.set(true);
            }
        });
    });

    let total_runs = Signal::derive(move || startup_records.with(|records| records.len()));
    let latest_record =
        Signal::derive(move || startup_records.with(|records| records.first().cloned()));
    let history_records = Signal::derive(move || {
        startup_records.with(|records| {
            let mut limited = records.clone();
            if limited.len() > STARTUP_HISTORY_LIMIT {
                limited.truncate(STARTUP_HISTORY_LIMIT);
            }
            limited
        })
    });
    let total_duration = Signal::derive(move || {
        startup_records.with(|records| {
            let total_ms: u128 = records
                .iter()
                .map(|record| record.duration_ms as u128)
                .sum();
            format_total_duration(total_ms as u64)
        })
    });
    let chart_points =
        Signal::derive(move || startup_records.with(|records| compute_chart_points(records)));
    let chart_max = Signal::derive(move || {
        chart_points.with(|points| {
            points
                .iter()
                .map(|point| point.duration_ms)
                .max()
                .unwrap_or(0)
        })
    });
    let chart_annotation_top = Signal::derive(move || format_duration_compact(chart_max.get()));
    let chart_annotation_middle =
        Signal::derive(move || format_duration_compact(chart_max.get() / 2));
    let category_usage =
        Signal::derive(move || startup_records.with(|records| compute_category_summary(records)));
    let tiles = Signal::derive(move || startup_records.with(|records| compute_tiles(records)));
    let usage_tiles =
        Signal::derive(move || usage_records.with(|records| compute_usage_tiles(records)));
    let usage_status_text = Signal::derive(move || {
        usage_records.with(|records| match active_app_count(records) {
            0 => "No active apps".to_string(),
            1 => "1 active app".to_string(),
            count => format!("{count} active apps"),
        })
    });
    let usage_last_updated = Signal::derive(move || {
        usage_records.with(|records| {
            latest_usage_timestamp(records)
                .map(|timestamp| format!("Last updated {timestamp}"))
                .unwrap_or_else(|| "Waiting for desktop activity…".to_string())
        })
    });

    view! {
        <main class="app">
            <section class="app__card">
                <div class="app__summary">
                    <header class="app__profile">
                        <div class="app__avatar">
                            "A"
                        </div>
                        <div>
                            <div class="app__total">{move || total_duration.get()}</div>
                            <div class="app__label">"Startup time collected"
                            </div>
                        </div>
                    </header>
                    <div class="app__startup">
                        <div class="app__startup-header">
                            <span class="app__startup-title">"Startup performance"</span>
                            <span class="app__startup-count">{move || {
                                let count = total_runs.get();
                                match count {
                                    0 => "No runs yet".to_string(),
                                    1 => "1 run recorded".to_string(),
                                    _ => format!("{count} runs recorded"),
                                }
                            }}</span>
                        </div>
                        <Show
                            when=move || latest_record.get().is_some()
                            fallback=move || {
                                let message = if loaded.get() {
                                    "Collecting first startup measurement…"
                                } else {
                                    "Loading startup metrics…"
                                };
                                view! { <div class="app__startup-empty">{message}</div> }
                            }
                        >
                            {move || {
                                let record = latest_record
                                    .get()
                                    .expect("checked by Show predicate");
                                view! {
                                    <div class="app__startup-latest">
                                        <span class="app__startup-value">{format_duration(record.duration_ms)}</span>
                                        <span class="app__startup-subtext">{
                                            let timestamp = format_timestamp(record.recorded_at_ms);
                                            match launcher_display_label(&record.launcher) {
                                                Some(launcher) => {
                                                    format!("Recorded {timestamp} • via {launcher}")
                                                }
                                                None => format!("Recorded {timestamp}"),
                                            }
                                        }</span>
                                    </div>
                                }
                            }}
                        </Show>
                        <Show
                            when=move || { history_records.get().len() > 1 }
                            fallback=move || { view! { <></> } }
                        >
                            {move || {
                                let mut records = history_records.get();
                                let _ = records.first();
                                let mut iter = records.into_iter();
                                let _ = iter.next();
                                let items = iter
                                    .map(|record| {
                                        view! {
                                            <li class="app__startup-list-item">
                                                <span class="app__startup-list-time">{format_duration(record.duration_ms)}</span>
                                                <span class="app__startup-list-date">{
                                                    let timestamp = format_timestamp(record.recorded_at_ms);
                                                    match launcher_display_label(&record.launcher) {
                                                        Some(launcher) => {
                                                            format!("{timestamp} • via {launcher}")
                                                        }
                                                        None => timestamp,
                                                    }
                                                }</span>
                                            </li>
                                        }
                                    })
                                    .collect::<Vec<_>>();
                                view! {
                                    <ul class="app__startup-list">
                                        {items.into_view()}
                                    </ul>
                                }
                            }}
                        </Show>
                    </div>
                    <div class="app__chart">
                        <div class="app__chart-overlay">
                            <div class="app__chart-grid-line app__chart-grid-line--top"></div>
                            <div class="app__chart-grid-line app__chart-grid-line--middle"></div>
                            <div class="app__chart-grid-line app__chart-grid-line--bottom"></div>
                        </div>
                        {move || {
                            let max_value = chart_max.get();
                            chart_points
                                .get()
                                .into_iter()
                                .map(|point| {
                                    let style = bar_height(point.duration_ms, max_value);
                                    view! {
                                        <div class="app__chart-column">
                                            <div class="app__chart-column-inner">
                                                <div class="app__chart-bar" style=style></div>
                                            </div>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()
                                .into_view()
                        }}
                        <div class="app__chart-labels">
                            {move || {
                                chart_points
                                    .get()
                                    .into_iter()
                                    .map(|point| view! { <span>{point.label}</span> })
                                    .collect::<Vec<_>>()
                                    .into_view()
                            }}
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--top">
                            {move || chart_annotation_top.get()}
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--middle">
                            {move || chart_annotation_middle.get()}
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--bottom">"0"
                        </div>
                    </div>
                    <div class="app__categories">
                        {move || {
                            category_usage
                                .get()
                                .into_iter()
                                .map(|category| {
                                    view! {
                                        <div class="app__category">
                                            <span class=category.class_names>
                                                {category.name}
                                            </span>
                                            <span class="app__category-minutes">{category.summary}</span>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()
                                .into_view()
                        }}
                    </div>
                </div>
                <div class="app__grid">
                    {move || {
                        tiles
                            .get()
                            .into_iter()
                            .map(|tile| {
                                view! {
                                    <div class="app__tile">
                                        <div class="app__tile-icon">
                                            {tile.icon}
                                        </div>
                                        <div class="app__tile-info">
                                            <span class="app__tile-name">{tile.label}</span>
                                            <span class="app__tile-minutes">{tile.duration}</span>
                                        </div>
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                            .into_view()
                    }}
                </div>
                <div class="app__usage">
                    <div class="app__usage-header">
                        <span class="app__usage-title">"Desktop usage"</span>
                        <span class="app__usage-count">{move || usage_status_text.get()}</span>
                    </div>
                    <span class="app__usage-updated">{move || usage_last_updated.get()}</span>
                    <Show
                        when=move || !usage_tiles.get().is_empty()
                        fallback=move || {
                            view! { <div class="app__usage-empty">"Desktop activity will appear once apps launch."</div> }
                        }
                    >
                        {move || {
                            let tiles = usage_tiles.get();
                            let rows = tiles
                                .into_iter()
                                .map(|tile| {
                                    let indicator_class = if tile.active {
                                        "app__usage-indicator app__usage-indicator--active"
                                    } else {
                                        "app__usage-indicator"
                                    };
                                    view! {
                                        <li class="app__usage-item">
                                            <div class="app__usage-main">
                                                <span class=indicator_class></span>
                                                <div class="app__usage-info">
                                                    <span class="app__usage-name">{tile.name}</span>
                                                    <span class="app__usage-subtitle">{tile.subtitle}</span>
                                                </div>
                                            </div>
                                            <span class="app__usage-duration">{tile.duration}</span>
                                        </li>
                                    }
                                })
                                .collect::<Vec<_>>();
                            view! { <ul class="app__usage-list">{rows.into_view()}</ul> }
                        }}
                    </Show>
                </div>
            </section>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::bar_height;

    #[test]
    fn bar_height_zero_max_returns_zero_percent() {
        let style = bar_height(0, 0);
        assert!(style.contains("height:0%"));
    }

    #[test]
    fn bar_height_scales_to_full_height() {
        let style = bar_height(15, 15);
        assert!(style.contains("height:100%"));
    }

    #[test]
    fn bar_height_applies_minimum_percentage() {
        let style = bar_height(0, 15);
        assert!(style.contains("height:8%"));
    }
}
