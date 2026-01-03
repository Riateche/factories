pub mod analyze;
pub mod config;
pub mod editor;
pub mod flowchart;
pub mod game_data;
pub mod info;
pub mod machine;
pub mod primitives;
pub mod snippet;
pub mod ui;

pub use crate::info::Info;
use {machine::Module, std::collections::BTreeMap, tracing::warn};

/// Round float to second decimal digit.
/// It's better than formatting it because we want values like "5.2", not "5.20".
fn rf(f: f64) -> f64 {
    if f.abs() > 0.1 {
        return (f * 100.0).round() / 100.0;
    }

    let decimals = 2;
    if f == 0. || decimals == 0 {
        0.0
    } else {
        let shift = decimals - f.abs().log10().ceil() as i32;
        let shift_factor = 10_f64.powi(shift);

        (f * shift_factor).round() / shift_factor
    }
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

fn module_counts(modules: &[Module]) -> BTreeMap<String, usize> {
    let mut module_counts = BTreeMap::<_, usize>::new();
    for module in modules {
        *module_counts
            .entry(format!("{}.q{}", module.name, module.quality.0))
            .or_default() += 1;
    }
    module_counts
}
