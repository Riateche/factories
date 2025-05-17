use {
    crate::{editor::Editor, flowchart, game_data::Recipe, info::Info, machine::Module},
    anyhow::{format_err, Context},
    arboard::Clipboard,
    itertools::Itertools,
    ordered_float::OrderedFloat,
    std::{
        collections::{BTreeSet, VecDeque},
        ffi::OsStr,
        fmt::Display,
        sync::mpsc::Receiver,
        time::Instant,
    },
    url::Url,
};

pub struct MyApp {
    pub msg_receiver: Receiver<String>,

    // Static data
    pub all_recipe_menu_items: Vec<String>,
    pub belt_speeds: Vec<(f64, String)>,
    pub default_speed_module: Module,
    pub default_productivity_module: Module,

    // Global
    pub editor: Editor,
    pub snippet_names: BTreeSet<String>,
    pub generation: u64, // used to generate new ids for tooltips when things change to force correct size
    pub alerts: VecDeque<(String, Instant)>,
    pub auto_focus: bool,

    // Save/load
    pub snippet_name: String,
    pub saved: bool,
    pub confirm_delete: Option<String>,

    // Add recipe
    pub recipe_search_text: String,

    // Machines view
    // (recipe_name_with_machine, display_text)
    pub replace_with_craft_options: Vec<(String, String)>,
    pub replace_with_craft_index: Option<usize>,

    // Item speed constraints
    pub item_speed_contraint_item: String,
    pub old_item_speed_contraint_item: String,
    pub item_speed_contraint_speed: String,

    // Edit machine
    pub edit_machine_index: Option<usize>,
    pub machine_count_constraint: String,
    pub focus_machine_constraint_input: bool,
    pub num_beacons: String,
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
    pub fn new(ui_msg_receiver: Receiver<String>) -> anyhow::Result<Self> {
        let editor = Editor::init()?;
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
        let mut belt_speeds = editor
            .info()
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

        let (speed_module_name, prod_module_name) = match editor.info().config.module_tier {
            1 => ("speed-module", "productivity-module"),
            2 => ("speed-module-2", "productivity-module-2"),
            3 => ("speed-module-3", "productivity-module-3"),
            _ => panic!("invalid module_tier in config, expected 1, 2 or 3"),
        };
        let default_speed_module = editor
            .info()
            .modules
            .get(speed_module_name)
            .unwrap()
            .clone();
        let default_productivity_module =
            editor.info().modules.get(prod_module_name).unwrap().clone();

        let mut app = MyApp {
            msg_receiver: ui_msg_receiver,
            editor,
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
            edit_machine_index: None,
            replace_with_craft_options: Vec::new(),
            replace_with_craft_index: None,
            belt_speeds,
            focus_machine_constraint_input: false,
            default_speed_module,
            default_productivity_module,
            num_beacons: String::new(),
        };
        app.all_recipe_menu_items = app
            .editor
            .info()
            .game_data
            .recipes
            .values()
            .flat_map(|recipe| recipe_menu_items(app.editor.info(), recipe))
            .collect();

        tracing::info!("test info");
        Ok(app)
    }

    pub fn add_recipe(&mut self, text: &str) -> anyhow::Result<()> {
        self.saved = false;
        self.alerts.clear();
        if let Some((recipe, machine)) = text.split_once(" @ ") {
            self.editor.add_crafter(recipe, Some(machine))?;
        } else {
            self.editor.add_crafter(text, None)?;
        };
        self.recipe_search_text.clear();
        self.after_machines_changed();
        Ok(())
    }

    pub fn save_chart(&self) -> anyhow::Result<()> {
        let chart = flowchart::generate(&self.editor, name_or_untitled(&self.snippet_name));
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
        self.editor.load_snippet(format!("snippets/{name}.json"))?;
        self.snippet_name = name.into();
        self.saved = true;
        Ok(())
    }

    pub fn save_snippet(&mut self) -> anyhow::Result<()> {
        if self.snippet_name.is_empty() {
            return Ok(());
        }
        self.editor.save_snippet(format!(
            "snippets/{}.json",
            name_or_untitled(&self.snippet_name)
        ))?;
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
        self.generation += 1;
        let r = self.save_snippet();
        self.show_error(&r);
    }

    pub fn after_constraint_changed(&mut self) {
        self.generation += 1;
        let r = self.save_snippet();
        self.show_error(&r);
    }

    pub fn new_snippet(&mut self) {
        self.alerts.clear();
        self.generation += 1;
        self.snippet_name = String::new();
        self.saved = false;
        self.editor.clear();
    }

    pub fn copy_description(&self) -> anyhow::Result<()> {
        let text = self.editor.description();
        Clipboard::new()?.set_text(&text)?;
        Ok(())
    }
}

pub fn recipe_menu_items(info: &Info, recipe: &Recipe) -> Vec<String> {
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

    let crafters = info
        .category_to_crafter
        .get(&recipe.category)
        .expect("missing item in category_to_crafter");

    if info.auto_select_crafter(crafters).is_some() {
        vec![recipe.name.clone()]
    } else {
        crafters
            .iter()
            .map(move |crafter| format!("{} @ {}", &recipe.name, crafter))
            .collect()
    }
}
