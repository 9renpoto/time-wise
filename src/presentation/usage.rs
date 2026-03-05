use js_sys::Date;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsValue;

use crate::domain::app_usage_record::AppUsageRecord;
use crate::infrastructure::tauri_adapter::load_app_usage_records;

fn to_pretty_json(records: &[AppUsageRecord]) -> String {
    serde_json::to_string_pretty(records).unwrap_or_else(|_| "[]".to_string())
}

#[component]
/// Usage view renders the raw SQLite-backed usage records as JSON for verification.
pub fn UsageView() -> impl IntoView {
    let (usage_records, set_usage_records) = signal(Vec::<AppUsageRecord>::new());
    let (loading, set_loading) = signal(true);
    let (last_loaded, set_last_loaded) = signal(None::<String>);
    let (load_error, set_load_error) = signal(None::<String>);

    let fetch_records = move || {
        spawn_local({
            let set_usage_records = set_usage_records;
            let set_loading = set_loading;
            let set_last_loaded = set_last_loaded;
            let set_load_error = set_load_error;
            async move {
                set_loading.set(true);
                set_load_error.set(None);
                match load_app_usage_records().await {
                    Ok(records) => {
                        set_usage_records.set(records);
                        let timestamp: String = Date::new_0()
                            .to_locale_string("en-US", &JsValue::UNDEFINED)
                            .into();
                        set_last_loaded.set(Some(timestamp));
                    }
                    Err(error_message) => {
                        set_load_error.set(Some(error_message));
                    }
                }
                set_loading.set(false);
            }
        });
    };

    fetch_records();

    let usage_json = Signal::derive(move || usage_records.with(|records| to_pretty_json(records)));

    view! {
        <section class="usage">
            <header class="usage__header">
                <div>
                    <h1 class="usage__title">"Usage"</h1>
                    <p class="usage__description">
                        "Inspect raw usage records captured by the desktop agent."
                    </p>
                </div>
                <button
                    type="button"
                    class="usage__refresh"
                    on:click=move |_| fetch_records()
                    disabled=move || loading.get()
                >
                    "Refresh"
                </button>
            </header>
            <div class="usage__status">
                <Show
                    when=move || loading.get()
                    fallback=move || {
                        if let Some(error_message) = load_error.get() {
                            view! {
                                <span class="usage__error">{format!("Load failed: {error_message}")}</span>
                            }
                                .into_any()
                        } else {
                            view! {
                                <span>
                                    {move || {
                                        last_loaded
                                            .get()
                                            .map(|value| format!("Loaded at {value}."))
                                            .unwrap_or_else(|| "Loaded.".to_string())
                                    }}
                                </span>
                            }
                                .into_any()
                        }
                    }
                >
                    <span>"Loading usage data…"</span>
                </Show>
                <span class="usage__count">{move || {
                    let count = usage_records.with(|records| records.len());
                    format!("{count} records")
                }}</span>
            </div>
            <pre class="usage__json">{move || usage_json.get()}</pre>
        </section>
    }
}
