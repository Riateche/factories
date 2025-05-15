use {
    crate::{
        game_data::{Ingredient, Product, Recipe},
        info::Info,
        machine::{Crafter, Machine, Module, ModuleType},
        rf,
    },
    anyhow::{bail, ensure, format_err, Context},
    fallible_iterator::{FallibleIterator, IteratorExt},
    itertools::Itertools,
    nalgebra::{DMatrix, DVector},
    serde::{Deserialize, Serialize},
    std::{
        collections::{BTreeMap, BTreeSet},
        path::Path,
    },
    tracing::{trace, warn},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SnippetMachine {
    Source {
        item: String,
    },
    Sink {
        item: String,
    },
    Crafter {
        crafter: String,
        modules: Vec<String>,
        beacons: Vec<Vec<String>>,
        recipe: String,
        count_constraint: Option<f64>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub machines: Vec<SnippetMachine>,
    pub item_speed_constraints: BTreeMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct EditorMachine {
    pub snippet: SnippetMachine,
    pub machine: Machine,
}

#[derive(Debug)]
pub struct SnippetEditor {
    info: Info,
    machines: Vec<EditorMachine>,
    item_speed_constraints: BTreeMap<String, f64>,
    solved: bool,
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
            machines: Vec::new(),
            item_speed_constraints: Default::default(),
            solved: true,
        })
    }

    fn create_machine(&self, snippet: &SnippetMachine) -> anyhow::Result<Machine> {
        let output = match snippet {
            SnippetMachine::Source { item } => Machine {
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
            },
            SnippetMachine::Sink { item } => Machine {
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
            },
            SnippetMachine::Crafter {
                crafter,
                modules,
                beacons,
                recipe,
                count_constraint: _,
            } => {
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
                ensure!(!crafters.is_empty());
                if !crafters.iter().any(|c| c == crafter) {
                    bail!("requested crafter {crafter:?}, but available crafters for {recipe:?} are: {crafters:?}");
                }
                let crafter = self
                    .info
                    .crafters
                    .get(crafter)
                    .with_context(|| format!("crafter not found: {crafter:?}"))?
                    .clone();

                Machine {
                    crafter,
                    crafter_count: 1.0,
                    modules: modules
                        .iter()
                        .map(|m| self.info.module(m))
                        .transpose_into_fallible()
                        .cloned()
                        .collect()?,
                    beacons: beacons
                        .iter()
                        .map(|beacon| {
                            beacon
                                .iter()
                                .map(|m| self.info.module(m))
                                .transpose_into_fallible()
                                .cloned()
                                .collect()
                        })
                        .transpose_into_fallible()
                        .collect()?,
                    recipe,
                }
            }
        };
        Ok(output)
    }

    pub fn load_snippet(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let snippet = serde_json::from_str::<Snippet>(&fs_err::read_to_string(path)?)?;
        let mut machines = Vec::new();
        for machine in snippet.machines {
            machines.push(self.create_machine(&machine)?);
        }
        self.after_machines_changed();
        Ok(())
    }

    pub fn save_snippet(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        fs_err::write(path, serde_json::to_string_pretty(&self.snippet())?)?;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.machines.clear();
        self.item_speed_constraints.clear();
        self.solved = true;
    }

    fn add_source(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = SnippetMachine::Source { item: item.into() };
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });
        Ok(())
    }

    fn add_sink(&mut self, item: &str) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = SnippetMachine::Sink { item: item.into() };
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });
        Ok(())
    }

    pub fn add_crafter(&mut self, recipe_name: &str, crafter: Option<&str>) -> anyhow::Result<()> {
        let recipe = self
            .info
            .game_data
            .recipes
            .get(recipe_name)
            .with_context(|| format!("recipe not found: {recipe_name:?}"))?
            .clone();
        let crafters = self
            .info
            .category_to_crafter
            .get(&recipe.category)
            .context("unknown recipe category")?;
        ensure!(!crafters.is_empty());
        let crafter = if let Some(crafter) = crafter {
            crafter.to_string()
        } else if let Some(crafter) = self.info.auto_select_crafter(crafters) {
            crafter
        } else {
            bail!("ambiguous crafter for {recipe:?}: {crafters:?}");
        };

        trace!("selected crafter: {crafter:?}");
        self.solved = false;
        let add_auto_constraint =
            self.machines.is_empty() && self.item_speed_constraints.is_empty();

        let snippet = SnippetMachine::Crafter {
            crafter,
            modules: vec![],
            beacons: vec![],
            recipe: recipe_name.into(),
            count_constraint: None,
        };
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });

        if add_auto_constraint {
            if let Some(product) = recipe.products.get(0) {
                self.item_speed_constraints
                    .insert(product.name.clone(), 1.0);
            }
        }
        self.after_machines_changed();
        Ok(())
    }

    pub fn remove_machine(&mut self, index: usize) -> anyhow::Result<()> {
        ensure!(index < self.machines.len(), "invalid machine index");
        self.machines.remove(index);
        self.after_machines_changed();
        Ok(())
    }

    pub fn show_machines(&self) {
        println!();
        let inputs = self
            .machines
            .iter()
            .filter(|m| m.machine.crafter.name == "source")
            .flat_map(|m| m.machine.item_speeds())
            .collect_vec();
        println!(
            "Inputs: {}",
            inputs
                .iter()
                .map(|i| { format!("{}/s {}", rf(i.speed), i.item) })
                .join(" + ")
        );

        for machine in &self.machines {
            if machine.machine.crafter.name != "source" && machine.machine.crafter.name != "sink" {
                println!("{}", machine.machine.io_text());
            }
        }

        let outputs = self
            .machines
            .iter()
            .filter(|m| m.machine.crafter.name == "sink")
            .flat_map(|m| m.machine.item_speeds())
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

    pub fn set_item_speed_constraint(
        &mut self,
        item: &str,
        speed: Option<f64>,
    ) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        if let Some(speed) = speed {
            self.item_speed_constraints.insert(item.into(), speed);
        } else {
            self.item_speed_constraints.remove(item);
        }
        self.solve();
        Ok(())
    }

    pub fn set_machine_count_constraint(
        &mut self,
        index: usize,
        count: Option<f64>,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(index)
            .context("invalid machine index")?;
        match &mut machine.snippet {
            SnippetMachine::Source { .. } | SnippetMachine::Sink { .. } => bail!(
                "machine count constraint is not allowed for sources \
                and sinks, use item speed constraint instead"
            ),
            SnippetMachine::Crafter {
                count_constraint, ..
            } => {
                *count_constraint = count;
            }
        }
        self.solve();
        Ok(())
    }

    pub fn add_module(&mut self, machine_index: usize, module: &str) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(machine_index)
            .context("invalid machine index")?;
        let module = self.info.module(module)?;
        match module.type_ {
            ModuleType::Speed => {}
            ModuleType::Productivity => {
                if !machine.machine.recipe.allowed_effects.productivity {
                    bail!("machin recipe doesn't allow productivity");
                }
            }
        }

        if machine.machine.crafter.module_inventory_size <= machine.machine.modules.len() as u64 {
            bail!("no more space for modules");
        }
        match &mut machine.snippet {
            SnippetMachine::Source { .. } | SnippetMachine::Sink { .. } => {
                bail!("modules are not supported for source and sink")
            }
            SnippetMachine::Crafter { modules, .. } => {
                modules.push(module.name.clone());
                machine.machine.modules.push(module.clone());
            }
        }

        self.solve();
        Ok(())
    }

    pub fn remove_module(
        &mut self,
        machine_index: usize,
        module_index: usize,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(machine_index)
            .context("invalid machine index")?;
        match &mut machine.snippet {
            SnippetMachine::Source { .. } | SnippetMachine::Sink { .. } => {
                bail!("modules are not supported for source and sink")
            }
            SnippetMachine::Crafter { modules, .. } => {
                ensure!(module_index < modules.len(), "invalid module index");
                modules.remove(module_index);
                ensure!(
                    module_index < machine.machine.modules.len(),
                    "snippet-machine desync"
                );
                machine.machine.modules.remove(module_index);
            }
        }

        self.solve();
        Ok(())
    }

    pub fn set_beacons(
        &mut self,
        machine_index: usize,
        new_beacons: Vec<Vec<Module>>,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(machine_index)
            .context("invalid machine index")?;
        if new_beacons.iter().any(|b| b.len() > 2) {
            bail!("too many modules in a beacon");
        }
        if new_beacons
            .iter()
            .flatten()
            .any(|m| m.type_ == ModuleType::Productivity)
        {
            bail!("productivity modules are not allowed in beacons");
        }
        match &mut machine.snippet {
            SnippetMachine::Source { .. } | SnippetMachine::Sink { .. } => {
                bail!("beacons are not supported for source and sink")
            }
            SnippetMachine::Crafter { beacons, .. } => {
                *beacons = new_beacons
                    .iter()
                    .map(|beacon| beacon.iter().map(|m| m.name.clone()).collect_vec())
                    .collect_vec();
                machine.machine.beacons = new_beacons;
            }
        }
        self.solve();
        Ok(())
    }

    pub fn added_items(&self) -> BTreeSet<String> {
        self.machines
            .iter()
            .flat_map(|m| {
                m.machine
                    .recipe
                    .ingredients
                    .iter()
                    .map(|i| i.name.to_string())
                    .chain(m.machine.recipe.products.iter().map(|i| i.name.to_string()))
            })
            .collect()
    }

    fn solve(&mut self) {
        if let Err(err) = self.try_solve() {
            warn!("failed to solve: {err}");
        }
    }

    fn try_solve(&mut self) -> anyhow::Result<()> {
        /*
            Ax = b
           vector row = matrix row = index of equation = index of constraint
           matrix column = index of variable = index of machine
        */

        self.solved = false;
        if self.machines.is_empty() {
            self.solved = true;
            return Ok(());
        }
        for machine in &mut self.machines {
            machine.machine.crafter_count = 1.0;
        }
        let items = self.added_items();
        let constraints: Vec<_> = items
            .iter()
            .map(|item| Constraint::ItemSumsToZero {
                item: item.to_string(),
            })
            .chain(self.item_speed_constraints.iter().map(|(item, speed)| {
                Constraint::ItemProduction {
                    item: item.into(),
                    speed: *speed,
                }
            }))
            .chain(
                self.machines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, machine)| match &machine.snippet {
                        SnippetMachine::Source { .. } | SnippetMachine::Sink { .. } => None,
                        SnippetMachine::Crafter {
                            count_constraint, ..
                        } => {
                            count_constraint.map(|count| Constraint::MachineCount { index, count })
                        }
                    }),
            )
            .collect();

        let a = DMatrix::from_fn(constraints.len(), self.machines.len(), |row, col| {
            let machine = &self.machines[col];
            match &constraints[row] {
                Constraint::ItemSumsToZero { item } => machine
                    .machine
                    .item_speeds()
                    .into_iter()
                    .filter(|i| &i.item == item)
                    .map(|i| i.speed)
                    .sum::<f64>(),
                Constraint::ItemProduction { item, speed: _ } => machine
                    .machine
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
        });
        let b = DVector::from_fn(constraints.len(), |row, _| match &constraints[row] {
            Constraint::ItemSumsToZero { item: _ } => 0.0,
            Constraint::ItemProduction { item: _, speed } => *speed,
            Constraint::MachineCount { index: _, count } => *count,
        });
        trace!("constraints: {constraints:?}");
        trace!("a=");
        for row in a.row_iter() {
            trace!("{:?}", row.iter().collect_vec());
        }
        trace!("b={b:?}");

        let svd = a.clone().svd(true, true);
        let output = svd
            .solve(&b, 0.000001)
            .map_err(|str| format_err!("{str}"))?;
        trace!("output {output:?}");

        if output.iter().all(|v| *v == 0.0) {
            bail!("solve result is zero; try adding more constraints");
        }

        for (machine, output_item) in self.machines.iter_mut().zip_eq(output.iter()) {
            machine.machine.crafter_count = *output_item;
        }

        let error = (a * output.clone() - b).norm();
        if error > 0.01 {
            bail!("couldn't fit all constraints (error = {}); try removing constraints or changing their values", rf(error));
        }
        if output.iter().any(|x| *x < 0.0) {
            bail!("solution is negative! try adding more constraints");
        }

        self.solved = true;
        Ok(())
    }

    fn add_sources_and_sinks(&mut self) -> anyhow::Result<()> {
        self.machines
            .retain(|m| m.machine.crafter.name != "source" && m.machine.crafter.name != "sink");
        let items = self.added_items();
        for item in items {
            let any_inputs = self
                .machines
                .iter()
                .any(|m| m.machine.recipe.ingredients.iter().any(|i| i.name == item));
            let any_outputs = self
                .machines
                .iter()
                .any(|m| m.machine.recipe.products.iter().any(|i| i.name == item));
            if any_inputs && !any_outputs {
                self.add_source(&item)?;
            } else if !any_inputs && any_outputs {
                self.add_sink(&item)?;
            }
        }
        Ok(())
    }

    fn auto_sort_machines(&mut self) {
        let mut new_machines = Vec::new();
        let mut remaining_machines = self.machines.clone();
        let mut crafted_items = BTreeSet::new();
        loop {
            let mut new_remaining_machines = Vec::new();
            let old_count = new_machines.len();
            for machine in remaining_machines {
                if machine
                    .machine
                    .recipe
                    .ingredients
                    .iter()
                    .all(|ing| crafted_items.contains(&ing.name))
                {
                    for product in &machine.machine.recipe.products {
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
            warn!("remaining_machines is not empty: {remaining_machines:?}");
            new_machines.extend(remaining_machines);
        }
        self.machines = new_machines;
    }

    fn after_machines_changed(&mut self) {
        if let Err(r) = self.add_sources_and_sinks() {
            warn!("failed to add sources and sinks: {r}");
        }
        self.auto_sort_machines();
        self.solve();
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn solved(&self) -> bool {
        self.solved
    }

    pub fn machines(&self) -> &[EditorMachine] {
        &self.machines
    }

    pub fn snippet(&self) -> Snippet {
        Snippet {
            machines: self.machines.iter().map(|m| m.snippet.clone()).collect(),
            item_speed_constraints: self.item_speed_constraints.clone(),
        }
    }

    pub fn item_speed_constraints(&self) -> &BTreeMap<String, f64> {
        &self.item_speed_constraints
    }
}
