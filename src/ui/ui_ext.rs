use {
    super::app::icon_url,
    eframe::egui::{self, Color32, Response, Sense, Ui},
    regex::Regex,
};

pub trait UiExt {
    fn with_tooltip(
        &mut self,
        tooltip: &str,
        add_contents: impl FnOnce(&mut Ui) -> Response,
    ) -> Response;
    fn icon(&mut self, icon: &str, tooltip: Option<&str>) -> Response;
    fn item_icon(&mut self, item: &str, tooltip: Option<&str>) -> Response;

    /// Examples:
    /// @[iron-plate] - item icon
    /// @[iron-plate]* - item icon followed by item name label
    /// @[iron-plate:] - item icon with item name tooltip
    /// @[iron-plate:Tooltip] - item icon with custom tooltip text
    /// @[$lock] - system icon
    /// @[$lock:Tooltip] - system icon with tooltip
    fn rich_label(&mut self, text: impl Into<String>) -> Response;
}

impl UiExt for Ui {
    fn icon(&mut self, icon: &str, tooltip: Option<&str>) -> Response {
        let ui = self;
        let r = ui.image(icon_url(icon)).interact(Sense::click());
        if let Some(tooltip) = tooltip {
            if r.contains_pointer() {
                egui::show_tooltip(ui.ctx(), ui.layer_id(), egui::Id::new(tooltip), |ui| {
                    ui.label(tooltip);
                });
            }
        }
        r
    }

    fn item_icon(&mut self, item: &str, tooltip: Option<&str>) -> Response {
        self.icon(&format!("factorio/{item}"), tooltip)
    }

    fn rich_label(&mut self, text: impl Into<String>) -> Response {
        let text = text.into();
        let ui = self;
        let re = Regex::new(r"@\[([^:\]]*)(:([^:\]]*)){0,1}\](\*){0,1}").unwrap();
        let mut current = 0;
        ui.scope(|ui| {
            ui.style_mut().visuals.panel_fill = Color32::RED;
            let mut r = ui.response();
            ui.spacing_mut().item_spacing.x = 0.;
            for capture in re.captures_iter(&text) {
                let full = capture.get(0).unwrap();
                let icon = capture.get(1).unwrap().as_str();
                let tooltip = capture.get(2).map(|_| {
                    capture
                        .get(3)
                        .map(|c| c.as_str())
                        .filter(|t| !t.is_empty())
                        .unwrap_or(icon)
                });

                if full.start() != current {
                    let plain_text = &text[current..full.start()];
                    r |= ui.label(plain_text);
                }
                if icon.starts_with('$') {
                    r |= ui.icon(&icon[1..], tooltip);
                } else {
                    r |= ui.item_icon(icon, tooltip);
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
}
