use super::prelude::*;

#[derive(Default)]
pub struct Center {
    label: String,
}

impl Widget for Center {
    fn start(&mut self, app_state: &mut AppState) {
        self.label = format!("fasing 1.0: {}字符", app_state.core_data.construction.len());
    }

    fn update(&mut self, ctx: &egui::Context, queue: &mut Vec<Task>) {
        let font_id = egui::FontId::proportional(60.0);

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = ui.style().noninteractive().bg_stroke.color;

                egui::CentralPanel::default()
                    .show_inside(ui, |ui| {
                        let button = egui::Button::new(
                            egui::RichText::new('永'.to_string()).font(font_id.clone()),
                        )
                        .frame(false);
        
                        ui.add(button);
                    });
            });

        egui::Window::new("Data")
            .show(ctx, |ui| {
                ui.label(self.label.clone());
            });
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}