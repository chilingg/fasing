use crate::gui::prelude::*;
use fasing::fas_file::*;

use egui::epaint::PathShape;

use std::{
    rc::Rc,
    cell::{ RefCell, RefMut },
    collections::{ HashMap, HashSet },
};

enum EditeTool {
    Select{ clicked: Option<egui::Pos2>, points: HashSet<(usize, usize)>, moved: Option<egui::Vec2>},
    Addition(Option<(usize, usize)>),
}

impl PartialEq for EditeTool {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EditeTool::Addition(_), EditeTool::Addition(_)) => true,
            (EditeTool::Select { .. }, EditeTool::Select { .. }) => true,
            _ => false,
        }
    }
}

impl Default for EditeTool {
    fn default() -> Self {
        Self::Select { clicked: None, points: HashSet::new(), moved: None }
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
        const CLICK_SIZE: f32 = 10.0;

        let (shift, pointer) = response.ctx.input(|input| {
            (input.modifiers.shift_only(), input.pointer.clone())
        });

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
        let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 255, 255));

        match &mut self.mode {
            EditeTool::Select { clicked, points, moved } => {
                if let Some(click_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    if pointer.primary_clicked() {
                        let mut target = false;
                        'outer: for (i, path) in self.paths.key_paths.iter().enumerate() {
                            for (j, pos) in path.points.iter().enumerate() {
                                let pos = egui::Pos2::from(pos.point().to_array());
                                if egui::Rect::from_center_size(pos, egui::Vec2::splat(CLICK_SIZE)).contains(click_p) {
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
                    } else if response.ctx.input(|input| input.key_pressed(egui::Key::Delete)) {
                        let map = points.iter().fold(HashMap::new(), |mut map, (i, j)| {
                            map.entry(i).or_insert(vec![]).push(j);
                            map
                        });
                        map.into_iter().for_each(|(&n_path, list)| {
                            let path = &mut self.paths.key_paths[n_path];
                            path.points = path.points.iter().enumerate().filter_map(|(i, p)| {
                                if list.contains(&&i) {
                                    None
                                } else {
                                    Some(*p)
                                }
                            }).collect();
                        });
                        self.paths.key_paths.retain(|path| path.points.len() > 1);

                        points.clear();
                        clicked.take();
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
                    response.ctx.input(|input| {
                        if input.key_pressed(egui::Key::C) {
                            points.iter().for_each(|(i, j)| self.paths.key_paths[*i].points[*j].point_mut().x = align_pos.x )
                        } else if input.key_pressed(egui::Key::E) {
                            points.iter().for_each(|(i, j)| self.paths.key_paths[*i].points[*j].point_mut().y = align_pos.y )
                        }
                    })
                }
            },
            EditeTool::Addition(picked) => {
                if let Some(click_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    match picked {
                        Some((n_path, n_pos)) => {
                            let mut current_p = WorkPoint::new(click_p.x, click_p.y);
                            if shift {
                                let pre_pos = if *n_pos == 0 {
                                    self.paths.key_paths[*n_path].points[1].point()
                                } else {
                                    self.paths.key_paths[*n_path].points[*n_pos - 1].point()
                                };
                                let delta = current_p - pre_pos;

                                if delta.x.abs() > delta.y.abs() {
                                    current_p.y = pre_pos.y;
                                } else {
                                    current_p.x = pre_pos.x;
                                }
                            }
                            *self.paths.key_paths[*n_path].points[*n_pos].point_mut() = current_p;

                            if pointer.primary_clicked() {
                                let path = &mut self.paths.key_paths[*n_path];
                                path.points.insert(*n_pos, path.points[*n_pos]);
                                if *n_pos != 0 {
                                    *n_pos = path.points.len() - 1;
                                }
                            } else if response.ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                                if self.paths.key_paths[*n_path].points.len() < 3 {
                                    self.paths.key_paths.remove(*n_path);
                                } else {
                                    self.paths.key_paths[*n_path].points.remove(*n_pos);
                                }
                                *picked = None;
                            }
                        },
                        None => {
                            if pointer.primary_clicked() {
                                let click_rect = egui::Rect::from_center_size(click_p, egui::Vec2::splat(CLICK_SIZE));
                                let mut target = false;
                                for (i, path) in self.paths.key_paths.iter_mut().enumerate() {
                                    if path.points.len() > 1 {
                                        if click_rect.contains(egui::Pos2::from(path.points[0].point().to_array())) {
                                            target = true;
                                            path.points.insert(0, path.points[0]);
                                            *picked = Some((i, 0));
                                            break;
                                        } else if click_rect.contains(egui::Pos2::from(path.points.last().unwrap().point().to_array())) {
                                            let n = path.points.len();
                                            target = true;
                                            path.points.insert(n, path.points[n-1]);
                                            *picked = Some((i, n));
                                            break;
                                        }
                                    }
                                }
                                if !target {
                                    let n = self.paths.key_paths.len();
                                    self.paths.key_paths.insert(n, KeyFloatPath::from_lines(
                                        [WorkPoint::new(click_p.x, click_p.y), WorkPoint::new(click_p.x, click_p.y)],
                                        false
                                    ));
                                    *picked = Some((n, 1));
                                }
                            }
                        }
                    }
                }
            }
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
                ui.input(|input| {
                    if input.key_pressed(egui::Key::V) {
                        self.mode = EditeTool::default();
                    } else if input.key_pressed(egui::Key::A) {
                        self.mode = EditeTool::Addition(None);
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.mode, EditeTool::default(), "选择");
                    ui.selectable_value(&mut self.mode, EditeTool::Addition(None), "添加");
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
                    if ui.button("退出").clicked() {
                        self.quit();
                    }
                });

                ui.separator();
                ui.label(self.msg);
            });

            if !self.run {
                open = false;
            }

            if open {
                Some(self)
            } else {
                None
            }
    }
}

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
    user_data: Rc<RefCell<UserData>>,
    editor_window: Option<EditingStruc>,
    filter_panel: FilterPanel,

    pub num_struc: usize,
    pub num_empty: usize,
    pub num_request: usize,
    pub num_display: usize,
    pub drag_target: Option<StrucProto>,

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
                egui::Shape::rect_stroke(
                    egui::Rect::from_center_size(points[0], egui::Vec2::splat(stroke.width * 4.0)),
                    egui::Rounding::none(),
                    mark_stroke
                )
            );
            points[1..].iter().for_each(|p| {
                marks.push(
                    egui::Shape::rect_stroke(
                        egui::Rect::from_center_size(*p, egui::Vec2::splat(stroke.width * 2.0)),
                        egui::Rounding::none(),
                        mark_stroke
                    )
                );
            });
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

