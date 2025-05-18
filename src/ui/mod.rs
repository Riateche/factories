use {
    crate::ui::{app::MyApp, tracing_layer::UiLayer},
    eframe::{
        egui::{style::ScrollStyle, vec2, TextStyle, ViewportBuilder},
        icon_data,
    },
    std::sync::mpsc,
    tracing::level_filters::LevelFilter,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter},
};

pub mod app;
pub mod app_ui;
pub mod drop_down;
pub mod tracing_layer;
pub mod ui_ext;

pub fn run() -> anyhow::Result<()> {
    //LogTracer::init()?;
    let (ui_msg_sender, ui_msg_receiver) = mpsc::channel();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .expect("invalid RUST_LOG env var"),
        )
        .finish()
        .with(UiLayer::new(ui_msg_sender))
        .try_init()?;

    // icon source: https://www.iconfinder.com/icons/3688428/
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_icon(icon_data::from_png_bytes(include_bytes!(
                "../../icons/factories.png"
            ))?),
        ..Default::default()
    };

    let app = MyApp::new(ui_msg_receiver)?;
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
