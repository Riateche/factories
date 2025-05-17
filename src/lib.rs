mod analyze;
mod config;
mod flowchart;
mod game_data;
mod info;
mod machine;
mod snippet;
pub mod ui;

use tracing::warn;

pub use crate::info::Info;

/// Round float to second decimal digit.
/// It's better than formatting it because we want values like "5.2", not "5.20".
fn rf(f: f64) -> f64 {
    (f * 100.0).round() / 100.0
}

fn report_error(error: impl Into<anyhow::Error>) {
    warn!("{}", error.into());
}

trait ResultExtOrWarn {
    type Output;
    fn or_warn(self) -> Option<Self::Output>;
}

impl<T, E> ResultExtOrWarn for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    type Output = T;

    fn or_warn(self) -> Option<Self::Output> {
        self.map_err(|err| report_error(err)).ok()
    }
}
