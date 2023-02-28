use super::{mete_comp_works::PAINT_SIZE, struc_editor_window::StrucEditing};
use crate::prelude::*;
use fasing::{fas_file::AllocateTable, struc::*};

use regex::Regex;
use std::collections::HashMap;

pub struct ExtendWorks {
    requests: HashMap<String, (Vec<String>, Vec<String>)>,
    editor_window: Option<StrucEditing>,
    scroll_state: (usize, usize),

    filter_str: String,
    filter: Option<Regex>,
    filter_msg: String,

    selected: Option<Vec<String>>,
    mark_colors: Vec<egui::Color32>,

    test_str: String,
    test_regex: Option<Regex>,
}

impl ExtendWorks {
    fn get_mark_color(&self, index: usize) -> egui::Color32 {
        self.mark_colors[index % self.mark_colors.len()]
    }
}

impl Default for ExtendWorks {
    fn default() -> Self {
        Self {
            requests: Default::default(),
            editor_window: Default::default(),
            scroll_state: Default::default(),
            filter_str: Default::default(),
            filter: Default::default(),
            filter_msg: Default::default(),
            selected: Some(Default::default()),
            test_str: Default::default(),
            test_regex: Default::default(),

            mark_colors: vec![
                egui::Color32::from_rgb(255, 0, 0),
                egui::Color32::from_rgb(255, 255, 0),
                egui::Color32::from_rgb(0, 255, 0),
                egui::Color32::from_rgb(0, 255, 255),
                egui::Color32::from_rgb(0, 0, 255),
                egui::Color32::from_rgb(255, 0, 255),
            ],
        }
    }
}

