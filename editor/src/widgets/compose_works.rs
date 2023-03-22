use super::{
    extend_works::{break_text_in_width, draw_circle},
    struc_editor_window::StrucEditing,
};
use crate::prelude::*;
use fasing::{
    construct,
    fas_file::{ComponetConfig, TransformValue},
    struc::{attribute::StrucAllocates, space::*, StrucVarietys},
    DataHV,
};

use std::fmt::Write;

use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(PartialEq)]
enum TestMode {
    Format,
    CharList,
}

struct CharInfo {
    name: char,
    size: AllocSize,
    alloc: StrucAllocates,
    attrs: construct::Attrs,
    components: Vec<CharInfo>,
}

impl CharInfo {
    fn draft(name: char, attrs: construct::Attrs) -> Self {
        Self {
            name,
            size: Default::default(),
            alloc: Default::default(),
            attrs,
            components: Default::default(),
        }
    }
}

pub struct ComposeWorks {
    editor_window: Option<StrucEditing>,

    test_mode: TestMode,
    test_formats: Vec<(construct::Format, bool)>,
    test_str: String,
    test_chars: BTreeSet<char>,

    cache: HashMap<String, StrucVarietys>,
    config: ComponetConfig,

    comp_box_color: egui::Color32,

    selected: Vec<CharInfo>,
    find: String,

    format_setting: construct::Format,
}

impl Default for ComposeWorks {
    fn default() -> Self {
        Self {
            test_mode: TestMode::Format,
            test_formats: construct::Format::list()
                .iter()
                .enumerate()
                .map(|(i, &f)| match i {
                    0 => (f, true),
                    _ => (f, false),
                })
                .collect(),
            test_str: Default::default(),
            test_chars: Default::default(),
            cache: Default::default(),
            config: Default::default(),
            editor_window: Default::default(),
            comp_box_color: egui::Color32::DARK_RED,
            selected: Default::default(),
            find: Default::default(),
            format_setting: construct::Format::Single,
        }
    }
}

impl ComposeWorks {
    const TEST_CHAR_NUMBUER: usize = 50;

    fn update_cache(&mut self, run_data: &RunData) {
        self.cache = run_data
            .requests_cache
            .iter()
            .map(|(name, attrs)| {
                let user_data = run_data.user_data();
                (
                    name.clone(),
                    StrucVarietys::from_attrs(
                        user_data
                            .components
                            .get(name.as_str())
                            .cloned()
                            .unwrap_or_default(),
                        attrs.clone(),
                        &user_data.alloc_tab,
                    ),
                )
            })
            .collect();
    }

