use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::infrastructure::tauri_adapter::complete_onboarding;

#[component]
/// First-run setup requiring an explicit automatic-launch choice.
pub fn Onboarding(on_complete: Callback<()>) -> impl IntoView {
    let (autostart_choice, set_autostart_choice) = signal(None::<bool>);
    let (submitting, set_submitting) = signal(false);
    let (error_message, set_error_message) = signal(None::<String>);

    view! {
        <main class="onboarding-shell">
            <section class="onboarding">
                <header class="onboarding__header">
                    <div class="onboarding__mark">"TW"</div>
                    <span class="onboarding__eyebrow">"WELCOME TO TIME WISE"</span>
                    <h1>"Understand where your time goes"</h1>
                    <p>
                        "Time Wise measures the application in focus and stores the history only on this computer. Window titles, URLs, and document names are never collected."
                    </p>
                </header>

                <div class="onboarding__step">
                    <div class="onboarding__step-number">"1"</div>
                    <div>
                        <h2>"Choose automatic launch"</h2>
                        <p>
                            "Time Wise measures usage while it is running in the system tray. Choose whether it should start when you sign in to Windows."
                        </p>
                    </div>
                </div>

                <div class="onboarding__choices" role="radiogroup" aria-label="Automatic launch preference">
                    <button
                        role="radio"
                        aria-checked=move || (autostart_choice.get() == Some(true)).to_string()
                        class:onboarding__choice=true
                        class:onboarding__choice--selected=move || autostart_choice.get() == Some(true)
                        disabled=move || submitting.get()
                        on:click=move |_| set_autostart_choice.set(Some(true))
                    >
                        <span class="onboarding__choice-icon">"↗"</span>
                        <span>
                            <strong>"Start automatically"</strong>
                            <small>"Recommended for complete daily history"</small>
                        </span>
                        <span class="onboarding__radio"></span>
                    </button>
                    <button
                        role="radio"
                        aria-checked=move || (autostart_choice.get() == Some(false)).to_string()
                        class:onboarding__choice=true
                        class:onboarding__choice--selected=move || autostart_choice.get() == Some(false)
                        disabled=move || submitting.get()
                        on:click=move |_| set_autostart_choice.set(Some(false))
                    >
                        <span class="onboarding__choice-icon">"○"</span>
                        <span>
                            <strong>"I'll start it myself"</strong>
                            <small>"You can change this later in Settings"</small>
                        </span>
                        <span class="onboarding__radio"></span>
                    </button>
                </div>

                <Show when=move || error_message.get().is_some()>
                    <p class="onboarding__error" role="alert">
                        {move || error_message.get().unwrap_or_default()}
                    </p>
                </Show>

                <footer class="onboarding__footer">
                    <span>"Your choice can be changed at any time."</span>
                    <button
                        class="onboarding__continue"
                        disabled=move || autostart_choice.get().is_none() || submitting.get()
                        on:click=move |_| {
                            let Some(enable_autostart) = autostart_choice.get_untracked() else {
                                return;
                            };
                            set_submitting.set(true);
                            set_error_message.set(None);
                            spawn_local(async move {
                                match complete_onboarding(enable_autostart).await {
                                    Ok(()) => on_complete.run(()),
                                    Err(_) => {
                                        set_error_message.set(Some(
                                            "Setup could not be saved. Check the system settings and try again."
                                                .to_string(),
                                        ));
                                        set_submitting.set(false);
                                    }
                                }
                            });
                        }
                    >{move || if submitting.get() { "Saving…" } else { "Continue" }}</button>
                </footer>
            </section>
        </main>
    }
}
