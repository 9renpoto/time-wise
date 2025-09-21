use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::window;

use crate::domain::startup_record::StartupRecord;

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

pub async fn load_startup_records() -> Vec<StartupRecord> {
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
