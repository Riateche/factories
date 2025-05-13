use {
    anyhow::{bail, format_err, Context},
    config::Config,
    game_data::{Crafter, GameData, Ingredient, Product, Recipe},
    itertools::Itertools,
    machine::Machine,
    nalgebra::{DMatrix, DVector},
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        env,
        path::Path,
    },
};

pub mod config;
pub mod game_data;
pub mod machine;
pub mod prelude;
pub mod ui;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<Machine>,
    pub item_speed_constraints: BTreeMap<String, f64>,
    pub solved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub energy_delta_percent: f64,
    pub speed_delta_percent: f64,
    pub productivity_delta_percent: f64,
}

pub struct Planner {
    pub config: Config,
    pub game_data: GameData,
    pub modules: BTreeMap<String, Module>,
    pub all_items: BTreeSet<String>,
    pub reachable_items: BTreeSet<String>,
    pub crafters: BTreeMap<String, Crafter>,
    pub category_to_crafter: BTreeMap<String, Vec<String>>,
    pub snippet: Snippet,
}

pub fn init() -> anyhow::Result<Planner> {
    if !env::current_dir().unwrap().join("game_data.json").exists() {
        if Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("game_data.json")
            .exists()
        {
            match env::set_current_dir(env!("CARGO_MANIFEST_DIR")) {
                Ok(()) => println!("changed current dir to {}", env!("CARGO_MANIFEST_DIR")),
                Err(err) => bail!(
                    "failed to change current dir to {}: {}",
                    env!("CARGO_MANIFEST_DIR"),
                    err
                ),
            }
        } else {
            bail!("game_data.json not found in working directory");
        }
    }

    let config: Config = toml::from_str(&fs_err::read_to_string("config.toml")?)?;
    let mut game_data: GameData = serde_json::from_str(&fs_err::read_to_string("game_data.json")?)?;

    let blacklist = [
        "turbo-loader",
        "express-loader",
        "fast-loader",
        "recipe-unknown",
    ];
    game_data.recipes.retain(|_, recipe| {
        recipe.category != "recycling"
            && recipe.category != "recycling-or-hand-crafting"
            && recipe.category != "parameters"
            && !blacklist.contains(&recipe.name.as_str())
    });
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
                    module_inventory_size: entity.module_inventory_size,
                },
            );
        }
    }

    let modules = [
        Module {
            name: "speed-module".into(),
            energy_delta_percent: 50.,
            speed_delta_percent: 20.,
            productivity_delta_percent: 0.,
        },
        Module {
            name: "speed-module-2".into(),
            energy_delta_percent: 60.,
            speed_delta_percent: 30.,
            productivity_delta_percent: 0.,
        },
        Module {
            name: "speed-module-3".into(),
            energy_delta_percent: 70.,
            speed_delta_percent: 50.,
            productivity_delta_percent: 0.,
        },
        Module {
            name: "productivity-module".into(),
            energy_delta_percent: 40.,
            speed_delta_percent: -5.,
            productivity_delta_percent: 4.0,
        },
        Module {
            name: "productivity-module-2".into(),
            energy_delta_percent: 60.,
            speed_delta_percent: -10.,
            productivity_delta_percent: 6.0,
        },
        Module {
            name: "productivity-module-3".into(),
            energy_delta_percent: 80.,
            speed_delta_percent: -15.,
            productivity_delta_percent: 10.0,
        },
    ]
    .into_iter()
    .map(|m| (m.name.clone(), m))
    .collect();

    Ok(Planner {
        config,
        game_data,
        all_items,
        modules,
        reachable_items,
        crafters,
        category_to_crafter,
        snippet: Snippet {
            machines: Vec::new(),
            item_speed_constraints: BTreeMap::new(),
            solved: true,
        },
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
        self.snippet.machines.push(Machine {
            crafter: Crafter {
                name: "source".into(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
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
                    ignored_by_productivity: 0.,
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
            count_constraint: None,
        });
        Ok(())
    }

    pub fn create_sink(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.snippet.machines.push(Machine {
            crafter: Crafter {
                name: "sink".into(),
                energy_usage: 0.0,
                crafting_speed: 1.0,
                module_inventory_size: 0,
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
                allowed_effects: Default::default(),
            },
            modules: Vec::new(),
            beacons: Vec::new(),
            count_constraint: None,
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

    pub fn auto_select_crafter(&self, crafters: &[String]) -> Option<String> {
        if crafters.len() == 1 {
            Some(crafters[0].clone())
        } else if crafters.iter().any(|c| c == &self.config.assembler_type) {
            Some(self.config.assembler_type.clone())
        } else if crafters.iter().any(|c| c == &self.config.furnace_type) {
            Some(self.config.furnace_type.clone())
        } else {
            None
        }
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
        let crafter = if let Some(crafter) = crafter {
            if !crafters.iter().any(|c| c == crafter) {
                bail!("requested crafter {crafter:?}, but available crafters for {recipe:?} are: {crafters:?}");
            }
            crafter.to_string()
        } else if let Some(crafter) = self.auto_select_crafter(crafters) {
            crafter
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
        self.snippet.machines.push(Machine {
            crafter,
            crafter_count: 1.0,
            modules: Vec::new(),
            beacons: Vec::new(),
            recipe: recipe.clone(),
            count_constraint: None,
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
            .snippet
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

        for machine in &self.snippet.machines {
            if machine.crafter.name != "source" && machine.crafter.name != "sink" {
                println!("{}", machine.io_text());
            }
        }

        let outputs = self
            .snippet
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

    pub fn add_item_speed_constraint(&mut self, item: &str, speed: f64) -> anyhow::Result<()> {
        if !self.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.snippet
            .item_speed_constraints
            .insert(item.into(), speed);
        Ok(())
    }

    pub fn added_items(&self) -> BTreeSet<String> {
        self.snippet
            .machines
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

        self.snippet.solved = false;
        if self.snippet.machines.is_empty() {
            self.snippet.solved = true;
            return Ok(());
        }
        for machine in &mut self.snippet.machines {
            machine.crafter_count = 1.0;
        }
        let items = self.added_items();
        let constraints: Vec<_> = items
            .iter()
            .map(|item| Constraint::ItemSumsToZero {
                item: item.to_string(),
            })
            .chain(
                self.snippet
                    .item_speed_constraints
                    .iter()
                    .map(|(item, speed)| Constraint::ItemProduction {
                        item: item.into(),
                        speed: *speed,
                    }),
            )
            .chain(
                self.snippet
                    .machines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, machine)| {
                        machine
                            .count_constraint
                            .map(|count| Constraint::MachineCount { index, count })
                    }),
            )
            .collect();

        let a = DMatrix::from_fn(
            constraints.len(),
            self.snippet.machines.len(),
            |row, col| {
                let machine = &self.snippet.machines[col];
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
                    Constraint::MachineCount {
                        index: machine_index,
                        count: _,
                    } => {
                        if *machine_index == col {
                            1.0
                        } else {
                            0.0
                        }
                    }
                }
            },
        );
        let b = DVector::from_fn(constraints.len(), |row, _| match &constraints[row] {
            Constraint::ItemSumsToZero { item: _ } => 0.0,
            Constraint::ItemProduction { item: _, speed } => *speed,
            Constraint::MachineCount { index: _, count } => *count,
        });
        if false {
            println!("constraints: {constraints:?}");
            println!("a=");
            for row in a.row_iter() {
                println!("{:?}", row.iter().collect_vec());
            }
            println!("b={b:?}");
        }

        let svd = a.clone().svd(true, true);
        let output = svd
            .solve(&b, 0.000001)
            .map_err(|str| format_err!("{str}"))?;
        if false {
            println!("output {output:?}");
        }

        if output.iter().all(|v| *v == 0.0) {
            bail!("solve result is zero, probably missing machines or constraints");
        }

        for (machine, output_item) in self.snippet.machines.iter_mut().zip_eq(output.iter()) {
            machine.crafter_count = *output_item;
        }

        let error = (a * output.clone() - b).norm();
        if error > 0.01 {
            bail!("couldn't fit all constraints (error = {}); try removing constraints or changing their values", rf(error));
        }
        if output.iter().any(|x| *x < 0.0) {
            bail!("solution is negative! try adding more constraints");
        }

        self.snippet.solved = true;

        Ok(())
    }

    pub fn add_sources_and_sinks(&mut self) {
        self.snippet
            .machines
            .retain(|m| m.crafter.name != "source" && m.crafter.name != "sink");
        let items = self.added_items();
        for item in items {
            let any_inputs = self
                .snippet
                .machines
                .iter()
                .any(|m| m.recipe.ingredients.iter().any(|i| i.name == item));
            let any_outputs = self
                .snippet
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

    pub fn auto_sort_machines(&mut self) {
        let mut new_machines = Vec::new();
        let mut remaining_machines = self.snippet.machines.clone();
        let mut crafted_items = BTreeSet::new();
        loop {
            let mut new_remaining_machines = Vec::new();
            let old_count = new_machines.len();
            for machine in remaining_machines {
                if machine
                    .recipe
                    .ingredients
                    .iter()
                    .all(|ing| crafted_items.contains(&ing.name))
                {
                    for product in &machine.recipe.products {
                        crafted_items.insert(product.name.to_string());
                    }
                    new_machines.push(machine);
                } else {
                    new_remaining_machines.push(machine);
                }
            }
            remaining_machines = new_remaining_machines;
            if new_machines.len() == old_count {
                break;
            }
        }
        if !remaining_machines.is_empty() {
            println!("WARN: remaining_machines is not empty: {remaining_machines:?}");
            new_machines.extend(remaining_machines);
        }
        self.snippet.machines = new_machines;
    }

    pub fn auto_refresh(&mut self) -> anyhow::Result<()> {
        self.add_sources_and_sinks();
        self.auto_sort_machines();
        self.solve()
    }
}

#[derive(Debug, Clone)]
pub enum Constraint {
    ItemSumsToZero { item: String },
    ItemProduction { item: String, speed: f64 },
    MachineCount { index: usize, count: f64 },
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
