use {
    super::{
        app::{recipe_menu_items, MyApp, RecipeMenuItem},
        drop_down::DropDownBox,
        ui_ext::UiExt,
    },
    crate::{
        machine::{Beacon, ModuleType},
        module_counts,
        primitives::{CrafterName, ItemNameAndQuality, ModuleName, Quality, RecipeName},
        rf, ResultExtOrWarn,
    },
    eframe::egui::{self, Color32, ComboBox, Frame, Key},
    egui::{Response, ScrollArea, TextEdit, Ui, Widget},
    itertools::Itertools,
    std::{
        cmp::min,
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

        ScrollArea::both()
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
                                    self.add_crafter(item.recipe(), item.crafter());
                                } else if drop_down_response.enter_pressed {
                                    self.add_crafter(
                                        &self.recipe_search_text.as_str().into(),
                                        None,
                                    );
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

                let mut auto_add_sources_and_sinks = self.editor.auto_add_sources_and_sinks();
                ui.checkbox(&mut auto_add_sources_and_sinks, "Auto add sources and sinks");
                if auto_add_sources_and_sinks != self.editor.auto_add_sources_and_sinks() {
                    self.editor.set_auto_add_sources_and_sinks(auto_add_sources_and_sinks);
                    self.after_machines_changed();
                }
                if !self.editor.auto_add_sources_and_sinks() {
                    ui.horizontal(|ui| {
                        ComboBox::new(("add_source_item", self.generation), "")
                            .selected_text(&self.add_source_item)
                            .show_ui(ui, |ui| {
                                for item in self.editor.all_inputs() {
                                    ui.selectable_value(
                                        &mut self.add_source_item,
                                        item.to_string(),
                                        item.to_string(),
                                    );
                                }
                            });
                        if ui.button("Add source").clicked() {
                            if let Ok(item) = self.add_source_item.parse() {
                                self.editor.add_source(&item).or_warn();
                                self.after_machines_changed();
                            }
                        }
                        ComboBox::new(("add_sink_item", self.generation), "")
                            .selected_text(&self.add_sink_item)
                            .show_ui(ui, |ui| {
                                for item in self.editor.all_outputs() {
                                    ui.selectable_value(
                                        &mut self.add_sink_item,
                                        item.to_string(),
                                        item.to_string(),
                                    );
                                }
                            });
                        if ui.button("Add sink").clicked() {
                            if let Ok(item) = self.add_sink_item.parse() {
                                self.editor.add_sink(&item).or_warn();
                                self.after_machines_changed();
                            }
                        }
                    });

                }

                ui.heading("Machines");
                let edit_machine_index = self.edit_machine_id.and_then(|id| self.editor.machines().iter().position(|m| m.id() == id));
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    if self.editor.machines().is_empty() {
                        ui.label("No machines.");
                    }
                    let mut index_to_remove = None;
                    let mut index_to_recycle = None;
                    let mut recipe_to_add: Option<(RecipeName, Option<CrafterName>)> = None;
                    for (i, editor_machine) in self.editor.machines().iter().enumerate() {
                        let color = if Some(i) == edit_machine_index {
                            Color32::from_rgb(230, 230, 255)
                        } else {
                            Color32::from_rgb(255, 255, 255)
                        };
                        let margin = if Some(i) == edit_machine_index {
                            5
                        } else {
                            0
                        };
                        Frame::new()
                            .fill(color)
                            .inner_margin(margin)
                            .show(ui, |ui| {
                                let machine = editor_machine.machine();
                                ui.horizontal(|ui| {
                                    let input_speeds = machine.input_speeds().collect_vec();
                                    let output_speeds = machine.output_speeds();
                                    let mut is_first = true;
                                    Frame::new().fill(Color32::from_rgb(255, 230, 230)).show(
                                        ui,
                                        |ui| {
                                            for stack in &input_speeds {
                                                ui.rich_label(format!(
                                                    "{}{} @[{}.q{}:]",
                                                    if is_first { "" } else { "+ " },
                                                    -stack.speed,
                                                    stack.item,
                                                    stack.quality.0,
                                                ));
                                                is_first = false;
                                            }
                                        },
                                    );
                                    let lock = if let Some(constraint) = &editor_machine.snippet().count_constraint {
                                        format!("@[$lock:Count constrained to {constraint}]")
                                    } else {
                                        String::new()
                                    };
                                    let crafter_count = if lock.is_empty() && machine.crafter.is_source_or_sink() {
                                        String::new()
                                    } else {
                                        format!("{}{} × ", lock, rf(machine.crafter_count))
                                    };

                                    let tooltip = if (machine.recipe.products.len() == 1
                                        && machine.recipe.name.as_str()
                                            == machine.recipe.products[0].name.as_str())
                                        || machine.crafter.is_source_or_sink()
                                    {
                                        machine.crafter.name.to_string()
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
                                        } else if machine.beacons.iter().all_equal() {
                                            let beacon_quality_text = if machine.beacons[0].quality.0 > 0 {
                                                format!(".q{}", machine.beacons[0].quality.0)
                                            } else {
                                                String::new()
                                            };
                                            let modules =
                                                module_counts(&machine.beacons[0].modules)
                                                    .into_iter()
                                                    .map(|(name, count)| {
                                                        format!("{count} × {name}")
                                                    })
                                                    .join(",");
                                            format!(
                                                "{} × beacon{}({})",
                                                machine.beacons.len(),
                                                beacon_quality_text,
                                                modules
                                            )
                                        } else {
                                            machine
                                                .beacons
                                                .iter()
                                                .map(|beacon| {
                                                    let beacon_quality_text = if beacon.quality.0 > 0 {
                                                        format!(".q{}", beacon.quality.0)
                                                    } else {
                                                        String::new()
                                                    };
                                                    let modules = module_counts(&beacon.modules)
                                                        .into_iter()
                                                        .map(|(name, count)| {
                                                            format!("{count} × {name}")
                                                        })
                                                        .join(",");
                                                    format!("beacon{}({})", beacon_quality_text, modules)
                                                })
                                                .join("\n")
                                        };
                                        let beacon_markup = if machine.beacons.is_empty() {
                                            None
                                        } else {
                                            Some(format!(
                                                "{}@[beacon.q{}:{}]",
                                                machine.beacons.len(),
                                                machine.beacons[0].quality.0,
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
                                        "{}{}@[{}.q{}:{}]{}",
                                        if is_first { "" } else { "➡ " },
                                        crafter_count,
                                        machine.crafter.name,
                                        machine.crafter.quality.0,
                                        tooltip,
                                        modules_text
                                    ));
                                    is_first = true;
                                    Frame::new().fill(Color32::from_rgb(230, 255, 230)).show(
                                        ui,
                                        |ui| {
                                            for stack in &output_speeds {
                                                ui.rich_label(format!(
                                                    "{}{} @[{}.q{}:]",
                                                    if is_first { "➡ " } else { "+ " },
                                                    stack.speed,
                                                    stack.item,
                                                    stack.quality.0,
                                                ));
                                                is_first = false;
                                            }
                                        },
                                    );

                                    // if ui
                                    //     .selectable_label(self.selected_machine == i, machine.io_text())
                                    //     .clicked()
                                    // {
                                    //     self.selected_machine = i;
                                    // }
                                    ui.add_space(10.0);
                                    if machine.crafter.is_source_or_sink() {
                                        let r = ui.with_tooltip(
                                            "Replace with a crafting machine",
                                            |ui| ui.button("Craft"),
                                        );
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
                                                for recipe in
                                                    self.editor.info().game_data.recipes.values()
                                                {
                                                    if recipe.is_recycling() {
                                                        continue;
                                                    }
                                                    let can_replace = if machine.crafter.is_source()
                                                    {
                                                        recipe
                                                            .products
                                                            .iter()
                                                            .any(|p| &p.name == item)
                                                    } else {
                                                        recipe
                                                            .ingredients
                                                            .iter()
                                                            .any(|p| &p.name == item)
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
                                                    } else if recipe.products.len() == 1
                                                        && recipe.products[0].name.as_str()
                                                            == recipe.name.as_str()
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
                                                    };
                                                    for menu_item in recipe_menu_items(
                                                        self.editor.info(),
                                                        recipe,
                                                    ) {
                                                        menu_items_and_hints
                                                            .push((menu_item, hint.clone()));
                                                    }
                                                }
                                                let show_hints = !menu_items_and_hints
                                                    .iter()
                                                    .map(|(_, hint)| hint)
                                                    .all_equal();
                                                self.replace_with_craft_options =
                                                    menu_items_and_hints
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
                                                    let item =
                                                        self.replace_with_craft_options.remove(0).0;
                                                    recipe_to_add = Some((
                                                        item.recipe().clone(),
                                                        item.crafter().cloned(),
                                                    ));
                                                } else {
                                                    self.replace_with_craft_index = Some(i);
                                                }
                                            }
                                        }
                                        if self.replace_with_craft_index == Some(i) {
                                            let mut value: Option<&RecipeMenuItem> = None;
                                            ComboBox::new(
                                                ("replace_source_item", self.generation),
                                                "",
                                            )
                                            .show_ui(
                                                ui,
                                                |ui| {
                                                    for (menu_item, item_text) in
                                                        &self.replace_with_craft_options
                                                    {
                                                        ui.selectable_value(
                                                            &mut value,
                                                            Some(menu_item),
                                                            item_text,
                                                        );
                                                    }
                                                },
                                            );
                                            if let Some(value) = value {
                                                recipe_to_add = Some((
                                                    value.recipe().clone(),
                                                    value.crafter().cloned(),
                                                ));
                                                self.replace_with_craft_index = None;
                                            }
                                        }

                                        if machine.crafter.is_sink() {
                                            let r = ui
                                                .with_tooltip("Replace with a recycler", |ui| {
                                                    ui.button("Recycle")
                                                });
                                            if r.clicked() {
                                                index_to_recycle = Some(i);
                                            }
                                        }
                                    }
                                    if !machine.crafter.is_source_or_sink() || !self.editor.auto_add_sources_and_sinks() {
                                        let r = ui.button("Edit");
                                        if r.clicked() {
                                            self.edit_machine_id = Some(editor_machine.id());
                                            self.machine_count_constraint =
                                                editor_machine.snippet().count_constraint.map(|c| c.to_string()).unwrap_or_default();
                                            self.num_beacons = machine.beacons.len().to_string();
                                            self.focus_machine_constraint_input = true;
                                        }

                                        if ui.button("🗙").clicked() {
                                            index_to_remove = Some(i);
                                        }
                                    }
                                });
                            });
                    }
                    if let Some(i) = index_to_remove {
                        self.saved = false;
                        self.alerts.clear();
                        self.editor.remove_machine(i).or_warn();
                        self.after_machines_changed();
                    }
                    if let Some(i) = index_to_recycle {
                        self.saved = false;
                        self.alerts.clear();
                        let id = self.editor.add_recycler(None, i, None).or_warn();
                        self.edit_machine_id = id;
                        self.after_machines_changed();
                    }
                    if let Some((recipe, crafter)) = recipe_to_add {
                        self.add_crafter(&recipe, crafter.as_ref());
                    }
                });

                if let Some(i) = edit_machine_index {
                    if i < self.editor.machines().len() {
                        ui.horizontal(|ui| {
                            let recipe_name = &self.editor.machines()[i].machine().recipe.name.0;
                            let recipe_name = recipe_name
                                .strip_suffix("-recycling")
                                .unwrap_or(recipe_name);
                            let quality = self.editor.machines()[i].machine().recipe_quality.0;
                            let recipe_name = if quality > 0 {
                                format!("{}.q{}", recipe_name, quality)
                            } else {
                                recipe_name.into()
                            };
                            ui.rich_label(format!(
                                "Edit machine: @[{}]*(@[{}]*)",
                                self.editor.machines()[i].machine().crafter.name,
                                recipe_name,
                            ));
                        });

                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            let crafters = self
                                .editor
                                .info()
                                .category_to_crafter
                                .get(&self.editor.machines()[i].machine().recipe.category)
                                .cloned()
                                .unwrap_or_default();
                            if crafters.len() > 1 {
                                ui.horizontal(|ui| {
                                    ui.label("Change crafter:");
                                    let mut text =
                                        self.editor.machines()[i].machine().crafter.name.clone();
                                    ui.item_icon(
                                        &self.editor.machines()[i].machine().crafter.name.0,
                                        None,
                                        1.,
                                    );
                                    ComboBox::new(("change_crafter", self.generation), "")
                                        .selected_text(
                                            self.editor.machines()[i]
                                                .machine()
                                                .crafter
                                                .name
                                                .as_str(),
                                        )
                                        .show_ui(ui, |ui| {
                                            for item in crafters {
                                                ui.selectable_value(
                                                    &mut text,
                                                    item.clone(),
                                                    item.as_str(),
                                                );
                                            }
                                        });
                                    if text != self.editor.machines()[i].machine().crafter.name {
                                        self.saved = false;
                                        self.alerts.clear();
                                        self.editor.set_crafter(i, &text).or_warn();
                                        self.after_machines_changed();
                                    }
                                    let mut new_crafter_quality =
                                        self.editor.machines()[i].machine().crafter.quality;
                                    ui.quality_dropdown(
                                        ("crafter_quality", self.generation),
                                        Some("Crafter quality"),
                                        &mut new_crafter_quality,
                                    );
                                    if new_crafter_quality
                                        != self.editor.machines()[i].machine().crafter.quality
                                    {
                                        self.saved = false;
                                        self.alerts.clear();
                                        self.editor
                                            .set_crafter_quality(i, new_crafter_quality)
                                            .or_warn();
                                        self.after_machines_changed();
                                    }
                                });
                            }
                            ui.horizontal(|ui| {
                                ui.label("Recipe quality:");
                                let mut new_recipe_quality =
                                    self.editor.machines()[i].machine().recipe_quality;
                                ui.quality_dropdown(
                                    ("recipe_quality", self.generation),
                                    Some("Recipe quality"),
                                    &mut new_recipe_quality,
                                );
                                if new_recipe_quality
                                    != self.editor.machines()[i].machine().recipe_quality
                                {
                                    self.saved = false;
                                    self.alerts.clear();
                                    self.editor
                                        .set_recipe_quality(i, new_recipe_quality)
                                        .or_warn();
                                    self.after_machines_changed();
                                }

                                if ui.button("Add all").clicked() {
                                    self.editor.duplicate_for_all_qualities(i).or_warn();
                                }
                            });
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
                                let set = ui.button("Set").clicked();
                                let replace_all = ui.button("Replace all").clicked();
                                if set
                                    || replace_all
                                    || (text_response.lost_focus()
                                        && ui.input(|i| i.key_pressed(Key::Enter)))
                                {
                                    let count = self.machine_count_constraint.parse().or_warn();
                                    if let Some(count) = count {
                                        self.saved = false;
                                        self.alerts.clear();
                                        self.editor
                                            .set_machine_count_constraint(
                                                i,
                                                Some(count),
                                                replace_all,
                                            )
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
                                        if ui
                                            .rich_label(format!(
                                                "@[{}.q{}:]",
                                                module.name, module.quality.0
                                            ))
                                            .clicked()
                                        {
                                            index_to_remove = Some(ii);
                                        }
                                    }
                                    for _ in 0..(num_empty_module_slots) {
                                        ui.with_tooltip("Empty module slot", |ui| ui.label("🚫"));
                                    }
                                    if !self.editor.machines()[i].machine().modules.is_empty() {
                                        ui.label("(Click on module to remove it, hold shift to remove all)");
                                    }

                                    if let Some(ii) = index_to_remove {
                                        self.saved = false;
                                        self.alerts.clear();
                                        let batch = ui.input(|i| i.modifiers.shift);
                                        let r = self.editor.remove_module(i, ii, batch).or_warn();
                                        if r.is_some() {
                                            self.after_machines_changed();
                                        }
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Add module:");
                                    let mut added = false;
                                    let mut allowed_modules = vec![ModuleType::Speed];
                                    if self.editor.machines()[i]
                                        .machine()
                                        .recipe
                                        .allowed_effects
                                        .productivity
                                    {
                                        allowed_modules.push(ModuleType::Productivity);
                                    }
                                    if self.editor.machines()[i]
                                        .machine()
                                        .recipe
                                        .allowed_effects
                                        .quality
                                    {
                                        allowed_modules.push(ModuleType::Quality);
                                    }
                                    for module_type in allowed_modules {
                                        let module = self.selected_modules.get_mut(&module_type).unwrap();
                                        ui.scope(|ui| {
                                            ui.spacing_mut().item_spacing.x = 0.;
                                            if ui
                                                .rich_label(format!(
                                                    "@[{}.q{}:]",
                                                    module.name, module.quality.0
                                                ))
                                                .clicked()
                                            {
                                                let num_added = if ui.input(|i| i.modifiers.shift) {
                                                    num_empty_module_slots
                                                } else {
                                                    min(num_empty_module_slots, 1)
                                                };
                                                if num_added > 0 {
                                                    self.saved = false;
                                                    self.alerts.clear();
                                                    self.editor.add_modules(i, module, num_added).or_warn();
                                                    added = true;
                                                }
                                            }

                                            ComboBox::new(("add_module", module_type), "")
                                                .selected_text("")
                                                .width(16.)
                                                .show_ui(ui, |ui| {
                                                    for item in module_type.module_items() {
                                                        ui.horizontal(|ui| {
                                                            if ui.item_icon(&item.0, Some(&item.0), 1.0).clicked() {
                                                                *module = ItemNameAndQuality {
                                                                    name: item.clone(),
                                                                    quality: Quality(0),
                                                                };
                                                            }
                                                            for quality in Quality::ALL {
                                                                if ui
                                                                    .item_icon(&format!("quality{}", quality.0), Some(&format!("{} with quality {}", item.0, quality.0)), 1.)
                                                                    .clicked()
                                                                {
                                                                    *module = ItemNameAndQuality {
                                                                        name: item.clone(),
                                                                        quality,
                                                                    };
                                                                }
                                                            }
                                                        });
                                                    }
                                                });
                                        });
                                    }
                                    ui.label("(Hold Shift to fill)");
                                    if added {
                                        self.after_machines_changed();
                                    }
                                });

                                ui.horizontal(|ui| {
                                    let module = self.selected_modules.get(&ModuleType::Speed).unwrap();
                                    ui.scope(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.;
                                        ui.rich_label("Number of @[beacon:]");
                                        ui.quality_dropdown("beacon_quality", Some("Beacon quality"), &mut self.beacon_quality);
                                        ui.rich_label(format!(
                                            "(2@[{}.q{}:]) per machine:",
                                            &module.name,
                                            module.quality.0,
                                        ));
                                    });
                                    let text_response = TextEdit::singleline(&mut self.num_beacons)
                                        .desired_width(50.0)
                                        .ui(ui);
                                    if ui.button("Set").clicked()
                                        || (text_response.lost_focus()
                                            && ui.input(|i| i.key_pressed(Key::Enter)))
                                    {
                                        if let Some(num_beacons) =
                                            self.num_beacons.parse::<u32>().or_warn()
                                        {
                                            self.saved = false;
                                            self.alerts.clear();
                                            let module = self.editor
                                                .info()
                                                .modules
                                                .get(&ModuleName(module.name.0.to_string()))
                                                .unwrap()
                                                .with_quality(module.quality);
                                            self.editor
                                                .set_beacons(
                                                    i,
                                                    (0..num_beacons)
                                                        .map(|_| Beacon {
                                                            quality: self.beacon_quality,
                                                            modules: (0..2)
                                                                .map(|_| {
                                                                    module
                                                                        .clone()
                                                                })
                                                                .collect_vec(),
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
                                self.edit_machine_id = None;
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
                                "@[$lock:Item speed constraint] @[{item}]*: {speed}"
                            ));
                            if ui.button("Edit").clicked() {
                                self.item_speed_contraint_item = item.to_string();
                                self.old_item_speed_contraint_item = item.to_string();
                                self.item_speed_contraint_speed = speed.0.to_string();
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
                        if let Some(count) = &machine.snippet().count_constraint {
                            ui.horizontal(|ui| {
                                ui.rich_label(format!(
                                    "@[$lock:Machine count constraint] {} × @[{}]*(@[{}]*)",
                                    count,
                                    machine.machine().crafter.name,
                                    machine.machine().recipe.name,
                                ));
                                if ui.button("Edit").clicked() {
                                    self.edit_machine_id = Some(machine.id());
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
                            .set_machine_count_constraint(index, None, false)
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
                                        item.to_string(),
                                        item.to_string(),
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
                                if let Some(item) = self.item_speed_contraint_item.parse().or_warn()
                                {
                                    self.alerts.clear();
                                    self.editor
                                        .set_item_speed_constraint(&item, Some(speed), replace_all)
                                        .or_warn();
                                }
                                self.after_constraint_changed();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tip:");
                        for (speed, item) in &self.belt_speeds {
                            ui.rich_label(format!("@[{item}:] = {speed}    "));
                        }
                    });
                });

                ui.add_space(10.0);
                let mut use_alt_solver = self.editor.use_alt_solver();
                ui.checkbox(&mut use_alt_solver, "Use alternative solver");
                if use_alt_solver != self.editor.use_alt_solver() {
                    self.editor.set_use_alt_solver(use_alt_solver);
                    self.after_machines_changed();
                }

                ui.horizontal(|ui| {
                    if ui.button("Open chart").clicked() {
                        self.open_chart().or_warn();
                    }
                    if ui.button("Solve").clicked() {
                        self.alerts.clear();
                        self.editor.solve();
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
