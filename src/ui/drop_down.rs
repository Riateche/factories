use {
    eframe::egui,
    egui::{
        text::{CCursor, CCursorRange},
        Id, Response, ScrollArea, TextEdit, Ui, Widget, WidgetText,
    },
    itertools::Itertools,
    std::{borrow::Cow, hash::Hash},
};

pub trait DropDownOption: Widget {
    fn search_text(&self) -> Cow<str>;
    fn insert_text(&self) -> Cow<str>;
    fn id(&self) -> Id;
}

pub struct DropDownBox<'a, I> {
    buf: &'a mut String,
    base_popup_id: Id,
    it: I,
    hint_text: WidgetText,
    filter_by_input: bool,
    select_on_focus: bool,
    desired_width: Option<f32>,
    max_height: Option<f32>,
    min_scrolled_height: Option<f32>,
}

impl<'a, 'b, V, I> DropDownBox<'a, I>
where
    I: Iterator<Item = &'b V>,
    &'b V: DropDownOption + 'b,
{
    pub fn from_iter(
        it: impl IntoIterator<IntoIter = I>,
        id_source: impl Hash,
        buf: &'a mut String,
    ) -> Self {
        Self {
            base_popup_id: Id::new(id_source),
            it: it.into_iter(),
            buf,
            hint_text: WidgetText::default(),
            filter_by_input: true,
            select_on_focus: false,
            desired_width: None,
            max_height: None,
            min_scrolled_height: None,
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

    pub fn min_scrolled_height(mut self, height: f32) -> Self {
        self.min_scrolled_height = height.into();
        self
    }

    pub fn show(self, ui: &mut Ui) -> DropDownBoxOutput<'b, V> {
        let mut edit = TextEdit::singleline(self.buf).hint_text(self.hint_text.clone());
        if let Some(dw) = self.desired_width {
            edit = edit.desired_width(dw);
        }
        let mut option_selected = None;
        let mut edit_output = edit.show(ui);
        let mut r = edit_output.response;

        let filtered_items = self
            .it
            .filter(|option| {
                if self.filter_by_input && !self.buf.is_empty() {
                    option.search_text().contains(&self.buf.to_lowercase())
                } else {
                    true
                }
            })
            .collect_vec();
        let ids = filtered_items.iter().map(|i| i.id()).collect_vec();
        let popup_id = Id::new((self.base_popup_id, ids));

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
            ui.memory_mut(|m| m.open_popup(popup_id));
        }

        let request_popup_focus =
            r.has_focus() && ui.input(|i| i.key_pressed(egui::Key::ArrowDown));

        let enter_pressed = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        let mut changed = false;
        egui::popup_below_widget(
            ui,
            popup_id,
            &r,
            egui::PopupCloseBehavior::CloseOnClick,
            |ui| {
                if let Some(max) = self.max_height {
                    ui.set_max_height(max);
                }

                let mut row = 0;
                let mut scroll_area = ScrollArea::vertical();
                if let Some(value) = self.min_scrolled_height {
                    scroll_area = scroll_area.min_scrolled_height(value);
                }
                scroll_area.show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    for option in filtered_items {
                        let response = option.ui(ui);
                        if row == 0 && request_popup_focus {
                            response.request_focus();
                        }
                        if response.clicked() {
                            *self.buf = option.insert_text().into();
                            changed = true;
                            option_selected = Some(option);
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
            option_selected,
            enter_pressed,
        }
    }
}

pub struct DropDownBoxOutput<'a, V> {
    pub response: Response,
    pub enter_pressed: bool,
    pub option_selected: Option<&'a V>,
}

impl<'a, 'b, V, I> Widget for DropDownBox<'a, I>
where
    I: Iterator<Item = &'b V>,
    &'b V: DropDownOption + 'b,
{
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