    fn top_panel(&mut self, ui: &mut egui::Ui, run_data: &mut RunData, core_data: &CoreData) {
        ui.style_mut().spacing.item_spacing.y = 4.0;

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.test_mode, TestMode::Format, "格式");
            ui.selectable_value(&mut self.test_mode, TestMode::CharList, "字集");
            ui.separator();
            if ui.button("生成").clicked() {
                match self.test_mode {
                    TestMode::Format => {
                        let formats: HashSet<_> = self
                            .test_formats
                            .iter()
                            .filter_map(|(f, i)| match i {
                                true => Some(*f),
                                false => None,
                            })
                            .collect();
                        self.test_chars = core_data
                            .construction
                            .iter()
                            .filter_map(|(chr, attrs)| match formats.contains(&attrs.format) {
                                true => Some(*chr),
                                false => None,
                            })
                            .collect();
                    }
                    TestMode::CharList => {
                        self.test_chars = self
                            .test_str
                            .chars()
                            .into_iter()
                            .filter(|chr| core_data.construction.contains_key(&chr))
                            .take(60)
                            .collect();
                        self.test_str = self.test_chars.iter().collect();
                    }
                }
            }
            if ui.button("更新").clicked() {
                self.update_cache(run_data);
            }
            ui.separator();
            draw_circle(self.comp_box_color, ui).context_menu(|ui| {
                egui::color_picker::color_picker_color32(
                    ui,
                    &mut self.comp_box_color,
                    egui::widgets::color_picker::Alpha::OnlyBlend,
                );
                if ui.input_mut(|input| input.key_released(egui::Key::Enter)) {
                    ui.close_menu();
                }
            });
            ui.label("部件框");
            ui.separator();
            ui.label("查找");
            ui.text_edit_singleline(&mut self.find);
            if ui.button("×").clicked() {
                self.find.clear();
            }
        });
        ui.separator();
        ui.horizontal(|ui| match self.test_mode {
            TestMode::Format => {
                self.test_formats.iter_mut().for_each(|(format, enable)| {
                    ui.checkbox(enable, format.to_symbol().unwrap_or("单体"));
                });
            }
            TestMode::CharList => {
                ui.label("测试字集");
                ui.label(format!(
                    "{}/{}",
                    self.test_chars.len(),
                    Self::TEST_CHAR_NUMBUER
                ));
                ui.text_edit_singleline(&mut self.test_str);
            }
        });
    }

    fn right_panel(&mut self, ui: &mut egui::Ui, _run_data: &mut RunData, core_data: &CoreData) {
        static BREACES: once_cell::sync::Lazy<Option<[String; 2]>> =
            once_cell::sync::Lazy::new(|| Some(["(".to_string(), ")".to_string()]));

        fn info_panel(ui: &mut egui::Ui, infos: &Vec<CharInfo>, core_data: &CoreData) {
            infos.iter().for_each(|info| {
                let construct_info = match info.attrs.format {
                    construct::Format::Single => "".to_string(),
                    _ => info.attrs.recursion_fmt(
                        info.name.to_string(),
                        &core_data.construction,
                        &BREACES,
                    ),
                };
                ui.horizontal_wrapped(|ui| {
                    let mut alloc_info_h = info.alloc.h.iter().fold(
                        String::with_capacity(info.alloc.h.len() * 2),
                        |mut str, n| {
                            write!(str, "{}+", n).unwrap();
                            str
                        },
                    );
                    alloc_info_h.pop();
                    let mut alloc_info_v = info.alloc.v.iter().fold(
                        String::with_capacity(info.alloc.v.len() * 2),
                        |mut str, n| {
                            write!(str, "{}+", n).unwrap();
                            str
                        },
                    );
                    alloc_info_v.pop();

                    let space_h = info.alloc.h.iter().filter(|n| **n != 0).count();
                    let space_v = info.alloc.v.iter().filter(|n| **n != 0).count();

                    ui.label(format!(
                        "{}{}: ({} * {}); 横轴 {}={}; 竖轴 {}={}",
                        info.name,
                        construct_info,
                        space_h,
                        space_v,
                        info.size.width,
                        alloc_info_h,
                        info.size.height,
                        alloc_info_v
                    ));
                });
                if info.attrs.format != construct::Format::Single {
                    ui.indent(construct_info, |ui| {
                        info_panel(ui, &info.components, core_data);
                    });
                }
                ui.separator();
            });
        }

        ui.collapsing("信息", |ui| {
            info_panel(ui, &self.selected, core_data);
        });
        ui.collapsing("配置", |ui| {
            ui.style_mut().spacing.item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                ui.label("最小值");
                ui.add(
                    egui::DragValue::new(&mut self.config.min_space)
                        .clamp_range(0.02..=0.1)
                        .speed(0.01),
                );
                ui.label("+");
                ui.add(
                    egui::DragValue::new(&mut self.config.increment)
                        .clamp_range(0.0..=(self.config.min_space * 5.0))
                        .speed(0.05),
                );
            });
            if !self.config.limit.h.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("横轴");
                    ui.vertical(|ui| {
                        self.config.limit.h.retain(|n, limit| {
                            ui.indent(format!("横轴分区限制{}", n), |ui| {
                                let mut delete = false;
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} 分区", n));
                                    ui.add(egui::DragValue::new(limit).speed(0.1));
                                    ui.label("最大值");
                                })
                                .response
                                .context_menu(|ui| {
                                    if ui.button("删除").clicked() {
                                        delete = true;
                                        ui.close_menu();
                                    }
                                });
                                !delete
                            })
                            .inner
                        });
                    });
                });
            }
            if !self.config.limit.v.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("竖轴");
                    ui.vertical(|ui| {
                        self.config.limit.v.retain(|n, limit| {
                            ui.indent(format!("竖轴分区限制{}", n), |ui| {
                                let mut delete = false;
                                ui.horizontal(|ui| {
                                    ui.label(format!("{} 分区", n));
                                    ui.add(egui::DragValue::new(limit).speed(0.1));
                                    ui.label("最大值");
                                })
                                .response
                                .context_menu(|ui| {
                                    if ui.button("删除").clicked() {
                                        delete = true;
                                        ui.close_menu();
                                    }
                                });
                                !delete
                            })
                            .inner
                        });
                    });
                });
            }
            ui.horizontal(|ui| {
                let id = ui.make_persistent_id("compose_works_limit_setting");
                let mut subarea = ui.data_mut(|d| d.get_persisted(id).unwrap_or(1));
                ui.label("分区");
                ui.add(egui::DragValue::new(&mut subarea).clamp_range(0..=10));
                if ui.button("横轴+").clicked() {
                    self.config.limit.h.entry(subarea).or_insert(1.0);
                }
                if ui.button("竖轴+").clicked() {
                    self.config.limit.h.entry(subarea).or_insert(1.0);
                }
                ui.data_mut(|d| d.insert_persisted(id, subarea));
            });
            ui.separator();
        });
    }

    fn main_panel(&mut self, ui: &mut egui::Ui, core_data: &CoreData, run_data: &mut RunData) {
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    self.test_chars
                        .iter()
                        .filter(|c| self.find.is_empty() || self.find.contains(**c))
                        .for_each(|chr| {
                            let bg_stroke = ui.style().visuals.widgets.hovered.fg_stroke;
                            let name = chr.to_string();

                            paint::struct_painter(
                                &name,
                                ui,
                                self.selected
                                    .iter()
                                    .find(|info| info.name == *chr)
                                    .is_some(),
                                |rect, painter, response| {
                                    let char_attr = &core_data.construction[chr];

                                    let mut char_info = if response.clicked() {
                                        if response.ctx.input(|i| i.modifiers.shift_only()) {
                                            match self
                                                .selected
                                                .iter()
                                                .position(|info| info.name == *chr)
                                            {
                                                Some(n) => {
                                                    self.selected.remove(n);
                                                    None
                                                }
                                                None => {
                                                    self.selected.push(CharInfo::draft(
                                                        *chr,
                                                        char_attr.clone(),
                                                    ));
                                                    self.selected.last_mut()
                                                }
                                            }
                                        } else {
                                            self.selected.clear();
                                            self.selected
                                                .push(CharInfo::draft(*chr, char_attr.clone()));
                                            self.selected.last_mut()
                                        }
                                    } else {
                                        self.selected.iter_mut().find(|info| info.name == *chr)
                                    };

                                    match char_attr.format {
                                        construct::Format::Single => {
                                            let variety = &self.cache[&name];
                                            let mut char_box = rect
                                                .shrink(rect.width() * paint::STRUCT_OUT_MARGIN);

                                            if response.hovered() || char_info.is_some() {
                                                let size = (1.0 / self.config.min_space).round();
                                                let advance = char_box.width() / size;
                                                (0..=size as usize).for_each(|n| {
                                                    let n = n as f32;
                                                    painter.line_segment(
                                                        [
                                                            char_box.left_top()
                                                                + egui::Vec2::X * n * advance,
                                                            char_box.left_bottom()
                                                                + egui::Vec2::X * n * advance,
                                                        ],
                                                        bg_stroke,
                                                    );
                                                    painter.line_segment(
                                                        [
                                                            char_box.left_top()
                                                                + egui::Vec2::Y * n * advance,
                                                            char_box.right_top()
                                                                + egui::Vec2::Y * n * advance,
                                                        ],
                                                        bg_stroke,
                                                    );
                                                })
                                            }

                                            let length = if variety.proto.tags.contains("top") {
                                                char_box.max.y -= char_box.width() * 0.5;
                                                WorkSize::new(1.0, 0.5)
                                            } else if variety.proto.tags.contains("bottom") {
                                                char_box.min.y += char_box.width() * 0.5;
                                                WorkSize::new(1.0, 0.5)
                                            } else if variety.proto.tags.contains("left") {
                                                char_box.max.x -= char_box.width() * 0.5;
                                                WorkSize::new(0.5, 1.0)
                                            } else if variety.proto.tags.contains("right") {
                                                char_box.min.x += char_box.width() * 0.5;
                                                WorkSize::new(0.5, 1.0)
                                            } else if variety.proto.tags.contains("middle") {
                                                char_box = char_box.shrink(char_box.width() * 0.25);
                                                WorkSize::new(0.5, 0.5)
                                            } else {
                                                WorkSize::new(1.0, 1.0)
                                            };

                                            let trans = match self
                                                .config
                                                .single_allocation(variety.allocs.clone(), length)
                                            {
                                                Ok(trans) => trans,
                                                Err(e) => {
                                                    response.on_hover_text(e.to_string());
                                                    return;
                                                }
                                            };

                                            let size = AllocSize::new(
                                                trans.h.allocs.iter().sum(),
                                                trans.v.allocs.iter().sum(),
                                            );
                                            if let Some(info) = &mut char_info {
                                                info.size = size;
                                                info.alloc = variety.allocs.clone();
                                            }
                                            if size.width == 0 && size.height == 0 {
                                                return;
                                            }

                                            let to_screen = egui::emath::RectTransform::from_to(
                                                egui::Rect::from_min_size(
                                                    egui::pos2(
                                                        match size.width {
                                                            0 => -0.5 * length.width,
                                                            _ => {
                                                                (trans.h.length - length.width)
                                                                    * 0.5
                                                            }
                                                        },
                                                        match size.height {
                                                            0 => -0.5 * length.height,
                                                            _ => {
                                                                (trans.v.length - length.height)
                                                                    * 0.5
                                                            }
                                                        },
                                                    ),
                                                    egui::vec2(length.width, length.height),
                                                ),
                                                char_box,
                                            );
                                            let struc_work =
                                                variety.proto.to_work_in_transform(trans);

                                            let mut marks = vec![];
                                            let mut paths = vec![egui::Shape::rect_stroke(
                                                char_box,
                                                egui::Rounding::none(),
                                                egui::Stroke::new(
                                                    paint::MARK_STROK.width,
                                                    self.comp_box_color,
                                                ),
                                            )];

                                            struc_work.key_paths.into_iter().for_each(|path| {
                                                let mut hide = false;
                                                let points = path
                                                    .points
                                                    .into_iter()
                                                    .map(|kp| {
                                                        let pos = to_screen
                                                            * egui::Pos2::from(kp.point.to_array());
                                                        if let KeyPointType::Mark
                                                        | KeyPointType::Horizontal
                                                        | KeyPointType::Vertical = kp.p_type
                                                        {
                                                            marks.push(paint::pos_mark(
                                                                pos,
                                                                kp.p_type,
                                                                paint::STRUC_STROK_NORMAL.width
                                                                    * 2.0,
                                                                *paint::MARK_STROK,
                                                            ))
                                                        } else if kp.p_type == KeyPointType::Hide {
                                                            hide = true;
                                                        }

                                                        pos
                                                    })
                                                    .collect();
                                                paths.push(egui::Shape::Path(
                                                    eframe::epaint::PathShape {
                                                        points,
                                                        fill: egui::Color32::TRANSPARENT,
                                                        stroke: match response.hovered()
                                                            || char_info.is_some()
                                                        {
                                                            true => match hide {
                                                                true => *paint::MARK_STROK,
                                                                false => {
                                                                    *paint::STRUC_STROK_SELECTED
                                                                }
                                                            },
                                                            false => match hide {
                                                                true => egui::Stroke::NONE,
                                                                false => *paint::STRUC_STROK_NORMAL,
                                                            },
                                                        },
                                                        closed: path.closed,
                                                    },
                                                ));
                                            });

                                            painter.add(paths);
                                            painter.add(marks);

                                            drop(variety);

                                            response.context_menu(|ui| {
                                                if char_attr.format == construct::Format::Single {
                                                    if ui.button("编辑").clicked() {
                                                        self.editor_window =
                                                            Some(StrucEditing::from_struc(
                                                                chr.to_string(),
                                                                &self.cache[&name].proto,
                                                            ));
                                                        ui.close_menu();
                                                    }
                                                }
                                                if ui.button(format!("复制`{}`", chr)).clicked() {
                                                    ui.output_mut(|o| o.copied_text = name.clone());
                                                    ui.close_menu();
                                                }
                                                ui.separator();
                                                ui.menu_button("添加标签", |ui| {
                                                    let add_tagets =
                                                        &run_data.tags - &struc_work.tags;
                                                    add_tagets.iter().for_each(|tag| {
                                                        if ui.button(tag).clicked() {
                                                            self.cache
                                                                .entry(name.clone())
                                                                .and_modify(|struc| {
                                                                    struc
                                                                        .proto
                                                                        .tags
                                                                        .insert(tag.clone());
                                                                });
                                                            run_data
                                                                .user_data_mut()
                                                                .components
                                                                .entry(name.clone())
                                                                .and_modify(|struc| {
                                                                    struc.tags.insert(tag.clone());
                                                                });
                                                            ui.close_menu();
                                                        }
                                                    });
                                                });
                                                struc_work.tags.iter().for_each(|tag| {
                                                    ui.menu_button(tag, |ui| {
                                                        if ui.button("删除").clicked() {
                                                            self.cache
                                                                .entry(name.clone())
                                                                .and_modify(|struc| {
                                                                    struc
                                                                        .proto
                                                                        .tags
                                                                        .remove(tag.as_str());
                                                                });
                                                            run_data
                                                                .user_data_mut()
                                                                .components
                                                                .entry(name.clone())
                                                                .and_modify(|struc| {
                                                                    struc.tags.remove(tag.as_str());
                                                                });
                                                            ui.close_menu();
                                                        }
                                                    });
                                                });
                                            });
                                        }
                                        _ => {}
                                    }
                                },
                            );
                        });
                });
            });
    }
}

