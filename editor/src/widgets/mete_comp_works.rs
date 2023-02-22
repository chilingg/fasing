use super::struc_editor_window::StrucEditing;
use crate::prelude::*;
use fasing::fas_file::*;

use eframe::egui;
use egui::epaint::PathShape;

use std::collections::HashSet;

const PAINT_SIZE: f32 = 160.0;

struct FilterPanel {
    pub requests: HashSet<String>,
    pub find: String,

    pub empty: bool,
    pub no_empty: bool,
    pub request: bool,
    pub no_request: bool,
    pub sigle_encode: bool,
    pub comb_encode: bool,
}

impl Default for FilterPanel {
    fn default() -> Self {
        Self {
            requests: Default::default(),
            find: Default::default(),
            empty: true,
            no_empty: true,
            request: true,
            no_request: true,
            sigle_encode: true,
            comb_encode: true,
        }
    }
}

impl FilterPanel {
    pub fn filter(&self, name: &str, struc: &StrucProto) -> bool {
        if !self.find.is_empty() && !self.find.contains(name) {
            false
        } else if !self.empty && struc.is_empty() {
            false
        } else if !self.no_empty && !struc.is_empty() {
            false
        } else if !self.request && self.requests.contains(name) {
            false
        } else if !self.no_request && !self.requests.contains(name) {
            false
        } else if !self.sigle_encode && name.chars().count() == 1 {
            false
        } else if !self.comb_encode && name.chars().count() > 1 {
            false
        } else {
            true
        }
    }
}

#[derive(Default)]
pub struct MeteCompWorks {
    editor_window: Option<StrucEditing>,
    filter_panel: FilterPanel,

    pub num_struc: usize,
    pub num_empty: usize,
    pub num_request: usize,
    pub num_display: usize,
    pub drag_target: Option<StrucProto>,
    pub scroll_state: (usize, usize),
}

pub fn struc_to_shape_and_mark(
    struc: &StrucWokr,
    fill: egui::Color32,
    stroke: egui::Stroke,
    mark_stroke: egui::Stroke,
    to_screen: egui::emath::RectTransform,
) -> (Vec<egui::Shape>, Vec<egui::Shape>) {
    fn pos_mark(
        pos: egui::Pos2,
        point_type: u32,
        width: f32,
        mark_stroke: egui::Stroke,
    ) -> egui::Shape {
        match point_type {
            key_point_type::LINE => egui::Shape::rect_stroke(
                egui::Rect::from_center_size(pos, egui::Vec2::splat(width)),
                egui::Rounding::none(),
                mark_stroke,
            ),
            key_point_type::MARK => {
                let half = width * 0.5;
                egui::Shape::Vec(vec![
                    egui::Shape::line_segment(
                        [pos + egui::vec2(-half, -half), pos + egui::vec2(half, half)],
                        mark_stroke,
                    ),
                    egui::Shape::line_segment(
                        [pos + egui::vec2(half, -half), pos + egui::vec2(-half, half)],
                        mark_stroke,
                    ),
                ])
            }
            _ => unreachable!(),
        }
    }
    let mut shapes = vec![];
    let mut marks = vec![];

    struc.key_paths.iter().for_each(|key_path| {
        if key_path.points.len() > 1 {
            let mut points =
                vec![to_screen * egui::Pos2::from(key_path.points[0].point().to_array())];
            marks.push(pos_mark(
                points[0],
                key_path.points[0].point_type(),
                stroke.width * 4.0,
                mark_stroke,
            ));

            points.extend(key_path.points[1..].iter().map(|kp| {
                let p = to_screen * egui::Pos2::from(kp.point().to_array());
                marks.push(pos_mark(
                    p,
                    kp.point_type(),
                    stroke.width * 2.0,
                    mark_stroke,
                ));
                p
            }));

            shapes.push(egui::Shape::Path(PathShape {
                points,
                fill,
                stroke,
                closed: key_path.closed,
            }));
        }
    });

    (shapes, marks)
}

