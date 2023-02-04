use crate::gui::prelude::*;
use fasing::fas_file::*;

use egui::epaint::PathShape;
use egui::epaint::CircleShape;

use std::rc::Rc;
use std::cell::RefCell;

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
    paths: Vec<Vec<egui::Pos2>>,
    run: bool,
    msg: &'static str,
}

impl EditingStruc {
    pub const PAINT_SIZE: f32 = 320.0;

    pub fn from_struc(name: String, struc: &Structure) -> Self {
        let size = struc.size();
        let unit = (Self::PAINT_SIZE / (size.width + 2) as f32).min(Self::PAINT_SIZE / (size.height + 2) as f32);
        let paths = struc_to_shape(
            &struc,
            egui::Vec2::splat(unit),
            egui::Pos2::ZERO,
        );

        Self {
            changed: false,
            mode: EditeTool::default(),
            name,
            paths,
            run: true,
            msg: "",
        }
    }

    pub fn save(&mut self) {
        todo!()
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

    pub fn input(&mut self, key: we::VirtualKeyCode, state: we::ElementState) -> bool {
        match key {
            we::VirtualKeyCode::Escape if state == we::ElementState::Pressed => {
                self.quit();
                true
            },
            we::VirtualKeyCode::V if state == we::ElementState::Pressed => {
                self.mode = EditeTool::default();
                true
            },
            we::VirtualKeyCode::D if state == we::ElementState::Pressed => {
                self.mode = EditeTool::Delete;
                true
            },
            we::VirtualKeyCode::A if state == we::ElementState::Pressed => {
                self.mode = EditeTool::Addition(None);
                true
            },
            we::VirtualKeyCode::S if state == we::ElementState::Pressed => {
                self.save();
                true
            },
            _ => false
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
                        'outer: for (i, path) in self.paths.iter().enumerate() {
                            for (j, pos) in path.iter().enumerate() {
                                if egui::Rect::from_center_size(*pos, egui::Vec2::splat(10.0)).contains(click_p) {
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
                        } else {
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
                                points.iter().for_each(|(i, j)| self.paths[*i][*j] += (delta - *moved_pos).to_vec2());
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
                                self.paths.iter().enumerate().for_each(|(i, path)| {
                                    path.iter().enumerate().for_each(|(j, pos)| {
                                        if rect.contains(to_screen * *pos) {
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

                points.iter().for_each(|(i, j)| {
                    let rect = to_screen.transform_rect(egui::Rect::from_center_size(self.paths[*i][*j], egui::Vec2::splat(5.0)));
                    marks.push(egui::Shape::rect_filled(rect, egui::Rounding::none(), stroke.color));
                });
            },
            _ => {}
        }

        marks
    }

    pub fn ui(mut self, ctx: &egui::Context) -> Option<Self> {
        let mut open = true;

        egui::Window::new(&self.name)
            .open(&mut open)
            .default_size(egui::Vec2::splat(EditingStruc::PAINT_SIZE))
            .anchor(egui::Align2::CENTER_CENTER, [0.0; 2])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.mode, EditeTool::default(), "选择");
                    ui.selectable_value(&mut self.mode, EditeTool::Addition(None), "添加");
                    ui.selectable_value(&mut self.mode, EditeTool::Delete, "删除");
                });

                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .show(ui, |ui| {
                        let (response, painter) = ui.allocate_painter(
                            egui::Vec2::splat(ui.available_width()),
                            egui::Sense::click()
                        );

                        let mode_marks = self.mode_process(&response);

                        let m_strokes = egui::Stroke::new(1.5, egui::Color32::LIGHT_RED);
                        let stroke = egui::Stroke::new(4.0, egui::Color32::BLACK);

                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::splat(EditingStruc::PAINT_SIZE)),
                            response.rect,
                        );
                        let mut marks = vec![];
                        let paths: Vec<egui::Shape> = self.paths.iter().map(|path| {
                            if let Some(p) = path.get(0) {
                                marks.push(
                                    egui::Shape::circle_stroke(to_screen.transform_pos(*p), stroke.width * 3.0, m_strokes)
                                );
                            }

                            let points = path.clone().into_iter().map(|p| to_screen.transform_pos(p)).collect();
                            egui::Shape::Path(PathShape {
                                points,
                                fill: egui::Color32::TRANSPARENT,
                                closed: false,
                                stroke
                            })
                        }).collect();

                        painter.add(marks);
                        painter.add(paths);
                        painter.add(mode_marks);
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        self.save();
                    }
                    if ui.button("取消").clicked() {
                        self.quit();
                    }
                });

                ui.separator();
                ui.label(self.msg);
            });

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

fn struc_to_shape(
    struc: &Structure,
    unit: egui::Vec2,
    offset: egui::Pos2,
) -> Vec<Vec<egui::Pos2>> {
    let mut shapes = vec![];

    struc.key_points.iter().fold(None, |pre, kp| {
        let to_screen_size = |point: OriginPoint| {
            egui::pos2((point.x + 1) as f32 * unit.x + offset.x, (point.y + 1) as f32 * unit.y + offset.y)
        };

        match pre {
            None => {
                match kp {
                    KeyPoint::Line(p) => {
                        Some(vec![to_screen_size(*p)])
                    },
                    KeyPoint::Break => None,
                }
            },
            Some(mut path) => {
                match kp {
                    KeyPoint::Line(p) => {
                        path.push(to_screen_size(*p));
                        Some(path)
                    },
                    KeyPoint::Break => {
                        shapes.push(path);
                        None
                    }
                }
            }
        }
    });

    shapes
}

fn update_mete_comp(name: &str, struc: &Structure, ui: &mut egui::Ui) -> Option<EditingStruc> {
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

                    let paths = struc_to_shape(
                        struc,
                        egui::Vec2::splat(unit),
                        rect.left_top(),
                    );
                    let marks: Vec<egui::Shape> = 
                        paths.iter().filter_map(|path| path.get(0).map(|p| egui::Shape::Circle(CircleShape {
                            center: *p,
                            radius: stroke.width * 3.0,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: m_stroke,
                        }))).collect();
                    painter.add(marks);
                    painter.add(egui::Shape::Vec(paths.into_iter().map(|points| egui::Shape::Path(
                        PathShape {
                            points,
                            closed: false,
                            fill: egui::Color32::TRANSPARENT,
                            stroke
                        }
                    )).collect()));

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
                            let mut sorted: Vec<(&String, &Structure)> = user_data.components.iter().collect();
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
            self.editor_window = editor_window.ui(&ctx);
        }
    }

    fn process(&mut self, window_event: &we::WindowEvent, _app_state: &mut AppState) -> bool {
        use we::WindowEvent::KeyboardInput;

        if let Some(editor_window) = self.editor_window.as_mut() {
            match window_event {
                KeyboardInput {
                    input: we::KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                    ..
                } => { 
                    editor_window.input(*key, *state)
                },
                _ => false
            }
        } else {
            false
        }
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}