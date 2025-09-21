mod application;
mod domain;
mod infrastructure;
mod presentation;

use leptos::prelude::*;
use presentation::dashboard::Dashboard;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <Dashboard />
        }
    })
}
