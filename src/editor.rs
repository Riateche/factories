use {
    crate::{
        info::Info,
        machine::{Beacon, ItemSpeed, Machine, Module, ModuleType},
        module_counts,
        primitives::{
            Amount, CrafterName, ItemNameAndQuality, MachineCount, ModuleName, Quality, RecipeName,
            Speed,
        },
        rf,
        snippet::{BeaconSnippet, CrafterSnippet, MachineSnippet, Snippet, SourceSinkSnippet},
    },
    anyhow::{bail, ensure, format_err, Context},
    fallible_iterator::{FallibleIterator, IteratorExt},
    itertools::Itertools,
    nalgebra::{DMatrix, DVector},
    ordered_float::OrderedFloat,
    std::{
        collections::{BTreeMap, BTreeSet},
        fmt::Write,
        path::Path,
    },
    tracing::{trace, warn},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EditorMachineId(pub u64);

#[derive(Debug, Clone)]
pub struct EditorMachine {
    id: EditorMachineId,
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

    pub fn id(&self) -> EditorMachineId {
        self.id
    }
}

#[derive(Debug)]
pub struct Editor {
    info: Info,
    machines: Vec<EditorMachine>,
    item_speed_constraints: BTreeMap<ItemNameAndQuality, Speed>,
    solved: bool,
    auto_add_sources_and_sinks: bool,
    use_alt_solver: bool,
}

fn create_crafter(info: &Info, snippet: &CrafterSnippet) -> anyhow::Result<Machine> {
    let recipe = info.game_data.recipe(&snippet.recipe)?.clone();
    let crafters = info
        .category_to_crafter
        .get(&recipe.category)
        .context("unknown recipe category")?;
    ensure!(!crafters.is_empty());
    let name = &snippet.crafter;
    if !crafters.iter().any(|c| c == name) {
        bail!(
            "requested crafter {name:?}, but available crafters for {recipe:?} are: {crafters:?}"
        );
    }
    let crafter = info
        .crafters
        .get(name)
        .with_context(|| format!("crafter not found: {name:?}"))?
        .clone()
        .with_quality(snippet.crafter_quality);

    let modules = snippet
        .modules
        .iter()
        .map(|item| {
            info.module(&item.name.0.to_string().into())
                .map(|module| module.with_quality(item.quality))
        })
        .transpose_into_fallible()
        .collect()?;

    let beacons = snippet
        .beacons
        .iter()
        .map(|beacon| {
            beacon
                .modules
                .iter()
                .map(|name| info.module(name))
                .transpose_into_fallible()
                .cloned()
                .collect()
                .map(|modules| (beacon.quality, modules))
        })
        .transpose_into_fallible()
        .map(|(quality, modules)| Ok(Beacon { quality, modules }))
        .collect()?;

    Ok(Machine {
        crafter,
        crafter_count: 1.0,
        modules,
        beacons,
        recipe,
        recipe_quality: snippet.recipe_quality,
    })
}

impl Editor {
    pub fn init() -> anyhow::Result<Self> {
        Ok(Editor {
            info: Info::load()?,
            machines: Vec::new(),
            item_speed_constraints: Default::default(),
            solved: true,
            auto_add_sources_and_sinks: true,
            use_alt_solver: false,
        })
    }

    fn create_machine(&self, snippet: &MachineSnippet) -> anyhow::Result<Machine> {
        match snippet {
            MachineSnippet::Source(snippet) => Ok(Machine::new_source(&snippet.item)),
            MachineSnippet::Sink(snippet) => Ok(Machine::new_sink(&snippet.item)),
            MachineSnippet::Crafter(snippet) => create_crafter(&self.info, snippet),
        }
    }

    pub fn load_snippet(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let snippet = serde_json::from_str::<Snippet>(&fs_err::read_to_string(path)?)?;
        let mut machines = Vec::new();
        for (i, machine) in snippet.machines.into_iter().enumerate() {
            machines.push(EditorMachine {
                id: EditorMachineId(i as u64),
                snippet: machine.clone(),
                machine: self.create_machine(&machine)?,
            });
        }
        self.machines = machines;
        self.item_speed_constraints = snippet.item_speed_constraints;
        self.auto_add_sources_and_sinks = snippet.auto_add_sources_and_sinks;
        self.use_alt_solver = snippet.use_alt_solver;
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

    fn next_machine_id(&self) -> EditorMachineId {
        (0..)
            .map(EditorMachineId)
            .find(|id| self.machines.iter().all(|m| &m.id != id))
            .unwrap()
    }

    pub fn add_source(&mut self, item: &ItemNameAndQuality) -> anyhow::Result<()> {
        self.add_source_internal(item)?;
        self.auto_sort_machines();
        self.quick_solve();
        Ok(())
    }

    fn add_source_internal(&mut self, item: &ItemNameAndQuality) -> anyhow::Result<()> {
        if !self.info.all_items.contains(&item.name) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = MachineSnippet::Source(SourceSinkSnippet { item: item.clone() });
        let machine = self.create_machine(&snippet)?;
        let id = self.next_machine_id();
        self.machines.push(EditorMachine {
            id,
            snippet,
            machine,
        });
        Ok(())
    }

    pub fn add_sink(&mut self, item: &ItemNameAndQuality) -> anyhow::Result<()> {
        self.add_sink_internal(item)?;
        self.auto_sort_machines();
        self.quick_solve();
        Ok(())
    }

    fn add_sink_internal(&mut self, item: &ItemNameAndQuality) -> anyhow::Result<()> {
        if !self.info.all_items.contains(&item.name) {
            bail!("unknown item: {item:?}");
        }
        self.solved = false;
        let snippet = MachineSnippet::Sink(SourceSinkSnippet { item: item.clone() });
        let machine = self.create_machine(&snippet)?;
        let id = self.next_machine_id();
        self.machines.push(EditorMachine {
            id,
            snippet,
            machine,
        });
        Ok(())
    }

    pub fn add_recycler(
        &mut self,
        recycler_quality: Option<Quality>,
        index: usize,
        fill_module: Option<&Module>,
    ) -> anyhow::Result<EditorMachineId> {
        let machine = self.machines.get(index).context("invalid index")?;
        ensure!(machine.machine.crafter.is_sink(), "not a sink");
        let input = machine
            .machine
            .input_speeds()
            .next()
            .context("missing input in sink")?;

        let recipe = self
            .info
            .game_data
            .recipes
            .values()
            .find(|recipe| {
                recipe.is_recycling() && recipe.ingredients.iter().any(|ing| ing.name == input.item)
            })
            .with_context(|| format!("recyling recipe not found for {:?}", input.item))?
            .clone();

        let id = self.add_crafter(
            &recipe.name,
            input.quality,
            Some(&"recycler".into()),
            recycler_quality,
            fill_module,
        )?;
        Ok(id)
    }

    pub fn add_crafter(
        &mut self,
        recipe_name: &RecipeName,
        recipe_quality: Quality,
        crafter: Option<&CrafterName>,
        crafter_quality: Option<Quality>,
        fill_module: Option<&Module>,
    ) -> anyhow::Result<EditorMachineId> {
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
        let crafter_quality = crafter_quality
            .or_else(|| self.info.config.crafter_qualities.get(&crafter).copied())
            .unwrap_or(Quality(0));

        trace!("selected crafter: {crafter:?}");
        self.solved = false;
        let add_auto_constraint =
            self.machines.is_empty() && self.item_speed_constraints.is_empty();

        let crafter_info = self
            .info
            .crafters
            .get(&crafter)
            .with_context(|| format!("crafter not found: {crafter:?}"))?;
        let snippet = CrafterSnippet {
            crafter,
            crafter_quality,
            modules: if let Some(module) = fill_module {
                (0..crafter_info.module_inventory_size)
                    .map(|_| ItemNameAndQuality {
                        name: module.name.0.as_str().into(),
                        quality: module.quality,
                    })
                    .collect()
            } else {
                vec![]
            },
            beacons: vec![],
            recipe: recipe_name.clone(),
            recipe_quality,
            count_constraint: None,
        }
        .into();
        let machine = self.create_machine(&snippet)?;
        let id = self.next_machine_id();
        self.machines.push(EditorMachine {
            id,
            snippet,
            machine,
        });

        if add_auto_constraint {
            if let Some(product) = recipe.products.first() {
                self.item_speed_constraints.insert(
                    ItemNameAndQuality {
                        name: product.name.clone(),
                        quality: recipe_quality,
                    },
                    Speed::ONE,
                );
            }
        }
        self.after_machines_changed();
        Ok(id)
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
        machine.machine = create_crafter(&self.info, snippet)?;

        self.after_machines_changed();
        Ok(())
    }

    pub fn set_crafter_quality(&mut self, index: usize, quality: Quality) -> anyhow::Result<()> {
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
        snippet.crafter_quality = quality;
        machine.machine = create_crafter(&self.info, snippet)?;

        self.after_machines_changed();
        Ok(())
    }

    pub fn set_recipe_quality(&mut self, index: usize, quality: Quality) -> anyhow::Result<()> {
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
        snippet.recipe_quality = quality;
        machine.machine = create_crafter(&self.info, snippet)?;

        self.after_machines_changed();
        Ok(())
    }

    pub fn duplicate_for_all_qualities(&mut self, index: usize) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(index)
            .context("invalid machine index")?;
        let snippet = match &mut machine.snippet {
            MachineSnippet::Source(_) | MachineSnippet::Sink(_) => {
                bail!("cannot set crafter for source or sink");
            }
            MachineSnippet::Crafter(snippet) => snippet.clone(),
        };
        for quality in Quality::ALL {
            let exists = self.machines.iter().any(|machine| {
                if let MachineSnippet::Crafter(s) = &machine.snippet {
                    s.recipe == snippet.recipe && s.recipe_quality == quality
                } else {
                    false
                }
            });
            if exists {
                continue;
            }
            let new_snippet = MachineSnippet::Crafter(CrafterSnippet {
                recipe_quality: quality,
                ..snippet.clone()
            });
            let machine = self.create_machine(&new_snippet)?;
            let id = self.next_machine_id();
            self.machines.push(EditorMachine {
                id,
                snippet: new_snippet,
                machine,
            });
        }

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
                writeln!(out, "[{modules_text}]").unwrap();
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
        item: &ItemNameAndQuality,
        speed: Option<Speed>,
        replace_all: bool,
    ) -> anyhow::Result<()> {
        if replace_all {
            self.clear_all_constraints_internal();
        }
        if !self.info.all_items.contains(&item.name) {
            bail!("unknown item: {item:?}");
        }
        if let Some(speed) = speed {
            self.item_speed_constraints.insert(item.clone(), speed);
        } else {
            self.item_speed_constraints.remove(item);
        }
        self.quick_solve();
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
        self.quick_solve();
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

    pub fn add_modules(
        &mut self,
        machine_index: usize,
        module: &ItemNameAndQuality,
        count: u64,
    ) -> anyhow::Result<()> {
        let machine = self
            .machines
            .get_mut(machine_index)
            .context("invalid machine index")?;

        let module = self
            .info
            .modules
            .get(&ModuleName(module.name.0.to_string()))
            .unwrap()
            .with_quality(module.quality);

        match module.type_ {
            ModuleType::Speed => {}
            ModuleType::Productivity => {
                if !machine.machine.recipe.allowed_effects.productivity {
                    bail!("machine recipe doesn't allow productivity");
                }
            }
            ModuleType::Quality => {
                if !machine.machine.recipe.allowed_effects.quality {
                    bail!("machine recipe doesn't allow quality");
                }
            }
        }

        if machine.machine.crafter.module_inventory_size
            < machine.machine.modules.len() as u64 + count
        {
            bail!("no more space for modules");
        }
        match &mut machine.snippet {
            MachineSnippet::Source { .. } | MachineSnippet::Sink { .. } => {
                bail!("modules are not supported for source and sink")
            }
            MachineSnippet::Crafter(snippet) => {
                for _ in 0..count {
                    snippet.modules.push(ItemNameAndQuality {
                        name: module.name.as_str().into(),
                        quality: module.quality,
                    });
                    machine.machine.modules.push(module.clone());
                }
            }
        }

        self.after_machines_changed();
        Ok(())
    }

    pub fn remove_module(
        &mut self,
        machine_index: usize,
        module_index: usize,
        batch: bool,
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
                if batch {
                    let module_type = snippet.modules[module_index].clone();
                    snippet.modules.retain(|m| m != &module_type);
                    machine.machine.modules.retain(|m| {
                        m.name.0 != *module_type.name.0 || m.quality != module_type.quality
                    });
                } else {
                    snippet.modules.remove(module_index);
                    ensure!(
                        module_index < machine.machine.modules.len(),
                        "snippet-machine desync"
                    );
                    machine.machine.modules.remove(module_index);
                }
            }
        }

        self.after_machines_changed();
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
                        quality: beacon.quality,
                        modules: beacon.modules.iter().map(|m| m.name.clone()).collect_vec(),
                    })
                    .collect_vec();
                machine.machine.beacons = new_beacons;
            }
        }
        self.quick_solve();
        Ok(())
    }

    pub fn added_items(&self) -> BTreeSet<ItemNameAndQuality> {
        self.machines
            .iter()
            .flat_map(|m| m.machine.item_speeds().map(|i| i.name_and_quality()))
            .collect()
    }

    pub fn all_inputs(&self) -> BTreeSet<ItemNameAndQuality> {
        self.machines
            .iter()
            .flat_map(|m| m.machine.input_speeds().map(|i| i.name_and_quality()))
            .collect()
    }

    pub fn all_outputs(&self) -> BTreeSet<ItemNameAndQuality> {
        self.machines
            .iter()
            .flat_map(|m| {
                m.machine
                    .output_speeds()
                    .into_iter()
                    .map(|i| i.name_and_quality())
            })
            .collect()
    }

    fn quick_solve(&mut self) {
        if !self.use_alt_solver {
            let r = self.try_solve();

            if let Err(err) = r {
                warn!("failed to solve: {err}");
            }
        } else {
            self.solved = false;
        }
    }

    pub fn solve(&mut self) {
        let r = if self.use_alt_solver {
            self.try_solve_alt()
        } else {
            self.try_solve()
        };

        if let Err(err) = r {
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
            ItemSumsToZero {
                item: ItemNameAndQuality,
            },
            ItemProduction {
                item: ItemNameAndQuality,
                speed: Speed,
            },
            MachineCount {
                index: usize,
                count: MachineCount,
            },
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
                    .filter(|i| &i.name_and_quality() == item)
                    .map(|i| i.speed)
                    .sum::<Speed>()
                    .into(),
                Constraint::ItemProduction { item, speed: _ } => machine
                    .machine
                    .item_speeds()
                    .filter(|i| &i.name_and_quality() == item && i.speed > Speed::ZERO)
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

    fn try_solve_alt(&mut self) -> anyhow::Result<()> {
        #[derive(Debug)]
        struct MachineInfo {
            input_speeds: Vec<ItemSpeed>,
            output_speeds: Vec<ItemSpeed>,
            #[allow(dead_code)]
            is_recycler: bool,
            count_constraint: Option<MachineCount>,
            crafter_ticks: OrderedFloat<f64>,
            crafter_ticks_on_this_step: OrderedFloat<f64>,
        }

        self.solved = false;
        let max_count = Amount::from(10_000.);
        for machine in &mut self.machines {
            machine.machine.crafter_count = 1.;
        }
        let mut machines = self
            .machines
            .iter()
            .map(|machine| MachineInfo {
                input_speeds: machine.machine.input_speeds().collect(),
                output_speeds: machine.machine.output_speeds(),
                is_recycler: machine.machine.crafter.is_recycler(),
                count_constraint: if let MachineSnippet::Crafter(crafter) = &machine.snippet {
                    crafter.count_constraint
                } else if machine.machine.crafter.is_source_or_sink() {
                    let item = machine
                        .machine
                        .input_speeds()
                        .chain(machine.machine.output_speeds())
                        .next()
                        .expect("missing i/o in source or sink")
                        .name_and_quality();

                    self.item_speed_constraints
                        .get(&item)
                        .map(|speed| MachineCount(speed.0))
                } else {
                    None
                },
                crafter_ticks: 0.0.into(),
                crafter_ticks_on_this_step: 0.0.into(),
            })
            .collect_vec();
        let mut storage = BTreeMap::<ItemNameAndQuality, Amount>::new();
        let steps = 5_000;
        let warmup_end = steps / 5;
        for step in 0..steps {
            println!("step={step}/{steps}");
            for machine in &mut machines {
                machine.crafter_ticks_on_this_step = 0.0.into();
            }
            if step == warmup_end {
                for machine in &mut machines {
                    machine.crafter_ticks = 0.0.into();
                }
            }
            loop {
                let mut any_progress = false;
                for machine in &mut machines {
                    // if machine.is_recycler
                    //     && machine.input_speeds.iter().any(|item| {
                    //         *storage.entry(item.name_and_quality()).or_default() < max_count / 2.
                    //     })
                    // {
                    //     continue;
                    // }

                    if let Some(constraint) = machine.count_constraint {
                        if machine.crafter_ticks_on_this_step >= constraint.0 {
                            continue;
                        }
                    }

                    let can_work = machine
                        .input_speeds
                        .iter()
                        .chain(&machine.output_speeds)
                        .all(|item_speed| {
                            let old_count = storage
                                .get(&item_speed.name_and_quality())
                                .copied()
                                .unwrap_or(Amount::ZERO);
                            let new_count = old_count + Amount(item_speed.speed.0);
                            (Amount::ZERO..max_count).contains(&new_count)
                            // if machine_index == 9 {
                            //     println!(
                            //         "{} q{} {} -> {}, {}",
                            //         item_speed.item, item_speed.quality.0, old_count, new_count, r,
                            //     );
                            // }
                        });
                    // if machine_index == 9 {
                    //     println!("can_work={can_work}\n");
                    // }
                    if can_work {
                        for item_speed in machine.input_speeds.iter().chain(&machine.output_speeds)
                        {
                            *storage.entry(item_speed.name_and_quality()).or_default() +=
                                Amount(item_speed.speed.0);
                        }
                        machine.crafter_ticks += 1.0;
                        machine.crafter_ticks_on_this_step += 1.0;
                        any_progress = true;
                        // println!("progress {:?}", machine);
                    }
                }
                if !any_progress {
                    break;
                }
            }
        }

        for (machine, info) in self.machines.iter_mut().zip(machines) {
            // println!("\n{}", machine.machine.description());
            // println!("crafter_ticks={:?}", info.crafter_ticks);
            machine.machine.crafter_count = (info.crafter_ticks / (steps as f64)).into();
        }

        self.solved = true;
        Ok(())
    }

    fn add_sources_and_sinks(&mut self) -> anyhow::Result<()> {
        if !self.auto_add_sources_and_sinks {
            return Ok(());
        }
        self.machines
            .retain(|m| !m.machine.crafter.is_source_or_sink());
        let items = self.added_items();
        for item in items {
            let any_inputs = self.machines.iter().any(|m| {
                m.machine
                    .input_speeds()
                    .any(|i| i.name_and_quality() == item)
            });
            let any_outputs = self.machines.iter().any(|m| {
                m.machine
                    .output_speeds()
                    .into_iter()
                    .any(|i| i.name_and_quality() == item)
            });
            if any_inputs && !any_outputs {
                self.add_source_internal(&item)?;
            } else if !any_inputs && any_outputs {
                self.add_sink_internal(&item)?;
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
            //warn!("remaining_machines is not empty: {remaining_machines:?}");
            new_machines.extend(remaining_machines);
        }
        self.machines = new_machines;
    }

    fn after_machines_changed(&mut self) {
        if let Err(r) = self.add_sources_and_sinks() {
            warn!("failed to add sources and sinks: {r}");
        }
        self.auto_sort_machines();
        self.quick_solve();
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
            auto_add_sources_and_sinks: self.auto_add_sources_and_sinks,
            use_alt_solver: self.use_alt_solver,
        }
    }

    pub fn item_speed_constraints(&self) -> &BTreeMap<ItemNameAndQuality, Speed> {
        &self.item_speed_constraints
    }

    pub fn auto_add_sources_and_sinks(&self) -> bool {
        self.auto_add_sources_and_sinks
    }

    pub fn set_auto_add_sources_and_sinks(&mut self, value: bool) {
        self.auto_add_sources_and_sinks = value;
        if value {
            self.after_machines_changed();
        }
    }

    pub fn use_alt_solver(&self) -> bool {
        self.use_alt_solver
    }

    pub fn set_use_alt_solver(&mut self, value: bool) {
        self.use_alt_solver = value;
        self.quick_solve();
    }
}
