use super::prelude::*;

#[derive(Default)]
pub struct Sidebar {}

impl Widget for Sidebar {
    fn update(&mut self, ctx: &egui::Context, _: &mut Vec<Task>) {
        egui::SidePanel::left("working_set")
            .resizable(false)
            .width_range(32.0..=32.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let size = egui::Vec2::new(32.0, 48.0);
                    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
                    let stroke = if response.clicked() || response.hovered() {
                        ctx.style().visuals.widgets.active.fg_stroke
                    } else {
                        ctx.style().visuals.widgets.inactive.fg_stroke
                    };

                    let rect = response.rect;
                    let c = rect.center();
                    let w = rect.width();
                    let r = w * 0.1;

                    let value = w * 0.5 * 0.6;
                    let horizontal = egui::vec2(value, 0.0);
                    let vertical = egui::vec2(0.0, value);
                    let direct = egui::vec2(value, -value).normalized() * r;

                    painter.line_segment([c - horizontal - vertical + egui::Vec2::X * r, c + horizontal - vertical - egui::Vec2::X * r], stroke);
                    painter.line_segment([c + horizontal - vertical - direct, c - horizontal + vertical + direct], stroke);
                    painter.line_segment([c - horizontal + vertical + egui::Vec2::X * r, c + horizontal + vertical - egui::Vec2::X * r], stroke);

                    painter.circle_stroke(c - horizontal - vertical, r, stroke);
                    painter.circle_stroke(c + horizontal - vertical, r, stroke);
                    painter.circle_stroke(c - horizontal + vertical, r, stroke);
                    painter.circle_stroke(c + horizontal + vertical, r, stroke);

                    response.on_hover_text("部件");
                });
            });
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}