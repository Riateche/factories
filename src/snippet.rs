use {
    crate::{
        game_data::{Ingredient, Product, Recipe},
        info::Info,
        machine::{Crafter, Machine},
        rf,
    },
    anyhow::{bail, format_err, Context},
    itertools::Itertools,
    nalgebra::{DMatrix, DVector},
    serde::{Deserialize, Serialize},
    std::collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<Machine>,
    pub item_speed_constraints: BTreeMap<String, f64>,
    pub solved: bool,
}

impl Default for Snippet {
    fn default() -> Self {
        Self {
            machines: Default::default(),
            item_speed_constraints: Default::default(),
            solved: true,
        }
    }
}

pub struct SnippetEditor {
    pub info: Info,
    pub snippet: Snippet,
}

#[derive(Debug, Clone)]
pub enum Constraint {
    ItemSumsToZero { item: String },
    ItemProduction { item: String, speed: f64 },
    MachineCount { index: usize, count: f64 },
}

impl SnippetEditor {
    pub fn init() -> anyhow::Result<Self> {
        Ok(SnippetEditor {
            info: Info::load()?,
            snippet: Snippet::default(),
        })
    }

    pub fn create_source(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
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
        if !self.info.all_items.contains(item) {
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

    pub fn auto_select_crafter(&self, crafters: &[String]) -> Option<String> {
        if crafters.len() == 1 {
            Some(crafters[0].clone())
        } else if crafters
            .iter()
            .any(|c| c == &self.info.config.assembler_type)
        {
            Some(self.info.config.assembler_type.clone())
        } else if crafters.iter().any(|c| c == &self.info.config.furnace_type) {
            Some(self.info.config.furnace_type.clone())
        } else {
            None
        }
    }

    pub fn create_machine(&mut self, recipe: &str, crafter: Option<&str>) -> anyhow::Result<()> {
        let recipe = self
            .info
            .game_data
            .recipes
            .get(recipe)
            .with_context(|| format!("recipe not found: {recipe:?}"))?
            .clone();
        let crafters = self
            .info
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
            .info
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
        if !self.info.all_items.contains(item) {
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
