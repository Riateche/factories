#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::collapsible_if)]

use {
    eframe::{
        egui::{style::ScrollStyle, vec2, TextStyle, ViewportBuilder},
        icon_data,
    },
    factories::ui::app::MyApp,
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // icon source: https://www.iconfinder.com/icons/3688428/
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_icon(icon_data::from_png_bytes(include_bytes!("../icon.png"))?),
        ..Default::default()
    };

    let app = MyApp::new()?;
    eframe::run_native(
        "Factories",
        options,
        Box::new(|cc| {
            cc.egui_ctx.all_styles_mut(|style| {
                style.text_styles.get_mut(&TextStyle::Body).unwrap().size = 20.0;
                style.text_styles.get_mut(&TextStyle::Button).unwrap().size = 15.0;
                style.spacing.item_spacing.y = 5.0;
                style.spacing.button_padding = vec2(5.0, 2.0);
                style.spacing.scroll = ScrollStyle::solid();
            });
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
    .unwrap();
    Ok(())
}
