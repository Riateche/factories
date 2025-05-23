use {
    crate::{
        info::Info,
        machine::{Beacon, Machine, ModuleType},
        module_counts,
        primitives::{CrafterName, ItemName, MachineCount, ModuleName, RecipeName, Speed},
        rf,
        snippet::{BeaconSnippet, CrafterSnippet, MachineSnippet, Snippet, SourceSinkSnippet},
    },
    anyhow::{bail, ensure, format_err, Context},
    fallible_iterator::{FallibleIterator, IteratorExt},
    itertools::Itertools,
    nalgebra::{DMatrix, DVector},
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::Write,
        path::Path,
    },
    tracing::{trace, warn},
};

#[derive(Debug, Clone)]
pub struct EditorMachine {
    snippet: MachineSnippet,
    machine: Machine,
}

impl EditorMachine {
    pub fn snippet(&self) -> &MachineSnippet {
        &self.snippet
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }
}

#[derive(Debug)]
pub struct Editor {
    info: Info,
    machines: Vec<EditorMachine>,
    item_speed_constraints: BTreeMap<ItemName, Speed>,
    solved: bool,
}

impl Editor {
    pub fn init() -> anyhow::Result<Self> {
        Ok(Editor {
            info: Info::load()?,
            machines: Vec::new(),
            item_speed_constraints: Default::default(),
            solved: true,
        })
    }

    fn create_crafter(&self, snippet: &CrafterSnippet) -> anyhow::Result<Machine> {
        let recipe = self.info.game_data.recipe(&snippet.recipe)?.clone();
        let crafters = self
            .info
            .category_to_crafter
            .get(&recipe.category)
            .context("unknown recipe category")?;
        ensure!(!crafters.is_empty());
        let name = &snippet.crafter;
        if !crafters.iter().any(|c| c == name) {
            bail!("requested crafter {name:?}, but available crafters for {recipe:?} are: {crafters:?}");
        }
        let crafter = self
            .info
            .crafters
            .get(name)
            .with_context(|| format!("crafter not found: {name:?}"))?
            .clone();

        let modules = snippet
            .modules
            .iter()
            .map(|name| self.info.module(name))
            .transpose_into_fallible()
            .cloned()
            .collect()?;

        let beacons = snippet
            .beacons
            .iter()
            .map(|beacon| {
                beacon
                    .modules
                    .iter()
                    .map(|name| self.info.module(name))
                    .transpose_into_fallible()
                    .cloned()
                    .collect()
            })
            .transpose_into_fallible()
            .map(|modules| Ok(Beacon { modules }))
            .collect()?;

        Ok(Machine {
            crafter,
            crafter_count: 1.0,
            modules,
            beacons,
            recipe,
        })
    }

    fn create_machine(&self, snippet: &MachineSnippet) -> anyhow::Result<Machine> {
        match snippet {
            MachineSnippet::Source(snippet) => Ok(Machine::new_source(&snippet.item)),
            MachineSnippet::Sink(snippet) => Ok(Machine::new_sink(&snippet.item)),
            MachineSnippet::Crafter(snippet) => self.create_crafter(snippet),
        }
    }