fn update_mete_comp(
    name: &str,
    struc: &StrucProto,
    ui: &mut egui::Ui,
    remove_lsit: &mut Vec<String>,
    change_list: &mut Vec<(String, StrucProto)>,
    requests: &HashSet<String>,
    drag_target: &mut Option<StrucProto>,
) -> Option<StrucEditing> {
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
                    let (response, painter) = ui.allocate_painter(
                        egui::Vec2::splat(PAINT_SIZE),
                        egui::Sense::click_and_drag(),
                    );

                    if response.dragged() {
                        ui.ctx()
                            .output_mut(|o| o.cursor_icon = egui::CursorIcon::Copy);
                    } else if response.drag_released() {
                        *drag_target = Some(struc.clone());
                    }
                    if drag_target.is_some() {
                        if response.hovered() {
                            change_list.push((name.to_owned(), drag_target.take().unwrap()));
                        }
                    }

                    let size = struc.size();
                    let rect = response.rect;
                    let unit = (rect.width() / (size.width + 2) as f32)
                        .min(rect.height() / (size.height + 2) as f32);

                    let (stroke, m_stroke) = if response.hovered() {
                        painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::WHITE);

                        let bg_stroke = ui.style().visuals.widgets.hovered.fg_stroke;

                        let offset = egui::vec2(unit, 0.0);
                        let num = size.width.max(size.height) + 1;
                        (1..=num).into_iter().for_each(|n| {
                            painter.line_segment(
                                [
                                    rect.left_top() + offset * n as f32,
                                    rect.left_bottom() + offset * n as f32,
                                ],
                                bg_stroke,
                            )
                        });
                        let offset = egui::vec2(0.0, unit as f32);
                        (1..=num).into_iter().for_each(|n| {
                            painter.line_segment(
                                [
                                    rect.left_top() + offset * n as f32,
                                    rect.right_top() + offset * n as f32,
                                ],
                                bg_stroke,
                            )
                        });

                        (
                            egui::Stroke::new(3.0, ui.style().visuals.faint_bg_color),
                            egui::Stroke::new(1.5, egui::Color32::LIGHT_RED),
                        )
                    } else {
                        let color = if requests.contains(name) {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::YELLOW
                        };

                        (
                            egui::Stroke::new(3.0, color),
                            egui::Stroke::new(1.5, egui::Color32::DARK_RED),
                        )
                    };

                    let to_screen = egui::emath::RectTransform::from_to(
                        egui::Rect::from_min_size(egui::Pos2::ZERO, rect.size()),
                        rect,
                    );

                    let mut struc_work = struc.to_work();
                    struc_work.transform(WorkVec::splat(unit), WorkVec::splat(unit));

                    let (paths, marks) = struc_to_shape_and_mark(
                        &struc_work,
                        egui::Color32::TRANSPARENT,
                        stroke,
                        m_stroke,
                        to_screen,
                    );

                    painter.add(paths);
                    painter.add(marks);

                    if response.clicked() {
                        result = Some(StrucEditing::from_struc(name.to_string(), struc));
                    }

                    response.context_menu(|ui| {
                        if ui.button("删除").clicked() {
                            remove_lsit.push(name.to_owned());
                            ui.close_menu();
                        }
                        if ui.button("置空").clicked() {
                            change_list.push((name.to_owned(), Default::default()));
                            ui.close_menu();
                        }
                    });
                });
            if ui.add(egui::Button::new(name).frame(false)).clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }
        },
    );

    result
}

