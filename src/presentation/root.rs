use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::infrastructure::tauri_adapter::fetch_onboarding_completed;
use crate::presentation::{dashboard::Dashboard, onboarding::Onboarding};

#[component]
/// Resolves first-run state before showing onboarding or the dashboard.
pub fn Root() -> impl IntoView {
    let (onboarding_completed, set_onboarding_completed) = signal(None::<bool>);
    let (load_error, set_load_error) = signal(false);
    let (retry, set_retry) = signal(0u64);

    Effect::new(move |_| {
        let _ = retry.get();
        set_load_error.set(false);
        spawn_local(async move {
            match fetch_onboarding_completed().await {
                Ok(completed) => set_onboarding_completed.set(Some(completed)),
                Err(_) => {
                    set_onboarding_completed.set(None);
                    set_load_error.set(true);
                }
            }
        });
    });

    view! {
        {move || match onboarding_completed.get() {
            Some(true) => view! { <Dashboard /> }.into_any(),
            Some(false) => view! {
                <Onboarding on_complete=Callback::new(move |()| {
                    set_onboarding_completed.set(Some(true));
                }) />
            }.into_any(),
            None if load_error.get() => view! {
                <main class="onboarding-shell">
                    <section class="onboarding-state" role="alert">
                        <span class="onboarding-state__icon">"!"</span>
                        <h1>"Setup unavailable"</h1>
                        <p>"Time Wise couldn't load its local setup state."</p>
                        <button on:click=move |_| {
                            set_retry.update(|value| *value = value.wrapping_add(1));
                        }>"Try again"</button>
                    </section>
                </main>
            }.into_any(),
            None => view! {
                <main class="onboarding-shell">
                    <section class="onboarding-state" aria-label="Loading setup">
                        <span class="onboarding-state__spinner"></span>
                        <p>"Preparing Time Wise…"</p>
                    </section>
                </main>
            }.into_any(),
        }}
    }
}
