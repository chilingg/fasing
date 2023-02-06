use crate::gui::prelude::*;
use fasing::fas_file::*;

use egui::epaint::PathShape;

use std::rc::Rc;
use std::cell::{ RefCell, RefMut };

enum EditeTool {
    Select{ clicked: Option<egui::Pos2>, points: std::collections::HashSet<(usize, usize)>, moved: Option<egui::Vec2>},
    Addition(Option<(usize, usize)>),
    Delete,
}

impl PartialEq for EditeTool {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EditeTool::Addition(_), EditeTool::Addition(_)) => true,
            (EditeTool::Select { .. }, EditeTool::Select { .. }) => true,
            (EditeTool::Delete, EditeTool::Delete) => true,
            _ => false,
        }
    }
}

impl Default for EditeTool {
    fn default() -> Self {
        Self::Select { clicked: None, points: std::collections::HashSet::new(), moved: None }
    }
}

struct EditingStruc {
    changed: bool,
    mode: EditeTool,
    name: String,
    paths: StrucWokr,
    run: bool,
    msg: &'static str,
}

impl EditingStruc {
    pub const PAINT_SIZE: f32 = 320.0;

    pub fn from_struc(name: String, struc: &StrucProto) -> Self {
        let size = struc.size();
        let unit = (Self::PAINT_SIZE / (size.width + 2) as f32).min(Self::PAINT_SIZE / (size.height + 2) as f32);
        let mut paths = StrucWokr::from_prototype(struc);
        paths.transform(WorkVec::splat(unit), WorkVec::splat(unit));

        Self {
            changed: false,
            mode: EditeTool::default(),
            name,
            paths,
            run: true,
            msg: "",
        }
    }

    pub fn save(&mut self, mut data: RefMut<UserData>) {
        data.components.insert(self.name.clone(), self.paths.to_prototype());
        self.changed = false;
        self.msg = "已保存";
    }

    pub fn normalization(&mut self) {
        let proto = self.paths.to_prototype_offset(5.0);
        let size = proto.size();
        let unit = (Self::PAINT_SIZE / (size.width + 2) as f32).min(Self::PAINT_SIZE / (size.height + 2) as f32);
        self.paths = StrucWokr::from_prototype(&proto);
        self.paths.transform(WorkVec::splat(unit), WorkVec::splat(unit));
    }

    pub fn quit(&mut self) -> bool {
        if self.changed {
            self.msg = "未保存";
            false
        } else {
            self.run = false;
            true
        }
    }

