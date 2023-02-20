use super::mete_comp_works::struc_to_shape_and_mark;
use crate::prelude::*;
use fasing::fas_file::*;

use eframe::egui;
use std::collections::{HashMap, HashSet};

enum EditeTool {
    Select {
        clicked: Option<egui::Pos2>,
        points: HashSet<(usize, usize)>,
        moved: Option<egui::Vec2>,
    },
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
        Self::Select {
            clicked: None,
            points: HashSet::new(),
            moved: None,
        }
    }
}

pub struct StrucEditing {
    pub run: bool,
    changed: bool,
    mode: EditeTool,
    name: String,
    paths: StrucWokr,
    msg: &'static str,
}

impl Widget<CoreData, RunData> for StrucEditing {
    fn children<'a>(&'a mut self) -> crate::gui::widget::Children<'a, CoreData, RunData> {
        vec![]
    }

    fn update_ui(
        &mut self,
        ui: &mut egui::Ui,
        _frame: &mut eframe::Frame,
        _core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        let mut open = true;

        egui::Window::new(&self.name)
            .open(&mut open)
            .default_width(StrucEditing::PAINT_SIZE)
            .anchor(egui::Align2::CENTER_CENTER, [0.0; 2])
            .show(ui.ctx(), |ui| {
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
                            egui::Vec2::splat(StrucEditing::PAINT_SIZE),
                            egui::Sense::click(),
                        );

                        let mode_marks = self.mode_process(&response);

                        let m_strokes = egui::Stroke::new(1.5, egui::Color32::LIGHT_RED);
                        let stroke = egui::Stroke::new(4.0, egui::Color32::BLACK);

                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::Vec2::splat(StrucEditing::PAINT_SIZE),
                            ),
                            response.rect,
                        );

                        let (paths, marks) = struc_to_shape_and_mark(
                            &self.paths,
                            egui::Color32::TRANSPARENT,
                            stroke,
                            m_strokes,
                            to_screen,
                        );

                        painter.add(marks);
                        painter.add(paths);
                        painter.add(mode_marks);
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        self.save(&mut run_data.user_data_mut());
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

        if !open {
            self.run = false;
        }
    }
}

impl StrucEditing {
    pub const PAINT_SIZE: f32 = 320.0;

