use {
    crate::{
        game_data::{Ingredient, Product, Recipe},
        module_counts,
        primitives::{
            Amount, CrafterName, ItemName, ItemNameAndQuality, ModuleName, Quality, Speed,
            SINK_CRAFTER_NAME, SINK_RECIPE_CATEGORY, SOURCE_CRAFTER_NAME, SOURCE_RECIPE_CATEGORY,
        },
        rf,
    },
    itertools::Itertools,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crafter {
    pub name: CrafterName,
    pub quality: Quality,
    pub energy_usage: f64,
    pub crafting_speed: f64,
    #[serde(default)] // only for compatibility
    pub module_inventory_size: u64,
}

impl Crafter {
    pub fn is_source(&self) -> bool {
        self.name == *SOURCE_CRAFTER_NAME
    }
    pub fn is_sink(&self) -> bool {
        self.name == *SINK_CRAFTER_NAME
    }
    pub fn is_source_or_sink(&self) -> bool {
        self.name == *SOURCE_CRAFTER_NAME || self.name == *SINK_CRAFTER_NAME
    }

    pub fn is_recycler(&self) -> bool {
        &self.name.0 == "recycler"
    }

    pub fn with_quality(self, quality: Quality) -> Self {
        assert_eq!(self.quality, Quality(0));
        Self {
            name: self.name,
            quality,
            energy_usage: self.energy_usage,
            crafting_speed: self.crafting_speed * (1. + 0.3 * quality.0 as f64),
            module_inventory_size: self.module_inventory_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleType {
    Speed,
    Productivity,
    Quality,
}

impl ModuleType {
    pub fn module_items(self) -> [ItemName; 3] {
        match self {
            ModuleType::Speed => [
                "speed-module".into(),
                "speed-module-2".into(),
                "speed-module-3".into(),
            ],
            ModuleType::Productivity => [
                "productivity-module".into(),
                "productivity-module-2".into(),
                "productivity-module-3".into(),
            ],
            ModuleType::Quality => [
                "quality-module".into(),
                "quality-module-2".into(),
                "quality-module-3".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: ModuleName,
    pub type_: ModuleType,
    pub quality: Quality,
    pub energy_delta_percent: f64,
    pub speed_delta_percent: f64,
    pub productivity_delta_percent: f64,
    pub quality_delta_percent: f64,
}

fn round_to_nearest(value: f64, precision: f64) -> f64 {
    (value / precision).round() * precision
}

impl Module {
    pub fn with_quality(&self, quality: Quality) -> Self {
        assert_eq!(self.quality, Quality(0));
        Self {
            name: self.name.clone(),
            type_: self.type_,
            quality,
            energy_delta_percent: if self.energy_delta_percent < 0. {
                round_to_nearest(
                    self.energy_delta_percent * (1. + 0.3 * quality.as_f64()),
                    1.,
                )
            } else {
                self.energy_delta_percent
            },
            speed_delta_percent: if self.speed_delta_percent > 0. {
                round_to_nearest(self.speed_delta_percent * (1. + 0.3 * quality.as_f64()), 1.)
            } else {
                self.speed_delta_percent
            },
            productivity_delta_percent: if self.productivity_delta_percent > 0. {
                round_to_nearest(
                    self.productivity_delta_percent * (1. + 0.3 * quality.as_f64()),
                    1.,
                )
            } else {
                self.productivity_delta_percent
            },
            quality_delta_percent: if self.quality_delta_percent > 0. {
                round_to_nearest(
                    self.quality_delta_percent * (1. + 0.3 * quality.as_f64()),
                    0.1,
                )
            } else {
                self.quality_delta_percent
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Beacon {
    pub quality: Quality,
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub crafter: Crafter,
    pub crafter_count: f64,
    pub modules: Vec<Module>,
    pub beacons: Vec<Beacon>,
    pub recipe: Recipe,
    pub recipe_quality: Quality,
}

#[derive(Debug, Clone)]
pub struct ItemSpeed {
    pub item: ItemName,
    pub quality: Quality,
    pub speed: Speed,
}

impl ItemSpeed {
    pub fn name_and_quality(&self) -> ItemNameAndQuality {
        ItemNameAndQuality {
            name: self.item.clone(),
            quality: self.quality,
        }
    }
}

impl Machine {
    pub fn new_source(item: &ItemNameAndQuality) -> Self {
        Machine {
            crafter: Crafter {
                name: SOURCE_CRAFTER_NAME.clone(),
                quality: Quality(0),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: (&*item.name.0).into(),
                enabled: true,
                category: SOURCE_RECIPE_CATEGORY.clone(),
                ingredients: Vec::new(),
                products: vec![Product {
                    amount: Amount::ONE,
                    name: item.name.clone(),
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
            recipe_quality: item.quality,
        }
    }

    pub fn new_sink(item: &ItemNameAndQuality) -> Self {
        Machine {
            crafter: Crafter {
                name: SINK_CRAFTER_NAME.clone(),
                quality: Quality(0),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: (&*item.name.0).into(),
                enabled: true,
                category: SINK_RECIPE_CATEGORY.clone(),
                ingredients: vec![Ingredient {
                    amount: Amount::ONE,
                    name: item.name.clone(),
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
            recipe_quality: item.quality,
            modules: Vec::new(),
            beacons: Vec::new(),
        }
    }

    // Not including productivity.
    pub fn crafts_per_second(&self) -> Speed {
        let module_speed_percents: f64 = self
            .modules
            .iter()
            .map(|module| module.speed_delta_percent)
            .sum();
        let beacon_speed_percents: f64 = self
            .beacons
            .iter()
            .map(|beacon| {
                let beacon_efficiency = 1.5 + 0.2 * beacon.quality.0 as f64;
                let transmission_strength = beacon_efficiency / (self.beacons.len() as f64).sqrt();
                let speed_delta_percent: f64 = beacon
                    .modules
                    .iter()
                    .map(|module| module.speed_delta_percent)
                    .sum();
                transmission_strength * speed_delta_percent
            })
            .sum();
        let speed_percents = 100. + module_speed_percents + beacon_speed_percents;

        ((speed_percents / 100.) * self.crafter.crafting_speed * self.crafter_count
            / self.recipe.energy)
            .into()
    }

    pub fn input_speeds(&self) -> impl Iterator<Item = ItemSpeed> + '_ {
        let crafts_per_second = self.crafts_per_second();
        self.recipe.ingredients.iter().map(move |ing| ItemSpeed {
            item: ing.name.clone(),
            quality: self.recipe_quality,
            speed: -crafts_per_second * ing.amount,
        })
    }

    pub fn output_speeds(&self) -> Vec<ItemSpeed> {
        let crafts_per_second = self.crafts_per_second();
        let module_prod_percents: f64 = self
            .modules
            .iter()
            .map(|module| module.productivity_delta_percent)
            .sum();
        let prod_percents = 100. + module_prod_percents + self.recipe.productivity_bonus;

        let output_speed = (prod_percents / 100.) * crafts_per_second;

        let yield_coef = if self.crafter.is_recycler() {
            0.25
        } else {
            1.0
        };
        let mut same_quality_products = self
            .recipe
            .products
            .iter()
            .map(move |product| ItemSpeed {
                item: product.name.clone(),
                quality: self.recipe_quality,
                speed: output_speed
                    * product.probability
                    * yield_coef
                    * (product.amount + Amount::from(product.extra_count_fraction)),
            })
            .collect_vec();

        let quality_percent: f64 = self
            .modules
            .iter()
            .map(|module| module.quality_delta_percent)
            .sum();
        if quality_percent <= 0. {
            return same_quality_products;
        }

        let Some(next_quality) = self.recipe_quality.next() else {
            return same_quality_products;
        };

        let mut next_quality_products = Vec::new();
        for item in &mut same_quality_products {
            next_quality_products.push(ItemSpeed {
                item: item.item.clone(),
                quality: next_quality,
                speed: item.speed * (quality_percent / 100.),
            });
            item.speed = item.speed * (1. - quality_percent / 100.);
        }

        let mut all_qualities = vec![same_quality_products, next_quality_products];
        let mut super_quality = next_quality;
        while let Some(next) = super_quality.next() {
            super_quality = next;
            let previous_quality_products = all_qualities.last_mut().unwrap();
            let mut super_quality_products = Vec::new();
            let super_quality_percent = 10.;
            for item in previous_quality_products {
                super_quality_products.push(ItemSpeed {
                    item: item.item.clone(),
                    quality: super_quality,
                    speed: item.speed * (super_quality_percent / 100.),
                });
                item.speed = item.speed * (1. - super_quality_percent / 100.);
            }
            all_qualities.push(super_quality_products);
        }

        all_qualities.into_iter().flatten().collect()
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
            .into_iter()
            .map(|ing| format!("{} {}", ing.speed, ing.item))
            .join(" + ");

        let inputs = if inputs.is_empty() {
            String::new()
        } else {
            format!("{inputs} ➡ ")
        };
        let outputs = if outputs.is_empty() {
            String::new()
        } else {
            format!(" ➡ {outputs}")
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

    pub fn beacon_text(&self) -> String {
        if self.beacons.is_empty() {
            return String::new();
        }
        if self.beacons.iter().all_equal() {
            let modules = module_counts(&self.beacons[0].modules)
                .into_iter()
                .map(|(name, count)| format!("{count} × {name}"))
                .join(",");
            format!("{} × beacon({})", self.beacons.len(), modules)
        } else {
            self.beacons
                .iter()
                .map(|beacon| {
                    let modules = module_counts(&beacon.modules)
                        .into_iter()
                        .map(|(name, count)| format!("{count} × {name}"))
                        .join(",");
                    format!("beacon({modules})")
                })
                .join("\n")
        }
    }
}