fn update_mete_comp(
    name: &str,
    struc: &mut StrucProto,
    ui: &mut egui::Ui,
    remove_lsit: &mut Vec<String>,
    requests: &HashSet<String>,
    drag_target: &mut Option<StrucProto>
) -> Option<EditingStruc> {
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
                    let (response, painter) = ui.allocate_painter(egui::Vec2::splat(SIZE), egui::Sense::click_and_drag());

                    if response.dragged() {
                        ui.ctx().output_mut(|o| o.cursor_icon = egui::CursorIcon::Copy);
                    } else if response.drag_released() {
                        *drag_target = Some(struc.clone());
                    }
                    if drag_target.is_some() {
                        if response.hovered() {
                            *struc = drag_target.take().unwrap();
                        }
                    }

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
                            egui::Stroke::new(3.0, ui.style().visuals.faint_bg_color),
                            egui::Stroke::new(1.5, egui::Color32::LIGHT_RED)
                        )
                    } else {
                        let color = if requests.contains(name) {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::YELLOW
                        };
    
                        (egui::Stroke::new(3.0, color),
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

                    response.context_menu(|ui| {
                        if ui.button("删除").clicked() {
                            remove_lsit.push(name.to_owned());
                            ui.close_menu();
                        }
                        if ui.button("置空").clicked() {
                            *struc = Default::default();
                            ui.close_menu();
                        }
                    });
                });
            if ui.add(egui::Button::new(name).frame(false)).clicked() {
                ui.output_mut(|o| o.copied_text = name.to_string());
            }
        });

    result
}

