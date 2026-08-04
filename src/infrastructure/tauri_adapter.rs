use js_sys::{Function, Promise, Reflect};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window};

use crate::domain::usage_summary::{DailyUsageSummary, WeeklyUsageSummary};

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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CompleteOnboardingPayload {
    enable_autostart: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageDatePayload<'a> {
    local_date: &'a str,
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

pub async fn fetch_onboarding_completed() -> Result<bool, String> {
    invoke_command("get_onboarding_completed")
        .await
        .map_err(|error| {
            log_error(&format!("failed to fetch onboarding state: {error:?}"));
            format!("failed to fetch onboarding state: {error:?}")
        })
}

pub async fn complete_onboarding(enable_autostart: bool) -> Result<(), String> {
    let payload = serde_wasm_bindgen::to_value(&CompleteOnboardingPayload { enable_autostart })
        .map_err(|error| error.to_string())?;
    invoke_command_with("complete_onboarding", payload)
        .await
        .map_err(|error| {
            log_error(&format!("failed to complete onboarding: {error:?}"));
            format!("failed to complete onboarding: {error:?}")
        })
}

pub async fn delete_all_usage_history() -> Result<(), String> {
    invoke_command("delete_all_usage_history")
        .await
        .map_err(|error| {
            log_error(&format!("failed to delete usage history: {error:?}"));
            format!("failed to delete usage history: {error:?}")
        })
}

pub async fn load_daily_usage_summary(local_date: &str) -> Result<DailyUsageSummary, String> {
    let payload = serde_wasm_bindgen::to_value(&UsageDatePayload { local_date })
        .map_err(|error| error.to_string())?;
    invoke_command_with("fetch_daily_usage_summary", payload)
        .await
        .map_err(|error| {
            log_error(&format!("failed to fetch daily usage summary: {error:?}"));
            format!("failed to fetch daily usage summary: {error:?}")
        })
}

pub async fn load_weekly_usage_summary(local_date: &str) -> Result<WeeklyUsageSummary, String> {
    let payload = serde_wasm_bindgen::to_value(&UsageDatePayload { local_date })
        .map_err(|error| error.to_string())?;
    invoke_command_with("fetch_weekly_usage_summary", payload)
        .await
        .map_err(|error| {
            log_error(&format!("failed to fetch weekly usage summary: {error:?}"));
            format!("failed to fetch weekly usage summary: {error:?}")
        })
}

fn log_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}
