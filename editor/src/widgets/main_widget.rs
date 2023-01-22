use super::{
    Widget,
    event::*,
};

pub struct  MainWidget {
    children: Vec<Box<dyn Widget>>,
}

impl MainWidget {
    pub fn new() -> Self {
        Self { children: vec![] }
    }
}

impl Widget for MainWidget {
    fn update(&mut self, event: UpdateEvent) {
        egui::Window::new("test")
            .show(&event.egui_ctx, |ui| {
                ui.label("this is text");
            });
    }
}