    pub fn mode_process(&mut self, response: &egui::Response) -> Vec<egui::Shape> {
        let shift = response.ctx.input().modifiers.shift;
        let pointer = &response.ctx.input().pointer;
        if let Some(p) = pointer.interact_pos() {
            if !response.rect.contains(p) {
                return vec![];
            }
        } else {
            return vec![];
        }

        let mut marks = vec![];
        let to_work = egui::emath::RectTransform::from_to(
            response.rect,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(EditingStruc::PAINT_SIZE)),
        );
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(EditingStruc::PAINT_SIZE)),
            response.rect,
        );
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 255));

        match &mut self.mode {
            EditeTool::Select { clicked, points, moved } => {
                if let Some(click_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    if pointer.primary_clicked() {
                        let mut target = false;
                        'outer: for (i, path) in self.paths.key_paths.iter().enumerate() {
                            for (j, pos) in path.points.iter().enumerate() {
                                let pos = egui::Pos2::from(pos.point().to_array());
                                if egui::Rect::from_center_size(pos, egui::Vec2::splat(10.0)).contains(click_p) {
                                    target = true;
                                    if !points.contains(&(i, j)) && !shift {
                                        points.clear();
                                    }
                                    points.insert((i, j));
                                    break 'outer;
                                }
                            }
                        }

                        if target {
                            *moved = Some(egui::Vec2::ZERO);
                        } else if !shift {
                            points.clear();
                        }
                        *clicked = Some(click_p);
                    } else if let Some(click_pos) = clicked {
                        if let Some(moved_pos) = moved {
                            if pointer.primary_down() {
                                let mut delta = click_p - click_pos.to_vec2();
                                if shift {
                                    if delta.x.abs() > delta.y.abs() {
                                        delta.y = 0.0;
                                    } else {
                                        delta.x = 0.0
                                    }
                                }
                                points.iter().for_each(|(i, j)| {
                                    let moved_vec = delta - *moved_pos;
                                    *self.paths.key_paths[*i].points[*j].point_mut() += WorkVec::new(moved_vec.x, moved_vec.y);
                                });
                                self.changed = true;

                                moved.replace(delta.to_vec2());
                            } else {
                                clicked.take();
                                moved.take();
                            }
                        } else {
                            let rect = to_screen.transform_rect(egui::Rect::from_two_pos(*click_pos, click_p));
                            if pointer.primary_down() {
                                marks.push(egui::Shape::rect_stroke(rect, egui::Rounding::none(), stroke));
                            } else {
                                self.paths.key_paths.iter().enumerate().for_each(|(i, path)| {
                                    path.points.iter().enumerate().for_each(|(j, pos)| {
                                        if rect.contains(to_screen * egui::Pos2::from(pos.point().to_array())) {
                                            points.insert((i, j));
                                        }
                                    })
                                });
                                clicked.take();
                                moved.take();
                            }
                        }
                    }
                }

                let align_pos = points.iter().fold(None, |mut align_pos, (i, j)| {
                    let pos = self.paths.key_paths[*i].points[*j].point();
                    let align_pos = pos - (pos - align_pos.get_or_insert(pos).to_vector()).to_vector() * 0.5;

                    let rect = to_screen.transform_rect(egui::Rect::from_center_size(
                        egui::Pos2::from(pos.to_array()),
                        egui::Vec2::splat(5.0)
                    ));
                    marks.push(egui::Shape::rect_filled(rect, egui::Rounding::none(), stroke.color));

                    Some(align_pos)
                });
                if let Some(align_pos) = align_pos {
                    if response.ctx.input().key_pressed(egui::Key::C) {
                        points.iter().for_each(|(i, j)| self.paths.key_paths[*i].points[*j].point_mut().x = align_pos.x )
                    } else if response.ctx.input().key_pressed(egui::Key::E) {
                        points.iter().for_each(|(i, j)| self.paths.key_paths[*i].points[*j].point_mut().y = align_pos.y )
                    }
                }
            },
            _ => {}
        }

        marks
    }

    pub fn ui(mut self, ctx: &egui::Context, data: RefMut<UserData>) -> Option<Self> {
        let mut open = true;

        egui::Window::new(&self.name)
            .open(&mut open)
            .default_width(EditingStruc::PAINT_SIZE)
            .anchor(egui::Align2::CENTER_CENTER, [0.0; 2])
            .show(ctx, |ui| {
                if ui.input().key_pressed(egui::Key::V) {
                    self.mode = EditeTool::default();
                } else if ui.input().key_pressed(egui::Key::A) {
                    self.mode = EditeTool::Addition(None);
                } else if ui.input().key_pressed(egui::Key::D) {
                    self.mode = EditeTool::Delete;
                }
                
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.mode, EditeTool::default(), "选择");
                    ui.selectable_value(&mut self.mode, EditeTool::Addition(None), "添加");
                    ui.selectable_value(&mut self.mode, EditeTool::Delete, "删除");
                });

                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .show(ui, |ui| {
                        let (response, painter) = ui.allocate_painter(
                            egui::Vec2::splat(EditingStruc::PAINT_SIZE),
                            egui::Sense::click()
                        );

                        let mode_marks = self.mode_process(&response);

                        let m_strokes = egui::Stroke::new(1.5, egui::Color32::LIGHT_RED);
                        let stroke = egui::Stroke::new(4.0, egui::Color32::BLACK);

                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(EditingStruc::PAINT_SIZE)),
                            response.rect,
                        );

                        let (paths, marks) =
                            struc_to_shape_and_mark(&self.paths, egui::Color32::TRANSPARENT, stroke, m_strokes, to_screen);
                            
                        painter.add(marks);
                        painter.add(paths);
                        painter.add(mode_marks);
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        self.save(data);
                    }
                    if ui.button("标准").clicked() {
                        self.normalization();
                    }
                    ui.separator();
                    if ui.button("取消").clicked() {
                        self.run = false;
                    }
                });

                ui.separator();
                ui.label(self.msg);
            });

            if !open {
                self.quit();
            }

            if self.run {
                Some(self)
            } else {
                None
            }
    }
}

#[derive(Default)]
pub struct MeteCompWorks {
    user_data: Rc<RefCell<UserData>>,
    editor_window: Option<EditingStruc>
}

