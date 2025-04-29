use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context};
use config::Config;
use game_data::{Crafter, GameData};
use itertools::Itertools;
use machine::Machine;

mod config;
mod game_data;
mod machine;
pub mod prelude;

pub struct Planner {
    config: Config,
    game_data: GameData,
    all_items: BTreeSet<String>,
    reachable_items: BTreeSet<String>,
    crafters: BTreeMap<String, Crafter>,
    category_to_crafter: BTreeMap<String, Vec<String>>,
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
    })
}

impl Planner {
    pub fn create_machine(&self, recipe: &str) -> anyhow::Result<Machine> {
        self.create_machine_ext(recipe, None)
    }

    pub fn create_machine_with_crafter(
        &self,
        recipe: &str,
        crafter: &str,
    ) -> anyhow::Result<Machine> {
        self.create_machine_ext(recipe, Some(crafter))
    }

    fn create_machine_ext(&self, recipe: &str, crafter: Option<&str>) -> anyhow::Result<Machine> {
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
        println!("selected crafter: {crafter:?}");
        //recipe.category
        Ok(Machine {
            crafter,
            crafter_count: 1.0,
            recipe,
        })
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
}

/*


key_values = {}; for (k in v) { if (!Array.isArray(v[k].products)) { continue; };  for(val of v[k].products) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].push(val[kk]); } } }; console.log(key_values)

key_values = {}; for (k in v) { if (!Array.isArray(v[k].ingredients)) { continue; };  for(val of v[k].ingredients) { for (kk in val) { if (!key_values[kk]) { key_values[kk] = new Set(); } key_values[kk].add(val[kk]); } } }; console.log(key_values)

key_values = {}; for (kk in v) { let item = v[kk]; for (k in item) { if (k == "ingredients" || k == "products") { continue; };  if (!key_values[k]) { key_values[k] = new Set(); } key_values[k].add(item[k]); } }; for (k in key_values) console.log(k, [...key_values[k]])
*/
