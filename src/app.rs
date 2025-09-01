use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    // Dummy data for the chart
    let app_usage = vec![
        ("VS Code", 80),
        ("Discord", 50),
        ("GitHub Desktop", 100),
        ("Obsidian", 20),
        ("Warp", 65),
    ];

    view! {
        <main style="padding: 16px; background-color: #111827; color: white; min-height: 100vh;">
            <h1 style="font-size: 1.25rem; text-align: center; margin-bottom: 1rem;">
                "Application Usage"
            </h1>
            <div
                class="chart-container"
                style="display: flex; justify-content: space-around; align-items: flex-end; height: 256px; padding: 16px; border: 1px solid #374151; border-radius: 0.5rem;"
            >
                {app_usage.into_iter()
                    .map(|(name, height)| view! {
                        <div class="bar-wrapper" style="display: flex; flex-direction: column; align-items: center;">
                            <div
                                class="bar"
                                style=format!(
                                    "background-color: #3b82f6; width: 2rem; border-top-left-radius: 0.125rem; border-top-right-radius: 0.125rem; height: {}%;",
                                    height
                                )
                            ></div>
                            <span style="font-size: 0.75rem; margin-top: 0.5rem;">{name}</span>
                        </div>
                    })
                    .collect_view()}
            </div>
        </main>
    }
}