impl Widget<CoreData, RunData> for MeteCompWorks {
    fn start(
        &mut self,
        _context: &eframe::CreationContext,
        core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        self.filter_panel.requests = fasing::construct::all_requirements(&core_data.construction);
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
                    ui.label("查找");
                    ui.text_edit_singleline(&mut self.filter_panel.find);
                    if ui.button("×").clicked() {
                        self.filter_panel.find.clear();
                    }

                    ui.separator();

                    ui.checkbox(&mut self.filter_panel.request, "需求");
                    ui.checkbox(&mut self.filter_panel.no_request, "非需求");
                    ui.checkbox(&mut self.filter_panel.empty, "空结构");
                    ui.checkbox(&mut self.filter_panel.no_empty, "非空结构");
                    ui.checkbox(&mut self.filter_panel.sigle_encode, "单字码");
                    ui.checkbox(&mut self.filter_panel.comb_encode, "组合码");
                });
            });

        egui::TopBottomPanel::bottom("Counter")
            .frame(
                egui::Frame::none()
                    .fill(panel_color)
                    .inner_margin(egui::style::Margin::symmetric(6.0, 4.0)),
            )
            .show_inside(ui, |ui| {
                ui.style_mut()
                    .visuals
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .width = bg_stroke_width;
                ui.horizontal(|ui| {
                    ui.label(format!("总计"));
                    if self.num_display != self.num_struc {
                        ui.colored_label(
                            ui.style().visuals.selection.stroke.color,
                            format!("{}/{}", self.num_display, self.num_struc),
                        );
                    } else {
                        ui.label(format!("{}", self.num_struc));
                    }

                    ui.separator();
                    ui.label("需求");
                    if self.num_request == self.filter_panel.requests.len() {
                        ui.label(format!(
                            "{}/{}",
                            self.num_request,
                            self.filter_panel.requests.len()
                        ));
                    } else {
                        ui.colored_label(
                            ui.style().visuals.warn_fg_color,
                            format!("{}/{}", self.num_request, self.filter_panel.requests.len()),
                        );

                        let response = ui.button("生成空结构");
                        if response.clicked() {
                            self.filter_panel.requests.iter().for_each(|name| {
                                run_data
                                    .user_data_mut()
                                    .components
                                    .entry(name.clone())
                                    .or_insert(StrucProto::default());
                            })
                        } else if response.hovered() {
                            let requests: String = self
                                .filter_panel
                                .requests
                                .iter()
                                .filter_map(|name| {
                                    match run_data.user_data().components.contains_key(name) {
                                        true => None,
                                        false => Some(String::from(format!("`{}`", name))),
                                    }
                                })
                                .collect();

                            response.on_hover_text(requests);
                        }
                    }

                    ui.separator();
                    ui.label("空结构");
                    if self.num_empty != 0 {
                        ui.colored_label(
                            ui.style().visuals.warn_fg_color,
                            format!("{}", self.num_empty),
                        );
                        if ui.button("清除").clicked() {
                            run_data
                                .user_data_mut()
                                .components
                                .retain(|_, struc| !struc.is_empty());
                        }
                    } else {
                        ui.label(format!("{}", self.num_empty));
                    }
                })
            });

        ui.style_mut()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width = bg_stroke_width;

        self.num_empty = 0;
        self.num_request = 0;
        self.num_struc = 0;
        self.num_display = 0;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::style::Margin::symmetric(12.0, 4.0)),
            )
            .show_inside(ui, |ui| {
                ui.set_enabled(self.editor_window.is_none());
                let mut to_edite = None;

                let scroll = egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.style_mut()
                            .visuals
                            .widgets
                            .noninteractive
                            .bg_stroke
                            .width = 0.0;

                        ui.horizontal_wrapped(|ui| {
                            let mut remove_list = vec![];
                            let mut change_list = vec![];

                            to_edite = run_data.user_data().components.iter().fold(
                                None,
                                |to, (name, struc)| {
                                    if self.filter_panel.requests.contains(name) {
                                        self.num_request += 1;
                                    }
                                    if struc.is_empty() {
                                        self.num_empty += 1;
                                    }
                                    self.num_struc += 1;

                                    if self.filter_panel.filter(name, struc) {
                                        self.num_display += 1;
                                        if self.num_display - 1 < self.scroll_state.0
                                            || self.num_display - 1 >= self.scroll_state.1
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
                                                &mut remove_list,
                                                &mut change_list,
                                                &self.filter_panel.requests,
                                                &mut self.drag_target,
                                            )
                                            .or(to)
                                        }
                                    } else {
                                        to
                                    }
                                },
                            );

                            remove_list.into_iter().for_each(|name| {
                                run_data.user_data_mut().components.remove(&name);
                            });
                        });
                    });

                let column = (scroll.content_size.x / PAINT_SIZE) as usize;
                let line_height =
                    scroll.content_size.y as usize * column / run_data.user_data().components.len();
                let start = scroll.state.offset.y as usize / line_height * column;
                let end = (scroll.inner_rect.height() as usize / line_height + 1) * column + start;
                self.scroll_state = (start.max(column) - column, end);

                if to_edite.is_some() {
                    self.editor_window = to_edite;
                }
            });

        if let Some(mut editor_window) = self.editor_window.take() {
            editor_window.update_ui(ui, frame, core_data, run_data);
            if editor_window.run {
                self.editor_window = Some(editor_window);
            }
        }
    }

    fn children(&mut self) -> Children {
        if let Some(editor_window) = &mut self.editor_window {
            vec![Box::new(editor_window)]
        } else {
            vec![]
        }
    }
}
