use leptos::prelude::*;

struct CategoryUsage {
    name: &'static str,
    color: &'static str,
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
        color: "#2563eb",
        minutes: 24,
    },
    CategoryUsage {
        name: "Utilities",
        color: "#0ea5e9",
        minutes: 5,
    },
    CategoryUsage {
        name: "Health & Fitness",
        color: "#f97316",
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
    format!(
        "width:16px; border-radius:6px; background:linear-gradient(180deg, #60a5fa, #3b82f6); height:{height:.0}%;"
    )
}

#[component]
pub fn App() -> impl IntoView {
    let chart_max = CHART_BINS.iter().copied().max().unwrap_or(0);

    view! {
        <main style="min-height:100vh; display:flex; align-items:flex-start; justify-content:center; padding:32px; background:linear-gradient(135deg,#0f172a,#1f2937); font-family:'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color:#0f172a;">
            <section style="width:360px; border-radius:28px; background-color:#f8fafc; box-shadow:0 24px 48px rgba(15,23,42,0.35); overflow:hidden;">
                <div style="padding:24px; background:linear-gradient(180deg,#ffffff,#f1f5f9);">
                    <header style="display:flex; align-items:center; gap:1rem; margin-bottom:20px;">
                        <div style="width:44px; height:44px; border-radius:50%; background:linear-gradient(135deg,#3b82f6,#a855f7); display:flex; align-items:center; justify-content:center; color:#fff; font-weight:600; font-size:1.25rem;">
                            "A"
                        </div>
                        <div>
                            <div style="font-size:2rem; font-weight:600; color:#0f172a;">{format!("{}m", TOTAL_MINUTES)}</div>
                            <div style="font-size:0.85rem; color:#64748b;">"Screen time today"
                            </div>
                        </div>
                    </header>
                    <div style="position:relative; height:120px; margin-bottom:20px; border-radius:18px; background:linear-gradient(180deg,#e2e8f0,#f8fafc); padding:16px 14px 12px; display:flex; align-items:flex-end; gap:12px;">
                        <div style="position:absolute; inset:0;">
                            <div style="position:absolute; left:0; right:0; top:12px; border-top:1px dashed rgba(148,163,184,0.5);"></div>
                            <div style="position:absolute; left:0; right:0; top:56px; border-top:1px dashed rgba(148,163,184,0.35);"></div>
                            <div style="position:absolute; left:0; right:0; bottom:12px; border-top:1px solid rgba(148,163,184,0.6);"></div>
                        </div>
                        {CHART_BINS
                            .iter()
                            .map(|&minutes| view! {
                                <div style="flex:1; display:flex; flex-direction:column; align-items:center; gap:6px;">
                                    <div style="height:72px; display:flex; align-items:flex-end;">
                                        <div style=bar_height(minutes, chart_max)></div>
                                    </div>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                        <div style="position:absolute; left:14px; right:14px; bottom:0; display:flex; justify-content:space-between; font-size:0.65rem; color:#64748b; text-transform:uppercase; letter-spacing:0.08em;">
                            {CHART_LABELS
                                .iter()
                            .map(|&label| view! { <span>{label}</span> })
                                .collect::<Vec<_>>()
                                .into_view()}
                        </div>
                        <div style="position:absolute; top:8px; right:14px; font-size:0.65rem; color:#94a3b8; text-transform:uppercase; letter-spacing:0.08em;">"60m"
                        </div>
                        <div style="position:absolute; top:52px; right:14px; font-size:0.65rem; color:#94a3b8; text-transform:uppercase; letter-spacing:0.08em;">"30m"
                        </div>
                        <div style="position:absolute; bottom:14px; right:14px; font-size:0.65rem; color:#94a3b8; text-transform:uppercase; letter-spacing:0.08em;">"0m"
                        </div>
                    </div>
                    <div style="display:flex; justify-content:space-between;">
                        {CATEGORY_USAGE
                            .iter()
                            .map(|category| view! {
                                <div style="display:flex; flex-direction:column; align-items:center; gap:4px; flex:1;">
                                    <span style=move || format!("color:{}; font-size:0.85rem; font-weight:500;", category.color)>
                                        {category.name}
                                    </span>
                                    <span style="font-size:0.8rem; color:#475569;">{format!("{}m", category.minutes)}</span>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                    </div>
                </div>
                <div style="padding:20px 24px 24px; background-color:#f1f5f9; display:grid; grid-template-columns:repeat(2, minmax(0, 1fr)); gap:16px;">
                    {APP_TILES
                        .iter()
                        .map(|tile| view! {
                            <div style="display:flex; align-items:center; gap:12px; padding:8px 12px; border-radius:16px; background-color:#ffffff; box-shadow:0 4px 12px rgba(15,23,42,0.08);">
                                <div style="width:36px; height:36px; border-radius:12px; display:flex; align-items:center; justify-content:center; font-size:1.4rem;">
                                    {tile.icon}
                                </div>
                                <div style="display:flex; flex-direction:column; gap:4px;">
                                    <span style="font-size:0.9rem; font-weight:500; color:#0f172a;">{tile.name}</span>
                                    <span style="font-size:0.75rem; color:#64748b;">{format!("{}m", tile.minutes)}</span>
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
