use {
    crate::{game_data::Recipe, init, rf, Planner, Snippet},
    anyhow::{format_err, Context},
    itertools::Itertools,
    ordered_float::OrderedFloat,
    std::{
        cmp::min,
        collections::{BTreeSet, VecDeque},
        ffi::OsStr,
        fmt::{Display, Write},
        time::Instant,
    },
    url::Url,
};

pub struct MyApp {
    pub planner: Planner,
    pub snippet_name: String,
    pub snippet_names: BTreeSet<String>,
    pub saved: bool,
    pub recipe_search_text: String,
    pub alerts: VecDeque<(String, Instant)>,
    pub auto_focus: bool,
    pub item_speed_contraint_item: String,
    pub old_item_speed_contraint_item: String,
    pub item_speed_contraint_speed: String,
    pub machine_count_constraint: String,
    pub all_recipe_menu_items: Vec<String>,
    pub confirm_delete: Option<String>,
    pub generation: u64,
    pub add_machine_count_constraint_index: Option<usize>,
    // (recipe_name_with_machine, display_text)
    pub replace_with_craft_options: Vec<(String, String)>,
    pub replace_with_craft_index: Option<usize>,
    pub belt_speeds: Vec<(f64, String)>,
}

const UNTITLED: &str = "Untitled";
pub fn name_or_untitled(name: &str) -> &str {
    if name.is_empty() {
        UNTITLED
    } else {
        name
    }
}

impl MyApp {
    pub fn new() -> anyhow::Result<Self> {
        let planner = init().unwrap();
        fs_err::create_dir_all("snippets")?;
        let mut snippet_names = BTreeSet::new();
        for item in fs_err::read_dir("snippets")? {
            let path = item?.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            snippet_names.insert(
                path.file_stem()
                    .context("missing file stem")?
                    .to_str()
                    .context("non-utf8 file name encountered")?
                    .to_string(),
            );
        }
        let mut belt_speeds = planner
            .game_data
            .entities
            .values()
            .filter(|e| e.type_ == "transport-belt")
            .map(|e| {
                // belt_speed is tiles per tick;
                // throughput per second = belt_speed * 60 (ticks/s) * 8 (density)
                (
                    e.belt_speed.expect("missing belt_speed") * 60. * 8.,
                    e.name.clone(),
                )
            })
            .collect_vec();
        belt_speeds.sort_by_key(|(speed, _)| OrderedFloat(*speed));

        let mut app = MyApp {
            planner,
            recipe_search_text: String::new(),
            auto_focus: true,
            alerts: VecDeque::new(),
            item_speed_contraint_item: String::new(),
            old_item_speed_contraint_item: String::new(),
            item_speed_contraint_speed: String::new(),
            machine_count_constraint: String::new(),
            all_recipe_menu_items: Vec::new(),
            snippet_name: String::new(),
            snippet_names,
            saved: false,
            confirm_delete: None,
            generation: 0,
            add_machine_count_constraint_index: None,
            replace_with_craft_options: Vec::new(),
            replace_with_craft_index: None,
            belt_speeds,
        };
        app.all_recipe_menu_items = app
            .planner
            .game_data
            .recipes
            .values()
            .flat_map(|recipe| recipe_menu_items(&app.planner, recipe))
            .collect();
        Ok(app)
    }

    pub fn add_recipe(&mut self, text: &str) -> anyhow::Result<()> {
        self.saved = false;
        self.planner.snippet.solved = false;
        let add_auto_constraint = self.planner.snippet.machines.is_empty()
            && self.planner.snippet.item_speed_constraints.is_empty();

        if let Some((recipe, machine)) = text.split_once(" @ ") {
            self.planner.create_machine_with_crafter(recipe, machine)?;
        } else {
            self.planner.create_machine(text)?;
        };
        self.recipe_search_text.clear();
        if add_auto_constraint {
            if let Some(product) = self.planner.snippet.machines[0]
                .recipe
                .products
                .get(0)
                .cloned()
            {
                self.planner.add_item_speed_constraint(&product.name, 1.0)?;
            }
        }
        self.after_machines_changed();
        Ok(())
    }

