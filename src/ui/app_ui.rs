use {
    super::{
        app::{name_or_untitled, MyApp},
        drop_down::DropDownBox,
    },
    crate::rf,
    eframe::egui::{self, Color32, ComboBox, Key},
    egui::{Response, ScrollArea, TextEdit, Ui, Widget},
    std::{env, path::Path, time::Duration},
    url::Url,
};

fn icon_url(name: &str) -> String {
    let path = env::current_dir()
        .unwrap()
        .join(format!("icons/{name}.png"));
    Url::from_file_path(&path).unwrap().to_string()
}

impl MyApp {
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let mut focus_speed_constraint_input = false;
        let mut focus_machine_constraint_input = false;

        ScrollArea::vertical()
            .show(ui, |ui| {
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
                                    |ui, text| {
                                        let mut r = None;
                                        let out_r = ui.horizontal(|ui| {
                                            let recipe = text
                                                .split_once(" @ ")
                                                .map_or(text, |parts| parts.0);
                                            ui.image(icon_url(&recipe));
                                            r = Some(ui.selectable_label(false, text));
                                        });
                                        r.unwrap_or(out_r.response)
                                    },
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
                                ComboBox::new(("replace_source_item", self.generation), "")
                                    .selected_text(&text)
                                    .show_ui(ui, |ui| {
                                        for machine in &self.planner.snippet.machines {
                                            if machine.crafter.name == "source" {
                                                let item = &machine.recipe.products[0].name;
                                                for recipe in
                                                    self.planner.game_data.recipes.values()
                                                {
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
                                                    let more_outputs = if recipe.products.len() > 1
                                                    {
                                                        " + ..."
                                                    } else {
                                                        ""
                                                    };
                                                    for menu_item in self.recipe_menu_items(recipe)
                                                    {
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
                                ComboBox::new(("replace_sink_item", self.generation), "")
                                    .selected_text(&text)
                                    .show_ui(ui, |ui| {
                                        for machine in &self.planner.snippet.machines {
                                            if machine.crafter.name == "sink" {
                                                let item = &machine.recipe.ingredients[0].name;
                                                for recipe in
                                                    self.planner.game_data.recipes.values()
                                                {
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
                                                    let more_inputs =
                                                        if recipe.ingredients.len() > 1 {
                                                            " + ..."
                                                        } else {
                                                            ""
                                                        };
                                                    for menu_item in self.recipe_menu_items(recipe)
                                                    {
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
                                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                                {
                                    let r = self.save_snippet();
                                    self.show_error(&r);
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label("Load snippet:");
                                let mut text = String::new();
                                ComboBox::new(("load_snippet", self.generation), "")
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
                                if ui.button("📥 Save").clicked() {
                                    let r = self.save_snippet();
                                    self.show_error(&r);
                                }

                                if ui.button("🗋 New").clicked() {
                                    self.new_snippet();
                                }

                                if ui.button("🗙 Delete").clicked() && !self.snippet_name.is_empty()
                                {
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
                    ui.vertical(|ui| {
                        ui.heading("");
                        ui.label(if self.planner.snippet.solved {
                            "✔ Solved"
                        } else {
                            "🗙 Unsolved"
                        });

                        ui.label(if self.saved { "✔ Saved" } else { "! Unsaved" });
                    });
                });

                ui.heading("Machines");
                let show_names = ui.input(|i| i.modifiers.alt);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    if self.planner.snippet.machines.is_empty() {
                        ui.label("No machines.");
                    }
                    let mut index_to_remove = None;
                    for (i, machine) in self.planner.snippet.machines.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let item_speeds = machine.item_speeds();
                            let mut is_first = true;
                            for stack in &item_speeds {
                                if stack.speed < 0.0 {
                                    ui.label(format!(
                                        "{}{}/s",
                                        if is_first { "" } else { "+ " },
                                        rf(-stack.speed)
                                    ));
                                    let r = ui.image(icon_url(&stack.item));
                                    if r.contains_pointer() {
                                        egui::show_tooltip(
                                            ui.ctx(),
                                            ui.layer_id(),
                                            egui::Id::new(&stack.item),
                                            |ui| {
                                                ui.label(&stack.item);
                                            },
                                        );
                                    }
                                    if show_names {
                                        ui.label(&stack.item);
                                    }
                                    is_first = false;
                                }
                            }
                            let crafter_count = if machine.crafter.name == "source"
                                || machine.crafter.name == "sink"
                            {
                                String::new()
                            } else {
                                format!("{} ×", rf(machine.crafter_count))
                            };
                            ui.label(format!(
                                "{}{}",
                                if is_first { "" } else { "➡ " },
                                crafter_count,
                            ));
                            let r = ui.image(icon_url(&machine.crafter.name));
                            if r.contains_pointer() {
                                let tooltip = if (machine.recipe.products.len() == 1
                                    && machine.recipe.name == machine.recipe.products[0].name)
                                    || machine.crafter.name == "source"
                                    || machine.crafter.name == "sink"
                                {
                                    machine.crafter.name.clone()
                                } else {
                                    format!("{}({})", machine.crafter.name, machine.recipe.name)
                                };
                                egui::show_tooltip(
                                    ui.ctx(),
                                    ui.layer_id(),
                                    egui::Id::new(&tooltip),
                                    |ui| {
                                        ui.label(&tooltip);
                                    },
                                );
                            }
                            if show_names {
                                ui.label(&machine.crafter.name);
                            }
                            is_first = true;
                            for stack in &item_speeds {
                                if stack.speed > 0.0 {
                                    ui.label(format!(
                                        "{}{}/s",
                                        if is_first { "➡ " } else { "+ " },
                                        rf(stack.speed)
                                    ));
                                    let r = ui.image(icon_url(&stack.item));
                                    if r.contains_pointer() {
                                        egui::show_tooltip(
                                            ui.ctx(),
                                            ui.layer_id(),
                                            egui::Id::new(&stack.item),
                                            |ui| {
                                                ui.label(&stack.item);
                                            },
                                        );
                                    }
                                    if show_names {
                                        ui.label(&stack.item);
                                    }
                                    is_first = false;
                                }
                            }

                            // if ui
                            //     .selectable_label(self.selected_machine == i, machine.io_text())
                            //     .clicked()
                            // {
                            //     self.selected_machine = i;
                            // }
                            ui.add_space(10.0);
                            if machine.crafter.name != "source" && machine.crafter.name != "sink" {
                                let r = ui.button("C");
                                if r.contains_pointer() {
                                    egui::show_tooltip(
                                        ui.ctx(),
                                        ui.layer_id(),
                                        egui::Id::new("Add machine count constraint"),
                                        |ui| {
                                            ui.label("Add machine count constraint");
                                        },
                                    );
                                }
                                if r.clicked() {
                                    self.add_machine_count_constraint_index = Some(i);
                                    self.machine_count_constraint = machine
                                        .count_constraint
                                        .map(|c| c.to_string())
                                        .unwrap_or_default();
                                    focus_machine_constraint_input = true;
                                }
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
                                    self.add_machine_count_constraint_index = Some(i);
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
                        ComboBox::new(("constraint_item", self.generation), "")
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
                            self.old_item_speed_contraint_item =
                                self.item_speed_contraint_item.clone();
                            focus_speed_constraint_input = true;
                        }
                        let speed_label = ui.label("Speed: ");
                        let text_response =
                            TextEdit::singleline(&mut self.item_speed_contraint_speed)
                                .desired_width(50.0)
                                .ui(ui)
                                .labelled_by(speed_label.id);
                        if focus_speed_constraint_input {
                            text_response.request_focus();
                        }
                        ui.label("/s");
                        if ui.button("Add").clicked()
                            || (text_response.lost_focus()
                                && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            self.saved = false;
                            self.planner.snippet.solved = false;
                            let r = self
                                .item_speed_contraint_speed
                                .parse()
                                .map_err(anyhow::Error::from)
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

                    if let Some(i) = self.add_machine_count_constraint_index {
                        if i < self.planner.snippet.machines.len() {
                            ui.horizontal(|ui| {
                                let label = ui.label(format!(
                                    "Add machine count constraint for {}({}):",
                                    self.planner.snippet.machines[i].crafter.name,
                                    self.planner.snippet.machines[i].recipe.name
                                ));
                                let text_response =
                                    TextEdit::singleline(&mut self.machine_count_constraint)
                                        .desired_width(50.0)
                                        .ui(ui)
                                        .labelled_by(label.id);
                                if focus_machine_constraint_input {
                                    text_response.request_focus();
                                }
                                if ui.button("Add").clicked()
                                    || (text_response.lost_focus()
                                        && ui.input(|i| i.key_pressed(Key::Enter)))
                                {
                                    let r = self.machine_count_constraint.parse();
                                    self.show_error(&r.as_ref().map(|_| ()));
                                    if let Ok(count) = r {
                                        self.saved = false;
                                        self.planner.snippet.solved = false;
                                        self.planner.snippet.machines[i].count_constraint =
                                            Some(count);
                                        self.add_machine_count_constraint_index = None;
                                        self.after_constraint_changed();
                                    }
                                }
                            });
                        }
                    }
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
                            || ui.input(|i| i.key_pressed(Key::Escape))
                        {
                            self.alerts.clear();
                        }
                    });
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        for (text, instant) in &self.alerts {
                            let is_recent = instant.elapsed() < Duration::from_secs(5);
                            if is_recent {
                                ui.ctx().request_repaint();
                            }
                            let response = ui.colored_label(
                                if is_recent {
                                    Color32::RED
                                } else {
                                    Color32::DARK_RED
                                },
                                text,
                            );
                            if instant.elapsed() < Duration::from_millis(500) {
                                response.scroll_to_me(None);
                                ui.ctx().request_repaint();
                            }
                        }
                    });
                }

                ui.response()

                // ui.image(egui::include_image!(
                //     "../../../crates/egui/assets/ferris.png"
                // ));
            })
            .inner
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show(ui);
        });
    }
}
