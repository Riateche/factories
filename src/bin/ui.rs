#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::{self, Color32};
use factories::prelude::*;

use egui::{
    text::{CCursor, CCursorRange},
    Id, Response, ScrollArea, TextEdit, Ui, Widget, WidgetText,
};
use std::hash::Hash;

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
                                r.request_focus();

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
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Factories",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp {
                planner: init().unwrap(),
                recipe_search_text: String::new(),
                auto_focus: true,
                alerts: Vec::new(),
                selected_machine: 0,
            }))
        }),
    )
}

struct MyApp {
    planner: Planner,
    recipe_search_text: String,
    alerts: Vec<String>,
    auto_focus: bool,
    selected_machine: usize,
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
                    self.planner.game_data.recipes.keys(),
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
                    println!("adding {}", self.recipe_search_text);
                    match self.planner.create_machine(&self.recipe_search_text) {
                        Err(err) => {
                            self.alerts.push(err.to_string());
                        }
                        Ok(()) => {
                            self.recipe_search_text.clear();
                        }
                    }
                }
            });

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
