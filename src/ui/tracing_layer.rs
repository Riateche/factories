use {
    std::sync::mpsc::Sender,
    tracing::{
        field::{Field, Visit},
        Level, Subscriber,
    },
    tracing_subscriber::Layer,
};

pub struct UiLayer {
    sender: Sender<String>,
}

impl UiLayer {
    pub fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }
}

impl<S: Subscriber> Layer<S> for UiLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        event.record(&mut UiLayerVisitor {
            layer: self,
            level: event.metadata().level(),
        });
    }
}

struct UiLayerVisitor<'a> {
    layer: &'a UiLayer,
    level: &'a Level,
}

impl Visit for UiLayerVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.layer
                .sender
                .send(format!("[{}] {:?}", self.level.as_str(), value))
                .expect("UiLayer: send failed");
        }
    }
}