impl Widget for MeteCompWorks {
    fn start(&mut self, app_state: &mut AppState) {
        self.user_data = app_state.user_data.clone();
        self.filter_panel.requests = fasing::construct::all_requirements(&app_state.core_data.construction);
    }

    fn update(&mut self, ctx: &egui::Context, _queue: &mut Vec<Task>) {
        egui::TopBottomPanel::top("Filter Panel")
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.faint_bg_color.linear_multiply(1.6))
                .inner_margin(egui::style::Margin::symmetric(6.0, 12.0))
            )
            .show(ctx, |ui| {
                ui.set_enabled(self.editor_window.is_none());

                ui.horizontal_wrapped(|ui| {
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
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.faint_bg_color.linear_multiply(1.6))
                .inner_margin(egui::style::Margin::symmetric(6.0, 4.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("总计"));
                    if self.num_display != self.num_struc{
                        ui.colored_label(ui.style().visuals.selection.stroke.color, format!("{}/{}", self.num_display, self.num_struc));
                    } else {
                        ui.label(format!("{}", self.num_struc));
                    }

                    ui.separator();
                    ui.label("需求");
                    if self.num_request == self.filter_panel.requests.len() {
                        ui.label(format!("{}/{}", self.num_request, self.filter_panel.requests.len()));
                    } else {
                        ui.colored_label(ui.style().visuals.warn_fg_color, format!("{}/{}", self.num_request, self.filter_panel.requests.len()));

                        let response = ui.button("生成空结构");
                        if response.clicked() {
                            let mut user_data = self.user_data.borrow_mut();

                            self.filter_panel.requests.iter().for_each(|name| {
                                user_data.components.entry(name.clone()).or_insert(StrucProto::default());
                            })
                        } else if response.hovered() {
                            let user_data = self.user_data.borrow();
                            let requests: String = self.filter_panel.requests.iter().filter_map(|name| {
                                match user_data.components.contains_key(name) {
                                    true => None,
                                    false => Some(String::from(format!("`{}`", name)))
                                }
                            }).collect();

                            response.on_hover_text(requests);
                        }
                    }

                    ui.separator();
                    ui.label("空结构");
                    if self.num_empty != 0 {
                        ui.colored_label(ui.style().visuals.warn_fg_color, format!("{}", self.num_empty));
                        if ui.button("清除").clicked() {
                            let mut user_data = self.user_data.borrow_mut();
                            user_data.components.retain(|_, struc| !struc.is_empty());
                        }
                    } else {
                        ui.label(format!("{}", self.num_empty));
                    }
                })
            });

        self.num_empty = 0;
        self.num_request = 0;
        self.num_struc = 0;
        self.num_display = 0;
        
        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(ctx.style().visuals.faint_bg_color)
                .inner_margin(egui::style::Margin::symmetric(12.0, 6.0))
            )
            .show(ctx, |ui| {
                ui.set_enabled(self.editor_window.is_none());
                let mut to_edite = None;

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.style_mut().visuals.widgets.noninteractive.bg_stroke.width = 0.0;
                        ui.horizontal_wrapped(|ui| {
                            let mut user_data = self.user_data.borrow_mut();
                            let mut remove_list = vec![];

                            let mut sorted: Vec<(&String, &mut StrucProto)> = user_data.components.iter_mut().collect();
                            sorted.sort_by_key(|(str, _)| str.clone());

                            to_edite = sorted.into_iter().fold(None, |to, (name, struc)| {
                                if self.filter_panel.requests.contains(name) {
                                    self.num_request += 1;
                                }
                                if struc.is_empty() {
                                    self.num_empty += 1;
                                }
                                self.num_struc += 1;
                                
                                if self.filter_panel.filter(name, struc) {
                                    self.num_display += 1;
                                    update_mete_comp(
                                        name.as_str(),
                                        struc,
                                        ui,
                                        &mut remove_list,
                                        &self.filter_panel.requests,
                                        &mut self.drag_target,
                                    ).or(to)
                                } else {
                                    to
                                }
                            });

                            remove_list.into_iter().for_each(|name| { user_data.components.remove(&name); });
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