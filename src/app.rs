use leptos::prelude::*;

struct CategoryUsage {
    name: &'static str,
    minutes: u32,
}

struct AppTile {
    name: &'static str,
    icon: &'static str,
    minutes: u32,
}

const TOTAL_MINUTES: u32 = 37;
const CHART_BINS: [u32; 5] = [4, 0, 2, 15, 7];
const CHART_LABELS: [&str; 4] = ["00", "06", "12", "18"];
const CATEGORY_USAGE: [CategoryUsage; 3] = [
    CategoryUsage {
        name: "Social",
        minutes: 24,
    },
    CategoryUsage {
        name: "Utilities",
        minutes: 5,
    },
    CategoryUsage {
        name: "Health & Fitness",
        minutes: 3,
    },
];

const APP_TILES: [AppTile; 6] = [
    AppTile {
        name: "Instagram",
        icon: "📸",
        minutes: 11,
    },
    AppTile {
        name: "Telegram",
        icon: "✉️",
        minutes: 5,
    },
    AppTile {
        name: "TikTok",
        icon: "🎵",
        minutes: 8,
    },
    AppTile {
        name: "Doc.ua",
        icon: "🩺",
        minutes: 3,
    },
    AppTile {
        name: "Chrome",
        icon: "🌐",
        minutes: 5,
    },
    AppTile {
        name: "Phone",
        icon: "📞",
        minutes: 1,
    },
];

fn bar_height(bin: u32, max_bin: u32) -> String {
    let height = if max_bin == 0 {
        0.0
    } else {
        (bin as f32 / max_bin as f32 * 100.0).max(8.0)
    };
    format!("w-4 rounded-md bg-gradient-to-t from-blue-500 to-blue-400 h-[{:.0}%]", height)
}

#[component]
pub fn App() -> impl IntoView {
    let chart_max = CHART_BINS.iter().copied().max().unwrap_or(0);

    view! {
        <main class="min-h-screen flex items-start justify-center p-8 bg-gradient-to-br from-slate-900 to-slate-800 font-sans text-slate-900">
            <section class="w-[360px] rounded-3xl bg-slate-50 shadow-2xl shadow-slate-900/40 overflow-hidden">
                <div class="p-6 bg-gradient-to-b from-white to-slate-100">
                    <header class="flex items-center gap-4 mb-5">
                        <div class="w-11 h-11 rounded-full bg-gradient-to-br from-blue-500 to-purple-500 flex items-center justify-center text-white font-semibold text-xl">
                            "A"
                        </div>
                        <div>
                            <div class="text-3xl font-bold text-slate-900">{format!("{}m", TOTAL_MINUTES)}</div>
                            <div class="text-sm text-slate-500">"Screen time today"</div>
                        </div>
                    </header>
                    <div class="relative h-32 mb-5 rounded-2xl bg-gradient-to-b from-slate-200 to-slate-50 p-4 pt-3 flex items-end gap-3">
                        <div class="absolute inset-0">
                            <div class="absolute inset-x-0 top-3 border-t border-dashed border-slate-400/50"></div>
                            <div class="absolute inset-x-0 top-14 border-t border-dashed border-slate-400/40"></div>
                            <div class="absolute inset-x-0 bottom-3 border-t border-solid border-slate-400/60"></div>
                        </div>
                        {CHART_BINS
                            .iter()
                            .map(|&minutes| view! {
                                <div class="flex-1 flex flex-col items-center gap-1.5">
                                    <div class="h-[72px] flex items-end">
                                        <div class=bar_height(minutes, chart_max)></div>
                                    </div>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                        <div class="absolute left-3.5 right-3.5 bottom-0 flex justify-between text-xs text-slate-500 uppercase tracking-widest">
                            {CHART_LABELS
                                .iter()
                                .map(|&label| view! { <span>{label}</span> })
                                .collect::<Vec<_>>()
                                .into_view()}
                        </div>
                        <div class="absolute top-2 right-3.5 text-xs text-slate-400 uppercase tracking-widest">"60m"</div>
                        <div class="absolute top-[52px] right-3.5 text-xs text-slate-400 uppercase tracking-widest">"30m"</div>
                        <div class="absolute bottom-3.5 right-3.5 text-xs text-slate-400 uppercase tracking-widest">"0m"</div>
                    </div>
                    <div class="flex justify-between">
                        {CATEGORY_USAGE
                            .iter()
                            .map(|category| view! {
                                <div class="flex flex-col items-center gap-1 flex-1">
                                    <span class=format!("text-sm font-medium {}",
                                        match category.name {
                                            "Social" => "text-blue-600",
                                            "Utilities" => "text-sky-500",
                                            "Health & Fitness" => "text-orange-500",
                                            _ => "text-slate-500",
                                        }
                                    )>
                                        {category.name}
                                    </span>
                                    <span class="text-xs text-slate-600">{format!("{}m", category.minutes)}</span>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                    </div>
                </div>
                <div class="p-6 pt-5 bg-slate-100 grid grid-cols-2 gap-4">
                    {APP_TILES
                        .iter()
                        .map(|tile| view! {
                            <div class="flex items-center gap-3 p-3 pr-2 rounded-2xl bg-white shadow-md shadow-slate-200/50">
                                <div class="w-9 h-9 rounded-xl flex items-center justify-center text-2xl">
                                    {tile.icon}
                                </div>
                                <div class="flex flex-col gap-1">
                                    <span class="text-sm font-medium text-slate-900">{tile.name}</span>
                                    <span class="text-xs text-slate-500">{format!("{}m", tile.minutes)}</span>
                                </div>
                            </div>
                        })
                        .collect::<Vec<_>>()
                        .into_view()}
                </div>
            </section>
        </main>
    }
}

#[cfg(test)]
mod tests {
    use super::bar_height;

    #[test]
    fn bar_height_zero_max_returns_zero_percent() {
        let style = bar_height(0, 0);
        assert!(style.contains("h-[0%]"));
    }

    #[test]
    fn bar_height_scales_to_full_height() {
        let style = bar_height(15, 15);
        assert!(style.contains("h-[100%]"));
    }

    #[test]
    fn bar_height_applies_minimum_percentage() {
        let style = bar_height(0, 15);
        assert!(style.contains("h-[8%]"));
    }
}
