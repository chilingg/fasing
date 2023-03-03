use super::{mete_comp_works::PAINT_SIZE, struc_editor_window::StrucEditing};
use crate::prelude::*;
use fasing::{
    fas_file::{AllocateTable, WeightRegex},
    struc::*,
};

use regex::Regex;
use std::collections::HashMap;

pub struct ExtendWorks {
    requests: HashMap<String, StrucAttributes>,
    editor_window: Option<StrucEditing>,
    scroll_state: (usize, usize),

    filter_str: String,
    filter: Option<Regex>,
    filter_cache: HashMap<String, (Vec<usize>, Vec<usize>)>,
    filter_msg: String,

    selected: Option<Vec<String>>,
    mark_colors: (Vec<egui::Color32>, Vec<egui::Color32>),

    test_str: String,
    test_regex: Option<Regex>,

    add_reg: String,
    add_weight: usize,
}

impl ExtendWorks {
    const COLORS: [egui::Color32; 6] = [
        egui::Color32::from_rgb(140, 0, 0),
        egui::Color32::from_rgb(140, 140, 0),
        egui::Color32::from_rgb(0, 140, 0),
        egui::Color32::from_rgb(0, 140, 140),
        egui::Color32::from_rgb(0, 0, 140),
        egui::Color32::from_rgb(140, 0, 140),
    ];

    fn set_filter(&mut self, filter: Option<Regex>) {
        if let Some(filter) = &filter {
            let mut buffer = String::new();
            self.filter_cache.clear();

            self.requests.iter().for_each(|(name, attr)| {
                let (h, v) = attr.match_indexes_all(filter, &mut buffer);
                if !h.is_empty() || !v.is_empty() {
                    self.filter_cache.insert(
                        name.to_owned(),
                        self.requests[name].match_indexes_all(&filter, &mut buffer),
                    );
                }
            });
        }
        self.filter = filter;
    }

    fn generate_color(&self) -> egui::Color32 {
        Self::COLORS[(self.mark_colors.0.len() % Self::COLORS.len())]
    }

    fn get_mark_color(&self, index: usize) -> egui::Color32 {
        self.mark_colors.0[index]
    }

    fn change_order_regex(&mut self, index: usize, up: bool, run_data: &mut RunData) {
        let table = &mut run_data.user_data_mut().alloc_tab;
        if up {
            table.swap(index, index - 1);
            self.mark_colors.0.swap(index, index - 1);
            self.mark_colors.1.swap(index, index - 1);
        } else {
            table.swap(index, index + 1);
            self.mark_colors.0.swap(index, index + 1);
            self.mark_colors.1.swap(index, index + 1);
        }
    }

    fn remove_regex(&mut self, index: usize, run_data: &mut RunData) {
        let table = &mut run_data.user_data_mut().alloc_tab;
        table.remove(index);
        self.mark_colors.0.remove(index);
        self.mark_colors.1.remove(index);
    }

    fn add_regex(&mut self, regex: WeightRegex, run_data: &mut RunData) {
        let tab = &mut run_data.user_data_mut().alloc_tab;
        tab.push(regex);
        self.mark_colors.0.push(self.generate_color());
        self.mark_colors.1.push(egui::Color32::TRANSPARENT);
    }

    fn switch_mark_color(&mut self, index: usize) {
        if self.mark_colors.0[index] != egui::Color32::TRANSPARENT {
            self.mark_colors.1[index] = self.mark_colors.0[index];
            self.mark_colors.0[index] = egui::Color32::TRANSPARENT;
        } else if self.mark_colors.1[index] == self.mark_colors.0[index] {
            self.mark_colors.0[index] = self.generate_color();
        } else {
            self.mark_colors.0[index] = self.mark_colors.1[index];
        }
    }

