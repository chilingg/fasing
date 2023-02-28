use crate::prelude::*;

use eframe::egui;

pub struct Sidebar {
    pub current: usize,
    work_icons:
        [Box<dyn Fn(egui::Rect, egui::Stroke, egui::Color32) -> (egui::Shape, &'static str)>; 2],
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            current: 0,
            work_icons: [
                Box::new(
                    |rect: egui::Rect, stroke: egui::Stroke, bg: egui::Color32| {
                        let c = rect.center();
                        let w = rect.width() * 0.8;
                        let dw = rect.width() * 0.15;

                        let horizontal = egui::Vec2::X * w * 0.3;
                        let vertical = egui::Vec2::Y * w * 0.3;

                        let mut shapes = vec![];

                        let points = vec![
                            c - horizontal - vertical,
                            c + horizontal - vertical,
                            c - horizontal + vertical,
                            c + horizontal + vertical,
                        ];

                        shapes.push(egui::Shape::Path(eframe::epaint::PathShape {
                            points: points.clone(),
                            closed: false,
                            fill: egui::Color32::TRANSPARENT,
                            stroke,
                        }));

                        points.into_iter().for_each(|p| {
                            shapes.push(egui::Shape::Rect(eframe::epaint::RectShape {
                                rect: egui::Rect::from_center_size(p, egui::Vec2::splat(dw)),
                                rounding: egui::Rounding::none(),
                                fill: bg,
                                stroke,
                            }))
                        });

                        (egui::Shape::Vec(shapes), "元部件编辑(Ctrl+1)")
                    },
                ),
                Box::new(|rect: egui::Rect, stroke: egui::Stroke, _: egui::Color32| {
                    let c = rect.center();
                    let w = rect.width();
                    let mut dw = w * 0.15;

                    let horizontal = egui::Vec2::X * w * 0.3;
                    let vertical = egui::Vec2::Y * w * 0.3;

                    let mut shapes = vec![];

                    let points = vec![c - horizontal - vertical, c + horizontal + vertical];
                    points.iter().for_each(|&p| {
                        shapes.push(egui::Shape::Path(eframe::epaint::PathShape::line(
                            vec![p + egui::vec2(dw, 0.0), p, p + egui::vec2(0.0, dw)],
                            stroke,
                        )));
                        dw = -dw;
                    });
                    shapes.push(egui::Shape::line_segment([points[0], points[1]], stroke));

                    let points = vec![c + vertical - horizontal, c - vertical + horizontal];
                    points.iter().for_each(|&p| {
                        shapes.push(egui::Shape::Path(eframe::epaint::PathShape::line(
                            vec![p + egui::vec2(dw, 0.0), p, p - egui::vec2(0.0, dw)],
                            stroke,
                        )));
                        dw = -dw;
                    });
                    shapes.push(egui::Shape::line_segment([points[0], points[1]], stroke));

                    (egui::Shape::Vec(shapes), "元部件展开(Ctrl+2)")
                }),
            ],
        }
    }
}

impl Widget<CoreData, RunData> for Sidebar {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        _core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        egui::SidePanel::left("working set")
            .resizable(false)
            .width_range(32.0..=32.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let size = egui::Vec2::new(32.0, 48.0);

                    (0..self.work_icons.len()).into_iter().for_each(|i| {
                        let (response, painter) = ui.allocate_painter(size, egui::Sense::click());

                        let clicked = response.clicked();
                        if clicked {
                            self.current = i;
                        }
                        let stroke = if i == self.current || response.hovered() {
                            ui.style().visuals.widgets.active.fg_stroke
                        } else {
                            ui.style().visuals.widgets.inactive.fg_stroke
                        };

                        let (icon, tip) = self.work_icons[i](
                            response.rect,
                            stroke,
                            ui.style().visuals.panel_fill,
                        );
                        painter.add(icon);
                        response.on_hover_text(tip);
                    })
                });
            });
    }

    fn children(&mut self) -> Children {
        vec![]
    }

    fn input_process(
        &mut self,
        input: &mut egui::InputState,
        _core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        if input.consume_key(egui::Modifiers::CTRL, egui::Key::Num1) {
            self.current = 0;
        }
        if input.consume_key(egui::Modifiers::CTRL, egui::Key::Num2) {
            self.current = 1;
        }
    }
}
