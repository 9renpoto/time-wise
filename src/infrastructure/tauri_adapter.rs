use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window};

use crate::domain::{app_usage_record::AppUsageRecord, startup_record::StartupRecord};

async fn invoke_command_with<T>(command: &str, payload: JsValue) -> Result<T, JsValue>
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

    let (invoke_owner, invoke_fn) =
        if let Ok(invoke_fn) = Reflect::get(&tauri, &JsValue::from_str("invoke")) {
            if invoke_fn.is_function() {
                (tauri.clone(), invoke_fn)
            } else {
                let core = Reflect::get(&tauri, &JsValue::from_str("core"))?;
                let core_invoke = Reflect::get(&core, &JsValue::from_str("invoke"))?;
                (core, core_invoke)
            }
        } else {
            let core = Reflect::get(&tauri, &JsValue::from_str("core"))?;
            let core_invoke = Reflect::get(&core, &JsValue::from_str("invoke"))?;
            (core, core_invoke)
        };

    if !invoke_fn.is_function() {
        return Err(JsValue::from_str("tauri invoke function unavailable"));
    }

    let function = invoke_fn.dyn_into::<Function>()?;
    let promise = function
        .call2(&invoke_owner, &JsValue::from_str(command), &payload)?
        .dyn_into::<Promise>()?;
    let response = JsFuture::from(promise).await?;
    serde_wasm_bindgen::from_value(response).map_err(|err| JsValue::from_str(&err.to_string()))
}

async fn invoke_command<T>(command: &str) -> Result<T, JsValue>
where
    T: serde::de::DeserializeOwned,
{
    invoke_command_with(command, JsValue::UNDEFINED).await
}

#[derive(Clone, Copy)]
pub struct AutostartStatus {
    pub enabled: bool,
    pub success: bool,
}

#[derive(serde::Serialize)]
struct AutostartPayload {
    enabled: bool,
}

pub async fn fetch_autostart_enabled() -> Result<bool, ()> {
    match invoke_command::<bool>("get_autostart_enabled").await {
        Ok(value) => Ok(value),
        Err(err) => {
            log_error(&format!("failed to fetch autostart state: {err:?}"));
            Err(())
        }
    }
}

async fn autostart_status_from_fetch(fallback: bool) -> AutostartStatus {
    match fetch_autostart_enabled().await {
        Ok(value) => AutostartStatus {
            enabled: value,
            success: false,
        },
        Err(_) => AutostartStatus {
            enabled: fallback,
            success: false,
        },
    }
}

pub async fn set_autostart_enabled(enabled: bool) -> AutostartStatus {
    let payload = match serde_wasm_bindgen::to_value(&AutostartPayload { enabled }) {
        Ok(payload) => payload,
        Err(err) => {
            log_error(&format!("failed to serialize autostart payload: {err}"));
            return autostart_status_from_fetch(enabled).await;
        }
    };

    match invoke_command_with::<bool>("set_autostart_enabled", payload).await {
        Ok(value) => AutostartStatus {
            enabled: value,
            success: value == enabled,
        },
        Err(err) => {
            log_error(&format!("failed to update autostart state: {err:?}"));
            autostart_status_from_fetch(enabled).await
        }
    }
}

pub async fn load_startup_records() -> Vec<StartupRecord> {
    match invoke_command::<Vec<StartupRecord>>("fetch_startup_records").await {
        Ok(mut records) => {
            records.sort_by(|a, b| b.recorded_at_ms.cmp(&a.recorded_at_ms));
            records
        }
        Err(err) => {
            log_error(&format!("failed to fetch startup records: {err:?}"));
            Vec::new()
        }
    }
}

pub async fn load_app_usage_records() -> Result<Vec<AppUsageRecord>, String> {
    match invoke_command::<Vec<AppUsageRecord>>("fetch_app_usage_records").await {
        Ok(mut records) => {
            sort_app_usage_records(&mut records);
            Ok(records)
        }
        Err(err) => {
            log_error(&format!("failed to fetch app usage records: {err:?}"));
            Err(format!("failed to fetch app usage records: {err:?}"))
        }
    }
}

fn log_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}

fn sort_app_usage_records(records: &mut [AppUsageRecord]) {
    records.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| b.total_active_ms.cmp(&a.total_active_ms))
            .then_with(|| b.last_seen_at_ms.cmp(&a.last_seen_at_ms))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        name: &str,
        active: bool,
        total_active_ms: u64,
        last_seen_at_ms: u64,
    ) -> AppUsageRecord {
        AppUsageRecord {
            name: name.to_string(),
            executable: None,
            total_active_ms,
            last_seen_at_ms,
            first_seen_at_ms: 0,
            active,
        }
    }

    #[test]
    fn sort_app_usage_records_prioritizes_active_then_duration_then_recent() {
        let mut records = vec![
            record("inactive-long", false, 5_000, 1_000),
            record("active-short", true, 1_000, 2_000),
            record("active-long", true, 10_000, 500),
            record("inactive-recent", false, 5_000, 10_000),
        ];

        sort_app_usage_records(&mut records);

        let names: Vec<_> = records.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "active-long",
                "active-short",
                "inactive-recent",
                "inactive-long"
            ]
        );
    }
}
