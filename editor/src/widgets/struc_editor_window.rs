use super::mete_comp_works::struc_to_shape_and_mark;
use crate::prelude::*;
use fasing::fas_file::*;
use fasing::struc::{space::*, *};

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
    pub name: String,
    paths: StrucWokr,
    key_press: HashSet<egui::Key>,
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
                if self.key_press.remove(&egui::Key::V) {
                    self.mode = EditeTool::default();
                } else if self.key_press.remove(&egui::Key::A) {
                    self.mode = EditeTool::Addition(None);
                }

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

                        let m_strokes = egui::Stroke::new(1.5, egui::Color32::LIGHT_RED);
                        let stroke = egui::Stroke::new(4.0, egui::Color32::BLACK);

                        let to_screen = egui::emath::RectTransform::from_to(
                            egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::Vec2::splat(StrucEditing::PAINT_SIZE),
                            ),
                            response.rect,
                        );

                        let mode_marks = self.mode_process(response);

                        let (paths, marks) = struc_to_shape_and_mark(
                            &self.paths,
                            egui::Color32::TRANSPARENT,
                            stroke,
                            m_strokes,
                            to_screen,
                        );

                        painter.add(paths);
                        painter.add(marks);
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
                        if !self.quit() {
                            run_data
                                .messages
                                .add_warning(format!("退出失败，部件`{}`未保存！", self.name));
                        }
                    }
                });

                ui.separator();
                match self.mode {
                    EditeTool::Select { .. } => ui.label("垂直居中(C) 水平居中(E) 删除(Del)"),
                    EditeTool::Addition(..) => ui.label(""),
                }
            });

        if !open {
            self.run = false;
        }
    }

    fn input_process(
        &mut self,
        input: &mut egui::InputState,
        _core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        const REQUEST_SHORTCUT: [egui::KeyboardShortcut; 6] = [
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::C),
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::E),
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::A),
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::V),
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Delete),
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Escape),
        ];

        REQUEST_SHORTCUT.iter().for_each(|shortcut| {
            if input.consume_shortcut(shortcut) {
                self.key_press.insert(shortcut.key);
            }
        });
    }
}

impl StrucEditing {
    pub const PAINT_SIZE: f32 = 320.0;

    pub fn from_struc(name: String, struc: &StrucProto) -> Self {
        let size = struc.size();
        let unit = (Self::PAINT_SIZE / (size.width + 1) as f32)
            .min(Self::PAINT_SIZE / (size.height + 1) as f32);
        let mut paths = StrucWokr::from_prototype(struc);
        paths.transform(WorkVec::splat(unit), WorkVec::splat(unit));

        Self {
            changed: false,
            mode: EditeTool::default(),
            name,
            paths,
            run: true,
            key_press: Default::default(),
        }
    }

    pub fn save(&mut self, data: &mut FasFile) {
        data.components
            .insert(self.name.clone(), self.paths.to_prototype());
        self.changed = false;
    }

    pub fn normalization(&mut self) {
        let proto = self.paths.to_prototype_offset(5.0);
        let size = proto.size();
        let unit = (Self::PAINT_SIZE / (size.width + 1) as f32)
            .min(Self::PAINT_SIZE / (size.height + 1) as f32);
        self.paths = StrucWokr::from_prototype(&proto);
        self.paths
            .transform(WorkVec::splat(unit), WorkVec::splat(unit));
    }

    pub fn quit(&mut self) -> bool {
        if self.changed {
            false
        } else {
            self.run = false;
            true
        }
    }