    pub fn load_snippet(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let snippet = serde_json::from_str::<Snippet>(&fs_err::read_to_string(path)?)?;
        let mut machines = Vec::new();
        for machine in snippet.machines {
            machines.push(EditorMachine {
                snippet: machine.clone(),
                machine: self.create_machine(&machine)?,
            });
        }
        self.machines = machines;
        self.item_speed_constraints = snippet.item_speed_constraints;
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

    fn add_source(&mut self, item: &ItemName) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = MachineSnippet::Source(SourceSinkSnippet { item: item.clone() });
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });
        Ok(())
    }

    fn add_sink(&mut self, item: &ItemName) -> anyhow::Result<()> {
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = MachineSnippet::Sink(SourceSinkSnippet { item: item.clone() });
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });
        Ok(())
    }

    pub fn add_crafter(
        &mut self,
        recipe_name: &RecipeName,
        crafter: Option<&CrafterName>,
    ) -> anyhow::Result<()> {
        let recipe = self.info.game_data.recipe(recipe_name)?.clone();
        let crafters = self
            .info
            .category_to_crafter
            .get(&recipe.category)
            .context("unknown recipe category")?;
        ensure!(!crafters.is_empty());
        let crafter = if let Some(crafter) = crafter {
            crafter.clone()
        } else if let Some(crafter) = self.info.auto_select_crafter(crafters) {
            crafter
        } else {
            bail!("ambiguous crafter for {recipe:?}: {crafters:?}");
        };

        trace!("selected crafter: {crafter:?}");
        self.solved = false;
        let add_auto_constraint =
            self.machines.is_empty() && self.item_speed_constraints.is_empty();

        let snippet = CrafterSnippet {
            crafter,
            modules: vec![],
            beacons: vec![],
            recipe: recipe_name.clone(),
            count_constraint: None,
        }
        .into();
        let machine = self.create_machine(&snippet)?;
        self.machines.push(EditorMachine { snippet, machine });

        if add_auto_constraint {
            if let Some(product) = recipe.products.get(0) {
                self.item_speed_constraints
                    .insert(product.name.clone(), Speed::ONE);
            }
        }
        self.after_machines_changed();
        Ok(())
    }

    pub fn remove_machine(&mut self, index: usize) -> anyhow::Result<()> {
        ensure!(index < self.machines.len(), "invalid machine index");
        self.machines.remove(index);
        self.after_machines_changed();
        if self.machines.is_empty() {
            self.item_speed_constraints.clear();
        }
        Ok(())
    }

    pub fn set_crafter(
        &mut self,
        index: usize,
        new_crafter_name: &CrafterName,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(index)
            .context("invalid machine index")?;
        let snippet = match &mut machine.snippet {
            MachineSnippet::Source(_) | MachineSnippet::Sink(_) => {
                bail!("cannot set crafter for source or sink");
            }
            MachineSnippet::Crafter(snippet) => snippet,
        };
        let crafters = self
            .info
            .category_to_crafter
            .get(&machine.machine.recipe.category)
            .context("unknown recipe category")?;
        if !crafters.iter().any(|s| s == new_crafter_name) {
            bail!(
                "crafter {:?} not allowed for recipe {:?}",
                new_crafter_name,
                snippet.recipe
            );
        }

        let new_crafter = self
            .info
            .crafters
            .get(new_crafter_name)
            .with_context(|| format!("crafter not found: {:?}", snippet.crafter))?
            .clone();
        snippet
            .modules
            .truncate(new_crafter.module_inventory_size as usize);
        snippet.crafter = new_crafter_name.clone();
        machine
            .machine
            .modules
            .truncate(new_crafter.module_inventory_size as usize);
        machine.machine.crafter = new_crafter;

        self.after_machines_changed();
        Ok(())
    }

    pub fn description(&self) -> String {
        let mut out = String::new();
        let inputs = self
            .machines
            .iter()
            .filter(|m| m.machine.crafter.is_source())
            .flat_map(|m| m.machine.item_speeds())
            .collect_vec();
        writeln!(
            out,
            "Inputs: {}",
            inputs
                .iter()
                .map(|i| { format!("{} {}", i.speed, i.item) })
                .join(" + ")
        )
        .unwrap();
        writeln!(out, "==============================").unwrap();

        for machine in &self.machines {
            if machine.machine.crafter.is_source_or_sink() {
                continue;
            }
            writeln!(out, "{}", machine.machine.description()).unwrap();
            let beacon_text = machine.machine.beacon_text();
            let modules_text = module_counts(&machine.machine.modules)
                .into_iter()
                .map(|(module, count)| format!("{count} × {module}"))
                .chain(Some(beacon_text).filter(|t| !t.is_empty()))
                .join(", ");
            if !modules_text.is_empty() {
                writeln!(out, "[{}]", modules_text).unwrap();
            }
            writeln!(out, "------------------------------").unwrap();
        }
        writeln!(out, "==============================").unwrap();

        let outputs = self
            .machines
            .iter()
            .filter(|m| m.machine.crafter.is_sink())
            .flat_map(|m| m.machine.item_speeds())
            .collect_vec();
        writeln!(
            out,
            "Outputs: {}",
            outputs
                .iter()
                .map(|i| { format!("{} {}", -i.speed, i.item) })
                .join(" + ")
        )
        .unwrap();
        out
    }

    pub fn set_item_speed_constraint(
        &mut self,
        item: &ItemName,
        speed: Option<Speed>,
        replace_all: bool,
    ) -> anyhow::Result<()> {
        if replace_all {
            self.clear_all_constraints_internal();
        }
        if !self.info.all_items.contains(item) {
            bail!("unknown item: {item:?}");
        }
        if let Some(speed) = speed {
            self.item_speed_constraints.insert(item.clone(), speed);
        } else {
            self.item_speed_constraints.remove(item);
        }
        self.solve();
        Ok(())
    }

    pub fn set_machine_count_constraint(
        &mut self,
        index: usize,
        count: Option<MachineCount>,
        replace_all: bool,
    ) -> anyhow::Result<()> {
        if replace_all {
            self.clear_all_constraints_internal();
        }
        let machine = self
            .machines
            .get_mut(index)
            .context("invalid machine index")?;
        match &mut machine.snippet {
            MachineSnippet::Source(_) | MachineSnippet::Sink(_) => bail!(
                "machine count constraint is not allowed for sources \
                and sinks, use item speed constraint instead"
            ),
            MachineSnippet::Crafter(snippet) => {
                snippet.count_constraint = count;
            }
        }
        self.solve();
        Ok(())
    }

    // Clear without solving
    fn clear_all_constraints_internal(&mut self) {
        self.item_speed_constraints.clear();
        for machine in &mut self.machines {
            match &mut machine.snippet {
                MachineSnippet::Source(_) | MachineSnippet::Sink(_) => {}
                MachineSnippet::Crafter(snippet) => {
                    snippet.count_constraint = None;
                }
            }
        }
    }

    pub fn add_module(&mut self, machine_index: usize, module: &ModuleName) -> anyhow::Result<()> {
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
            MachineSnippet::Source { .. } | MachineSnippet::Sink { .. } => {
                bail!("modules are not supported for source and sink")
            }
            MachineSnippet::Crafter(snippet) => {
                snippet.modules.push(module.name.clone());
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
            MachineSnippet::Source { .. } | MachineSnippet::Sink { .. } => {
                bail!("modules are not supported for source and sink")
            }
            MachineSnippet::Crafter(snippet) => {
                ensure!(module_index < snippet.modules.len(), "invalid module index");
                snippet.modules.remove(module_index);
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
        new_beacons: Vec<Beacon>,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(machine_index)
            .context("invalid machine index")?;
        if new_beacons.iter().any(|b| b.modules.len() > 2) {
            bail!("too many modules in a beacon");
        }
        if new_beacons
            .iter()
            .flat_map(|b| &b.modules)
            .any(|m| m.type_ == ModuleType::Productivity)
        {
            bail!("productivity modules are not allowed in beacons");
        }
        match &mut machine.snippet {
            MachineSnippet::Source { .. } | MachineSnippet::Sink { .. } => {
                bail!("beacons are not supported for source and sink")
            }
            MachineSnippet::Crafter(snippet) => {
                snippet.beacons = new_beacons
                    .iter()
                    .map(|beacon| BeaconSnippet {
                        modules: beacon.modules.iter().map(|m| m.name.clone()).collect_vec(),
                    })
                    .collect_vec();
                machine.machine.beacons = new_beacons;
            }
        }
        self.solve();
        Ok(())
    }

    pub fn added_items(&self) -> BTreeSet<ItemName> {
        self.machines
            .iter()
            .flat_map(|m| {
                m.machine
                    .recipe
                    .ingredients
                    .iter()
                    .map(|i| i.name.clone())
                    .chain(m.machine.recipe.products.iter().map(|i| i.name.clone()))
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

        #[derive(Debug, Clone)]
        enum Constraint {
            ItemSumsToZero { item: ItemName },
            ItemProduction { item: ItemName, speed: Speed },
            MachineCount { index: usize, count: MachineCount },
        }

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
            .map(|item| Constraint::ItemSumsToZero { item: item.clone() })
            .chain(self.item_speed_constraints.iter().map(|(item, speed)| {
                Constraint::ItemProduction {
                    item: item.clone(),
                    speed: *speed,
                }
            }))
            .chain(
                self.machines
                    .iter()
                    .enumerate()
                    .filter_map(|(index, machine)| match &machine.snippet {
                        MachineSnippet::Source(_) | MachineSnippet::Sink(_) => None,
                        MachineSnippet::Crafter(snippet) => snippet
                            .count_constraint
                            .map(|count| Constraint::MachineCount { index, count }),
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
                    .sum::<Speed>()
                    .into(),
                Constraint::ItemProduction { item, speed: _ } => machine
                    .machine
                    .item_speeds()
                    .into_iter()
                    .filter(|i| &i.item == item && i.speed > Speed::ZERO)
                    .map(|i| i.speed)
                    .sum::<Speed>()
                    .into(),
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
            Constraint::ItemProduction { item: _, speed } => (*speed).into(),
            Constraint::MachineCount { index: _, count } => (*count).into(),
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
            .retain(|m| !m.machine.crafter.is_source_or_sink());
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
                        crafted_items.insert(product.name.clone());
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

    pub fn item_speed_constraints(&self) -> &BTreeMap<ItemName, Speed> {
        &self.item_speed_constraints
    }
}
