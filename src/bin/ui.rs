#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::collapsible_if)]

use eframe::egui::{self, Color32, ComboBox};
use factories::{game_data::Recipe, prelude::*, rf};

use anyhow::format_err;
use egui::{
    text::{CCursor, CCursorRange},
    Id, Response, ScrollArea, TextEdit, Ui, Widget, WidgetText,
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::{cmp::min, hash::Hash};
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

fn main() -> eframe::Result {
    //env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    let mut app = MyApp {
        planner: init().unwrap(),
        recipe_search_text: String::new(),
        auto_focus: true,
        alerts: Vec::new(),
        selected_machine: 0,
        item_speed_contraint_item: String::new(),
        item_speed_contraint_speed: String::new(),
        machine_count_constraint: String::new(),
        all_recipe_menu_items: Vec::new(),
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
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

struct MyApp {
    planner: Planner,
    recipe_search_text: String,
    alerts: Vec<String>,
    auto_focus: bool,
    selected_machine: usize,
    item_speed_contraint_item: String,
    item_speed_contraint_speed: String,
    machine_count_constraint: String,
    all_recipe_menu_items: Vec<String>,
}

impl MyApp {
    fn add_recipe(&mut self, text: &str) {
        let add_auto_constraint =
            self.planner.machines.is_empty() && self.planner.item_speed_constraints.is_empty();

        let mut r = if let Some((recipe, machine)) = text.split_once(" @ ") {
            self.planner.create_machine_with_crafter(recipe, machine)
        } else {
            self.planner.create_machine(text)
        };
        if add_auto_constraint {
            self.planner.machines[0].count_constraint = Some(1.0);
        }
        r = r.and_then(|()| self.planner.auto_refresh());
        match r {
            Err(err) => {
                self.alerts.push(err.to_string());
            }
            Ok(()) => {
                self.recipe_search_text.clear();
            }
        }
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
        let mut out = format!(
            "--- \n\
            title: {}\n\
            --- \n\
            flowchart TD \n",
            "Untitled Factory"
        );
        for (index, machine) in self.planner.machines.iter().enumerate() {
            writeln!(
                out,
                r#"    machine{}["{}{}"]"#,
                index,
                if machine.crafter.name == "source" || machine.crafter.name == "sink" {
                    String::new()
                } else {
                    format!("{} × ", rf(machine.crafter_count))
                },
                machine.crafter.name
            )
            .unwrap();
        }

        let all_items = self.planner.added_items();
        for item in all_items {
            let sources = self
                .planner
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
                            item, remaining_speed
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

    fn open_chart(&self) -> anyhow::Result<()> {
        let chart = self.generate_chart();

        let template = include_str!("../../mermaid.html");
        let html = template.replacen("$1", &chart, 1);
        fs_err::create_dir_all("mermaid")?;
        let file_path = std::env::current_dir()?.join("mermaid/file1.html");
        fs_err::write(&file_path, html)?;
        let url = Url::from_file_path(file_path)
            .map_err(|()| format_err!("Url::from_file_path failed"))?;
        open::that(url.as_str())?;
        Ok(())
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.alerts.clear();
            }
            for text in &self.alerts {
                ui.colored_label(Color32::RED, text);
            }

            ui.horizontal(|ui| {
                ui.label("Add a recipe:");

                let r = DropDownBox::from_iter(
                    &self.all_recipe_menu_items,
                    "recipe",
                    &mut self.recipe_search_text,
                    |ui, text| ui.selectable_label(false, text),
                )
                .show(ui);
                if self.auto_focus && ui.memory(|m| m.focused()).is_none() {
                    r.response.request_focus();
                    self.auto_focus = false;
                }

                if r.committed {
                    self.add_recipe(&self.recipe_search_text.clone());
                }
            });
            ui.horizontal(|ui| {
                ui.label("Replace source with craft:");
                let mut text = String::new();
                ComboBox::new("replace_source_item", "")
                    .selected_text(&text)
                    .show_ui(ui, |ui| {
                        for machine in &self.planner.machines {
                            if machine.crafter.name == "source" {
                                let item = &machine.recipe.products[0].name;
                                let recipes = self
                                    .planner
                                    .game_data
                                    .recipes
                                    .values()
                                    .filter(|recipe| {
                                        recipe.category != "recycling"
                                            && recipe.category != "recycling-or-hand-crafting"
                                            && recipe.products.iter().any(|p| &p.name == item)
                                    })
                                    .flat_map(|recipe| self.recipe_menu_items(recipe));
                                for recipe in recipes {
                                    ui.selectable_value(&mut text, recipe.clone(), &recipe);
                                }
                            }
                        }
                    });
                if !text.is_empty() {
                    self.add_recipe(&text);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Replace sink with craft:");
                let mut text = String::new();
                ComboBox::new("replace_sink_item", "")
                    .selected_text(&text)
                    .show_ui(ui, |ui| {
                        for machine in &self.planner.machines {
                            if machine.crafter.name == "sink" {
                                let item = &machine.recipe.ingredients[0].name;
                                let recipes = self
                                    .planner
                                    .game_data
                                    .recipes
                                    .values()
                                    .filter(|recipe| {
                                        recipe.category != "recycling"
                                            && recipe.category != "recycling-or-hand-crafting"
                                            && recipe.ingredients.iter().any(|p| &p.name == item)
                                    })
                                    .flat_map(|recipe| self.recipe_menu_items(recipe));
                                for recipe in recipes {
                                    ui.selectable_value(&mut text, recipe.clone(), &recipe);
                                }
                            }
                        }
                    });
                if !text.is_empty() {
                    self.add_recipe(&text);
                }
            });

            ui.heading("Machines");
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for (i, machine) in self.planner.machines.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_machine == i, machine.io_text())
                            .clicked()
                        {
                            self.selected_machine = i;
                        }
                    }
                });
            });
            ui.label(if self.planner.solved {
                "✔ Solved"
            } else {
                "🗙 Unsolved"
            });
            ui.horizontal(|ui| {
                if self.selected_machine < self.planner.machines.len() {
                    let machine = &self.planner.machines[self.selected_machine];
                    if machine.crafter.name != "source" && machine.crafter.name != "sink" {
                        if ui.button("Remove").clicked() {
                            self.planner.machines.remove(self.selected_machine);
                            if let Err(err) = self.planner.auto_refresh() {
                                self.alerts.push(err.to_string());
                            }
                        }
                    }
                }
            });
            ui.heading("Constraints");

            egui::Frame::group(ui.style()).show(ui, |ui| {
                let mut constraint_to_delete = None;
                for (item, speed) in &self.planner.item_speed_constraints {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: {}/s", item, speed));
                        if ui.button("X").clicked() {
                            constraint_to_delete = Some(item.clone());
                        }
                    });
                }
                if let Some(item) = constraint_to_delete {
                    self.planner.item_speed_constraints.remove(&item);
                    if let Err(err) = self.planner.solve() {
                        self.alerts.push(err.to_string());
                    }
                }
                let mut constraint_to_delete2 = None;
                for (i, machine) in self.planner.machines.iter().enumerate() {
                    if let Some(count) = machine.count_constraint {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "{} × {}({})",
                                count, machine.crafter.name, machine.recipe.name
                            ));
                            if ui.button("X").clicked() {
                                constraint_to_delete2 = Some(i);
                            }
                        });
                    }
                }
                if let Some(index) = constraint_to_delete2 {
                    self.planner.machines[index].count_constraint = None;
                    if let Err(err) = self.planner.solve() {
                        self.alerts.push(err.to_string());
                    }
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
                    let speed_label = ui.label("Speed: ");
                    TextEdit::singleline(&mut self.item_speed_contraint_speed)
                        .desired_width(50.0)
                        .ui(ui)
                        .labelled_by(speed_label.id);
                    ui.label("/s");
                    if ui.button("Add").clicked() {
                        let r = self
                            .item_speed_contraint_speed
                            .parse()
                            .map_err(Error::from)
                            .and_then(|speed| {
                                self.planner.add_item_speed_constraint(
                                    &self.item_speed_contraint_item,
                                    speed,
                                )
                            })
                            .and_then(|()| self.planner.solve());
                        if let Err(err) = r {
                            self.alerts.push(err.to_string());
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let label = ui.label("Add selected machine count constraint:");
                    TextEdit::singleline(&mut self.machine_count_constraint)
                        .desired_width(50.0)
                        .ui(ui)
                        .labelled_by(label.id);
                    if ui.button("Add").clicked() {
                        let r = self
                            .machine_count_constraint
                            .parse()
                            .map_err(Error::from)
                            .and_then(|count| {
                                self.planner.machines[self.selected_machine].count_constraint =
                                    Some(count);
                                self.planner.solve()
                            });
                        if let Err(err) = r {
                            self.alerts.push(err.to_string());
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                if ui.button("Chart").clicked() {
                    if let Err(err) = self.open_chart() {
                        self.alerts.push(err.to_string());
                    }
                }
            });

            // egui::ComboBox::from_label("Select one!")
            //     .selected_text(format!("{:?}", &mut self.search_text))
            //     .show_ui(ui, |ui| {
            //         ui.selectable_value(&mut self.search_text, "First".to_string(), "First");
            //         ui.selectable_value(&mut self.search_text, "Second".to_string(), "Second");
            //         ui.selectable_value(&mut self.search_text, "Third".to_string(), "Third");
            //     });

            // ui.heading("My egui Application");
            // ui.horizontal(|ui| {
            //     let name_label = ui.label("Your name: ");
            //     ui.text_edit_singleline(&mut self.name)
            //         .labelled_by(name_label.id);
            // });
            // ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            // if ui.button("Increment").clicked() {
            //     self.age += 1;
            // }
            // ui.label(format!("Hello '{}', age {}", self.name, self.age));

            // ui.image(egui::include_image!(
            //     "../../../crates/egui/assets/ferris.png"
            // ));
        });
    }
}
