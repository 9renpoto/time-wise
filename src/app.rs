use leptos::task::spawn_local;
use leptos::{ev::SubmitEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}

#[component]
pub fn App() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (greet_msg, set_greet_msg) = signal(String::new());

    let update_name = move |ev| {
        let v = event_target_value(&ev);
        set_name.set(v);
    };

    let greet = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let name = name.get_untracked();
            if name.is_empty() {
                return;
            }

            let args = serde_wasm_bindgen::to_value(&GreetArgs { name: &name }).unwrap();
            // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
            let new_msg = invoke("greet", args).await.as_string().unwrap();
            set_greet_msg.set(new_msg);
        });
    };

    view! {
        <main class="my-0 mx-auto pt-[10vh] flex flex-col justify-center text-center">
            <h1 class="text-4xl text-center">"Welcome to Tauri + Leptos"</h1>

            <div class="flex justify-center">
                <a href="https://tauri.app" target="_blank">
                    <img src="public/tauri.svg" class="h-24 p-6 will-change-filter duration-700 hover:drop-shadow-[0_0_2em_#24c8db]" alt="Tauri logo"/>
                </a>
                <a href="https://docs.rs/leptos/" target="_blank">
                    <img src="public/leptos.svg" class="h-24 p-6 will-change-filter duration-700 hover:drop-shadow-[0_0_2em_#a82e20]" alt="Leptos logo"/>
                </a>
            </div>
            <p>"Click on the Tauri and Leptos logos to learn more."</p>

            <form class="flex justify-center" on:submit=greet>
                <input
                    id="greet-input"
                    class="rounded-lg border border-transparent px-3 py-2 text-base font-medium font-sans bg-white text-[#0f0f0f] transition-colors duration-300 shadow-[0_2px_2px_rgba(0,0,0,0.2)] focus:outline-none focus:border-[#396cd8] mr-1 dark:bg-[#0f0f0f98] dark:text-white"
                    placeholder="Enter a name..."
                    on:input=update_name
                />
                <button
                    type="submit"
                    class="rounded-lg border border-transparent px-3 py-2 text-base font-medium font-sans bg-white text-[#0f0f0f] transition-colors duration-300 shadow-[0_2px_2px_rgba(0,0,0,0.2)] cursor-pointer hover:border-[#396cd8] active:border-[#396cd8] active:bg-[#e8e8e8] dark:bg-[#0f0f0f98] dark:text-white dark:active:bg-[#0f0f0f69]"
                >
                    "Greet"
                </button>
            </form>
            <p>{ move || greet_msg.get() }</p>
        </main>
    }
}