impl Widget<CoreData, RunData> for ComposeWorks {
    fn children<'a>(&'a mut self) -> crate::gui::widget::Children<'a, CoreData, RunData> {
        if let Some(editor_window) = &mut self.editor_window {
            vec![Box::new(editor_window)]
        } else {
            vec![]
        }
    }

    fn input_process(
        &mut self,
        input: &mut egui::InputState,
        _core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        if !self.selected.is_empty() {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.selected.clear();
            }
        }
    }

    fn start(
        &mut self,
        context: &eframe::CreationContext,
        _core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        if let Some(tests) = context
            .storage
            .unwrap()
            .get_string("compose_works_tests")
            .as_ref()
        {
            self.test_chars = serde_json::from_str(tests).unwrap_or_default();
        }
        if let Some(color) = context
            .storage
            .unwrap()
            .get_string("compose_works_box_color")
            .as_ref()
        {
            self.comp_box_color = serde_json::from_str(color).unwrap_or_default();
        }
        if let Some(config) = context
            .storage
            .unwrap()
            .get_string("compose_works_config")
            .as_ref()
        {
            self.config = serde_json::from_str(config).unwrap_or_default();
        }
        self.update_cache(run_data);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string(
            "compose_works_tests",
            serde_json::to_string(&self.test_chars.clone()).unwrap(),
        );
        storage.set_string(
            "compose_works_box_color",
            serde_json::to_string(&self.comp_box_color).unwrap(),
        );
        storage.set_string(
            "compose_works_config",
            serde_json::to_string(&self.config).unwrap(),
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

        egui::SidePanel::right("Compose works Panel")
            .frame(
                egui::Frame::none()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::style::Margin::symmetric(6.0, 4.0)),
            )
            .default_width(180.0)
            .show_inside(ui, |ui| {
                ui.set_enabled(self.editor_window.is_none());

                let style = ui.style_mut();
                style.visuals.faint_bg_color = style.visuals.window_fill.linear_multiply(0.4);

                self.right_panel(ui, run_data, core_data);
            });

        ui.style_mut()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .width = 0.0;

        egui::TopBottomPanel::top("Compose settings Panel")
            .frame(
                egui::Frame::none()
                    .fill(panel_color)
                    .inner_margin(egui::style::Margin::symmetric(12.0, 8.0)),
            )
            .show_inside(ui, |ui| {
                ui.set_enabled(self.editor_window.is_none());

                ui.style_mut()
                    .visuals
                    .widgets
                    .noninteractive
                    .bg_stroke
                    .width = bg_stroke_width;

                self.top_panel(ui, run_data, core_data);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::style::Margin::symmetric(12.0, 4.0)),
            )
            .show_inside(ui, |ui| {
                ui.set_enabled(self.editor_window.is_none());
                ui.spacing_mut().item_spacing = egui::Vec2::splat(5.0);

                self.main_panel(ui, core_data, run_data);
            });

        if let Some(mut editor_window) = self.editor_window.take() {
            editor_window.update_ui(ui, frame, core_data, run_data);
            let attrs = run_data.get_comp_attrs(editor_window.name.as_str());
            run_data
                .requests_cache
                .insert(editor_window.name.clone(), attrs.clone());
            self.cache.insert(
                editor_window.name.clone(),
                StrucVarietys::from_attrs(
                    run_data.user_data().components[&editor_window.name].clone(),
                    attrs,
                    &run_data.user_data().alloc_tab,
                ),
            );

            if editor_window.run {
                self.editor_window = Some(editor_window);
            }
        }
    }
}