fn update_mete_comp(
    name: &str,
    struc: &StrucProto,
    ui: &mut egui::Ui,
    table: &AllocateTable,
    h_attrs: &Vec<String>,
    v_attrs: &Vec<String>,
    selected: &mut Vec<String>,
    mark_colors: &Vec<egui::Color32>,
) -> Option<StrucEditing> {
    const OUT_MARGIN: f32 = 0.15;
    let mut result = None;

    ui.allocate_ui_with_layout(
        egui::vec2(PAINT_SIZE, ui.available_height()),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            egui::Frame::none()
                .outer_margin(egui::style::Margin {
                    top: 12.0,
                    ..Default::default()
                })
                .fill(ui.style().visuals.extreme_bg_color)
                .show(ui, |ui| {
                    let (response, painter) =
                        ui.allocate_painter(egui::Vec2::splat(PAINT_SIZE), egui::Sense::click());

                    let h_alloc: Vec<(usize, usize)> = h_attrs
                        .iter()
                        .map(|attr| table.get_weight_in(attr))
                        .collect();
                    let v_alloc: Vec<(usize, usize)> = v_attrs
                        .iter()
                        .map(|attr| table.get_weight_in(attr))
                        .collect();

                    let mut size = IndexSize::new(
                        h_alloc.iter().map(|(_, v)| v).sum(),
                        v_alloc.iter().map(|(_, v)| v).sum(),
                    );
                    if size.is_empty() {
                        return;
                    }

                    let rect = response.rect;
                    let unit = egui::vec2(
                        rect.width() * (1.0 - 2.0 * OUT_MARGIN) / (size.width.max(2) - 1) as f32,
                        rect.height() * (1.0 - 2.0 * OUT_MARGIN) / (size.height.max(2) - 1) as f32,
                    );

                    let offset = egui::vec2(
                        match size.width {
                            1 => rect.width() * 0.5,
                            _ => rect.width() * OUT_MARGIN,
                        },
                        0.0,
                    );
                    let hovered = response.hovered();
                    let stroke = if hovered {
                        painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::WHITE);

                        let bg_stroke = ui.style().visuals.widgets.hovered.fg_stroke;

                        (0..size.width).into_iter().for_each(|n| {
                            let advent = offset + egui::Vec2::X * unit.x * n as f32;
                            painter.line_segment(
                                [rect.left_top() + advent, rect.left_bottom() + advent],
                                bg_stroke,
                            )
                        });
                        let offset = egui::vec2(
                            0.0,
                            match size.height {
                                1 => rect.height() * 0.5,
                                _ => rect.height() * OUT_MARGIN,
                            },
                        );
                        (0..size.height).into_iter().for_each(|n| {
                            let advent = offset + egui::Vec2::Y * unit.y * n as f32;
                            painter.line_segment(
                                [rect.left_top() + advent, rect.right_top() + advent],
                                bg_stroke,
                            )
                        });

                        egui::Stroke::new(3.0, ui.style().visuals.faint_bg_color)
                    } else {
                        egui::Stroke::new(3.0, egui::Color32::WHITE)
                    };

                    size.width -= 1;
                    size.height -= 1;

                    let to_screen = egui::emath::RectTransform::from_to(
                        egui::Rect::from_center_size(
                            egui::pos2(size.width as f32 * 0.5, size.height as f32 * 0.5),
                            egui::vec2(
                                match size.width {
                                    0 => -0.5,
                                    v => v as f32,
                                } / (1.0 - 2.0 * OUT_MARGIN),
                                match size.height {
                                    0 => -0.5,
                                    v => v as f32,
                                } / (1.0 - 2.0 * OUT_MARGIN),
                            ),
                        ),
                        rect,
                    );

                    let struc_work = struc.to_work_in_weight(
                        h_alloc.iter().map(|(_, v)| *v).collect(),
                        v_alloc.iter().map(|(_, v)| *v).collect(),
                    );

                    let mut h_values = vec![];
                    let mut v_values = vec![];

                    let shapes =
                        struc_work
                            .key_paths
                            .iter()
                            .fold(vec![], |mut shapes, key_path| {
                                if key_path.points.len() > 1 {
                                    let stroke = if key_path
                                        .points
                                        .iter()
                                        .find(|p| p.p_type == KeyPointType::Hide)
                                        .is_some()
                                    {
                                        if !hovered {
                                            return shapes;
                                        }
                                        egui::Stroke::new(stroke.width, egui::Color32::GRAY)
                                    } else {
                                        stroke
                                    };
                                    let points = key_path
                                        .points
                                        .iter()
                                        .map(|kp| {
                                            let pos =
                                                to_screen * egui::Pos2::from(kp.point.to_array());
                                            h_values.push(pos.x);
                                            v_values.push(pos.y);
                                            pos
                                        })
                                        .collect();

                                    shapes.push(egui::Shape::Path(eframe::epaint::PathShape {
                                        points,
                                        fill: egui::Color32::TRANSPARENT,
                                        stroke,
                                        closed: key_path.closed,
                                    }));
                                }
                                shapes
                            });

                    h_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    h_values.dedup();
                    let (l, r) = if h_values.len() == 1 {
                        (-0.5 - offset.x, 0.5 + offset.x)
                    } else {
                        (h_values[0] - offset.x, *h_values.last().unwrap() + offset.x)
                    };

                    v_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    v_values.dedup();
                    let (t, b) = if v_values.len() == 1 {
                        (-0.5 - offset.x, 0.5 + offset.x)
                    } else {
                        (v_values[0] - offset.x, *v_values.last().unwrap() + offset.x)
                    };

                    let h_marks: Vec<egui::Shape> = h_values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &v)| {
                            if h_alloc[i].0 == table.table.len() {
                                None
                            } else {
                                Some(egui::Shape::line_segment(
                                    [egui::pos2(v, t), egui::pos2(v, b)],
                                    egui::Stroke::new(
                                        1.5,
                                        mark_colors[h_alloc[i].0 % mark_colors.len()],
                                    ),
                                ))
                            }
                        })
                        .collect();
                    let v_marks: Vec<egui::Shape> = v_values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &v)| {
                            if v_alloc[i].0 == table.table.len() {
                                None
                            } else {
                                Some(egui::Shape::line_segment(
                                    [egui::pos2(l, v), egui::pos2(r, v)],
                                    egui::Stroke::new(
                                        1.5,
                                        mark_colors[v_alloc[i].0 % mark_colors.len()],
                                    ),
                                ))
                            }
                        })
                        .collect();

                    painter.add(h_marks);
                    painter.add(v_marks);
                    painter.add(shapes);

                    if response.clicked_by(egui::PointerButton::Primary) {
                        result = Some(StrucEditing::from_struc(name.to_string(), struc));
                    }
                    if response.clicked_by(egui::PointerButton::Secondary) {
                        if selected.iter().find(|&sn| sn == name).is_none() {
                            selected.push(name.to_string());
                        }
                    }
                });

            if ui.add(egui::Button::new(name).frame(false)).clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }
        },
    );

    result
}

