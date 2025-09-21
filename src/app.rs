use leptos::prelude::*;

struct CategoryUsage {
    name: &'static str,
    class_names: &'static str,
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
        class_names: "app__category-name app__category-name--social",
        minutes: 24,
    },
    CategoryUsage {
        name: "Utilities",
        class_names: "app__category-name app__category-name--utilities",
        minutes: 5,
    },
    CategoryUsage {
        name: "Health & Fitness",
        class_names: "app__category-name app__category-name--health",
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
    format!("height:{height:.0}%")
}

#[component]
pub fn App() -> impl IntoView {
    let chart_max = CHART_BINS.iter().copied().max().unwrap_or(0);

    view! {
        <main class="app">
            <section class="app__card">
                <div class="app__summary">
                    <header class="app__profile">
                        <div class="app__avatar">
                            "A"
                        </div>
                        <div>
                            <div class="app__total">{format!("{}m", TOTAL_MINUTES)}</div>
                            <div class="app__label">"Screen time today"
                            </div>
                        </div>
                    </header>
                    <div class="app__chart">
                        <div class="app__chart-overlay">
                            <div class="app__chart-grid-line app__chart-grid-line--top"></div>
                            <div class="app__chart-grid-line app__chart-grid-line--middle"></div>
                            <div class="app__chart-grid-line app__chart-grid-line--bottom"></div>
                        </div>
                        {CHART_BINS
                            .iter()
                            .map(|&minutes| view! {
                                <div class="app__chart-column">
                                    <div class="app__chart-column-inner">
                                        <div class="app__chart-bar" style=bar_height(minutes, chart_max)></div>
                                    </div>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                        <div class="app__chart-labels">
                            {CHART_LABELS
                                .iter()
                            .map(|&label| view! { <span>{label}</span> })
                                .collect::<Vec<_>>()
                                .into_view()}
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--top">"60m"
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--middle">"30m"
                        </div>
                        <div class="app__chart-annotation app__chart-annotation--bottom">"0m"
                        </div>
                    </div>
                    <div class="app__categories">
                        {CATEGORY_USAGE
                            .iter()
                            .map(|category| view! {
                                <div class="app__category">
                                    <span class=category.class_names>
                                        {category.name}
                                    </span>
                                    <span class="app__category-minutes">{format!("{}m", category.minutes)}</span>
                                </div>
                            })
                            .collect::<Vec<_>>()
                            .into_view()}
                    </div>
                </div>
                <div class="app__grid">
                    {APP_TILES
                        .iter()
                        .map(|tile| view! {
                            <div class="app__tile">
                                <div class="app__tile-icon">
                                    {tile.icon}
                                </div>
                                <div class="app__tile-info">
                                    <span class="app__tile-name">{tile.name}</span>
                                    <span class="app__tile-minutes">{format!("{}m", tile.minutes)}</span>
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
        assert!(style.contains("height:0%"));
    }

    #[test]
    fn bar_height_scales_to_full_height() {
        let style = bar_height(15, 15);
        assert!(style.contains("height:100%"));
    }

    #[test]
    fn bar_height_applies_minimum_percentage() {
        let style = bar_height(0, 15);
        assert!(style.contains("height:8%"));
    }
}
