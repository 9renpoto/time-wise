use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window};

use crate::domain::startup_record::StartupRecord;

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
    let invoke_fn = Reflect::get(&tauri, &JsValue::from_str("invoke"))?;
    let function = invoke_fn.dyn_into::<Function>()?;
    let promise = function
        .call2(&tauri, &JsValue::from_str(command), &payload)?
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

fn log_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}
