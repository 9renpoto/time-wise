mod application;
mod domain;
mod infrastructure;
mod presentation;

use leptos::prelude::*;
use presentation::dashboard::Dashboard;
use presentation::settings::Settings;
use web_sys::window;

fn should_render_settings() -> bool {
    window()
        .and_then(|win| win.location().search().ok())
        .map(|query| query.contains("view=settings"))
        .unwrap_or(false)
}

fn main() {
    console_error_panic_hook::set_once();
    if should_render_settings() {
        mount_to_body(|| view! { <Settings /> });
    } else {
        mount_to_body(|| view! { <Dashboard /> });
    }
}
