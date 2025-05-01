use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, format_err, Context};
use config::Config;
use game_data::{Crafter, GameData, Ingredient, Product, Recipe};
use itertools::Itertools;
use machine::Machine;
use nalgebra::{DMatrix, DVector};

mod config;
mod game_data;
mod machine;
pub mod prelude;

pub struct Planner {
    config: Config,
    pub game_data: GameData,
    pub all_items: BTreeSet<String>,
    reachable_items: BTreeSet<String>,
    crafters: BTreeMap<String, Crafter>,
    category_to_crafter: BTreeMap<String, Vec<String>>,
    pub machines: Vec<Machine>,
    pub constraints: Vec<Constraint>,
}

pub fn init() -> anyhow::Result<Planner> {
    let config: Config = toml::from_str(&fs_err::read_to_string("config.toml")?)?;
    let game_data: GameData = serde_json::from_str(&fs_err::read_to_string("game_data.json")?)?;
    // println!("{game_data:#?}");

    // let mut set = BTreeSet::new();
    // for entity in game_data.entities.values() {
    //     set.insert(&entity.type_);
    // }
    // println!("{set:?}");

    let mut all_items = BTreeSet::new();
    for recipe in game_data.recipes.values() {
        for item in &recipe.ingredients {
            all_items.insert(item.name.clone());
        }
        for item in &recipe.products {
            all_items.insert(item.name.clone());
        }
    }

    let mut all_reachable_items: BTreeSet<String> = game_data
        .entities
        .values()
        .filter(|v| v.resource_category.is_some() || v.type_ == "plant" || v.type_ == "tree")
        .flat_map(|v| &v.mineable_properties.as_ref().unwrap().products)
        .map(|v| v.name.clone())
        .collect();
    for s in ["water", "lava", "heavy-oil", "ammoniacal-solution"] {
        all_reachable_items.insert(s.into());
    }
    if false {
        println!("all_reachable_items #0: {all_reachable_items:?}\n");
    }

    let mut reachable_items: BTreeSet<String> = [
        "coal",
        "copper-ore",
        "crude-oil",
        "iron-ore",
        "stone",
        "water",
        "wood",
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let mut verified_recipes = BTreeSet::new();

    let mut i = 0;
    loop {
        i += 1;
        let mut new_reachable_items = BTreeSet::new();

        for recipe in game_data.recipes.values() {
            if recipe
                .ingredients
                .iter()
                .all(|ing| reachable_items.contains(&ing.name))
                && recipe.category != "recycling"
                && recipe.category != "recycling-or-hand-crafting"
                && recipe.category != "captive-spawner-process"
            {
                for product in &recipe.products {
                    if !reachable_items.contains(&product.name) {
                        if false {
                            println!(
                                "{} | {} -> {}",
                                recipe.name,
                                recipe.ingredients.iter().map(|ing| &ing.name).join(", "),
                                product.name
                            );
                        }
                        new_reachable_items.insert(product.name.clone());
                    }
                }
                if !verified_recipes.contains(&recipe.name) {
                    if let Some(bad_product) = recipe
                        .products
                        .iter()
                        .find(|product| reachable_items.contains(&product.name))
                    {
                        if false {
                            println!("loop detected for {}: {recipe:?}\n\n", bad_product.name);
                        }
                        // println!("verified_recipes {verified_recipes:?}");
                        // println!("reachable_items {reachable_items:?}\n\n");
                        // if recipe.name == "copper-plate" {
                        //     std::process::exit(1);
                        // }
                    }
                    verified_recipes.insert(recipe.name.to_string());
                }
            }
        }
        if new_reachable_items.is_empty() {
            break;
        }
        if false {
            println!("#{i}: {new_reachable_items:?}\n");
        }
        reachable_items.extend(new_reachable_items);
    }
    if false {
        println!(
            "unreachable items: {:?}",
            all_items.difference(&reachable_items).collect_vec()
        );
    }

    let mut crafters = BTreeMap::new();
    let mut category_to_crafter = BTreeMap::<_, Vec<_>>::new();
    for entity in game_data.entities.values() {
        if entity.name == "character" {
            continue;
        }
        if let Some(categories) = &entity.crafting_categories {
            for category in categories.keys() {
                category_to_crafter
                    .entry(category.clone())
                    .or_default()
                    .push(entity.name.clone());
            }
            crafters.insert(
                entity.name.clone(),
                Crafter {
                    name: entity.name.clone(),
                    energy_usage: entity
                        .energy_usage
                        .with_context(|| format!("missing energy_usage for crafter: {entity:?}"))?,
                    crafting_speed: entity.crafting_speed.with_context(|| {
                        format!("missing crafting_speed for crafter: {entity:?}")
                    })?,
                },
            );
        }
    }

    Ok(Planner {
        config,
        game_data,
        all_items,
        reachable_items,
        crafters,
        category_to_crafter,
        machines: Vec::new(),
        constraints: Vec::new(),
    })
}

impl Planner {
    pub fn create_machine(&mut self, recipe: &str) -> anyhow::Result<()> {
        self.create_machine_ext(recipe, None)
    }

    pub fn create_source(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.machines.push(Machine {
            crafter: Crafter {
                name: "source".into(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: format!("{item}-source"),
                enabled: true,
                category: "source".into(),
                ingredients: Vec::new(),
                products: vec![Product {
                    amount: 1.0,
                    name: item.into(),
                    type_: String::new(),
                    extra_count_fraction: 0.0,
                    probability: 1.0,
                    temperature: None,
                }],
                hidden: false,
                hidden_from_flow_stats: false,
                energy: 1.0,
                order: String::new(),
                productivity_bonus: 0.0,
            },
        });
        Ok(())
    }

    pub fn create_sink(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.machines.push(Machine {
            crafter: Crafter {
                name: "sink".into(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
            },
            crafter_count: 1.0,
            recipe: Recipe {
                name: format!("{item}-sink"),
                enabled: true,
                category: "sink".into(),
                ingredients: vec![Ingredient {
                    amount: 1.0,
                    name: item.into(),
                    type_: String::new(),
                }],
                products: Vec::new(),
                hidden: false,
                hidden_from_flow_stats: false,
                energy: 1.0,
                order: String::new(),
                productivity_bonus: 0.0,
            },
        });
        Ok(())
    }

    pub fn create_machine_with_crafter(
        &mut self,
        recipe: &str,
        crafter: &str,
    ) -> anyhow::Result<()> {
        self.create_machine_ext(recipe, Some(crafter))
    }

    fn create_machine_ext(&mut self, recipe: &str, crafter: Option<&str>) -> anyhow::Result<()> {
        let recipe = self
            .game_data
            .recipes
            .get(recipe)
            .with_context(|| format!("recipe not found: {recipe:?}"))?
            .clone();
        let crafters = self
            .category_to_crafter
            .get(&recipe.category)
            .context("unknown recipe category")?;
        assert!(!crafters.is_empty());
        let crafter = if crafters.len() == 1 {
            crafters[0].clone()
        } else if let Some(crafter) = crafter {
            if !crafters.iter().any(|c| c == crafter) {
                bail!("requested crafter {crafter:?}, but available crafters for {recipe:?} are: {crafters:?}");
            }
            crafter.to_string()
        } else if crafters.iter().any(|c| c == &self.config.assembler_type) {
            self.config.assembler_type.clone()
        } else if crafters.iter().any(|c| c == &self.config.furnace_type) {
            self.config.furnace_type.clone()
        } else {
            bail!("ambiguous crafter for {recipe:?}: {crafters:?}");
        };
        let crafter = self
            .crafters
            .get(&crafter)
            .with_context(|| format!("crafter not found: {crafter:?}"))?
            .clone();
        if false {
            println!("selected crafter: {crafter:?}");
        }
        self.machines.push(Machine {
            crafter,
            crafter_count: 1.0,
            recipe,
        });
        Ok(())
    }

    pub fn list_ambigous_sources(&self) {
        for item in &self.all_items {
            let recipes = self
                .game_data
                .recipes
                .values()
                .filter(|r| {
                    r.category != "recycling"
                        && r.category != "recycling-or-hand-crafting"
                        && r.products.iter().any(|p| &p.name == item)
                        && r.ingredients
                            .iter()
                            .all(|ing| self.reachable_items.contains(&ing.name))
                })
                .collect_vec();
            if recipes.len() > 1 {
                println!("{item}");
                for recipe in recipes {
                    println!(
                        "- [{}] {}",
                        recipe.name,
                        recipe.ingredients.iter().map(|ing| &ing.name).join(" + ")
                    );
                }
                println!();
            }
        }
    }

    pub fn show_category_to_crafter(&self) {
        for (category, crafters) in &self.category_to_crafter {
            println!("{}: {}     {:?}", category, crafters.len(), crafters);
        }
    }

    pub fn show_machines(&self) {
        println!();
        let inputs = self
            .machines
            .iter()
            .filter(|m| m.crafter.name == "source")
            .flat_map(|m| m.item_speeds())
            .collect_vec();
        println!(
            "Inputs: {}",
            inputs
                .iter()
                .map(|i| { format!("{}/s {}", rf(i.speed), i.item) })
                .join(" + ")
        );

        for machine in &self.machines {
            if machine.crafter.name != "source" && machine.crafter.name != "sink" {
                println!("{}", machine.io_text());
            }
        }

        let outputs = self
            .machines
            .iter()
            .filter(|m| m.crafter.name == "sink")
            .flat_map(|m| m.item_speeds())
            .collect_vec();
        println!(
            "Outputs: {}",
            outputs
                .iter()
                .map(|i| { format!("{}/s {}", rf(-i.speed), i.item) })
                .join(" + ")
        );
        println!();
    }

    pub fn add_constraint(&mut self, item: &str, speed: f64) -> anyhow::Result<()> {
        if !self.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.constraints.push(Constraint::ItemProduction {
            item: item.into(),
            speed,
        });
        Ok(())
    }

    pub fn added_items(&self) -> BTreeSet<String> {
        self.machines
            .iter()
            .flat_map(|m| {
                m.recipe
                    .ingredients
                    .iter()
                    .map(|i| i.name.to_string())
                    .chain(m.recipe.products.iter().map(|i| i.name.to_string()))
            })
            .collect()
    }

    pub fn solve(&mut self) -> anyhow::Result<()> {
        /*
            Ax = b
           vector row = matrix row = index of equation = index of constraint
           matrix column = index of variable = index of machine
        */

        for machine in &mut self.machines {
            machine.crafter_count = 1.0;
        }
        let items = self.added_items();
        let constraints: Vec<_> = items
            .iter()
            .map(|item| Constraint::ItemSumsToZero {
                item: item.to_string(),
            })
            .chain(self.constraints.iter().cloned())
            .collect();

        let a = DMatrix::from_fn(constraints.len(), self.machines.len(), |row, col| {
            let machine = &self.machines[col];
            match &constraints[row] {
                Constraint::ItemSumsToZero { item } => machine
                    .item_speeds()
                    .into_iter()
                    .filter(|i| &i.item == item)
                    .map(|i| i.speed)
                    .sum::<f64>(),
                Constraint::ItemProduction { item, speed: _ } => machine
                    .item_speeds()
                    .into_iter()
                    .filter(|i| &i.item == item && i.speed > 0.0)
                    .map(|i| i.speed)
                    .sum::<f64>(),
            }
        });
        let b = DVector::from_fn(constraints.len(), |row, _| match &constraints[row] {
            Constraint::ItemSumsToZero { item: _ } => 0.0,
            Constraint::ItemProduction { item: _, speed } => *speed,
        });
        if false {
            println!("constraints: {constraints:?}");
            println!("a=");
            for row in a.row_iter() {
                println!("{:?}", row.iter().collect_vec());
            }
            println!("b={b:?}");
        }

        let svd = a.svd(true, true);
        let output = svd
            .solve(&b, 0.000001)
            .map_err(|str| format_err!("{str}"))?;
        if false {
            println!("output {output:?}");
        }

        if output.iter().all(|v| *v == 0.0) {
            bail!("solve result is zero, probably missing machines or constraints");
        }

        for (machine, output_item) in self.machines.iter_mut().zip_eq(output.iter()) {
            machine.crafter_count = *output_item;
        }

        Ok(())
    }

    pub fn add_sources_and_sinks(&mut self) {
        self.machines
            .retain(|m| m.crafter.name != "source" && m.crafter.name != "sink");
        let items = self.added_items();
        for item in items {
            let any_inputs = self
                .machines
                .iter()
                .any(|m| m.recipe.ingredients.iter().any(|i| i.name == item));
            let any_outputs = self
                .machines
                .iter()
                .any(|m| m.recipe.products.iter().any(|i| i.name == item));
            if any_inputs && !any_outputs {
                self.create_source(&item).unwrap();
            } else if !any_inputs && any_outputs {
                self.create_sink(&item).unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Constraint {
    ItemSumsToZero { item: String },
    ItemProduction { item: String, speed: f64 },
}

/*


key_values = {}; for (k in v) { if (!Array.isArray(v[k].products)) { continue; };  for(val of v[k].products) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].push(val[kk]); } } }; console.log(key_values)

key_values = {}; for (k in v) { if (!Array.isArray(v[k].ingredients)) { continue; };  for(val of v[k].ingredients) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].add(val[kk]); } } }; console.log(key_values)

key_values = {}; for (kk in v) { let item = v[kk]; for (k in item) { if (k == "ingredients" || k == "products") { continue; };  if (!key_values[k]) { key_values[k] = new Set(); } key_values[k].add(item[k]); } }; for (k in key_values) console.log(k, [...key_values[k]])
*/

// round float
pub fn rf(f: f64) -> f64 {
    (f * 100.0).round() / 100.0
}
