use {
    crate::{
        game_data::{Crafter, Recipe},
        rf, Module,
    },
    itertools::Itertools,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    // TODO: non-crafting machines?
    pub crafter: Crafter,
    pub crafter_count: f64,
    pub modules: Vec<Module>,
    pub beacons: Vec<Vec<Module>>,
    pub recipe: Recipe,
    pub count_constraint: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ItemSpeed {
    pub item: String,
    pub speed: f64,
}

impl Machine {
    pub fn beacon_transmission_strength(&self) -> f64 {
        if self.beacons.is_empty() {
            0.0
        } else {
            1.5 * (self.beacons.len() as f64).powf(0.5)
        }
    }

    // Not including productivity.
    pub fn crafts_per_second(&self) -> f64 {
        let beacon_transmission_strength = self.beacon_transmission_strength();
        let module_speed_percents: f64 = self
            .modules
            .iter()
            .map(|module| module.speed_delta_percent)
            .sum();
        let beacon_speed_percents: f64 = self
            .beacons
            .iter()
            .flatten()
            .map(|module| module.speed_delta_percent)
            .sum();
        let speed_percents =
            100. + module_speed_percents + beacon_transmission_strength * beacon_speed_percents;

        (speed_percents / 100.) * self.crafter.crafting_speed * self.crafter_count
            / self.recipe.energy
    }

    pub fn input_speeds(&self) -> impl Iterator<Item = ItemSpeed> + '_ {
        let crafts_per_second = self.crafts_per_second();
        self.recipe.ingredients.iter().map(move |ing| ItemSpeed {
            item: ing.name.clone(),
            speed: -crafts_per_second * ing.amount,
        })
    }

    pub fn output_speeds(&self) -> impl Iterator<Item = ItemSpeed> + '_ {
        let crafts_per_second = self.crafts_per_second();
        let beacon_transmission_strength = self.beacon_transmission_strength();
        let module_prod_percents: f64 = self
            .modules
            .iter()
            .map(|module| module.productivity_delta_percent)
            .sum();
        let beacon_prod_percents: f64 = self
            .beacons
            .iter()
            .flatten()
            .map(|module| module.productivity_delta_percent)
            .sum();
        let prod_percents =
            100. + module_prod_percents + beacon_transmission_strength * beacon_prod_percents;

        let output_speed = (prod_percents / 100.) * crafts_per_second;

        self.recipe.products.iter().map(move |product| ItemSpeed {
            item: product.name.clone(),
            speed: output_speed * product.amount,
        })
    }

    pub fn item_speeds(&self) -> impl Iterator<Item = ItemSpeed> + '_ {
        self.input_speeds().chain(self.output_speeds())
    }

    pub fn io_text(&self) -> String {
        let inputs = self
            .input_speeds()
            .map(|ing| format!("{}/s {}", rf(ing.speed), ing.item))
            .join(" + ");

        let outputs = self
            .output_speeds()
            .map(|ing| format!("{}/s {}", rf(ing.speed), ing.item))
            .join(" + ");

        let inputs = if inputs.is_empty() {
            String::new()
        } else {
            format!("{} ➡ ", inputs)
        };
        let outputs = if outputs.is_empty() {
            String::new()
        } else {
            format!(" ➡ {}", outputs)
        };
        let crafter_count = if self.crafter.name == "source" || self.crafter.name == "sink" {
            String::new()
        } else {
            format!("{} × ", rf(self.crafter_count))
        };
        let emoji = if self.crafter.name == "source" {
            "∞"
        } else if self.crafter.name == "sink" {
            "🗑"
        } else {
            "🖩"
        };

        format!(
            "{}{}{} {}{}",
            inputs, crafter_count, emoji, self.crafter.name, outputs
        )
    }
}
