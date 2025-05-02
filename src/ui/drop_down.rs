use {
    eframe::egui,
    egui::{
        text::{CCursor, CCursorRange},
        Id, Response, ScrollArea, TextEdit, Ui, Widget, WidgetText,
    },
    std::hash::Hash,
};

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
