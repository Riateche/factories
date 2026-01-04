use {
    super::app::icon_url,
    crate::primitives::Quality,
    eframe::egui::{self, vec2, Color32, ComboBox, Image, Response, Sense, Ui, Widget},
    regex::Regex,
    std::hash::Hash,
    tracing::error,
};

pub trait UiExt {
    fn with_tooltip(
        &mut self,
        tooltip: &str,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response;
    fn icon(&mut self, icon: &str, tooltip: Option<&str>, scale: f32) -> Response;
    fn item_icon(&mut self, item: &str, tooltip: Option<&str>, scale: f32) -> Response;

    /// Examples:
    /// @[iron-plate] - item icon
    /// @[iron-plate]* - item icon followed by item name label
    /// @[iron-plate:] - item icon with item name tooltip
    /// @[iron-plate:Tooltip] - item icon with custom tooltip text
    /// @[$lock] - system icon
    /// @[$lock:Tooltip] - system icon with tooltip
    fn rich_label(&mut self, text: impl Into<String>) -> Response;

    fn quality_dropdown(&mut self, id_salt: impl Hash, tooltip: Option<&str>, value: &mut Quality);
}

impl UiExt for Ui {
    fn icon(&mut self, icon: &str, tooltip: Option<&str>, scale: f32) -> Response {
        let ui = self;
        let r = Image::new(icon_url(icon))
            .fit_to_exact_size(vec2(24. * scale, 24. * scale))
            .ui(ui)
            .interact(Sense::click());
        if let Some(tooltip) = tooltip {
            if r.contains_pointer() {
                egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new(tooltip), |ui| {
                    ui.label(tooltip);
                });
            }
        }
        r
    }

    fn item_icon(&mut self, item: &str, tooltip: Option<&str>, scale: f32) -> Response {
        self.icon(&format!("factorio/{item}"), tooltip, scale)
    }

    fn rich_label(&mut self, text: impl Into<String>) -> Response {
        let text = text.into();
        let ui = self;
        /*
            @[icon]
            @[icon:tooltip]
            @[icon]*
            @[icon:tooltip]*
            * at the end - show icon name after icon
            if tooltip is not specified, tooltip is icon name.

            icon:
            iron-plate - item/recipe icon
            iron-plate.q1 - item/recipe and quality icon
            $lock - ui icon
        */
        let re = Regex::new(r"@\[([^:\]]*)(:([^:\]]*)){0,1}\](\*){0,1}").unwrap();
        let mut current = 0;
        ui.scope(|ui| {
            ui.style_mut().visuals.panel_fill = Color32::RED;
            let mut r = ui.response();
            ui.spacing_mut().item_spacing.x = 0.;
            for capture in re.captures_iter(&text) {
                let full = capture.get(0).unwrap();
                let icon = capture.get(1).unwrap().as_str();
                let tooltip = capture
                    .get(2)
                    .and_then(|_| capture.get(3).map(|c| c.as_str()).filter(|t| !t.is_empty()));

                if full.start() != current {
                    let plain_text = &text[current..full.start()];
                    r |= ui.label(plain_text);
                }
                if let Some(icon_remaining) = icon.strip_prefix('$') {
                    r |= ui.icon(icon_remaining, Some(tooltip.unwrap_or(icon_remaining)), 1.);
                } else if let Some((icon, quality)) = icon.split_once(".q") {
                    r |= ui.item_icon(icon, Some(tooltip.unwrap_or(icon)), 1.);
                    let quality = quality.parse::<u32>().unwrap_or_else(|_| {
                        error!("invalid quality in rich label: {quality:?}");
                        0
                    });
                    if quality > 0 {
                        r |= ui.item_icon(
                            &format!("quality{quality}"),
                            Some(&format!("Quality {quality}")),
                            0.5,
                        );
                    }
                } else {
                    r |= ui.item_icon(icon, Some(tooltip.unwrap_or(icon)), 1.);
                }
                if capture.get(4).is_some() {
                    r |= ui.label(icon);
                }
                current = full.end();
            }
            if current != text.len() {
                let plain_text = &text[current..];
                r |= ui.label(plain_text);
            }
            r
        })
        .inner
    }

    fn with_tooltip(
        &mut self,
        tooltip: &str,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response {
        let ui = self;
        let r = add_contents(ui);
        if r.contains_pointer() {
            egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new(tooltip), |ui| {
                ui.label(tooltip);
            });
        }
        r
    }

    fn quality_dropdown(&mut self, id_salt: impl Hash, tooltip: Option<&str>, value: &mut Quality) {
        let ui = self;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            ui.item_icon(&format!("quality{}", value.0), tooltip, 1.);
            ComboBox::new(id_salt, "").width(16.).show_ui(ui, |ui| {
                ui.horizontal(|ui| {
                    for quality in Quality::ALL {
                        if ui
                            .item_icon(&format!("quality{}", quality.0), None, 1.0)
                            .clicked()
                        {
                            *value = quality;
                        }
                    }
                });
            });
        });
    }
}
