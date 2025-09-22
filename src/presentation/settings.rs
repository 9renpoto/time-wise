use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::infrastructure::tauri_adapter::{
    fetch_autostart_enabled, set_autostart_enabled, AutostartStatus,
};

#[component]
/// Settings screen exposing application preferences.
pub fn Settings() -> impl IntoView {
    let (autostart_enabled, set_autostart_enabled_signal) = signal(false);
    let (loaded, set_loaded) = signal(false);
    let (status_message, set_status_message) = signal(None::<String>);
    let (saving, set_saving) = signal(false);

    Effect::new(move |_| {
        if loaded.get() {
            return;
        }
        spawn_local({
            let set_autostart = set_autostart_enabled_signal;
            let set_loaded = set_loaded;
            let set_message = set_status_message;
            async move {
                match fetch_autostart_enabled().await {
                    Ok(state) => {
                        set_autostart.set(state);
                        set_message.set(None);
                    }
                    Err(()) => {
                        set_message.set(Some(
                            "Unable to load automatic launch preference.".to_string(),
                        ));
                    }
                }
                set_loaded.set(true);
            }
        });
    });

    view! {
        <main class="settings-app">
            <section class="settings">
                <header class="settings__header">
                    <h1 class="settings__title">"Settings"</h1>
                    <p class="settings__subtitle">
                        "Control how Time Wise behaves on startup."
                    </p>
                </header>
                <div class="settings__content">
                    <label class="settings__item">
                        <input
                            type="checkbox"
                            class="settings__checkbox"
                            prop:checked=move || autostart_enabled.get()
                            on:change=move |ev| {
                                let Some(target) = ev
                                    .target()
                                    .and_then(|value| value.dyn_into::<HtmlInputElement>().ok())
                                else {
                                    return;
                                };
                                let desired = target.checked();

                                if saving.get() {
                                    target.set_checked(autostart_enabled.get());
                                    return;
                                }

                                set_status_message.set(None);
                                set_autostart_enabled_signal.set(desired);
                                set_saving.set(true);

                                spawn_local({
                                    let set_autostart = set_autostart_enabled_signal;
                                    let set_message = set_status_message;
                                    let set_saving = set_saving;
                                    async move {
                                        let AutostartStatus { enabled, success } =
                                            set_autostart_enabled(desired).await;
                                        set_autostart.set(enabled);
                                        if success {
                                            set_message.set(None);
                                        } else {
                                            set_message.set(Some(
                                                "Could not update automatic launch preference."
                                                    .to_string(),
                                            ));
                                        }
                                        set_saving.set(false);
                                    }
                                });
                            }
                            disabled=move || !loaded.get() || saving.get()
                        />
                        <div class="settings__details">
                            <span class="settings__label">"Launch on startup"</span>
                            <span class="settings__description">
                                "Start Time Wise automatically when your machine boots."
                            </span>
                        </div>
                    </label>
                    <Show when=move || status_message.get().is_some()>
                        {move || {
                            status_message
                                .get()
                                .map(|message| view! { <p class="settings__status">{message}</p> })
                        }}
                    </Show>
                </div>
            </section>
        </main>
    }
}
