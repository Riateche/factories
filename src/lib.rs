pub mod analyze;
pub mod config;
pub mod flowchart;
pub mod game_data;
pub mod info;
pub mod machine;
pub mod snippet;
pub mod ui;

// round float
pub fn rf(f: f64) -> f64 {
    (f * 100.0).round() / 100.0
}
