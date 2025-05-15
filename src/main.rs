#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(clippy::collapsible_if)]

use {
    eframe::{
        egui::{style::ScrollStyle, vec2, TextStyle, ViewportBuilder},
        icon_data,
    },
    factories::ui::app::MyApp,
    itertools::Itertools,
    tracing::{
        field::{Field, Visit},
        level_filters::LevelFilter,
        span, Subscriber,
    },
    tracing_log::LogTracer,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer},
};

struct MyLayer;

impl<S: Subscriber> Layer<S> for MyLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        pub struct MyVisitor;
        impl Visit for MyVisitor {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                //eprintln!("\nfield {:?} {:?}\n", field.name(), value);
            }
        }
        //eprintln!("\non_event! {event:?}\n");
        event.record(&mut MyVisitor);
    }
}

fn main() -> anyhow::Result<()> {
    //LogTracer::init()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()
                .expect("invalid RUST_LOG env var"),
        )
        .finish()
        .with(MyLayer)
        .try_init()?;

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