fn struc_to_shape_and_mark(
    struc: &StrucWokr,
    fill: egui::Color32,
    stroke: egui::Stroke,
    mark_stroke: egui::Stroke,
    to_screen: egui::emath::RectTransform,
) -> (Vec<egui::Shape>, Vec<egui::Shape>) {
    let mut shapes = vec![];
    let mut marks = vec![];

    struc.key_paths.iter().for_each(|key_path| {
        let points = Vec::from_iter(key_path.points.iter().map(|kp| {
            to_screen * egui::Pos2::from(kp.point().to_array())
        }));
        if points.len() > 1 {
            marks.push(
                egui::Shape::circle_stroke(points[0], stroke.width * 3.0, mark_stroke)
            );
            shapes.push(egui::Shape::Path(PathShape {
                points,
                fill,
                stroke,
                closed: key_path.closed
            }));
        }
    });

    (shapes, marks)
}

fn update_mete_comp(name: &str, struc: &StrucProto, ui: &mut egui::Ui) -> Option<EditingStruc> {
    const SIZE: f32 = 160.0;
    let mut result = None;

    ui.allocate_ui_with_layout(
        egui::vec2(SIZE,ui.available_height()),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            egui::Frame::none()
                .outer_margin(egui::style::Margin {
                    top: 12.0,
                    ..Default::default()
                })
                .fill(ui.style().visuals.extreme_bg_color)
                .show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(egui::Vec2::splat(SIZE), egui::Sense::click());

                    let size = struc.size();
                    let rect = response.rect;
                    let unit = (rect.width() / (size.width + 2) as f32).min(rect.height() / (size.height + 2) as f32);

                    let (stroke, m_stroke) = if response.hovered() {
                        painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::WHITE);

                        let bg_stroke = ui.style().visuals.widgets.hovered.fg_stroke;

                        let offset = egui::vec2(unit, 0.0);
                        let num = size.width.max(size.height) + 1;
                        (1..=num).into_iter().for_each(|n| {
                            painter.line_segment([
                                    rect.left_top() + offset * n as f32,
                                    rect.left_bottom() + offset * n as f32
                                ], bg_stroke)
                        });
                        let offset = egui::vec2(0.0, unit as f32,);
                        (1..=num).into_iter().for_each(|n| {
                            painter.line_segment([
                                    rect.left_top() + offset * n as f32,
                                    rect.right_top() + offset * n as f32
                                ], bg_stroke)
                        });
    
                        (
                            egui::Stroke::new(4.0, ui.style().visuals.extreme_bg_color),
                            egui::Stroke::new(1.5, egui::Color32::LIGHT_RED)
                        )
                    } else {
                        (egui::Stroke::new(4.0, egui::Color32::WHITE),
                        egui::Stroke::new(1.5, egui::Color32::DARK_RED))
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
                        to_screen
                    );

                    painter.add(marks);
                    painter.add(paths);

                    if response.clicked() {
                        result = Some(EditingStruc::from_struc(name.to_string(), struc));
                    }
                });
            if ui.add(egui::Button::new(name).frame(false)).clicked() {
                ui.output().copied_text = name.to_string();
            }
        });

    result
}

impl Widget for MeteCompWorks {
    fn start(&mut self, app_state: &mut AppState) {
        self.user_data = app_state.user_data.clone();
    }

    fn update(&mut self, ctx: &egui::Context, _queue: &mut Vec<Task>) {
        let mut to_edite = None;
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.faint_bg_color)
                .inner_margin(egui::style::Margin::same(12.0))
            )
            .show(ctx, |ui| {
                ui.set_enabled(self.editor_window.is_none());
                ui.style_mut().visuals.widgets.noninteractive.bg_stroke.width = 0.0;

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            let user_data = self.user_data.borrow();
                            let mut sorted: Vec<(&String, &StrucProto)> = user_data.components.iter().collect();
                            sorted.sort_by_key(|(str, _)| str.clone());

                            to_edite = sorted.iter().fold(None, |to, (name, struc)| {
                                update_mete_comp(name.as_str(), struc, ui).or(to)
                            });
                    });
                });

                if to_edite.is_some() {
                    self.editor_window = to_edite;
                }
            });
        
        if let Some(editor_window) = self.editor_window.take() {
            let user_data = self.user_data.borrow_mut();
            self.editor_window = editor_window.ui(&ctx, user_data);
        }
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}