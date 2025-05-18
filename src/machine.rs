use {
    crate::{
        game_data::{Ingredient, Product, Recipe},
        primitives::{Amount, CrafterName, ItemName, ModuleName, RecipeCategory, Speed},
        rf,
    },
    itertools::Itertools,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crafter {
    pub name: CrafterName,
    pub energy_usage: f64,
    pub crafting_speed: f64,
    #[serde(default)] // only for compatibility
    pub module_inventory_size: u64,
}

impl Crafter {
    pub fn is_source(&self) -> bool {
        self.name == *CrafterName::SOURCE
    }
    pub fn is_sink(&self) -> bool {
        self.name == *CrafterName::SINK
    }
    pub fn is_source_or_sink(&self) -> bool {
        self.name == *CrafterName::SOURCE || self.name == *CrafterName::SINK
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleType {
    Speed,
    Productivity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: ModuleName,
    pub type_: ModuleType,
    pub energy_delta_percent: f64,
    pub speed_delta_percent: f64,
    pub productivity_delta_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub crafter: Crafter,
    pub crafter_count: f64,
    pub modules: Vec<Module>,
    pub beacons: Vec<Vec<Module>>,
    pub recipe: Recipe,
}

#[derive(Debug, Clone)]
pub struct ItemSpeed {
    pub item: ItemName,
    pub speed: Speed,
}

impl Machine {
    pub fn new_source(item: &ItemName) -> Self {
        Machine {
            crafter: Crafter {
                name: CrafterName::SOURCE.clone(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: format!("{item}-source").into(),
                enabled: true,
                category: RecipeCategory::SOURCE.clone(),
                ingredients: Vec::new(),
                products: vec![Product {
                    amount: Amount::ONE,
                    name: item.clone(),
                    type_: String::new(),
                    extra_count_fraction: 0.0,
                    probability: 1.0,
                    temperature: None,
                    ignored_by_productivity: Amount::ZERO,
                }],
                hidden: false,
                hidden_from_flow_stats: false,
                energy: 1.0,
                order: String::new(),
                productivity_bonus: 0.0,
                allowed_effects: Default::default(),
            },
            modules: Vec::new(),
            beacons: Vec::new(),
        }
    }

    pub fn new_sink(item: &ItemName) -> Self {
        Machine {
            crafter: Crafter {
                name: CrafterName::SINK.clone(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: format!("{item}-sink").into(),
                enabled: true,
                category: RecipeCategory::SINK.clone(),
                ingredients: vec![Ingredient {
                    amount: Amount::ONE,
                    name: item.clone(),
                    type_: String::new(),
                }],
                products: Vec::new(),
                hidden: false,
                hidden_from_flow_stats: false,
                energy: 1.0,
                order: String::new(),
                productivity_bonus: 0.0,
                allowed_effects: Default::default(),
            },
            modules: Vec::new(),
            beacons: Vec::new(),
        }
    }

    pub fn beacon_transmission_strength(&self) -> f64 {
        if self.beacons.is_empty() {
            0.0
        } else {
            1.5 / (self.beacons.len() as f64).sqrt()
        }
    }

    // Not including productivity.
    pub fn crafts_per_second(&self) -> Speed {
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

        ((speed_percents / 100.) * self.crafter.crafting_speed * self.crafter_count
            / self.recipe.energy)
            .into()
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
        let prod_percents = 100.
            + module_prod_percents
            + beacon_transmission_strength * beacon_prod_percents
            + self.recipe.productivity_bonus;

        let output_speed = (prod_percents / 100.) * crafts_per_second;

        self.recipe.products.iter().map(move |product| ItemSpeed {
            item: product.name.clone(),
            speed: output_speed * (product.amount - product.ignored_by_productivity),
        })
    }

    pub fn item_speeds(&self) -> impl Iterator<Item = ItemSpeed> + '_ {
        self.input_speeds().chain(self.output_speeds())
    }

    pub fn description(&self) -> String {
        let inputs = self
            .input_speeds()
            .map(|ing| format!("{} {}", ing.speed, ing.item))
            .join(" + ");

        let outputs = self
            .output_speeds()
            .map(|ing| format!("{} {}", ing.speed, ing.item))
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
        let crafter_count = if self.crafter.is_source_or_sink() {
            String::new()
        } else {
            format!("{} × ", rf(self.crafter_count))
        };

        format!(
            "{}{} {}{}",
            inputs, crafter_count, self.crafter.name, outputs
        )
    }
}
