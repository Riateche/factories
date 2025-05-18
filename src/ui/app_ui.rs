use {
    super::{
        app::{recipe_menu_items, MyApp, RecipeMenuItem},
        drop_down::DropDownBox,
        ui_ext::UiExt,
    },
    crate::{machine::Module, rf, snippet::SnippetMachine, ResultExtOrWarn},
    eframe::egui::{self, Color32, ComboBox, Key},
    egui::{Response, ScrollArea, TextEdit, Ui, Widget},
    itertools::Itertools,
    std::{
        collections::BTreeMap,
        time::{Duration, Instant},
    },
};

impl MyApp {
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let mut focus_speed_constraint_input = false;

        while let Ok(msg) = self.msg_receiver.try_recv() {
            self.alerts.push_back((msg, Instant::now()));
            if self.alerts.len() > 5 {
                self.alerts.pop_front();
            }
        }

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
                                )
                                .min_scrolled_height(
                                    ui.input(|input| {
                                        input
                                            .viewport()
                                            .inner_rect
                                            .map_or(0., |rect| rect.height() - 100.)
                                    }) - ui.next_widget_position().y,
                                )
                                .show(ui);
                                if self.auto_focus && ui.memory(|m| m.focused()).is_none() {
                                    drop_down_response.response.request_focus();
                                    self.auto_focus = false;
                                }

                                if let Some(item) = drop_down_response.option_selected.cloned() {
                                    self.add_crafter(item.recipe(), item.crafter()).or_warn();
                                } else if drop_down_response.enter_pressed {
                                    self.add_crafter(&self.recipe_search_text.clone(), None)
                                        .or_warn();
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.heading("");
                            ui.label(if self.editor.solved() {
                                "✔ Solved"
                            } else {
                                "🗙 Unsolved"
                            });

                            ui.label(if self.saved { "✔ Saved" } else { "! Unsaved" });
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
                                    self.save_snippet().or_warn();
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
                                    self.load_snippet(&text).or_warn();
                                }
                            });

                            ui.horizontal(|ui| {
                                if ui.button("📥 Save").clicked() {
                                    self.save_snippet().or_warn();
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
                                        self.delete_snippet(&name).or_warn();
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
                //let show_names = ui.input(|i| i.modifiers.alt);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    if self.editor.machines().is_empty() {
                        ui.label("No machines.");
                    }
                    let mut index_to_remove = None;
                    let mut recipe_to_add: Option<(String, Option<String>)> = None;
                    for (i, editor_machine) in self.editor.machines().iter().enumerate() {
                        let machine = editor_machine.machine();
                        ui.horizontal(|ui| {
                            let item_speeds = machine.item_speeds().collect_vec();
                            let mut is_first = true;
                            for stack in &item_speeds {
                                if stack.speed < 0.0 {
                                    ui.rich_label(format!(
                                        "{}{}/s @[{}:]",
                                        if is_first { "" } else { "+ " },
                                        rf(-stack.speed),
                                        stack.item,
                                    ));
                                    is_first = false;
                                }
                            }
                            let crafter_count = if machine.crafter.is_source_or_sink() {
                                String::new()
                            } else {
                                format!("{} × ", rf(machine.crafter_count))
                            };
                            let tooltip = if (machine.recipe.products.len() == 1
                                && machine.recipe.name == machine.recipe.products[0].name)
                                || machine.crafter.is_source_or_sink()
                            {
                                machine.crafter.name.clone()
                            } else {
                                format!("{}({})", machine.crafter.name, machine.recipe.name)
                            };
                            let modules_text = if machine.modules.is_empty()
                                && machine.beacons.is_empty()
                            {
                                String::new()
                            } else {
                                let beacon_text = if machine.beacons.is_empty() {
                                    String::new()
                                } else {
                                    if machine.beacons.iter().all_equal() {
                                        let modules = module_counts(&machine.beacons[0])
                                            .into_iter()
                                            .map(|(name, count)| format!("{count} × {name}"))
                                            .join(",");
                                        format!("{} × beacon({})", machine.beacons.len(), modules)
                                    } else {
                                        machine
                                            .beacons
                                            .iter()
                                            .map(|beacon| {
                                                let modules = module_counts(beacon)
                                                    .into_iter()
                                                    .map(|(name, count)| {
                                                        format!("{count} × {name}")
                                                    })
                                                    .join(",");
                                                format!("beacon({})", modules)
                                            })
                                            .join("\n")
                                    }
                                };
                                let beacon_markup = if machine.beacons.is_empty() {
                                    None
                                } else {
                                    Some(format!(
                                        "{}@[beacon:{}]",
                                        machine.beacons.len(),
                                        beacon_text
                                    ))
                                };
                                let text = module_counts(&machine.modules)
                                    .into_iter()
                                    .map(|(name, count)| format!("{count}@[{name}:]"))
                                    .chain(beacon_markup)
                                    .join(",");
                                format!("[{text}]")
                            };
                            ui.rich_label(format!(
                                "{}{}@[{}:{}]{}",
                                if is_first { "" } else { "➡ " },
                                crafter_count,
                                machine.crafter.name,
                                tooltip,
                                modules_text
                            ));
                            is_first = true;
                            for stack in &item_speeds {
                                if stack.speed > 0.0 {
                                    ui.rich_label(format!(
                                        "{}{}/s @[{}:]",
                                        if is_first { "➡ " } else { "+ " },
                                        rf(stack.speed),
                                        stack.item,
                                    ));
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
                            if machine.crafter.is_source_or_sink() {
                                let r = ui.with_tooltip("Replace with a crafting machine", |ui| {
                                    ui.button("Craft")
                                });
                                if r.clicked() {
                                    if self.replace_with_craft_index == Some(i) {
                                        self.replace_with_craft_index = None;
                                    } else {
                                        let item = if machine.crafter.is_source() {
                                            &machine.recipe.products[0].name
                                        } else {
                                            &machine.recipe.ingredients[0].name
                                        };
                                        let mut menu_items_and_hints = Vec::new();
                                        for recipe in self.editor.info().game_data.recipes.values()
                                        {
                                            let can_replace = if machine.crafter.is_source() {
                                                recipe.products.iter().any(|p| &p.name == item)
                                            } else {
                                                recipe.ingredients.iter().any(|p| &p.name == item)
                                            };
                                            if !can_replace {
                                                continue;
                                            }
                                            let hint = if machine.crafter.is_source() {
                                                format!(
                                                    "({} ➡) ",
                                                    recipe
                                                        .ingredients
                                                        .iter()
                                                        .map(|i| &i.name)
                                                        .join(" + ")
                                                )
                                            } else {
                                                if recipe.products.len() == 1
                                                    && recipe.products[0].name == recipe.name
                                                {
                                                    String::new()
                                                } else {
                                                    format!(
                                                        " (➡ {})",
                                                        recipe
                                                            .products
                                                            .iter()
                                                            .map(|i| &i.name)
                                                            .join(" + ")
                                                    )
                                                }
                                            };
                                            for menu_item in
                                                recipe_menu_items(self.editor.info(), recipe)
                                            {
                                                menu_items_and_hints
                                                    .push((menu_item, hint.clone()));
                                            }
                                        }
                                        let show_hints = !menu_items_and_hints
                                            .iter()
                                            .map(|(_, hint)| hint)
                                            .all_equal();
                                        self.replace_with_craft_options = menu_items_and_hints
                                            .into_iter()
                                            .map(|(menu_item, hint)| {
                                                let menu_text = menu_item.text();
                                                let text = if show_hints {
                                                    if machine.crafter.is_source() {
                                                        format!("{hint}{menu_text}")
                                                    } else {
                                                        format!("{menu_text}{hint}")
                                                    }
                                                } else {
                                                    menu_text.to_string()
                                                };
                                                (menu_item, text)
                                            })
                                            .collect();

                                        self.generation += 1;
                                        if self.replace_with_craft_options.len() == 1 {
                                            self.replace_with_craft_index = None;
                                            let item = self.replace_with_craft_options.remove(0).0;
                                            recipe_to_add = Some((
                                                item.recipe().to_string(),
                                                item.crafter().map(|s| s.to_string()),
                                            ));
                                        } else {
                                            self.replace_with_craft_index = Some(i);
                                        }
                                    }
                                }
                                if self.replace_with_craft_index == Some(i) {
                                    let mut value: Option<&RecipeMenuItem> = None;
                                    ComboBox::new(("replace_source_item", self.generation), "")
                                        .show_ui(ui, |ui| {
                                            for (menu_item, item_text) in
                                                &self.replace_with_craft_options
                                            {
                                                ui.selectable_value(
                                                    &mut value,
                                                    Some(menu_item),
                                                    item_text,
                                                );
                                            }
                                        });
                                    if let Some(value) = value {
                                        recipe_to_add = Some((
                                            value.recipe().to_string(),
                                            value.crafter().map(|s| s.to_string()),
                                        ));
                                        self.replace_with_craft_index = None;
                                    }
                                }
                            } else {
                                // not source or sink
                                let r = ui.button("Edit");
                                if r.clicked() {
                                    self.edit_machine_index = Some(i);
                                    self.machine_count_constraint = match editor_machine.snippet() {
                                        SnippetMachine::Source { .. }
                                        | SnippetMachine::Sink { .. } => {
                                            unreachable!()
                                        }
                                        SnippetMachine::Crafter {
                                            count_constraint, ..
                                        } => count_constraint
                                            .map(|c| c.to_string())
                                            .unwrap_or_default(),
                                    };
                                    self.num_beacons = machine.beacons.len().to_string();
                                    self.focus_machine_constraint_input = true;
                                }

                                if ui.button("🗙").clicked() {
                                    index_to_remove = Some(i);
                                }
                            }
                        });
                    }
                    if let Some(i) = index_to_remove {
                        self.saved = false;
                        self.alerts.clear();
                        self.editor.remove_machine(i).or_warn();
                        self.after_machines_changed();
                    }
                    if let Some((recipe, crafter)) = recipe_to_add {
                        self.add_crafter(&recipe, crafter.as_deref()).or_warn();
                    }
                });

                if let Some(i) = self.edit_machine_index {
                    if i < self.editor.machines().len() {
                        ui.horizontal(|ui| {
                            ui.rich_label(format!(
                                "Edit machine: @[{}]*(@[{}]*)",
                                self.editor.machines()[i].machine().crafter.name,
                                &self.editor.machines()[i].machine().recipe.name,
                            ));
                        });

                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let label = ui.label("Set machine count constraint:");
                                let text_response =
                                    TextEdit::singleline(&mut self.machine_count_constraint)
                                        .desired_width(50.0)
                                        .ui(ui)
                                        .labelled_by(label.id);
                                if self.focus_machine_constraint_input {
                                    text_response.request_focus();
                                    self.focus_machine_constraint_input = false;
                                }
                                if ui.button("Add").clicked()
                                    || (text_response.lost_focus()
                                        && ui.input(|i| i.key_pressed(Key::Enter)))
                                {
                                    let count = self.machine_count_constraint.parse().or_warn();
                                    if let Some(count) = count {
                                        self.saved = false;
                                        self.alerts.clear();
                                        self.editor
                                            .set_machine_count_constraint(i, Some(count))
                                            .or_warn();
                                        self.after_constraint_changed();
                                    }
                                }
                            });
                            if self.editor.machines()[i]
                                .machine()
                                .crafter
                                .module_inventory_size
                                > 0
                            {
                                let num_empty_module_slots = self.editor.machines()[i]
                                    .machine()
                                    .crafter
                                    .module_inventory_size
                                    .saturating_sub(
                                        self.editor.machines()[i].machine().modules.len() as u64,
                                    );
                                ui.horizontal(|ui| {
                                    ui.label("Modules:");
                                    let mut index_to_remove = None;
                                    for (ii, module) in self.editor.machines()[i]
                                        .machine()
                                        .modules
                                        .iter()
                                        .enumerate()
                                    {
                                        if ui.rich_label(format!("@[{}:]", module.name)).clicked() {
                                            index_to_remove = Some(ii);
                                        }
                                    }
                                    for _ in 0..(num_empty_module_slots) {
                                        ui.with_tooltip("Empty module slot", |ui| ui.label("🚫"));
                                    }
                                    if !self.editor.machines()[i].machine().modules.is_empty() {
                                        ui.label("(Click on module to remove it)");
                                    }

                                    if let Some(ii) = index_to_remove {
                                        self.saved = false;
                                        self.alerts.clear();
                                        let r = self.editor.remove_module(i, ii).or_warn();
                                        if r.is_some() {
                                            self.after_machines_changed();
                                        }
                                    }
                                });

                                if num_empty_module_slots > 0 {
                                    ui.horizontal(|ui| {
                                        ui.label("Add module:");
                                        let mut added = false;
                                        let mut allowed_modules = vec![&self.default_speed_module];
                                        if self.editor.machines()[i]
                                            .machine()
                                            .recipe
                                            .allowed_effects
                                            .productivity
                                        {
                                            allowed_modules.push(&self.default_productivity_module);
                                        }
                                        for module in allowed_modules {
                                            if ui
                                                .rich_label(format!("@[{}:]", module.name))
                                                .clicked()
                                            {
                                                let num_added = if ui.input(|i| i.modifiers.shift) {
                                                    num_empty_module_slots
                                                } else {
                                                    1
                                                };
                                                for _ in 0..num_added {
                                                    self.saved = false;
                                                    self.alerts.clear();
                                                    self.editor
                                                        .add_module(i, &module.name)
                                                        .or_warn();
                                                    added = true;
                                                }
                                            }
                                        }
                                        if added {
                                            self.after_machines_changed();
                                        }
                                    });
                                }
                                ui.horizontal(|ui| {
                                    let label = ui.label(
                                        "Beacons with speed modules connected to each machine: ",
                                    );
                                    let text_response = TextEdit::singleline(&mut self.num_beacons)
                                        .desired_width(50.0)
                                        .ui(ui)
                                        .labelled_by(label.id);
                                    if ui.button("Set").clicked()
                                        || (text_response.lost_focus()
                                            && ui.input(|i| i.key_pressed(Key::Enter)))
                                    {
                                        if let Some(num_beacons) =
                                            self.num_beacons.parse::<u32>().or_warn()
                                        {
                                            self.saved = false;
                                            self.alerts.clear();
                                            self.editor
                                                .set_beacons(
                                                    i,
                                                    (0..num_beacons)
                                                        .map(|_| {
                                                            (0..2)
                                                                .map(|_| {
                                                                    self.default_speed_module
                                                                        .clone()
                                                                })
                                                                .collect_vec()
                                                        })
                                                        .collect(),
                                                )
                                                .or_warn();
                                            self.after_machines_changed();
                                        }
                                    }
                                });
                            }

                            if ui.button("Cancel").clicked() {
                                self.edit_machine_index = None;
                            }
                        });
                    }
                }

                ui.heading("Constraints");
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    let mut constraint_to_delete = None;
                    let mut any_constraints = false;
                    for (item, speed) in self.editor.item_speed_constraints() {
                        ui.horizontal(|ui| {
                            ui.rich_label(format!(
                                "@[$lock:Item speed constraint] @[{}]*: {}/s",
                                item, speed
                            ));
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
                        self.alerts.clear();
                        self.editor
                            .set_item_speed_constraint(&item, None, false)
                            .or_warn();
                        self.after_constraint_changed();
                    }
                    let mut constraint_to_delete2 = None;
                    for (i, machine) in self.editor.machines().iter().enumerate() {
                        if let SnippetMachine::Crafter {
                            count_constraint: Some(count),
                            ..
                        } = machine.snippet()
                        {
                            ui.horizontal(|ui| {
                                ui.rich_label(&format!(
                                    "@[$lock:Machine count constraint] {} × @[{}]*(@[{}]*)",
                                    count,
                                    machine.machine().crafter.name,
                                    machine.machine().recipe.name,
                                ));
                                if ui.button("Edit").clicked() {
                                    self.edit_machine_index = Some(i);
                                    self.machine_count_constraint = count.to_string();
                                    self.num_beacons = machine.machine().beacons.len().to_string();
                                    self.focus_machine_constraint_input = true;
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
                        self.alerts.clear();
                        self.editor
                            .set_machine_count_constraint(index, None)
                            .or_warn();
                        self.after_constraint_changed();
                    }
                    if any_constraints {
                        ui.add_space(10.0);
                    }

                    ui.horizontal(|ui| {
                        ui.label("Set item speed constraint: ");
                        ComboBox::new(("constraint_item", self.generation), "")
                            .selected_text(&self.item_speed_contraint_item)
                            .show_ui(ui, |ui| {
                                for item in self.editor.added_items() {
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
                        let set = ui.button("Set").clicked();
                        let replace_all = ui.button("Replace all").clicked();
                        if set
                            || replace_all
                            || (text_response.lost_focus()
                                && ui.input(|i| i.key_pressed(Key::Enter)))
                        {
                            self.saved = false;
                            if let Some(speed) = self.item_speed_contraint_speed.parse().or_warn() {
                                self.alerts.clear();
                                self.editor
                                    .set_item_speed_constraint(
                                        &self.item_speed_contraint_item,
                                        Some(speed),
                                        replace_all,
                                    )
                                    .or_warn();
                                self.after_constraint_changed();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tip:");
                        for (speed, item) in &self.belt_speeds {
                            ui.rich_label(format!("@[{item}:] = {speed}/s    "));
                        }
                    });
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Open chart").clicked() {
                        self.open_chart().or_warn();
                    }
                    if ui.button("Solve again").clicked() {
                        self.alerts.clear();
                        self.after_machines_changed();
                    }
                    if ui.button("Copy description").clicked() {
                        self.copy_description().or_warn();
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

fn module_counts(modules: &[Module]) -> BTreeMap<&str, usize> {
    let mut module_counts = BTreeMap::<_, usize>::new();
    for module in modules {
        *module_counts.entry(module.name.as_str()).or_default() += 1;
    }
    module_counts
}