    pub fn generate_chart(&self) -> String {
        let mut out = String::new();
        if !self.snippet_name.is_empty() {
            writeln!(
                out,
                "--- \n\
                title: {}\n\
                ---",
                &self.snippet_name
            )
            .unwrap();
        }
        writeln!(out, "flowchart TD",).unwrap();
        for (index, machine) in self.planner.snippet.machines.iter().enumerate() {
            let (left_bracket, right_bracket) = if machine.crafter.name == "source" {
                ("[\\", "/]")
            } else if machine.crafter.name == "sink" {
                ("[/", "\\]")
            } else {
                ("([", "])")
            };
            writeln!(
                out,
                r#"    machine{}{}"{}*{}*(*{}*)"{}"#,
                index,
                left_bracket,
                if machine.crafter.name == "source" || machine.crafter.name == "sink" {
                    String::new()
                } else {
                    format!("{} × ", rf(machine.crafter_count))
                },
                machine.crafter.name,
                if machine.crafter.name == "source" || machine.crafter.name == "sink" {
                    machine
                        .recipe
                        .ingredients
                        .get(0)
                        .map(|i| &i.name)
                        .unwrap_or_else(|| {
                            machine
                                .recipe
                                .products
                                .get(0)
                                .map(|i| &i.name)
                                .expect("invalid source or sink recipe")
                        })
                } else {
                    &machine.recipe.name
                },
                right_bracket,
            )
            .unwrap();
        }

        let all_items = self.planner.added_items();
        for item in all_items {
            let sources = self
                .planner
                .snippet
                .machines
                .iter()
                .enumerate()
                .filter_map(|(machine_index, machine)| {
                    machine
                        .item_speeds()
                        .into_iter()
                        .find(|item_speed| item_speed.item == item && item_speed.speed > 0.0)
                        .map(|item_speed| (machine_index, item_speed.speed))
                })
                .collect_vec();

            let mut destinations: VecDeque<_> = self
                .planner
                .snippet
                .machines
                .iter()
                .enumerate()
                .filter_map(|(machine_index, machine)| {
                    machine
                        .item_speeds()
                        .into_iter()
                        .find(|item_speed| item_speed.item == item && item_speed.speed < 0.0)
                        .map(|item_speed| (machine_index, -item_speed.speed))
                })
                .collect();

            let epsilon = 0.001;
            'outer: for (source_machine, source_speed) in sources {
                let mut remaining_speed = source_speed;
                loop {
                    let Some((destination_machine, destination_speed)) = destinations.front_mut()
                    else {
                        println!(
                            "WARN: unable to allocate remaining {}/s {} to destinations",
                            remaining_speed, item
                        );
                        break 'outer;
                    };
                    let current_speed = min(
                        OrderedFloat(remaining_speed),
                        OrderedFloat(*destination_speed),
                    )
                    .0;
                    writeln!(
                        out,
                        "    machine{}-->|{}/s *{}*|machine{}",
                        source_machine,
                        rf(current_speed),
                        item,
                        destination_machine
                    )
                    .unwrap();
                    *destination_speed -= current_speed;
                    if *destination_speed < epsilon {
                        destinations.pop_front().unwrap();
                    }
                    remaining_speed -= current_speed;
                    if remaining_speed < epsilon {
                        break; // Move on to the next source machine.
                    }
                }
            }
            if destinations.len() > 2
                || destinations
                    .front()
                    .is_some_and(|(_, speed)| *speed > epsilon)
            {
                println!(
                    "WARN: not all destinations of {} are satisfied: {:?}",
                    item, destinations
                );
            }
        }
        out
    }

    pub fn save_chart(&self) -> anyhow::Result<()> {
        let chart = self.generate_chart();
        let template = include_str!("../../mermaid.html");
        let html = template.replacen("$1", &chart, 1);
        fs_err::create_dir_all("mermaid")?;
        let file_path = std::env::current_dir()?.join(format!(
            "mermaid/{}.html",
            name_or_untitled(&self.snippet_name)
        ));
        fs_err::write(&file_path, html)?;
        Ok(())
    }

    pub fn open_chart(&self) -> anyhow::Result<()> {
        self.save_chart()?;
        let file_path = std::env::current_dir()?.join(format!(
            "mermaid/{}.html",
            name_or_untitled(&self.snippet_name)
        ));
        let url = Url::from_file_path(file_path)
            .map_err(|()| format_err!("Url::from_file_path failed"))?;
        open::that(url.as_str())?;
        Ok(())
    }

    pub fn load_snippet(&mut self, name: &str) -> anyhow::Result<()> {
        self.generation += 1;
        let snippet = serde_json::from_str::<Snippet>(&fs_err::read_to_string(format!(
            "snippets/{name}.json"
        ))?)?;
        self.planner.snippet = snippet;
        self.snippet_name = name.into();
        self.saved = true;
        Ok(())
    }

    pub fn save_snippet(&mut self) -> anyhow::Result<()> {
        if self.snippet_name.is_empty() {
            return Ok(());
        }
        fs_err::write(
            format!("snippets/{}.json", name_or_untitled(&self.snippet_name)),
            serde_json::to_string_pretty(&self.planner.snippet)?,
        )?;
        self.save_chart()?;
        self.saved = true;
        self.snippet_names.insert(self.snippet_name.clone());
        Ok(())
    }

    pub fn show_error<E: Display>(&mut self, result: &Result<(), E>) {
        if let Err(err) = result {
            self.alerts.push_back((err.to_string(), Instant::now()));
            if self.alerts.len() > 5 {
                self.alerts.pop_front();
            }
        }
    }

    pub fn after_machines_changed(&mut self) {
        self.alerts.clear();
        self.generation += 1;
        let r = self
            .planner
            .auto_refresh()
            .and_then(|()| self.save_snippet());
        self.show_error(&r);
    }

    pub fn after_constraint_changed(&mut self) {
        self.alerts.clear();
        self.generation += 1;
        let r = self.planner.solve().and_then(|()| self.save_snippet());
        self.show_error(&r);
    }

    pub fn new_snippet(&mut self) {
        self.alerts.clear();
        self.generation += 1;
        self.snippet_name = String::new();
        self.saved = false;
        self.planner.snippet = Snippet::default();
        self.planner.snippet.solved = true;
    }
}

pub fn recipe_menu_items(planner: &Planner, recipe: &Recipe) -> Vec<String> {
    // let recipe_text = if recipe.products.len() != 1 || recipe.products[0].name != recipe.name {
    //     format!(
    //         "{} ({} ➡ {})",
    //         recipe.name,
    //         recipe.ingredients.iter().map(|i| &i.name).join(" + "),
    //         recipe.products.iter().map(|i| &i.name).join(" + "),
    //     )
    // } else {
    //     recipe.name.clone()
    // };

    let crafters = planner
        .category_to_crafter
        .get(&recipe.category)
        .expect("missing item in category_to_crafter");

    if planner.auto_select_crafter(crafters).is_some() {
        vec![recipe.name.clone()]
    } else {
        crafters
            .iter()
            .map(move |crafter| format!("{} @ {}", &recipe.name, crafter))
            .collect()
    }
}
