//! Leptos component definitions that render startup metrics fetched from the Tauri backend.

use js_sys::{Date, Function, Promise, Reflect};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

#[derive(Clone)]
/// Data point backing the histogram chart.
struct ChartPoint {
    label: String,
    duration_ms: u64,
}

#[derive(Clone)]
/// Aggregated summary per performance bucket.
struct CategorySummary {
    name: &'static str,
    class_names: &'static str,
    summary: String,
}

#[derive(Clone)]
/// UI model for each startup tile.
struct StartupTile {
    icon: &'static str,
    label: String,
    duration: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct StartupRecord {
    recorded_at_ms: u64,
    duration_ms: u64,
}

const STARTUP_HISTORY_LIMIT: usize = 5;

async fn invoke_command<T>(command: &str) -> Result<T, JsValue>
where
    T: serde::de::DeserializeOwned,
{
    let Some(window) = window() else {
        return Err(JsValue::from_str("missing window"));
    };
    let tauri = Reflect::get(&window, &JsValue::from_str("__TAURI__"))?;
    if tauri.is_undefined() || tauri.is_null() {
        return Err(JsValue::from_str("tauri bridge unavailable"));
    }
    let invoke_fn = Reflect::get(&tauri, &JsValue::from_str("invoke"))?;
    let function = invoke_fn.dyn_into::<Function>()?;
    let promise = function
        .call2(&tauri, &JsValue::from_str(command), &JsValue::UNDEFINED)?
        .dyn_into::<Promise>()?;
    let response = JsFuture::from(promise).await?;
    serde_wasm_bindgen::from_value(response).map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn load_startup_records() -> Vec<StartupRecord> {
    match invoke_command::<Vec<StartupRecord>>("fetch_startup_records").await {
        Ok(mut records) => {
            records.sort_by(|a, b| b.recorded_at_ms.cmp(&a.recorded_at_ms));
            records
        }
        Err(err) => {
            leptos::logging::log!("failed to fetch startup records: {err:?}");
            Vec::new()
        }
    }
}

/// Formats the total startup duration for the header.
fn format_total_duration(total_ms: u64) -> String {
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
fn format_duration_compact(ms: u64) -> String {
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

/// Builds the chart points from the latest samples.
fn compute_chart_points(records: &[StartupRecord]) -> Vec<ChartPoint> {
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
fn compute_category_summary(records: &[StartupRecord]) -> Vec<CategorySummary> {
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
fn compute_tiles(records: &[StartupRecord]) -> Vec<StartupTile> {
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

fn format_duration(ms: u64) -> String {
    if ms >= 1_000 {
        format!("{:.2} s", ms as f64 / 1_000.0)
    } else {
        format!("{ms} ms")
    }
}

/// Formats the timestamp into a locale-aware date string.
fn format_timestamp(ms: u64) -> String {
    let date = Date::new(&JsValue::from_f64(ms as f64));
    Date::to_locale_string(&date, "default", &JsValue::UNDEFINED).into()
}

/// Formats the timestamp into a locale-aware time string.
fn format_time_of_day(ms: u64) -> String {
    let date = Date::new(&JsValue::from_f64(ms as f64));
    Date::to_locale_time_string(&date, "default").into()
}

/// Returns percentage height style for chart bars.
fn bar_height(bin: u64, max_bin: u64) -> String {
    let height = if max_bin == 0 {
        0.0
    } else {
        (bin as f64 / max_bin as f64 * 100.0).max(8.0)
    };
    format!("height:{height:.0}%")
}

#[component]
/// Main dashboard component rendering startup metrics.
pub fn App() -> impl IntoView {
    let (startup_records, set_startup_records) = signal(Vec::<StartupRecord>::new());
    let (loaded, set_loaded) = signal(false);

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
                                        <span class="app__startup-subtext">{format!("Recorded {}", format_timestamp(record.recorded_at_ms))}</span>
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
                                                <span class="app__startup-list-date">{format_timestamp(record.recorded_at_ms)}</span>
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
