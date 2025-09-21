#![allow(clippy::module_name_repetitions)]

#[derive(Clone)]
/// Data point backing the histogram chart.
pub struct ChartPoint {
    pub label: String,
    pub duration_ms: u64,
}

#[derive(Clone)]
/// Aggregated summary per performance bucket.
pub struct CategorySummary {
    pub name: &'static str,
    pub class_names: &'static str,
    pub summary: String,
}

#[derive(Clone)]
/// UI model for each startup tile.
pub struct StartupTile {
    pub icon: &'static str,
    pub label: String,
    pub duration: String,
}
