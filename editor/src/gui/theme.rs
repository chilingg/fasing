use super::widget::*;

use std::path::Path;

use eframe::egui;

pub fn default_style() -> egui::Style {
    let file_path: &Path = Path::new("style.json");

    if file_path.exists() {
        if let Ok(file) = std::fs::read_to_string(file_path) {
            if let Ok(style) = serde_json::from_str::<egui::Style>(&file) {
                return style;
            }
        }
    }
    serde_json::from_str(include_str!("style.json")).unwrap()
}

pub fn save_style_to_json<P: AsRef<Path>>(style: &egui::Style, path: P) -> anyhow::Result<()> {
    Ok(std::fs::write(path, serde_json::to_string_pretty(style)?)?)
}

pub struct StyleEditor {
    pub open: bool,
    save_path: String,
}

impl StyleEditor {
    pub fn new(open: bool, save_path: String) -> Self {
        Self { open, save_path }
    }
}

impl<C, U> Widget<C, U> for StyleEditor {
    fn children<'a>(&'a mut self) -> Children<'a, C, U> {
        vec![]
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
        _core_data: &C,
        _run_data: &mut U,
    ) {
        let mut changed = false;
        let mut style = (*ctx.style()).clone();

        egui::Window::new("Style")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Some(e) = save_style_to_json(&style, self.save_path.clone()).err() {
                            eprintln!("Failed to save style file in {}: {}", self.save_path, e)
                        }
                    }
                    if ui.button("Default").clicked() {
                        style = default_style();
                        changed = true;
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::CollapsingHeader::new("Visuals")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.collapsing("Widget visuals", |ui| {
                                [
                                    (&mut style.visuals.widgets.noninteractive, "noninteractive"),
                                    (&mut style.visuals.widgets.inactive, "inactive"),
                                    (&mut style.visuals.widgets.hovered, "hovered"),
                                    (&mut style.visuals.widgets.active, "active"),
                                    (&mut style.visuals.widgets.open, "open"),
                                ]
                                .iter_mut()
                                .for_each(|(widget, name)| {
                                    ui.collapsing(*name, |ui| {
                                        ui.horizontal(|ui| {
                                            changed |= ui
                                                .color_edit_button_srgba(&mut widget.bg_fill)
                                                .changed();
                                            ui.label("bg fill");
                                            changed |= ui
                                                .color_edit_button_srgba(&mut widget.weak_bg_fill)
                                                .changed();
                                            ui.label("weak bg fill");
                                        });

                                        [
                                            (&mut widget.bg_stroke, "Background stroke"),
                                            (&mut widget.fg_stroke, "Foreground stroke"),
                                        ]
                                        .iter_mut()
                                        .for_each(
                                            |(stroke, name)| {
                                                ui.horizontal(|ui| {
                                                    changed |= ui
                                                        .add(
                                                            egui::DragValue::new(&mut stroke.width)
                                                                .speed(0.2),
                                                        )
                                                        .changed();
                                                    changed |= ui
                                                        .color_edit_button_srgba(&mut stroke.color)
                                                        .changed();
                                                    ui.label(*name);
                                                });
                                            },
                                        );

                                        ui.horizontal(|ui| {
                                            ui.label("Rounding");
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut widget.rounding.nw)
                                                        .speed(0.2),
                                                )
                                                .changed();
                                        });
                                        widget.rounding = egui::Rounding::same(widget.rounding.nw);

                                        ui.horizontal(|ui| {
                                            ui.label("Frame expansion");
                                            changed |= ui
                                                .add(
                                                    egui::DragValue::new(&mut widget.expansion)
                                                        .speed(0.2),
                                                )
                                                .changed();
                                        });
                                    });
                                });
                            });

                            ui.collapsing("Window", |ui| {
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.window_fill)
                                        .changed();
                                    ui.label("Fill");
                                });

                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(
                                                &mut style.visuals.window_stroke.width,
                                            )
                                            .speed(0.2),
                                        )
                                        .changed();
                                    changed |= ui
                                        .color_edit_button_srgba(
                                            &mut style.visuals.window_stroke.color,
                                        )
                                        .changed();
                                    ui.label("Stroke");
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Rounding");
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(
                                                &mut style.visuals.window_rounding.nw,
                                            )
                                            .speed(0.2),
                                        )
                                        .changed();
                                });
                                style.visuals.window_rounding =
                                    egui::Rounding::same(style.visuals.window_rounding.nw);

                                ui.label("Shadow:");
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .add(
                                            egui::DragValue::new(
                                                &mut style.visuals.window_shadow.extrusion,
                                            )
                                            .speed(0.2),
                                        )
                                        .changed();
                                    changed |= ui
                                        .color_edit_button_srgba(
                                            &mut style.visuals.window_shadow.color,
                                        )
                                        .changed();
                                });
                            });

                            ui.collapsing("Color", |ui| {
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.panel_fill)
                                        .changed();
                                    ui.label("panel");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.hyperlink_color)
                                        .changed();
                                    ui.label("hyperlink");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.faint_bg_color)
                                        .changed();
                                    ui.label("faint bg");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(
                                            &mut style.visuals.extreme_bg_color,
                                        )
                                        .changed();
                                    ui.label("extreme bg");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.code_bg_color)
                                        .changed();
                                    ui.label("code bg");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.warn_fg_color)
                                        .changed();
                                    ui.label("warn fg");
                                });
                                ui.horizontal(|ui| {
                                    changed |= ui
                                        .color_edit_button_srgba(&mut style.visuals.error_fg_color)
                                        .changed();
                                    ui.label("error fg");
                                });
                            });

                            ui.horizontal(|ui| {
                                ui.label("Selection");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut style.visuals.selection.stroke.width,
                                        )
                                        .speed(0.2),
                                    )
                                    .changed();
                                changed |= ui
                                    .color_edit_button_srgba(&mut style.visuals.selection.bg_fill)
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Popup shadow:");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut style.visuals.popup_shadow.extrusion,
                                        )
                                        .speed(0.2),
                                    )
                                    .changed();
                                changed |= ui
                                    .color_edit_button_srgba(&mut style.visuals.popup_shadow.color)
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Resize corner size:");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut style.visuals.resize_corner_size)
                                            .speed(0.2),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Text corner width:");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut style.visuals.text_cursor_width)
                                            .speed(0.2),
                                    )
                                    .changed();
                                changed |= ui
                                    .checkbox(&mut style.visuals.text_cursor_preview, "Preview")
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Clip rect margin:");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut style.visuals.clip_rect_margin)
                                            .speed(0.2),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label("Menu rounding");
                                changed |= ui
                                    .add(
                                        egui::DragValue::new(&mut style.visuals.menu_rounding.nw)
                                            .speed(0.2),
                                    )
                                    .changed();
                            });
                            style.visuals.menu_rounding =
                                egui::Rounding::same(style.visuals.menu_rounding.nw);

                            changed |= ui
                                .checkbox(&mut style.visuals.button_frame, "Button frame")
                                .changed();
                            changed |= ui
                                .checkbox(
                                    &mut style.visuals.collapsing_header_frame,
                                    "Collapsing header frame",
                                )
                                .changed();
                            changed |= ui
                                .checkbox(
                                    &mut style.visuals.indent_has_left_vline,
                                    "Indent has left vline",
                                )
                                .changed();
                            changed |= ui.checkbox(&mut style.visuals.striped, "Striped").changed();
                            changed |= ui
                                .checkbox(
                                    &mut style.visuals.slider_trailing_fill,
                                    "Slider trailing fill",
                                )
                                .changed();
                        });

                    ui.collapsing("Text", |ui| {
                        style.text_styles.iter_mut().for_each(|(style, font_id)| {
                            ui.horizontal(|ui| {
                                ui.label(format!("{:?}", style));
                                let mut size = font_id.size;
                                if ui.add(egui::DragValue::new(&mut size).speed(0.2)).changed() {
                                    changed = true;
                                    *font_id = egui::FontId::proportional(size);
                                }
                            });
                        });
                    });
                });
            });

        if changed {
            ctx.set_style(style);
        }
    }
}