impl Widget<CoreData, RunData> for ExtendWorks {
    fn children<'a>(&'a mut self) -> crate::gui::widget::Children<'a, CoreData, RunData> {
        if let Some(editor_window) = &mut self.editor_window {
            vec![Box::new(editor_window)]
        } else {
            vec![]
        }
    }

    fn start(
        &mut self,
        context: &eframe::CreationContext,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        self.requests = fasing::construct::all_requirements(&core_data.construction)
            .iter()
            .fold(HashMap::new(), |mut map, name| {
                map.insert(
                    name.to_string(),
                    run_data
                        .user_data()
                        .components
                        .get(name)
                        .get_or_insert(&Default::default())
                        .attributes_string(),
                );
                map
            });
        if let Some(selected) = context.storage.unwrap().get_string("extend_works_selected") {
            if let Ok(selected) = serde_json::from_str::<Vec<String>>(selected.as_str()) {
                self.selected.as_mut().unwrap().extend(selected.into_iter())
            }
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            "extend_works_selected",
            serde_json::to_string(&self.selected.as_ref().unwrap()).unwrap(),
        );
    }

    fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        frame: &mut eframe::Frame,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        let panel_color = ui.style().visuals.faint_bg_color.linear_multiply(1.6);
        let bg_stroke_width = ui.style().noninteractive().bg_stroke.width;

        egui::SidePanel::right("Extend works Panel")
            .frame(
                egui::Frame::none()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::style::Margin::symmetric(6.0, 4.0)),
            )
            .default_width(180.0)
            .show_inside(ui, |ui| {
                let style = ui.style_mut();
                style.visuals.widgets.noninteractive.bg_stroke.width = bg_stroke_width;
                style.visuals.faint_bg_color = style.visuals.window_fill.linear_multiply(0.4);

                ui.set_enabled(self.editor_window.is_none());

                egui::CollapsingHeader::new("权重分配表")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("权重分配")
                            .num_columns(2)
                            .spacing([40.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let table = &run_data.user_data().alloc_tab;

                                ui.label("权重").rect;
                                ui.label("规则");
                                ui.end_row();

                                table.table.iter().enumerate().for_each(|(i, wr)| {
                                    // let mut weight = wr.weight;
                                    // ui.add(egui::DragValue::new(&mut weight).speed(1).);
                                    ui.colored_label(self.get_mark_color(i), wr.weight.to_string());
                                    ui.label(wr.regex.as_str());
                                    ui.end_row()
                                });

                                ui.label(table.default.to_string());
                                ui.label("默认");
                                ui.end_row()
                            })
                    });

                let response =
                    egui::CollapsingHeader::new("选中")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.allocate_ui(ui.available_size(), |ui| {
                                ui.horizontal(|ui| {
                                    let mut text = egui::RichText::new("测试");
                                    if self.test_regex.is_some() {
                                        text = text.color(egui::Color32::GREEN);
                                    }

                                    if ui.add(egui::Button::new(text)).clicked() {
                                        self.test_regex = match Regex::new(&self.test_str) {
                                            Ok(reg) => Some(reg),
                                            Err(e) => {
                                                run_data.messages.add_error(e.to_string());
                                                None
                                            }
                                        };
                                    }
                                    if ui.button("停止").clicked()
                                        || ui.text_edit_singleline(&mut self.test_str).changed()
                                    {
                                        self.test_regex = None;
                                    }
                                })
                            });

                            ui.separator();

                            let table = &run_data.user_data().alloc_tab;

                            self.selected = Some(
                                self.selected
                                    .take()
                                    .unwrap()
                                    .into_iter()
                                    .filter(|name| {
                                        let mut remove = false;

                                        ui.horizontal(|ui| {
                                            let response = ui.add(
                                                egui::Label::new(name).sense(egui::Sense::click()),
                                            );
                                            response.context_menu(|ui| {
                                                if ui.button("删除").clicked() {
                                                    remove = true;
                                                    ui.close_menu();
                                                }
                                            });

                                            ui.vertical(|ui| {
                                                let (h, v) = &self.requests[name];
                                                egui::CollapsingHeader::new("横轴")
                                                    .id_source(name.clone() + "横轴")
                                                    .default_open(true)
                                                    .show(ui, |ui| {
                                                        h.iter().for_each(|attr| {
                                                            let ok = if let Some(test) =
                                                                &self.test_regex
                                                            {
                                                                if test.is_match(attr) {
                                                                    ui.colored_label(
                                                                        egui::Color32::WHITE,
                                                                        attr,
                                                                    );
                                                                    true
                                                                } else {
                                                                    false
                                                                }
                                                            } else {
                                                                false
                                                            };

                                                            if !ok {
                                                                match table.match_in_regex(attr) {
                                                                    Some(i) => ui.colored_label(
                                                                        self.get_mark_color(i),
                                                                        attr,
                                                                    ),
                                                                    None => ui.label(attr),
                                                                };
                                                            }
                                                        });
                                                    });
                                                egui::CollapsingHeader::new("竖轴")
                                                    .id_source(name.clone() + "竖轴")
                                                    .default_open(true)
                                                    .show(ui, |ui| {
                                                        v.iter().for_each(|attr| {
                                                            let ok = if let Some(test) =
                                                                &self.test_regex
                                                            {
                                                                if test.is_match(attr) {
                                                                    ui.colored_label(
                                                                        egui::Color32::WHITE,
                                                                        attr,
                                                                    );
                                                                    true
                                                                } else {
                                                                    false
                                                                }
                                                            } else {
                                                                false
                                                            };

                                                            if !ok {
                                                                match table.match_in_regex(attr) {
                                                                    Some(i) => ui.colored_label(
                                                                        self.get_mark_color(i),
                                                                        attr,
                                                                    ),
                                                                    None => ui.label(attr),
                                                                };
                                                            }
                                                        });
                                                    });
                                            });
                                        });
                                        !remove
                                    })
                                    .collect(),
                            );
                        });

                if let Some(response) = response.body_response {
                    response.context_menu(|ui| {
                        if ui.button("清除全部").clicked() {
                            self.selected.as_mut().unwrap().clear();
                            ui.close_menu();
                        };
                    });
                };
            });

        ui.style_mut()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width = 0.0;

        egui::TopBottomPanel::top("Filter Panel")
            .frame(
                egui::Frame::none()
                    .fill(panel_color)
                    .inner_margin(egui::style::Margin::symmetric(6.0, 12.0)),
            )
            .show_inside(ui, |ui| {
                ui.style_mut()
                    .visuals
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .width = bg_stroke_width;
                ui.set_enabled(self.editor_window.is_none());

                ui.horizontal(|ui| {
                    ui.label("属性:");

                    if ui.text_edit_singleline(&mut self.filter_str).changed() {
                        self.filter_msg.clear();
                    };

                    let mut text = egui::RichText::new("过滤");
                    if self.filter.is_some() {
                        text = text.color(egui::Color32::GREEN);
                    }

                    if ui.button(text).clicked() {
                        self.filter = match Regex::new(&self.filter_str) {
                            Ok(re) => Some(re),
                            Err(e) => {
                                self.filter_msg = e.to_string();
                                None
                            }
                        };
                    }
                    if self.filter.is_some() {
                        if ui.button("停止").clicked() {
                            self.filter = None;
                        }
                        if ui.button("测试").clicked() {
                            self.test_str = self.filter_str.clone();
                            self.test_regex = self.filter.clone();
                        }
                    }
                    if !self.filter_msg.is_empty() {
                        ui.colored_label(ui.visuals().error_fg_color, &self.filter_msg);
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::style::Margin::symmetric(12.0, 4.0)),
            )
            .show_inside(ui, |ui| {
                ui.set_enabled(self.editor_window.is_none());
                let mut to_edite = None;
                let mut num_display = 0;

                let scroll = egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            to_edite = run_data.user_data().components.iter().fold(
                                None,
                                |to, (name, struc)| {
                                    let (h_attrs, v_attrs) =
                                        (&self.requests[name].0, &self.requests[name].1);

                                    if let Some(filter) = &self.filter {
                                        if h_attrs
                                            .iter()
                                            .find(|attr| filter.is_match(attr))
                                            .is_none()
                                            && v_attrs
                                                .iter()
                                                .find(|attr| filter.is_match(attr))
                                                .is_none()
                                        {
                                            return to;
                                        }
                                    };

                                    num_display += 1;

                                    if num_display - 1 < self.scroll_state.0
                                        || num_display - 1 >= self.scroll_state.1
                                    {
                                        ui.allocate_space(egui::vec2(
                                            PAINT_SIZE,
                                            PAINT_SIZE + 24.0 + 12.0,
                                        ));
                                        to
                                    } else {
                                        update_mete_comp(
                                            name.as_str(),
                                            struc,
                                            ui,
                                            &run_data.user_data().alloc_tab,
                                            h_attrs,
                                            v_attrs,
                                            self.selected.as_mut().unwrap(),
                                            &self.mark_colors,
                                        )
                                        .or(to)
                                    }
                                },
                            );
                        });
                    });

                if num_display != 0 {
                    let column = (scroll.content_size.x / PAINT_SIZE) as usize;
                    let line_height = scroll.content_size.y as usize * column / num_display;
                    let start = scroll.state.offset.y as usize / line_height * column;
                    let end =
                        (scroll.inner_rect.height() as usize / line_height + 2) * column + start;
                    self.scroll_state = (start, end);
                }

                if to_edite.is_some() {
                    self.editor_window = to_edite;
                }
            });

        if let Some(mut editor_window) = self.editor_window.take() {
            editor_window.update_ui(ui, frame, core_data, run_data);
            if editor_window.run {
                self.editor_window = Some(editor_window);
            } else {
                let attrs = run_data
                    .user_data()
                    .components
                    .get(&editor_window.name)
                    .get_or_insert(&Default::default())
                    .attributes_string();
                self.requests.insert(editor_window.name, attrs);
            }
        }
    }
}