    pub fn from_struc(name: String, struc: &StrucProto) -> Self {
        let size = struc.size();
        let unit = (Self::PAINT_SIZE / (size.width + 2) as f32)
            .min(Self::PAINT_SIZE / (size.height + 2) as f32);
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

    pub fn save(&mut self, data: &mut FasFile) {
        data.components
            .insert(self.name.clone(), self.paths.to_prototype());
        self.changed = false;
        self.msg = "已保存";
    }

    pub fn normalization(&mut self) {
        let proto = self.paths.to_prototype_offset(5.0);
        let size = proto.size();
        let unit = (Self::PAINT_SIZE / (size.width + 2) as f32)
            .min(Self::PAINT_SIZE / (size.height + 2) as f32);
        self.paths = StrucWokr::from_prototype(&proto);
        self.paths
            .transform(WorkVec::splat(unit), WorkVec::splat(unit));
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

        let (shift, pointer) = response
            .ctx
            .input(|input| (input.modifiers.shift_only(), input.pointer.clone()));

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
            egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::splat(StrucEditing::PAINT_SIZE),
            ),
        );
        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::Vec2::splat(StrucEditing::PAINT_SIZE),
            ),
            response.rect,
        );
        let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 255, 255));

        match &mut self.mode {
            EditeTool::Select {
                clicked,
                points,
                moved,
            } => {
                if let Some(click_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    if pointer.primary_clicked() {
                        let mut target = false;
                        'outer: for (i, path) in self.paths.key_paths.iter().enumerate() {
                            for (j, pos) in path.points.iter().enumerate() {
                                let pos = egui::Pos2::from(pos.point().to_array());
                                if egui::Rect::from_center_size(pos, egui::Vec2::splat(CLICK_SIZE))
                                    .contains(click_p)
                                {
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
                    } else if response
                        .ctx
                        .input(|input| input.key_pressed(egui::Key::Delete))
                    {
                        let map = points.iter().fold(HashMap::new(), |mut map, (i, j)| {
                            map.entry(i).or_insert(vec![]).push(j);
                            map
                        });
                        map.into_iter().for_each(|(&n_path, list)| {
                            let path = &mut self.paths.key_paths[n_path];
                            path.points = path
                                .points
                                .iter()
                                .enumerate()
                                .filter_map(
                                    |(i, p)| {
                                        if list.contains(&&i) {
                                            None
                                        } else {
                                            Some(*p)
                                        }
                                    },
                                )
                                .collect();
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
                                    *self.paths.key_paths[*i].points[*j].point_mut() +=
                                        WorkVec::new(moved_vec.x, moved_vec.y);
                                });
                                self.changed = true;

                                moved.replace(delta.to_vec2());
                            } else {
                                clicked.take();
                                moved.take();
                            }
                        } else {
                            let rect = to_screen
                                .transform_rect(egui::Rect::from_two_pos(*click_pos, click_p));
                            if pointer.primary_down() {
                                marks.push(egui::Shape::rect_stroke(
                                    rect,
                                    egui::Rounding::none(),
                                    stroke,
                                ));
                            } else {
                                self.paths
                                    .key_paths
                                    .iter()
                                    .enumerate()
                                    .for_each(|(i, path)| {
                                        path.points.iter().enumerate().for_each(|(j, pos)| {
                                            if rect.contains(
                                                to_screen
                                                    * egui::Pos2::from(pos.point().to_array()),
                                            ) {
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
                    let align_pos =
                        pos - (pos - align_pos.get_or_insert(pos).to_vector()).to_vector() * 0.5;

                    let rect = to_screen.transform_rect(egui::Rect::from_center_size(
                        egui::Pos2::from(pos.to_array()),
                        egui::Vec2::splat(5.0),
                    ));
                    marks.push(egui::Shape::rect_filled(
                        rect,
                        egui::Rounding::none(),
                        stroke.color,
                    ));

                    Some(align_pos)
                });
                if let Some(align_pos) = align_pos {
                    response.ctx.input(|input| {
                        if input.key_pressed(egui::Key::C) {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].point_mut().x = align_pos.x
                            })
                        } else if input.key_pressed(egui::Key::E) {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].point_mut().y = align_pos.y
                            })
                        }
                    })
                }
            }
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
                            } else if response
                                .ctx
                                .input(|input| input.key_pressed(egui::Key::Escape))
                            {
                                if self.paths.key_paths[*n_path].points.len() < 3 {
                                    self.paths.key_paths.remove(*n_path);
                                } else {
                                    self.paths.key_paths[*n_path].points.remove(*n_pos);
                                }
                                *picked = None;
                            }
                        }
                        None => {
                            if pointer.primary_clicked() {
                                let click_rect = egui::Rect::from_center_size(
                                    click_p,
                                    egui::Vec2::splat(CLICK_SIZE),
                                );
                                let mut target = false;
                                for (i, path) in self.paths.key_paths.iter_mut().enumerate() {
                                    if path.points.len() > 1 {
                                        if click_rect.contains(egui::Pos2::from(
                                            path.points[0].point().to_array(),
                                        )) {
                                            target = true;
                                            path.points.insert(0, path.points[0]);
                                            *picked = Some((i, 0));
                                            break;
                                        } else if click_rect.contains(egui::Pos2::from(
                                            path.points.last().unwrap().point().to_array(),
                                        )) {
                                            let n = path.points.len();
                                            target = true;
                                            path.points.insert(n, path.points[n - 1]);
                                            *picked = Some((i, n));
                                            break;
                                        }
                                    }
                                }
                                if !target {
                                    let n = self.paths.key_paths.len();
                                    self.paths.key_paths.insert(
                                        n,
                                        KeyFloatPath::from_lines(
                                            [
                                                WorkPoint::new(click_p.x, click_p.y),
                                                WorkPoint::new(click_p.x, click_p.y),
                                            ],
                                            false,
                                        ),
                                    );
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
}