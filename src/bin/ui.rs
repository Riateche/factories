#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::collapsible_if)]

use eframe::{
    egui::{self, style::ScrollStyle, Color32, ComboBox},
    icon_data,
};
use factories::{game_data::Recipe, prelude::*, rf, Snippet};

use anyhow::{format_err, Context};
use egui::{
    text::{CCursor, CCursorRange},
    Id, Response, ScrollArea, TextEdit, Ui, Widget, WidgetText,
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::{
    cmp::min,
    collections::BTreeSet,
    ffi::OsStr,
    fmt::Display,
    hash::Hash,
    path::Path,
    time::{Duration, Instant},
};
use std::{collections::VecDeque, fmt::Write};
use url::Url;

/// Dropdown widget
pub struct DropDownBox<
    'a,
    F: FnMut(&mut Ui, &str) -> Response,
    V: AsRef<str>,
    I: Iterator<Item = V>,
> {
    buf: &'a mut String,
    popup_id: Id,
    display: F,
    it: Option<I>,
    hint_text: WidgetText,
    filter_by_input: bool,
    select_on_focus: bool,
    desired_width: Option<f32>,
    max_height: Option<f32>,
}

impl<'a, F: FnMut(&mut Ui, &str) -> Response, V: AsRef<str>, I: Iterator<Item = V>>
    DropDownBox<'a, F, V, I>
{
    /// Creates new dropdown box.
    pub fn from_iter(
        it: impl IntoIterator<IntoIter = I>,
        id_source: impl Hash,
        buf: &'a mut String,
        display: F,
    ) -> Self {
        Self {
            popup_id: Id::new(id_source),
            it: Some(it.into_iter()),
            display,
            buf,
            hint_text: WidgetText::default(),
            filter_by_input: true,
            select_on_focus: false,
            desired_width: None,
            max_height: None,
        }
    }

    /// Add a hint text to the Text Edit
    pub fn hint_text(mut self, hint_text: impl Into<WidgetText>) -> Self {
        self.hint_text = hint_text.into();
        self
    }

    /// Determine whether to filter box items based on what is in the Text Edit already
    pub fn filter_by_input(mut self, filter_by_input: bool) -> Self {
        self.filter_by_input = filter_by_input;
        self
    }

    /// Determine whether to select the text when the Text Edit gains focus
    pub fn select_on_focus(mut self, select_on_focus: bool) -> Self {
        self.select_on_focus = select_on_focus;
        self
    }

    /// Passes through the desired width value to the underlying Text Edit
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = desired_width.into();
        self
    }

    /// Set a maximum height limit for the opened popup
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = height.into();
        self
    }

    pub fn show(&mut self, ui: &mut Ui) -> DropDownBoxOutput {
        let mut edit = TextEdit::singleline(self.buf).hint_text(self.hint_text.clone());
        if let Some(dw) = self.desired_width {
            edit = edit.desired_width(dw);
        }
        let mut committed = false;
        let mut edit_output = edit.show(ui);
        let mut r = edit_output.response;
        if r.has_focus() {
            if self.select_on_focus {
                edit_output
                    .state
                    .cursor
                    .set_char_range(Some(CCursorRange::two(
                        CCursor::new(0),
                        CCursor::new(self.buf.len()),
                    )));
                edit_output.state.store(ui.ctx(), r.id);
            }
            ui.memory_mut(|m| m.open_popup(self.popup_id));
        }

        let request_popup_focus =
            r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::ArrowDown));

        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            committed = true;
        }

        let mut changed = false;
        egui::popup_below_widget(
            ui,
            self.popup_id,
            &r,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                if let Some(max) = self.max_height {
                    ui.set_max_height(max);
                }

                let mut row = 0;
                ScrollArea::vertical()
                    .max_height(500.)
                    .min_scrolled_height(500.)
                    .show(ui, |ui| {
                        for var in self.it.take().unwrap() {
                            let text = var.as_ref();
                            if self.filter_by_input
                                && !self.buf.is_empty()
                                && !text.to_lowercase().contains(&self.buf.to_lowercase())
                            {
                                continue;
                            }

                            let response = (self.display)(ui, text);
                            if row == 0 && request_popup_focus {
                                response.request_focus();
                            }
                            if response.clicked() {
                                *self.buf = text.to_owned();
                                changed = true;
                                committed = true;

                                ui.memory_mut(|m| m.close_popup());
                            }
                            row += 1;
                        }
                    });
            },
        );

        if changed {
            r.mark_changed();
        }

        DropDownBoxOutput {
            response: r,
            committed,
        }
    }
}