    pub fn mode_process(&mut self, response: egui::Response) -> Vec<egui::Shape> {
        const CLICK_SIZE: f32 = 10.0;

        let (shift, alt, pointer) = response.ctx.input(|input| {
            (
                input.modifiers.shift,
                input.modifiers.alt,
                input.pointer.clone(),
            )
        });

        let in_rect = if let Some(p) = pointer.interact_pos() {
            response.rect.contains(p)
        } else {
            false
        };

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
                let mut in_menu = false;
                if !points.is_empty() {
                    response.context_menu(|ui| {
                        in_menu = true;
                        if ui.button("Line").clicked() {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].p_type = KeyPointType::Line;
                            });
                            ui.close_menu();
                            in_menu = false;
                        }
                        if ui.button("Horizontal").clicked() {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].p_type =
                                    KeyPointType::Horizontal;
                            });
                            ui.close_menu();
                            in_menu = false;
                        }
                        if ui.button("Vertical").clicked() {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].p_type = KeyPointType::Vertical;
                            });
                            ui.close_menu();
                            in_menu = false;
                        }
                        if ui.button("Mark").clicked() {
                            points.iter().for_each(|(i, j)| {
                                self.paths.key_paths[*i].points[*j].p_type = KeyPointType::Mark;
                            });
                            ui.close_menu();
                            in_menu = false;
                        }
                        if ui.button("Hide").clicked() {
                            points.iter().for_each(|(i, _)| {
                                self.paths.key_paths[*i].hide();
                            });
                            ui.close_menu();
                            in_menu = false;
                        }
                    });
                }

                if let Some(cursor_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    if pointer.primary_clicked() && in_rect && !in_menu {
                        let mut target = false;
                        'outer: for (i, path) in self.paths.key_paths.iter().enumerate() {
                            for (j, pos) in path.points.iter().enumerate() {
                                let pos = egui::Pos2::from(pos.point.to_array());
                                if egui::Rect::from_center_size(pos, egui::Vec2::splat(CLICK_SIZE))
                                    .contains(cursor_p)
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
                        *clicked = Some(cursor_p);
                    } else if self.key_press.remove(&egui::Key::Delete) {
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
                            if pointer.primary_down() && in_rect {
                                let mut delta = cursor_p - click_pos.to_vec2();
                                if shift {
                                    if delta.x.abs() > delta.y.abs() {
                                        delta.y = 0.0;
                                    } else {
                                        delta.x = 0.0
                                    }
                                }
                                points.iter().for_each(|(i, j)| {
                                    let moved_vec = delta - *moved_pos;
                                    self.paths.key_paths[*i].points[*j].point +=
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
                                .transform_rect(egui::Rect::from_two_pos(*click_pos, cursor_p));
                            if pointer.primary_down() && in_rect {
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
                                                to_screen * egui::Pos2::from(pos.point.to_array()),
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
                    let pos = self.paths.key_paths[*i].points[*j].point;
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
                    if self.key_press.remove(&egui::Key::C) {
                        self.changed = true;
                        points.iter().for_each(|(i, j)| {
                            self.paths.key_paths[*i].points[*j].point.x = align_pos.x
                        })
                    } else if self.key_press.remove(&egui::Key::E) {
                        self.changed = true;
                        points.iter().for_each(|(i, j)| {
                            self.paths.key_paths[*i].points[*j].point.y = align_pos.y
                        })
                    }
                }
            }
            EditeTool::Addition(picked) => {
                if let Some(click_p) = pointer.interact_pos().and_then(|p| Some(to_work * p)) {
                    match picked {
                        Some((n_path, n_pos)) => {
                            let mut current_p = WorkPoint::new(click_p.x, click_p.y);
                            if shift {
                                let pre_pos = if *n_pos == 0 {
                                    self.paths.key_paths[*n_path].points[1].point
                                } else {
                                    self.paths.key_paths[*n_path].points[*n_pos - 1].point
                                };
                                let delta = current_p - pre_pos;

                                if delta.x.abs() > delta.y.abs() {
                                    current_p.y = pre_pos.y;
                                } else {
                                    current_p.x = pre_pos.x;
                                }
                            }
                            self.paths.key_paths[*n_path].points[*n_pos].point = current_p;

                            if pointer.primary_clicked() && in_rect {
                                let path = &mut self.paths.key_paths[*n_path];
                                path.points.insert(*n_pos, path.points[*n_pos]);
                                if *n_pos != 0 {
                                    *n_pos = path.points.len() - 1;
                                }
                            } else if self.key_press.remove(&egui::Key::Escape) {
                                if self.paths.key_paths[*n_path].points.len() < 3 {
                                    self.paths.key_paths.remove(*n_path);
                                } else {
                                    self.paths.key_paths[*n_path].points.remove(*n_pos);
                                }
                                *picked = None;
                            }
                        }
                        None => {
                            if pointer.primary_clicked() && in_rect {
                                let click_rect = egui::Rect::from_center_size(
                                    click_p,
                                    egui::Vec2::splat(CLICK_SIZE),
                                );
                                let mut target = false;
                                for (i, path) in self.paths.key_paths.iter_mut().enumerate() {
                                    if path.points.len() > 1 {
                                        if click_rect.contains(egui::Pos2::from(
                                            path.points[0].point.to_array(),
                                        )) {
                                            target = true;
                                            path.points.insert(0, path.points[0]);
                                            *picked = Some((i, 0));
                                            break;
                                        } else if click_rect.contains(egui::Pos2::from(
                                            path.points.last().unwrap().point.to_array(),
                                        )) {
                                            let n = path.points.len();
                                            target = true;
                                            path.points.insert(n, path.points[n - 1]);
                                            *picked = Some((i, n));
                                            break;
                                        } else {
                                            let mut p1 = path.points[0];
                                            let mut is_intersect = false;
                                            let mut intersect_n = 0;
                                            let mut p_type = KeyPointType::Line;

                                            for (i, p2) in path.points[1..].iter().enumerate() {
                                                is_intersect = intersect(
                                                    p1.point, p2.point, click_p, CLICK_SIZE,
                                                );
                                                if is_intersect {
                                                    intersect_n = i + 1;
                                                    p_type = p1.p_type;
                                                    break;
                                                } else {
                                                    p1 = *p2;
                                                }
                                            }

                                            if is_intersect {
                                                path.points.insert(
                                                    intersect_n,
                                                    KeyPoint::new(
                                                        WorkPoint::new(click_p.x, click_p.y),
                                                        p_type,
                                                    ),
                                                );
                                                target = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if !target {
                                    self.paths.key_paths.push(KeyFloatPath::from_lines(
                                        [
                                            WorkPoint::new(click_p.x, click_p.y),
                                            WorkPoint::new(click_p.x, click_p.y),
                                        ],
                                        false,
                                    ));
                                    if alt {
                                        self.paths.key_paths.last_mut().unwrap().hide();
                                    }
                                    *picked = Some((self.paths.key_paths.len() - 1, 1));
                                }
                                self.changed = true;
                            }
                        }
                    }
                }
            }
        }

        marks
    }
}

fn intersect(p1: WorkPoint, p2: WorkPoint, click_p: egui::Pos2, offset: f32) -> bool {
    let a = p2.y - p1.y;
    let b = p1.x - p2.x;

    if a == 0.0 && b == 0.0 {
        (p1.x - click_p.x).powi(2) + (p1.y - click_p.y).powi(2) < offset.powi(2)
    } else {
        let c = p1.x * -a + p1.y * -b;
        if (a * click_p.x + b * click_p.y + c).abs() / (a.powi(2) + b.powi(2)).sqrt() < offset {
            let mut range_y = [p1.y, p2.y];
            let mut range_x = [p1.x, p2.x];
            range_y.sort_by(|a, b| a.partial_cmp(b).unwrap());
            range_x.sort_by(|a, b| a.partial_cmp(b).unwrap());
            range_x[0] - offset < click_p.x
                && click_p.x < range_x[1] + offset
                && range_y[0] - offset < click_p.y
                && click_p.y < range_y[1] + offset
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersect() {
        assert!(intersect(
            WorkPoint::new(1.0, 1.0),
            WorkPoint::new(10.0, 10.0),
            egui::pos2(5.1, 5.1),
            1.0
        ));
        assert!(!intersect(
            WorkPoint::new(1.0, 2.0),
            WorkPoint::new(10.0, 2.0),
            egui::pos2(-5.1, 2.1),
            1.0
        ));
        assert!(intersect(
            WorkPoint::new(1.0, 2.0),
            WorkPoint::new(10.0, 2.0),
            egui::pos2(5.1, 2.1),
            1.0
        ));
        assert!(intersect(
            WorkPoint::new(1.0, 2.0),
            WorkPoint::new(10.0, 20.0),
            egui::pos2(5.1, 10.1),
            1.0
        ));
        assert!(intersect(
            WorkPoint::new(10.0, 20.0),
            WorkPoint::new(10.0, 2.0),
            egui::pos2(10.1, 2.1),
            1.0
        ));
        assert!(!intersect(
            WorkPoint::new(1.0, 1.0),
            WorkPoint::new(10.0, 10.0),
            egui::pos2(15.1, 15.1),
            1.0
        ));
    }
}
