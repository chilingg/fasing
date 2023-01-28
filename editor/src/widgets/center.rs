use super::prelude::*;

pub struct Center {}

impl Widget for Center {
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
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}