pub struct DropDownBoxOutput {
    pub response: Response,
    pub committed: bool,
}

impl<F: FnMut(&mut Ui, &str) -> Response, V: AsRef<str>, I: Iterator<Item = V>> Widget
    for DropDownBox<'_, F, V, I>
{
    fn ui(mut self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

fn main() -> anyhow::Result<()> {
    // icon source: https://www.iconfinder.com/icons/3688428/
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_icon(icon_data::from_png_bytes(include_bytes!("../../icon.png"))?),
        ..Default::default()
    };

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

    let mut app = MyApp {
        planner,
        recipe_search_text: String::new(),
        auto_focus: true,
        alerts: VecDeque::new(),
        selected_machine: 0,
        item_speed_contraint_item: String::new(),
        old_item_speed_contraint_item: String::new(),
        item_speed_contraint_speed: String::new(),
        machine_count_constraint: String::new(),
        all_recipe_menu_items: Vec::new(),
        snippet_name: String::new(),
        snippet_names,
        saved: false,
        confirm_delete: None,
    };
    app.all_recipe_menu_items = app
        .planner
        .game_data
        .recipes
        .values()
        .flat_map(|recipe| app.recipe_menu_items(recipe))
        .collect();
    eframe::run_native(
        "Factories",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(1.25);
            cc.egui_ctx
                .all_styles_mut(|style| style.spacing.scroll = ScrollStyle::solid());
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .unwrap();
    Ok(())
}

struct MyApp {
    planner: Planner,
    snippet_name: String,
    snippet_names: BTreeSet<String>,
    saved: bool,
    recipe_search_text: String,
    alerts: VecDeque<(String, Instant)>,
    auto_focus: bool,
    selected_machine: usize,
    item_speed_contraint_item: String,
    old_item_speed_contraint_item: String,
    item_speed_contraint_speed: String,
    machine_count_constraint: String,
    all_recipe_menu_items: Vec<String>,
    confirm_delete: Option<String>,
}

const UNTITLED: &str = "Untitled";
fn name_or_untitled(name: &str) -> &str {
    if name.is_empty() {
        UNTITLED
    } else {
        name
    }
}

impl MyApp {
    fn add_recipe(&mut self, text: &str) -> anyhow::Result<()> {
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
            self.planner.snippet.machines[0].count_constraint = Some(1.0);
        }
        self.after_machines_changed();
        Ok(())
    }

    fn recipe_menu_items(&self, recipe: &Recipe) -> Vec<String> {
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

        let crafters = self
            .planner
            .category_to_crafter
            .get(&recipe.category)
            .expect("missing item in category_to_crafter");

        if self.planner.auto_select_crafter(crafters).is_some() {
            vec![recipe.name.clone()]
        } else {
            crafters
                .iter()
                .map(move |crafter| format!("{} @ {}", &recipe.name, crafter))
                .collect()
        }
    }

    fn generate_chart(&self) -> String {
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
                r#"    machine{}{}"{}{}"{}"#,
                index,
                left_bracket,
                if machine.crafter.name == "source" || machine.crafter.name == "sink" {
                    String::new()
                } else {
                    format!("{} × ", rf(machine.crafter_count))
                },
                machine.crafter.name,
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
                        "    machine{}-->|{}/s {}|machine{}",
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

    fn save_chart(&self) -> anyhow::Result<()> {
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

    fn open_chart(&self) -> anyhow::Result<()> {
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

    fn load_snippet(&mut self, name: &str) -> anyhow::Result<()> {
        let snippet = serde_json::from_str::<Snippet>(&fs_err::read_to_string(format!(
            "snippets/{name}.json"
        ))?)?;
        self.planner.snippet = snippet;
        self.snippet_name = name.into();
        self.saved = true;
        Ok(())
    }

    fn save_snippet(&mut self) -> anyhow::Result<()> {
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

    fn show_error<E: Display>(&mut self, result: &Result<(), E>) {
        if let Err(err) = result {
            self.alerts.push_back((err.to_string(), Instant::now()));
            if self.alerts.len() > 5 {
                self.alerts.pop_front();
            }
        }
    }

    fn after_machines_changed(&mut self) {
        let r = self
            .planner
            .auto_refresh()
            .and_then(|()| self.save_snippet());
        self.show_error(&r);
    }

    fn after_constraint_changed(&mut self) {
        let r = self.planner.solve().and_then(|()| self.save_snippet());
        self.show_error(&r);
    }

    fn new_snippet(&mut self) {
        self.snippet_name = String::new();
        self.saved = false;
        self.planner.snippet = Snippet::default();
        self.planner.snippet.solved = true;
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut focus_speed_constraint_input = false;
            let mut focus_machine_constraint_input = false;
            ui.horizontal(|ui| {
                ui.label(if self.planner.snippet.solved {
                    "✔ Solved"
                } else {
                    "🗙 Unsolved"
                });

                ui.label(if self.saved { "✔ Saved" } else { "! Unsaved" });
            });
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Add recipes");
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Add a new recipe:");

                            let drop_down_response = DropDownBox::from_iter(
                                &self.all_recipe_menu_items,
                                "recipe",
                                &mut self.recipe_search_text,
                                |ui, text| ui.selectable_label(false, text),
                            )
                            .show(ui);
                            if self.auto_focus && ui.memory(|m| m.focused()).is_none() {
                                drop_down_response.response.request_focus();
                                self.auto_focus = false;
                            }

                            if drop_down_response.committed {
                                let r = self.add_recipe(&self.recipe_search_text.clone());
                                self.show_error(&r);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Replace source with craft:");
                            let mut text = String::new();
                            ComboBox::new("replace_source_item", "")
                                .selected_text(&text)
                                .show_ui(ui, |ui| {
                                    for machine in &self.planner.snippet.machines {
                                        if machine.crafter.name == "source" {
                                            let item = &machine.recipe.products[0].name;
                                            for recipe in self.planner.game_data.recipes.values() {
                                                if !(recipe.category != "recycling"
                                                    && recipe.category
                                                        != "recycling-or-hand-crafting"
                                                    && recipe
                                                        .products
                                                        .iter()
                                                        .any(|p| &p.name == item))
                                                {
                                                    continue;
                                                }
                                                let more_outputs = if recipe.products.len() > 1 {
                                                    " + ..."
                                                } else {
                                                    ""
                                                };
                                                for menu_item in self.recipe_menu_items(recipe) {
                                                    ui.selectable_value(
                                                        &mut text,
                                                        menu_item.clone(),
                                                        if &recipe.name == item {
                                                            item.to_string()
                                                        } else {
                                                            format!(
                                                                "{} (➡ {}{})",
                                                                menu_item, item, more_outputs
                                                            )
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }
                                });
                            if !text.is_empty() {
                                let r = self.add_recipe(&text);
                                self.show_error(&r);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Replace sink with craft:");
                            let mut text = String::new();
                            ComboBox::new("replace_sink_item", "")
                                .selected_text(&text)
                                .show_ui(ui, |ui| {
                                    for machine in &self.planner.snippet.machines {
                                        if machine.crafter.name == "sink" {
                                            let item = &machine.recipe.ingredients[0].name;
                                            for recipe in self.planner.game_data.recipes.values() {
                                                if !(recipe.category != "recycling"
                                                    && recipe.category
                                                        != "recycling-or-hand-crafting"
                                                    && recipe
                                                        .ingredients
                                                        .iter()
                                                        .any(|p| &p.name == item))
                                                {
                                                    continue;
                                                }
                                                let more_inputs = if recipe.ingredients.len() > 1 {
                                                    " + ..."
                                                } else {
                                                    ""
                                                };
                                                for menu_item in self.recipe_menu_items(recipe) {
                                                    ui.selectable_value(
                                                        &mut text,
                                                        menu_item.clone(),
                                                        format!(
                                                            "({}{}) ➡ {}",
                                                            item, more_inputs, menu_item,
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                });
                            if !text.is_empty() {
                                let r = self.add_recipe(&text);
                                self.show_error(&r);
                            }
                        });
                    });
                });
                ui.vertical(|ui| {
                    ui.heading("Save and load");
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let label_name = ui.label("Snippet name:");
                            let response = TextEdit::singleline(&mut self.snippet_name)
                                .desired_width(150.0)
                                .ui(ui)
                                .labelled_by(label_name.id);
                            if response.changed() {
                                self.saved = false;
                            }
                            if response.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                let r = self.save_snippet();
                                self.show_error(&r);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Load snippet:");
                            let mut text = String::new();
                            ComboBox::new("load_snippet", "")
                                .selected_text(&text)
                                .show_ui(ui, |ui| {
                                    for item in &self.snippet_names {
                                        ui.selectable_value(&mut text, item.clone(), item);
                                    }
                                });
                            if !text.is_empty() {
                                let r = self.load_snippet(&text);
                                self.show_error(&r);
                            }
                        });

                        ui.horizontal(|ui| {
                            if ui.button("🗋 New").clicked() {
                                self.new_snippet();
                            }

                            if ui.button("🗙 Delete").clicked() && !self.snippet_name.is_empty() {
                                self.confirm_delete = Some(self.snippet_name.clone());
                            }
                        });
                        if let Some(name) = &self.confirm_delete {
                            let name = name.clone();
                            ui.horizontal(|ui| {
                                ui.label(format!("Confirm deletion of snippet {name:?}?"));
                                if ui.button("Yes").clicked() {
                                    self.confirm_delete = None;
                                    let snippet_path =
                                        format!("snippets/{}.json", name_or_untitled(&name));
                                    let mermaid_path =
                                        format!("mermaid/{}.html", name_or_untitled(&name));
                                    let r = fs_err::remove_file(snippet_path).and_then(|()| {
                                        if Path::new(&mermaid_path).exists() {
                                            fs_err::remove_file(mermaid_path)
                                        } else {
                                            Ok(())
                                        }
                                    });
                                    self.show_error(&r);
                                    if r.is_ok() {
                                        self.snippet_names.remove(&name);
                                        self.new_snippet();
                                    }
                                }
                                if ui.button("No").clicked() {
                                    self.confirm_delete = None;
                                }
                            });
                        }
                    });
                });
            });

            ui.heading("Machines");
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    if self.planner.snippet.machines.is_empty() {
                        ui.label("No machines.");
                    }
                    let mut index_to_remove = None;
                    for (i, machine) in self.planner.snippet.machines.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(self.selected_machine == i, machine.io_text())
                                .clicked()
                            {
                                self.selected_machine = i;
                            }
                            if machine.crafter.name != "source" && machine.crafter.name != "sink" {
                                if ui.button("🗙").clicked() {
                                    index_to_remove = Some(i);
                                }
                            }
                        });
                    }
                    if let Some(i) = index_to_remove {
                        self.saved = false;
                        self.planner.snippet.solved = false;
                        self.planner.snippet.machines.remove(i);
                        self.after_machines_changed();
                    }
                });
            });
            // ui.horizontal(|ui| {

            // });
            ui.heading("Constraints");

            egui::Frame::group(ui.style()).show(ui, |ui| {
                let mut constraint_to_delete = None;
                let mut any_constraints = false;
                for (item, speed) in &self.planner.snippet.item_speed_constraints {
                    ui.horizontal(|ui| {
                        ui.label(format!("• {}: {}/s", item, speed));
                        if ui.button("Edit").clicked() {
                            self.item_speed_contraint_item = item.clone();
                            self.old_item_speed_contraint_item = item.clone();
                            self.item_speed_contraint_speed = speed.to_string();
                            focus_speed_constraint_input = true;
                        }
                        if ui.button("🗙").clicked() {
                            constraint_to_delete = Some(item.clone());
                        }
                        any_constraints = true;
                    });
                }
                if let Some(item) = constraint_to_delete {
                    self.saved = false;
                    self.planner.snippet.solved = false;
                    self.planner.snippet.item_speed_constraints.remove(&item);
                    self.after_constraint_changed();
                }
                let mut constraint_to_delete2 = None;
                for (i, machine) in self.planner.snippet.machines.iter().enumerate() {
                    if let Some(count) = machine.count_constraint {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "• {} × {}({})",
                                count, machine.crafter.name, machine.recipe.name
                            ));
                            if ui.button("Edit").clicked() {
                                self.selected_machine = i;
                                self.machine_count_constraint = count.to_string();
                                focus_machine_constraint_input = true;
                            }
                            if ui.button("🗙").clicked() {
                                constraint_to_delete2 = Some(i);
                            }
                            any_constraints = true;
                        });
                    }
                }
                if let Some(index) = constraint_to_delete2 {
                    self.saved = false;
                    self.planner.snippet.solved = false;
                    self.planner.snippet.machines[index].count_constraint = None;
                    self.after_constraint_changed();
                }
                if any_constraints {
                    ui.add_space(10.0);
                }

                ui.horizontal(|ui| {
                    ui.label("Add item speed constraint: ");
                    ComboBox::new("constraint_item", "")
                        .selected_text(&self.item_speed_contraint_item)
                        .show_ui(ui, |ui| {
                            for item in self.planner.added_items() {
                                ui.selectable_value(
                                    &mut self.item_speed_contraint_item,
                                    item.clone(),
                                    item,
                                );
                            }
                        });
                    if self.old_item_speed_contraint_item != self.item_speed_contraint_item {
                        self.old_item_speed_contraint_item = self.item_speed_contraint_item.clone();
                        focus_speed_constraint_input = true;
                    }
                    let speed_label = ui.label("Speed: ");
                    let text_response = TextEdit::singleline(&mut self.item_speed_contraint_speed)
                        .desired_width(50.0)
                        .ui(ui)
                        .labelled_by(speed_label.id);
                    if focus_speed_constraint_input {
                        text_response.request_focus();
                    }
                    ui.label("/s");
                    if ui.button("Add").clicked()
                        || (text_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.saved = false;
                        self.planner.snippet.solved = false;
                        let r = self
                            .item_speed_contraint_speed
                            .parse()
                            .map_err(Error::from)
                            .and_then(|speed| {
                                self.planner.add_item_speed_constraint(
                                    &self.item_speed_contraint_item,
                                    speed,
                                )
                            });
                        self.show_error(&r);
                        self.after_constraint_changed();
                    }
                });

                ui.horizontal(|ui| {
                    let label = ui.label("Add selected machine count constraint:");
                    let text_response = TextEdit::singleline(&mut self.machine_count_constraint)
                        .desired_width(50.0)
                        .ui(ui)
                        .labelled_by(label.id);
                    if focus_machine_constraint_input {
                        text_response.request_focus();
                    }
                    if ui.button("Add").clicked()
                        || (text_response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        let r = self.machine_count_constraint.parse();
                        self.show_error(&r.as_ref().map(|_| ()));
                        if let Ok(count) = r {
                            self.saved = false;
                            self.planner.snippet.solved = false;
                            self.planner.snippet.machines[self.selected_machine].count_constraint =
                                Some(count);
                            self.after_constraint_changed();
                        }
                    }
                });
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Open chart").clicked() {
                    let r = self.open_chart();
                    self.show_error(&r);
                }
                if ui.button("Solve again").clicked() {
                    self.after_machines_changed();
                }
            });

            if !self.alerts.is_empty() {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.heading("Logs");
                    if ui.button("Clear (Esc)").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        self.alerts.clear();
                    }
                });
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    for (text, instant) in &self.alerts {
                        ui.colored_label(
                            if instant.elapsed() < Duration::from_secs(5) {
                                ui.ctx().request_repaint();
                                Color32::RED
                            } else {
                                Color32::DARK_RED
                            },
                            text,
                        );
                    }
                });
            }

            // ui.image(egui::include_image!(
            //     "../../../crates/egui/assets/ferris.png"
            // ));
        });
    }
}
