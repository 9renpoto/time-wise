use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::infrastructure::tauri_adapter::{
    delete_all_usage_history, fetch_autostart_enabled, set_autostart_enabled, AutostartStatus,
};

#[component]
/// Settings screen exposing application preferences.
pub fn Settings() -> impl IntoView {
    let (autostart_enabled, set_autostart_enabled_signal) = signal(false);
    let (loaded, set_loaded) = signal(false);
    let (status_message, set_status_message) = signal(None::<String>);
    let (status_is_error, set_status_is_error) = signal(false);
    let (saving, set_saving) = signal(false);
    let (show_delete_confirmation, set_show_delete_confirmation) = signal(false);
    let (deleting, set_deleting) = signal(false);

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
                        set_status_is_error.set(true);
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
                        "Control startup behavior and locally stored usage data."
                    </p>
                </header>
                <div class="settings__content">
                    <section class="settings__group">
                        <div class="settings__group-heading">
                            <span class="settings__eyebrow">"GENERAL"</span>
                            <h2>"Startup"</h2>
                        </div>
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
                                                set_status_is_error.set(true);
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
                                    "Start Time Wise automatically when you sign in to your computer."
                                </span>
                            </div>
                        </label>
                    </section>

                    <section class="settings__group settings__group--danger">
                        <div class="settings__group-heading">
                            <span class="settings__eyebrow">"DATA"</span>
                            <h2>"Usage history"</h2>
                        </div>
                        <div class="settings__danger-item">
                            <div class="settings__details">
                                <span class="settings__label">"Delete all usage history"</span>
                                <span class="settings__description">
                                    "Permanently remove every recorded session and cached application icon from this computer."
                                </span>
                            </div>
                            <button
                                class="settings__delete-button"
                                disabled=move || deleting.get()
                                on:click=move |_| {
                                    set_status_message.set(None);
                                    set_show_delete_confirmation.set(true);
                                }
                            >"Delete…"</button>
                        </div>
                    </section>

                    <Show when=move || status_message.get().is_some()>
                        {move || {
                            status_message
                                .get()
                                .map(|message| view! {
                                    <p
                                        class="settings__status"
                                        class:settings__status--success=move || !status_is_error.get()
                                    >{message}</p>
                                })
                        }}
                    </Show>
                </div>
            </section>

            <Show when=move || show_delete_confirmation.get()>
                <div class="settings-dialog-backdrop">
                    <section
                        class="settings-dialog"
                        role="alertdialog"
                        aria-modal="true"
                        aria-labelledby="delete-history-title"
                    >
                        <div class="settings-dialog__icon">"!"</div>
                        <h2 id="delete-history-title">"Delete all usage history?"</h2>
                        <p>
                            "This permanently deletes every measured session and application entry. This action cannot be undone. Your preferences will be kept."
                        </p>
                        <div class="settings-dialog__actions">
                            <button
                                class="settings-dialog__cancel"
                                disabled=move || deleting.get()
                                on:click=move |_| set_show_delete_confirmation.set(false)
                            >"Cancel"</button>
                            <button
                                class="settings-dialog__confirm"
                                disabled=move || deleting.get()
                                on:click=move |_| {
                                    set_deleting.set(true);
                                    set_status_message.set(None);
                                    spawn_local(async move {
                                        match delete_all_usage_history().await {
                                            Ok(()) => {
                                                set_status_is_error.set(false);
                                                set_status_message.set(Some(
                                                    "All usage history was deleted.".to_string(),
                                                ));
                                                set_show_delete_confirmation.set(false);
                                            }
                                            Err(_) => {
                                                set_status_is_error.set(true);
                                                set_status_message.set(Some(
                                                    "Could not delete usage history.".to_string(),
                                                ));
                                            }
                                        }
                                        set_deleting.set(false);
                                    });
                                }
                            >{move || if deleting.get() { "Deleting…" } else { "Delete permanently" }}</button>
                        </div>
                    </section>
                </div>
            </Show>
        </main>
    }
}