    fn update_requestes(&mut self, run_data: &RunData) {
        self.requests.iter_mut().for_each(|(name, attr)| {
            *attr = run_data
                .user_data()
                .components
                .get(name)
                .get_or_insert(&Default::default())
                .attributes()
        });
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
            filter_cache: Default::default(),
            selected: Some(Default::default()),
            test_str: Default::default(),
            test_regex: Default::default(),
            add_reg: Default::default(),
            add_weight: 1,

            mark_colors: (vec![], vec![]),
        }
    }
}

fn update_mete_comp(
    name: &str,
    struc: &StrucProto,
    ui: &mut egui::Ui,
    table: &AllocateTable,
    attrs: &StrucAttributes,
    h_targets: &Vec<usize>,
    v_targets: &Vec<usize>,
    selected: &mut Vec<String>,
    mark_colors: &Vec<egui::Color32>,
) -> Option<StrucEditing> {
    const OUT_MARGIN: f32 = 0.15;
    const MARK_STROK_WIDTH: f32 = 1.5;
    let mut result = None;
    let mut buffer = String::new();

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

                    let h_alloc: Vec<(usize, usize)> = (0..attrs.horizontal_len())
                        .map(|i| {
                            table.get_weight_in(attrs.get_horizontal_attr(i, &mut buffer).unwrap())
                        })
                        .collect();
                    let v_alloc: Vec<(usize, usize)> = (0..attrs.vertical_len())
                        .map(|i| {
                            table.get_weight_in(attrs.get_vertical_attr(i, &mut buffer).unwrap())
                        })
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
                    v_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    v_values.dedup();

                    let h_marks: Vec<egui::Shape> = h_values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &v)| {
                            let hit = h_targets.iter().find(|&&n| n == i);
                            let mut marks = vec![];

                            if hit.is_some() {
                                marks.push(egui::Shape::line_segment(
                                    [egui::pos2(v, rect.top()), egui::pos2(v, rect.bottom())],
                                    egui::Stroke::new(
                                        stroke.width * 2.0,
                                        egui::Color32::from_rgba_unmultiplied(120, 120, 120, 24),
                                    ),
                                ))
                            }

                            if h_alloc[i].0 != table.len() {
                                marks.push(egui::Shape::line_segment(
                                    [egui::pos2(v, rect.top()), egui::pos2(v, rect.bottom())],
                                    egui::Stroke::new(
                                        MARK_STROK_WIDTH,
                                        mark_colors[h_alloc[i].0 % mark_colors.len()],
                                    ),
                                ))
                            }

                            match marks.len() {
                                0 => None,
                                1 => marks.pop(),
                                _ => Some(marks.into()),
                            }
                        })
                        .collect();
                    let v_marks: Vec<egui::Shape> = v_values
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &v)| {
                            let hit = v_targets.iter().find(|&&n| n == i);
                            let mut marks = vec![];

                            if hit.is_some() {
                                marks.push(egui::Shape::line_segment(
                                    [egui::pos2(rect.left(), v), egui::pos2(rect.right(), v)],
                                    egui::Stroke::new(
                                        stroke.width * 2.0,
                                        egui::Color32::from_rgba_unmultiplied(120, 120, 120, 24),
                                    ),
                                ))
                            }

                            if v_alloc[i].0 != table.len() {
                                marks.push(egui::Shape::line_segment(
                                    [egui::pos2(rect.left(), v), egui::pos2(rect.right(), v)],
                                    egui::Stroke::new(
                                        MARK_STROK_WIDTH,
                                        mark_colors[v_alloc[i].0 % mark_colors.len()],
                                    ),
                                ))
                            }

                            match marks.len() {
                                0 => None,
                                1 => marks.pop(),
                                _ => Some(marks.into()),
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
                        .attributes(),
                );
                map
            });
        if let Some(selected) = context.storage.unwrap().get_string("extend_works_selected") {
            if let Ok(selected) = serde_json::from_str::<Vec<String>>(selected.as_str()) {
                self.selected.as_mut().unwrap().extend(selected.into_iter())
            }
        }
        if let Some(str) = context
            .storage
            .unwrap()
            .get_string("extend_works_filter_str")
        {
            self.filter_str = serde_json::from_str(&str).unwrap();
        }
        if let Some(str) = context
            .storage
            .unwrap()
            .get_string("extend_works_filter_str")
        {
            self.filter_str = serde_json::from_str(&str).unwrap();
        }

        if let Some(str) = context.storage.unwrap().get_string("extend_works_colors") {
            self.mark_colors.0 = serde_json::from_str(&str).unwrap();
        }
        let table = &run_data.user_data().alloc_tab;
        if table.len() > self.mark_colors.0.len() {
            (0..table.len() - self.mark_colors.0.len())
                .for_each(|_| self.mark_colors.0.push(self.generate_color()));
        } else if table.len() < self.mark_colors.0.len() {
            self.mark_colors.0.truncate(table.len());
        }
        self.mark_colors.1 = vec![egui::Color32::TRANSPARENT; table.len()];
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            "extend_works_selected",
            serde_json::to_string(&self.selected.as_ref().unwrap()).unwrap(),
        );
        storage.set_string(
            "extend_works_filter_str",
            serde_json::to_string(&self.filter_str).unwrap(),
        );
        storage.set_string(
            "extend_works_colors",
            serde_json::to_string(&self.mark_colors.0).unwrap(),
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
                        let mut is_changed = false;

                        egui::Grid::new("权重分配")
                            .num_columns(2)
                            .spacing([8.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                let table = &run_data.user_data().alloc_tab;
                                let mut change_op = None;
                                let mut remove_op = None;
                                let mut order_op = None;

                                let ui_size = ui.label("权重").rect.size();
                                ui.label("规则");
                                ui.end_row();

                                table.iter().enumerate().for_each(|(i, wr)| {
                                    let (response, painter) =
                                        ui.allocate_painter(ui_size, egui::Sense::click());
                                    let cur_color = self.get_mark_color(i);
                                    let rect = response.rect;

                                    let stroke = match response.hovered() {
                                        true => ui.visuals().widgets.hovered.fg_stroke,
                                        false => ui.visuals().widgets.inactive.fg_stroke,
                                    };
                                    painter.add(egui::Shape::Circle(eframe::epaint::CircleShape {
                                        center: rect.center(),
                                        radius: 6.0,
                                        fill: cur_color,
                                        stroke,
                                    }));

                                    if response.clicked_by(egui::PointerButton::Primary) {
                                        self.switch_mark_color(i);
                                    }

                                    response.context_menu(|ui| {
                                        egui::color_picker::color_picker_color32(
                                            ui,
                                            &mut self.mark_colors.0[i],
                                            egui::widgets::color_picker::Alpha::OnlyBlend,
                                        );
                                    });

                                    ui.label(wr.weight.to_string()).context_menu(|ui| {
                                        let mut weight = wr.weight;
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut weight)
                                                    .speed(1)
                                                    .clamp_range(0..=12),
                                            )
                                            .changed()
                                        {
                                            change_op = Some((i, weight));
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        if ui.button("↑").clicked() && i != 0 {
                                            order_op = Some((i, true));
                                        }
                                        if ui.button("↓").clicked() && i != table.len() - 1 {
                                            order_op = Some((i, false));
                                        }
                                        if ui.button("×").clicked() {
                                            remove_op = Some(i);
                                        }
                                        ui.separator();
                                        let button =
                                            egui::Button::new(wr.regex.as_str()).frame(false);
                                        if ui.add(button).clicked() {
                                            ui.output_mut(|o| {
                                                o.copied_text = wr.regex.as_str().to_string()
                                            })
                                        }
                                    });
                                    ui.end_row()
                                });

                                drop(table);

                                if let Some((i, weight)) = change_op {
                                    let table = &mut run_data.user_data_mut().alloc_tab;
                                    table[i].weight = weight;
                                    is_changed = true;
                                }
                                if let Some((i, action)) = order_op {
                                    self.change_order_regex(i, action, run_data);
                                    is_changed = true;
                                }
                                if let Some(i) = remove_op {
                                    self.remove_regex(i, run_data);
                                    is_changed = true;
                                }
                            });

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut self.add_weight));
                            ui.add(
                                egui::TextEdit::singleline(&mut self.add_reg).desired_width(120.0),
                            );
                            if ui.button("添加").clicked() {
                                match Regex::new(&self.add_reg) {
                                    Ok(reg) => {
                                        self.add_regex(
                                            WeightRegex::new(reg, self.add_weight),
                                            run_data,
                                        );
                                        is_changed = true;
                                    }
                                    Err(e) => {
                                        run_data.messages.add_error(e.to_string());
                                    }
                                };
                            }
                        });

                        if is_changed {
                            self.update_requestes(&run_data);
                        }
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

                                    let response = ui.add(egui::Button::new(text));
                                    if response.clicked() {
                                        if self.test_regex.is_some() {
                                            self.test_regex = None;
                                        } else {
                                            self.test_regex = match Regex::new(&self.test_str) {
                                                Ok(reg) => Some(reg),
                                                Err(e) => {
                                                    run_data.messages.add_error(e.to_string());
                                                    None
                                                }
                                            };
                                        }
                                    }

                                    ui.allocate_ui(response.rect.size(), |ui| {
                                        ui.set_enabled(self.test_regex.is_some());
                                        if ui.add(egui::Button::new("过滤")).clicked() {
                                            self.set_filter(self.test_regex.clone());
                                            self.filter_str = self.test_str.clone();
                                        }
                                    });
                                    if ui.text_edit_singleline(&mut self.test_str).changed() {
                                        self.test_regex = None;
                                    }
                                })
                            });

                            ui.separator();

                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
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
                                                        egui::Label::new(name)
                                                            .sense(egui::Sense::click()),
                                                    );
                                                    response.context_menu(|ui| {
                                                        if ui.button("删除").clicked() {
                                                            remove = true;
                                                            ui.close_menu();
                                                        }
                                                    });

                                                    let mut buffer = String::new();
                                                    ui.vertical(|ui| {
                                                        let attrs = &self.requests[name];
                                                        egui::CollapsingHeader::new("横轴")
                                                    .id_source(name.clone() + "横轴")
                                                    .default_open(true)
                                                    .show(ui, |ui| {
                                                        (0..attrs.horizontal_len()).for_each(|i| {
                                                            attrs
                                                                .get_horizontal_attr(i, &mut buffer)
                                                                .unwrap();

                                                            let mut color = ui
                                                                .style()
                                                                .visuals
                                                                .widgets
                                                                .noninteractive
                                                                .fg_stroke
                                                                .color;
                                                            if let Some(test) = &self.test_regex {
                                                                if test.is_match(buffer.as_str()) {
                                                                    color = egui::Color32::WHITE;
                                                                }
                                                            } else if let Some(i) = table
                                                                .match_in_regex(buffer.as_str())
                                                            {
                                                                match self.get_mark_color(i) {
                                                                    egui::Color32::TRANSPARENT => {}
                                                                    mark_color => {
                                                                        color = mark_color
                                                                    }
                                                                }
                                                            }

                                                            ui.horizontal_wrapped(|ui| {
                                                                let (rect, _) = ui
                                                                    .allocate_at_least(
                                                                        egui::Vec2::splat(12.0),
                                                                        egui::Sense::hover(),
                                                                    );
                                                                ui.painter().circle_filled(
                                                                    rect.center(),
                                                                    3.0,
                                                                    ui.style()
                                                                        .visuals
                                                                        .widgets
                                                                        .noninteractive
                                                                        .fg_stroke
                                                                        .color,
                                                                );
                                                                ui.style_mut()
                                                                    .spacing
                                                                    .item_spacing
                                                                    .x = 4.0;
                                                                buffer
                                                                    .split_inclusive(';')
                                                                    .for_each(|text| {
                                                                        ui.colored_label(
                                                                            color, text,
                                                                        );
                                                                    });
                                                            });
                                                        });
                                                    });
                                                        egui::CollapsingHeader::new("竖轴")
                                                    .id_source(name.clone() + "竖轴")
                                                    .default_open(true)
                                                    .show(ui, |ui| {
                                                        (0..attrs.vertical_len()).for_each(|i| {
                                                            attrs
                                                                .get_vertical_attr(i, &mut buffer)
                                                                .unwrap();

                                                            let mut color = ui
                                                                .style()
                                                                .visuals
                                                                .widgets
                                                                .noninteractive
                                                                .fg_stroke
                                                                .color;
                                                            if let Some(test) = &self.test_regex {
                                                                if test.is_match(buffer.as_str()) {
                                                                    color = egui::Color32::WHITE;
                                                                }
                                                            } else if let Some(i) = table
                                                                .match_in_regex(buffer.as_str())
                                                            {
                                                                match self.get_mark_color(i) {
                                                                    egui::Color32::TRANSPARENT => {}
                                                                    mark_color => {
                                                                        color = mark_color
                                                                    }
                                                                }
                                                            }

                                                            ui.horizontal_wrapped(|ui| {
                                                                let (rect, _) = ui
                                                                    .allocate_at_least(
                                                                        egui::Vec2::splat(12.0),
                                                                        egui::Sense::hover(),
                                                                    );
                                                                ui.painter().circle_filled(
                                                                    rect.center(),
                                                                    3.0,
                                                                    ui.style()
                                                                        .visuals
                                                                        .widgets
                                                                        .noninteractive
                                                                        .fg_stroke
                                                                        .color,
                                                                );
                                                                ui.style_mut()
                                                                    .spacing
                                                                    .item_spacing
                                                                    .x = 4.0;
                                                                buffer
                                                                    .split_inclusive(';')
                                                                    .for_each(|text| {
                                                                        ui.colored_label(
                                                                            color, text,
                                                                        );
                                                                    });
                                                            });
                                                        });
                                                    });
                                                    });
                                                });
                                                !remove
                                            })
                                            .collect(),
                                    );
                                })
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
                        self.set_filter(None);
                    };

                    let mut text = egui::RichText::new("过滤");
                    if self.filter.is_some() {
                        text = text.color(egui::Color32::GREEN);
                    }

                    if ui.button(text).clicked() {
                        if self.filter.is_some() {
                            self.set_filter(None);
                        } else {
                            match Regex::new(&self.filter_str) {
                                Ok(re) => self.set_filter(Some(re)),
                                Err(e) => {
                                    self.filter_msg = e.to_string();
                                }
                            };
                        }
                    }
                    if self.filter.is_some() {
                        ui.separator();
                        if ui.button("测试").clicked() {
                            self.test_str = self.filter_str.clone();
                            self.test_regex = self.filter.clone();
                        }
                        if ui.button("添加").clicked() {
                            self.add_reg = self.filter_str.clone();
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
                                    let (h_empty, v_empty) = (vec![], vec![]);
                                    let (mut h_targets, mut v_targets) = (&h_empty, &v_empty);

                                    if self.filter.is_some() {
                                        if self.filter_cache.contains_key(name) {
                                            h_targets = &self.filter_cache[name].0;
                                            v_targets = &self.filter_cache[name].1;
                                        } else {
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
                                            &self.requests[name],
                                            h_targets,
                                            v_targets,
                                            self.selected.as_mut().unwrap(),
                                            &self.mark_colors.0,
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
            let attrs = run_data
                .user_data()
                .components
                .get(&editor_window.name)
                .get_or_insert(&Default::default())
                .attributes();
            if let Some(filter) = &self.filter {
                let mut buf = String::new();
                let (h, v) = attrs.match_indexes_all(&filter, &mut buf);
                if !h.is_empty() || !v.is_empty() {
                    self.filter_cache
                        .insert(editor_window.name.to_owned(), (h, v));
                }
            }
            self.requests.insert(editor_window.name.clone(), attrs);

            if editor_window.run {
                self.editor_window = Some(editor_window);
            }
        }
    }
}
