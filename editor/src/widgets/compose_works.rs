use super::{extend_works::draw_circle, struc_editor_window::StrucEditing};
use crate::prelude::*;
use fasing::{
    construct::{self, Component, Format},
    fas_file::{ComponetConfig, Error, TransformValue, WeightRegex},
    struc::{
        self,
        attribute::{StrucAllocates, StrucAttributes},
        space::*,
        StrucVarietys,
    },
    DataHV,
};

use std::fmt::Write;

use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(PartialEq)]
enum TestMode {
    Format,
    CharList,
}

#[derive(Clone)]
struct CharInfo {
    name: char,
    size: AllocSize,
    alloc: StrucAllocates,
    attrs: construct::Attrs,
    info: String,
}

impl CharInfo {
    fn draft(name: char, attrs: construct::Attrs) -> Self {
        Self {
            name,
            size: Default::default(),
            alloc: Default::default(),
            attrs,
            info: Default::default(),
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
}

impl Default for ComposeWorks {
    fn default() -> Self {
        Self {
            test_mode: TestMode::Format,
            test_formats: Format::list()
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
            comp_box_color: egui::Color32::from_rgba_unmultiplied(32, 16, 16, 86),
            selected: Default::default(),
            find: Default::default(),
        }
    }
}

impl ComposeWorks {
    const TEST_CHAR_NUMBUER: usize = 50;

    fn top_panel(&mut self, ui: &mut egui::Ui, _run_data: &mut RunData, core_data: &CoreData) {
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
                self.cache.clear();
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

    fn right_panel(&mut self, ui: &mut egui::Ui, run_data: &mut RunData, core_data: &CoreData) {
        static BREACES: once_cell::sync::Lazy<Option<[String; 2]>> =
            once_cell::sync::Lazy::new(|| Some(["(".to_string(), ")".to_string()]));

        ui.collapsing("信息", |ui| {
            self.selected.iter().for_each(|info| {
                let construct_info = match info.attrs.format {
                    Format::Single => "".to_string(),
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

                    if !info.info.is_empty() {
                        ui.label(info.info.as_str()).context_menu(|ui| {
                            if ui.button("复制").clicked() {
                                ui.output_mut(|o| o.copied_text = info.info.clone());
                                ui.close_menu();
                            }
                        });
                    }
                });
            });
            ui.separator();
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
                ui.label("~");
                let mut min_max = self.config.min_space + self.config.increment;
                if ui
                    .add(
                        egui::DragValue::new(&mut min_max)
                            .clamp_range(self.config.min_space..=(self.config.min_space * 5.0))
                            .speed(0.05),
                    )
                    .changed()
                {
                    self.config.increment = min_max - self.config.min_space;
                }
            });

            ui.horizontal(|ui| {
                ui.label("间隙");
                if ui.button("+").clicked() {
                    self.config
                        .interval_judge
                        .push(WeightRegex::new(regex::Regex::new("^$").unwrap(), 0.0));
                }
                ui.add(egui::Separator::default().horizontal());
            });
            let interval_id = egui::Id::new("Interval judge");
            let mut drag_target = None;
            let mut drop_target = None;
            let mut delete = None;
            self.config
                .interval_judge
                .iter_mut()
                .enumerate()
                .for_each(|(i, wr)| {
                    let item_id = interval_id.with(i);
                    ui.indent(item_id, |ui| {
                        ui.horizontal(|ui| {
                            paint::orger_drag_drop(
                                ui,
                                interval_id,
                                i,
                                &mut drag_target,
                                &mut drop_target,
                            );
                            ui.add(
                                egui::DragValue::new(&mut wr.weight)
                                    .clamp_range(0.0..=3.0)
                                    .speed(0.1),
                            );
                            paint::regex_edite_label(
                                format!("Interval judge reg {}", i).as_str(),
                                &mut wr.regex,
                                ui,
                            )
                            .context_menu(|ui| {
                                if ui.button("删除").clicked() {
                                    delete = Some(i);
                                    ui.close_menu();
                                }
                            });
                        })
                        .response;
                    });
                });
            if let Some(drag_target_n) = drag_target {
                if let Some(drop_target_n) = drop_target {
                    let vec: Vec<_> = if drop_target_n > drag_target_n {
                        (drag_target_n..=drop_target_n).collect()
                    } else {
                        (drop_target_n..=drag_target_n).rev().collect()
                    };
                    vec.iter().zip(vec.iter().skip(1)).for_each(|(&n, &m)| {
                        self.config.interval_judge.swap(n, m);
                    });
                }
            }
            if let Some(delete) = delete {
                self.config.interval_judge.remove(delete);
            }

            ui.horizontal(|ui| {
                ui.label("格式限制");
                ui.menu_button("+", |ui| {
                    construct::Format::list().iter().for_each(|f| {
                        if !self.config.format_limit.contains_key(f) {
                            if ui.button(f.to_symbol().unwrap_or("单体")).clicked() {
                                self.config.format_limit.insert(*f, Default::default());
                                ui.close_menu();
                            }
                        }
                    })
                });
                ui.add(egui::Separator::default().horizontal());
            });
            if !self.config.format_limit.is_empty() {
                let format_setting_id = egui::Id::new("compose_works_format_limit_setting");
                let (mut state, mut group_add_str): (construct::Format, String) =
                    ui.data_mut(|d| {
                        d.get_temp(format_setting_id).unwrap_or((
                            *self.config.format_limit.first_key_value().unwrap().0,
                            String::new(),
                        ))
                    });

                ui.horizontal(|ui| {
                    let fmt_symbol = state.to_symbol().unwrap_or("单体");
                    let mut delete = None;
                    self.config
                        .format_limit
                        .iter_mut()
                        .for_each(|(fmt, settings)| {
                            ui.selectable_value(&mut state, *fmt, fmt_symbol)
                                .context_menu(|ui| {
                                    (0..fmt.number_of())
                                        .filter(|n| !settings.contains_key(n))
                                        .collect::<Vec<usize>>()
                                        .into_iter()
                                        .for_each(|in_fmt| {
                                            if ui.button(format!("添加 {}", in_fmt)).clicked() {
                                                settings.insert(in_fmt, Default::default());
                                                ui.close_menu();
                                            }
                                        });
                                    settings.retain(|in_fmt, _| {
                                        if ui.button(format!("删除 {}", in_fmt)).clicked() {
                                            ui.close_menu();
                                            false
                                        } else {
                                            true
                                        }
                                    });
                                    if ui.button("删除").clicked() {
                                        delete = Some(*fmt);
                                        ui.close_menu();
                                    }
                                });
                        });
                    if let Some(delete) = delete {
                        self.config.format_limit.remove(&delete);
                    }
                });

                ui.style_mut().visuals.collapsing_header_frame = false;
                self.config
                    .format_limit
                    .get_mut(&state)
                    .unwrap()
                    .iter_mut()
                    .for_each(|(in_fmt, settings)| {
                        egui::CollapsingHeader::new(in_fmt.to_string())
                            .id_source(format_setting_id.with(in_fmt))
                            .show(ui, |ui| {
                                let mut delete = None;
                                settings
                                    .iter_mut()
                                    .enumerate()
                                    .for_each(|(i, (group, size))| {
                                        egui::CollapsingHeader::new(format!("组 {}", i))
                                            .id_source(ui.id().with(i))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label("宽");
                                                    ui.add(
                                                        egui::DragValue::new(&mut size.width)
                                                            .clamp_range(0.0..=1.0),
                                                    );
                                                    ui.label("高");
                                                    ui.add(
                                                        egui::DragValue::new(&mut size.height)
                                                            .clamp_range(0.0..=1.0),
                                                    );
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    group.retain(|name| {
                                                        let mut delete = false;
                                                        ui.add(
                                                            egui::Button::new(name).frame(false),
                                                        )
                                                        .context_menu(|ui| {
                                                            if ui.button("删除").clicked() {
                                                                delete = true;
                                                                ui.close_menu();
                                                            }
                                                        });
                                                        !delete
                                                    })
                                                });
                                            })
                                            .header_response
                                            .context_menu(|ui| {
                                                if ui.button("删除").clicked() {
                                                    delete = Some(i);
                                                    ui.close_menu();
                                                }
                                                ui.horizontal(|ui| {
                                                    ui.label("添加");
                                                    if ui
                                                        .text_edit_singleline(&mut group_add_str)
                                                        .lost_focus()
                                                    {
                                                        if !group_add_str.is_empty() {
                                                            group.insert(group_add_str.clone());
                                                        }
                                                        ui.close_menu();
                                                    }
                                                });
                                            });
                                    });
                                if let Some(delete) = delete {
                                    settings.remove(delete);
                                }
                            })
                            .header_response
                            .context_menu(|ui| {
                                if ui.button("添加").clicked() {
                                    settings.push((Default::default(), WorkSize::new(1.0, 1.0)));
                                    ui.close_menu();
                                }
                            });
                    });
                ui.data_mut(|d| d.insert_temp(format_setting_id, (state, group_add_str)));
            }

            ui.horizontal(|ui| {
                ui.label("部件映射");
                ui.menu_button("+", |ui| {
                    construct::Format::list().iter().for_each(|f| {
                        if !self.config.replace_list.contains_key(f) {
                            if ui.button(f.to_symbol().unwrap_or("单体")).clicked() {
                                self.config.replace_list.insert(*f, Default::default());
                                ui.close_menu();
                            }
                        }
                    })
                });
                ui.add(egui::Separator::default().horizontal());
            });
            if !self.config.replace_list.is_empty() {
                let replace_setting_id = ui.id().with("compose_works_replace_setting");
                let (mut state, mut from_str, mut to_str): (construct::Format, String, String) = ui
                    .data_mut(|d| {
                        d.get_temp(replace_setting_id).unwrap_or((
                            *self.config.replace_list.first_key_value().unwrap().0,
                            String::new(),
                            String::new(),
                        ))
                    });

                ui.horizontal(|ui| {
                    let fmt_symbol = state.to_symbol().unwrap_or("单体");
                    let mut delete = None;
                    self.config
                        .replace_list
                        .iter_mut()
                        .for_each(|(fmt, settings)| {
                            ui.selectable_value(&mut state, *fmt, fmt_symbol)
                                .context_menu(|ui| {
                                    (0..fmt.number_of())
                                        .filter(|n| !settings.contains_key(n))
                                        .collect::<Vec<usize>>()
                                        .into_iter()
                                        .for_each(|in_fmt| {
                                            if ui.button(format!("添加 {}", in_fmt)).clicked() {
                                                settings.insert(in_fmt, Default::default());
                                                ui.close_menu();
                                            }
                                        });
                                    settings.retain(|in_fmt, _| {
                                        if ui.button(format!("删除 {}", in_fmt)).clicked() {
                                            ui.close_menu();
                                            false
                                        } else {
                                            true
                                        }
                                    });
                                    if ui.button("删除").clicked() {
                                        delete = Some(*fmt);
                                        ui.close_menu();
                                    }
                                });
                        });
                    if let Some(delete) = delete {
                        self.config.replace_list.remove(&delete);
                    }
                });

                ui.visuals_mut().collapsing_header_frame = false;
                self.config
                    .replace_list
                    .get_mut(&state)
                    .unwrap()
                    .iter_mut()
                    .for_each(|(in_fmt, settings)| {
                        egui::CollapsingHeader::new(in_fmt.to_string())
                            .id_source(replace_setting_id.with(in_fmt))
                            .show(ui, |ui| {
                                settings.retain(|from, to| {
                                    let mut delete = false;
                                    ui.label(format!("{} -> {}", from, to)).context_menu(|ui| {
                                        if ui.button("删除").clicked() {
                                            delete = true;
                                            ui.close_menu();
                                        }
                                    });
                                    !delete
                                })
                            })
                            .header_response
                            .context_menu(|ui| {
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut from_str)
                                            .desired_width(48.0),
                                    );
                                    ui.label("->");
                                    ui.text_edit_singleline(&mut to_str);
                                    if ui.button("+").clicked() {
                                        ui.close_menu();
                                        settings.insert(from_str.clone(), to_str.clone());
                                    }
                                });
                            });
                    });
                ui.data_mut(|d| d.insert_temp(replace_setting_id, (state, from_str, to_str)));
            }

            let limit_setting_id = ui.make_persistent_id("compose_works_limit_setting");
            let mut limit_setting = ui.data_mut(|d| d.get_persisted(limit_setting_id).unwrap_or(1));

            ui.horizontal(|ui| {
                ui.label("分区限制");
                ui.add(egui::DragValue::new(&mut limit_setting).clamp_range(1..=10));
                if ui.button("横轴+").clicked() {
                    self.config.limit.h.entry(limit_setting).or_insert(1.0);
                }
                if ui.button("竖轴+").clicked() {
                    self.config.limit.v.entry(limit_setting).or_insert(1.0);
                }
                ui.add(egui::Separator::default().horizontal());

                ui.data_mut(|d| d.insert_persisted(limit_setting_id, limit_setting));
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
                                    ui.add(egui::DragValue::new(limit).speed(0.01));
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
                ui.label("缩减:");
                paint::regex_edite_label("reduce regex endite", &mut self.config.reduce_check, ui)
            });

            ui.separator();
        });

        let mut remove_list = vec![];
        let mut remove_tag = None;
        let tag_setting_id = ui.make_persistent_id("Tag setting");
        ui.collapsing("标签管理", |ui| {
            run_data.tags.iter().for_each(|(tag, items)| {
                ui.style_mut().visuals.collapsing_header_frame = false;
                egui::CollapsingHeader::new(format!("{} ({})", tag, items.len()))
                    .id_source(ui.id().with(tag))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            items.iter().for_each(|item| {
                                ui.add(egui::Button::new(item).frame(false))
                                    .context_menu(|ui| {
                                        if ui.button("删除").clicked() {
                                            remove_list.push((tag.to_string(), item.to_string()));
                                            ui.close_menu();
                                        }
                                        if ui.button("复制").clicked() {
                                            ui.output_mut(|o| o.copied_text = item.to_string());
                                            ui.close_menu();
                                        }
                                    });
                            })
                        })
                    })
                    .header_response
                    .context_menu(|ui| {
                        if ui.button("清空").clicked() {
                            items.iter().for_each(|name| {
                                remove_list.push((tag.to_string(), name.to_string()))
                            });
                            ui.close_menu();
                        }
                        ui.set_enabled(items.is_empty());
                        if ui.button("删除").clicked() {
                            remove_tag = Some(tag.to_string());
                            ui.close_menu();
                        }
                    });
            });
            remove_list
                .into_iter()
                .for_each(|(tag, name)| run_data.remove_comp_tag(name, tag));
            remove_tag.iter().for_each(|tag| {
                run_data.tags.remove(tag);
            });

            ui.separator();
        })
        .body_response
        .and_then(|response| {
            response.context_menu(|ui| {
                if ui.button("新建").clicked() {
                    ui.data_mut(|d| {
                        d.insert_temp::<String>(tag_setting_id, "New Tag".to_string());
                    });
                    ui.close_menu();
                }
            });
            None::<egui::Response>
        });

        if let Some(mut new_tag) = ui.data_mut(|d| d.get_temp::<String>(tag_setting_id)) {
            let pos = ui.input(|i| i.pointer.interact_pos()).unwrap_or_default();
            let mut open = true;
            let response = egui::Window::new("New Tag")
                .collapsible(false)
                .default_pos(pos)
                .resizable(false)
                .open(&mut open)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        let built_in_tags: Vec<_> = construct::Format::list()
                            .iter()
                            .flat_map(|f| {
                                (1..=f.number_of()).filter_map(|n| {
                                    let tag = format!("{}{}", f.to_symbol().unwrap_or("单体"), n);
                                    match run_data.tags.contains_key(tag.as_str()) {
                                        true => None,
                                        false => Some(tag),
                                    }
                                })
                            })
                            .collect();

                        ui.menu_button(">", |ui| {
                            built_in_tags.into_iter().for_each(|tag| {
                                if ui.button(tag.as_str()).clicked() {
                                    new_tag = tag;
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.add(egui::TextEdit::singleline(&mut new_tag).desired_width(180.0));
                        if ui.button("+").clicked() && !run_data.tags.contains_key(new_tag.as_str())
                        {
                            run_data.tags.insert(new_tag.clone(), Default::default());
                            true
                        } else {
                            false
                        }
                    })
                    .inner
                });
            if let Some(response) = response {
                if !open || response.inner.unwrap() {
                    ui.data_mut(|d| d.remove::<String>(tag_setting_id));
                } else {
                    ui.data_mut(|d| d.insert_temp(tag_setting_id, new_tag));
                }
            }
        }
    }

    fn single_construct(
        &mut self,
        name: String,
        char_box: &mut egui::Rect,
        char_info: &mut Option<CharInfo>,
        mut show_hidden: bool,
        run_data: &mut RunData,
    ) -> Result<egui::Shape, Error> {
        let variety = Self::get_variety(
            &mut self.cache,
            name.clone(),
            Format::Single,
            0,
            0,
            run_data,
        )?;

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
            let amnt = egui::vec2(char_box.width() * 0.25, char_box.height() * 0.25);
            *char_box = egui::Rect::from_min_max(char_box.min + amnt, char_box.max - amnt);
            WorkSize::new(0.5, 0.5)
        } else {
            WorkSize::new(1.0, 1.0)
        };

        let trans = self
            .config
            .single_allocation(variety.allocs.clone(), length)?;
        let size = AllocSize::new(trans.h.allocs.iter().sum(), trans.v.allocs.iter().sum());

        if let Some(info) = char_info {
            info.size = size;
            info.alloc = variety.allocs.clone();
            show_hidden = true;
        }
        if size.width == 0 && size.height == 0 {
            return Err(Error::Empty(name));
        }

        let to_screen = egui::emath::RectTransform::from_to(
            egui::Rect::from_min_size(
                egui::pos2(
                    match size.width {
                        0 => -0.5 * length.width,
                        _ => (trans.h.length - length.width) * 0.5,
                    },
                    match size.height {
                        0 => -0.5 * length.height,
                        _ => (trans.v.length - length.height) * 0.5,
                    },
                ),
                egui::vec2(length.width, length.height),
            ),
            *char_box,
        );
        let struc_work = variety.proto.to_work_in_transform(trans);

        let mut paths = vec![];
        let mut marks = vec![];

        struc_work.key_paths.into_iter().for_each(|path| {
            let mut hide = false;
            let points = path
                .points
                .into_iter()
                .map(|kp| {
                    let pos = to_screen * egui::Pos2::from(kp.point.to_array());
                    if let KeyPointType::Mark | KeyPointType::Horizontal | KeyPointType::Vertical =
                        kp.p_type
                    {
                        marks.push(paint::pos_mark(
                            pos,
                            kp.p_type,
                            paint::STRUC_STROK_NORMAL.width * 2.0,
                            *paint::MARK_STROK,
                        ))
                    } else if kp.p_type == KeyPointType::Hide {
                        hide = true;
                    }

                    pos
                })
                .collect();
            paths.push(egui::Shape::Path(eframe::epaint::PathShape {
                points,
                fill: egui::Color32::TRANSPARENT,
                stroke: match show_hidden {
                    true => match hide {
                        true => *paint::MARK_STROK,
                        false => *paint::STRUC_STROK_SELECTED,
                    },
                    false => match hide {
                        true => egui::Stroke::NONE,
                        false => *paint::STRUC_STROK_NORMAL,
                    },
                },
                closed: path.closed,
            }));
        });

        Ok(egui::Shape::Vec(
            paths.into_iter().chain(marks.into_iter()).collect(),
        ))
    }

    fn composing(
        &mut self,
        name: String,
        char_box: &mut egui::Rect,
        char_info: &mut Option<CharInfo>,
        char_attr: &construct::Attrs,
        mut show_hidden: bool,
        run_data: &mut RunData,
    ) -> Result<egui::Shape, Error> {
        let construct::Attrs { components, format } = char_attr;
        let level = vec![0; components.len()];
        loop {
            let mut varietys = Vec::with_capacity(format.number_of());
            let mut size_list: Vec<WorkSize> = Vec::with_capacity(format.number_of());

            for (in_fmt, comp) in components.iter().enumerate() {
                varietys.push(
                    match &comp {
                        Component::Char(comp_name) => {
                            let comp_name = self
                                .config
                                .replace_list
                                .get(format)
                                .and_then(|fs| fs.get(&in_fmt).and_then(|is| is.get(comp_name)))
                                .unwrap_or(comp_name);
                            size_list.push(
                                self.config
                                    .format_limit
                                    .get(format)
                                    .and_then(|fs| {
                                        fs.get(&in_fmt).and_then(|group| {
                                            group.iter().find_map(|(group, size)| {
                                                if group.contains(comp_name) {
                                                    Some(size.min(WorkSize::new(1.0, 1.0)))
                                                } else {
                                                    None
                                                }
                                            })
                                        })
                                    })
                                    .unwrap_or(WorkSize::new(1.0, 1.0)),
                            );

                            Self::get_variety(
                                &mut self.cache,
                                comp_name.to_owned(),
                                *format,
                                in_fmt,
                                level[in_fmt],
                                run_data,
                            )
                        }
                        Component::Complex(attrs) => Err(Error::Empty(name.to_string())),
                    }?
                    .clone(),
                );
            }

            let mut secondary_trans = vec![];
            for (allocs, length) in varietys
                .iter()
                .map(|v| v.allocs.v.clone())
                .zip(size_list.iter().map(|s| s.height))
            {
                secondary_trans.push(TransformValue::from_allocs(
                    allocs,
                    length,
                    self.config.min_space,
                    self.config.increment,
                    &self.config.limit.v,
                )?);
            }

            let segments: Vec<usize> = varietys.iter().map(|v| v.allocs.h.len()).collect();
            let mut primary_trans: Vec<TransformValue> = vec![];
            let size;
            let intervals;
            loop {
                let mut connect_attr = Vec::with_capacity(varietys.len().max(1) - 1);
                intervals = {
                    let cur = varietys.iter().map(|v| &v.view);
                    let next = cur.clone().skip(1);
                    cur.zip(next)
                        .map(|(view1, view2)| {
                            if view1.width() == 0 || view2.width() == 0 {
                                return 0.0;
                            }
                            let attr = view1.read_row(view1.width() - 1, 0..view1.height())
                                + view2.read_row(0, 0..view2.height()).as_str();

                            let mut interval = 0.0;
                            for wr in &self.config.interval_judge {
                                if wr.regex.is_match(attr.as_str()) {
                                    interval = wr.weight;
                                    break;
                                }
                            }
                            connect_attr.push(attr);

                            interval
                        })
                        .collect::<Vec<f32>>()
                };
                let allocs: Vec<_> = varietys.iter().flat_map(|v| v.allocs.h.clone()).collect();

                let mut tvs = TransformValue::from_allocs_interval(
                    allocs,
                    // size_list.iter().map(|s| s.width).sum::<f32>() / size_list.len() as f32,
                    1.0,
                    self.config.min_space,
                    self.config.increment,
                    intervals.iter().sum(),
                    &self.config.limit.h,
                )?;

                let (primary_length_in, primary_length) = {
                    let mut sort: Vec<_> = secondary_trans
                        .iter()
                        .enumerate()
                        .map(|(i, t)| (i, t.allocs.iter().sum::<usize>()))
                        .collect();
                    sort.sort_by(|(_, n1), (_, n2)| n1.cmp(n2));
                    sort.last().cloned().unwrap_or_default()
                };

                size = AllocSize::new(tvs.allocs.iter().sum(), primary_length);

                if let Some(info) = char_info {
                    info.size = size;
                    info.alloc = DataHV {
                        h: tvs.allocs.iter().cloned().filter(|n| *n != 0).collect(),
                        v: varietys[primary_length_in].allocs.v.clone(),
                    };
                    info.info = format!(
                        "\ninterval:{}",
                        intervals.iter().zip(connect_attr).enumerate().fold(
                            String::new(),
                            |mut buf, (_, (v, a))| {
                                write!(buf, " {}:{}", v, a).unwrap();
                                buf
                            }
                        )
                    );
                    show_hidden = true;
                }
                if size.width == 0 && size.height == 0 {
                    return Err(Error::Empty(name));
                }

                let equally = tvs.allocs.iter().all(|n| *n < 2);
                for (n, length) in segments.into_iter().zip(size_list.iter().map(|s| s.width)) {
                    let allocs: Vec<usize> = tvs.allocs.drain(0..n).collect();
                    let (min_step, step) = match allocs.iter().all(|n| *n < 2) {
                        true if !equally => (tvs.min_step, tvs.min_step),
                        _ => (tvs.min_step, tvs.step),
                    };

                    let tv = TransformValue::from_step(allocs.clone(), min_step, step);
                    if tv.length > length {
                        // let increment = (tv.length - length)
                        //     / tv.min_step.max(self.config.min_space)
                        //     / intervals.len() as f32;
                        // intervals.iter_mut().for_each(|i| {
                        //     if *i < 1.0 {
                        //         *i = (*i + increment).min(1.0)
                        //     }
                        // });
                        primary_trans.push(TransformValue::from_allocs(
                            allocs,
                            length,
                            self.config.min_space,
                            self.config.increment,
                            &self.config.limit.h,
                        )?)
                    } else {
                        primary_trans.push(tv)
                    }
                }

                // let equally = tvs.allocs.iter().all(|n| *n < 2);
                // primary_trans = segments
                //     .into_iter()
                //     .zip(size_list.iter().map(|s| s.width))
                //     .map(|(n, length)| {
                //         let allocs: Vec<usize> = tvs.allocs.drain(0..n).collect();
                //         let (min_step, step) = match allocs.iter().all(|n| *n < 2) {
                //             true if !equally => (tvs.min_step, tvs.min_step),
                //             _ => (tvs.min_step, tvs.step),
                //         };
                //         TransformValue::from_step(allocs, min_step, step)
                //     })
                //     .collect();

                break;
            }

            let trans_list: Vec<DataHV<TransformValue>> = primary_trans
                .into_iter()
                .zip(secondary_trans.into_iter())
                .map(|t| t.into())
                .collect();
            let min_step = trans_list.last().map_or(0.0, |t| t.h.min_step);

            // 同步step
            // let interval = {
            //     let min_step = trans_list
            //         .iter()
            //         .filter_map(|t| match t.h.length == 0.0 {
            //             true => None,
            //             false => Some(t.h.step),
            //         })
            //         .reduce(f32::min);
            //     if let Some(min_step) = min_step {
            //         trans_list.iter_mut().enumerate().for_each(|(i, t)| {
            //             if min_step != t.h.step {
            //                 let mut limit = self.config.limit.h.clone();
            //                 limit
            //                     .entry(t.h.allocs.iter().filter(|&&n| n != 0).count())
            //                     .and_modify(|n| *n = min_step)
            //                     .or_insert(min_step);

            //                 t.h = fasing::fas_file::TransformValue::from_allocs(
            //                     t.h.allocs.drain(..).collect(),
            //                     lengths[i].width,
            //                     self.config.min_space,
            //                     self.config.increment,
            //                     &limit,
            //                 )
            //                 .unwrap();
            //             }
            //         });
            //     }

            //     match (1.0 - trans_list.iter().map(|t| t.h.length).sum::<f32>())
            //         .min(self.config.min_max_step())
            //     {
            //         n if n < self.config.min_space => 0.0,
            //         n => n,
            //     }
            // };

            let mut paths = vec![];
            let mut marks = vec![];

            let comp_box_size = egui::vec2(
                trans_list.iter().map(|t| t.h.length).sum::<f32>()
                    + min_step * intervals.iter().sum::<f32>(),
                trans_list
                    .iter()
                    .map(|t| t.v.length)
                    .reduce(f32::max)
                    .unwrap_or_default(),
            );
            let mut offset = egui::Vec2::ZERO;
            let mut interval_iter = intervals.into_iter();
            let mut gen_offset = |w: f32, h: f32| -> egui::Vec2 {
                let mut new_offset = offset;
                offset.x += w + interval_iter.next().unwrap_or_default() * min_step;
                new_offset.y = (comp_box_size.y - h) * 0.5;
                new_offset
            };
            let to_screen = egui::emath::RectTransform::from_to(
                egui::Rect::from_min_size(
                    egui::pos2(
                        match size.width {
                            0 => -0.5,
                            _ => (comp_box_size.x - 1.0) * 0.5,
                        },
                        match size.height {
                            0 => -0.5,
                            _ => (comp_box_size.y - 1.0) * 0.5,
                        },
                    ),
                    egui::Vec2::splat(1.0),
                ),
                *char_box,
            );

            trans_list
                .into_iter()
                .zip(varietys.into_iter())
                .for_each(|(trans, variety)| {
                    let offset = gen_offset(trans.h.length, trans.v.length);
                    variety
                        .proto
                        .to_work_in_transform(trans)
                        .key_paths
                        .into_iter()
                        .for_each(|path| {
                            let mut hide = false;
                            let points = path
                                .points
                                .into_iter()
                                .map(|kp| {
                                    let pos = to_screen
                                        * (egui::Pos2::from(kp.point.to_array()) + offset);
                                    if let KeyPointType::Mark
                                    | KeyPointType::Horizontal
                                    | KeyPointType::Vertical = kp.p_type
                                    {
                                        marks.push(paint::pos_mark(
                                            pos,
                                            kp.p_type,
                                            paint::STRUC_STROK_NORMAL.width * 2.0,
                                            *paint::MARK_STROK,
                                        ))
                                    } else if kp.p_type == KeyPointType::Hide {
                                        hide = true;
                                    }

                                    pos
                                })
                                .collect();
                            paths.push(egui::Shape::Path(eframe::epaint::PathShape {
                                points,
                                fill: egui::Color32::TRANSPARENT,
                                stroke: match show_hidden {
                                    true => match hide {
                                        true => *paint::MARK_STROK,
                                        false => *paint::STRUC_STROK_SELECTED,
                                    },
                                    false => match hide {
                                        true => egui::Stroke::NONE,
                                        false => *paint::STRUC_STROK_NORMAL,
                                    },
                                },
                                closed: path.closed,
                            }));
                        });
                });

            return Ok(egui::Shape::Vec(
                paths.into_iter().chain(marks.into_iter()).collect(),
            ));
        }
    }

    fn get_variety<'c>(
        cache: &'c mut HashMap<String, StrucVarietys>,
        name: String,
        fmt: construct::Format,
        in_fmt: usize,
        level: usize,
        run_data: &mut RunData,
    ) -> Result<&'c StrucVarietys, Error> {
        use Format::*;

        let mut gen_variety = || {
            let proto = run_data
                .user_data()
                .components
                .get(name.as_str())
                .ok_or(Error::Empty(name.to_string()))?;
            let attrs = match run_data.requests_cache.get(name.as_str()) {
                Some(attrs) => attrs,
                None => {
                    let attrs = run_data.get_comp_attrs(name.as_str());
                    run_data.requests_cache.insert(name.to_owned(), attrs);

                    return Err(Error::Empty(name.to_string()));
                }
            };

            Ok(StrucVarietys::from_attrs(
                proto.clone(),
                attrs.clone(),
                &run_data.user_data().alloc_tab,
            ))
        };

        if level == 0 {
            let is_proto = if in_fmt == 0 {
                match fmt {
                    Single
                    | AboveToBelow
                    | AboveToMiddleAndBelow
                    | LeftToMiddleAndRight
                    | LeftToRight => true,
                    _ => false,
                }
            } else {
                true
            };
            if is_proto {
                return Ok(cache.entry(name.to_string()).or_insert(gen_variety()?));
            }
        }

        match fmt {
            Format::AboveToBelow => Err(Error::Empty(name.to_string())),
            _ => Err(Error::Empty(name.to_string())),
        }
    }

    fn main_panel(&mut self, ui: &mut egui::Ui, core_data: &CoreData, run_data: &mut RunData) {
        paint::struc_scroll_area(
            match self.find.is_empty() {
                true => "Compose Struc List",
                false => "Compose Struc List Find",
            },
            ui,
            |ui, range| {
                let range = match range {
                    Some(range) if range.is_empty() => 0..self.test_chars.len(),
                    Some(range) => range,
                    None => 0..self.test_chars.len(),
                };
                let count = match self.find.is_empty() {
                    true => self.test_chars.len(),
                    false => self
                        .find
                        .chars()
                        .filter(|c| self.test_chars.contains(c))
                        .count(),
                };

                let chars: Vec<char> = self
                    .test_chars
                    .iter()
                    .filter(|c| self.find.is_empty() || self.find.contains(**c))
                    .skip(range.start)
                    .take(range.len())
                    .cloned()
                    .collect();

                chars.iter().for_each(|chr| {
                    let name = chr.to_string();
                    let bg_stroke = ui.style().visuals.widgets.hovered.fg_stroke;
                    let error_color = ui.visuals().error_fg_color;
                    paint::struc_painter(
                        &name,
                        ui,
                        self.selected
                            .iter()
                            .find(|info| info.name == *chr)
                            .is_some(),
                        |rect, painter, mut response| {
                            let char_attr = &core_data.construction[chr];

                            let mut char_info = if response.clicked() {
                                if response.ctx.input(|i| i.modifiers.shift_only()) {
                                    match self.selected.iter().position(|info| info.name == *chr) {
                                        Some(n) => {
                                            self.selected.remove(n);
                                            None
                                        }
                                        None => Some(CharInfo::draft(*chr, char_attr.clone())),
                                    }
                                } else {
                                    self.selected.clear();
                                    Some(CharInfo::draft(*chr, char_attr.clone()))
                                }
                            } else {
                                self.selected
                                    .iter_mut()
                                    .find(|info| info.name == *chr)
                                    .cloned()
                            };

                            let mut char_box = rect.shrink(rect.width() * paint::STRUCT_OUT_MARGIN);

                            if response.hovered() || char_info.is_some() {
                                let size = (1.0 / self.config.min_space).round();
                                let advance = char_box.width() / size;
                                (0..=size as usize).for_each(|n| {
                                    let n = n as f32;
                                    painter.line_segment(
                                        [
                                            char_box.left_top() + egui::Vec2::X * n * advance,
                                            char_box.left_bottom() + egui::Vec2::X * n * advance,
                                        ],
                                        bg_stroke,
                                    );
                                    painter.line_segment(
                                        [
                                            char_box.left_top() + egui::Vec2::Y * n * advance,
                                            char_box.right_top() + egui::Vec2::Y * n * advance,
                                        ],
                                        bg_stroke,
                                    );
                                })
                            }

                            let info_strock =
                                egui::Stroke::new(paint::MARK_STROK.width, self.comp_box_color);
                            painter.line_segment(
                                [rect.center_top(), rect.center_bottom()],
                                info_strock,
                            );
                            painter.line_segment(
                                [rect.left_center(), rect.right_center()],
                                info_strock,
                            );

                            let result = match char_attr.format {
                                Format::Single => self.single_construct(
                                    name.clone(),
                                    &mut char_box,
                                    &mut char_info,
                                    response.hovered(),
                                    run_data,
                                ),
                                Format::LeftToRight => self.composing(
                                    name.clone(),
                                    &mut char_box,
                                    &mut char_info,
                                    char_attr,
                                    response.hovered(),
                                    run_data,
                                ),
                                _ => Err(Error::Empty(name.clone())),
                            };

                            if let Some(char_info) = char_info {
                                match self
                                    .selected
                                    .iter_mut()
                                    .find(|info| info.name == char_info.name)
                                {
                                    Some(info) => *info = char_info,
                                    None => self.selected.push(char_info),
                                }
                            }

                            painter.add(egui::Shape::rect_stroke(
                                char_box,
                                egui::Rounding::none(),
                                egui::Stroke::new(paint::MARK_STROK.width, self.comp_box_color),
                            ));

                            match result {
                                Ok(shape) => {
                                    painter.add(shape);
                                }
                                Err(e) => {
                                    response = response.on_hover_text_at_pointer(
                                        egui::RichText::new(e.to_string()).color(error_color),
                                    );
                                }
                            }

                            let name = chr.to_string();
                            response.context_menu(|ui| {
                                if char_attr.format == construct::Format::Single {
                                    if ui.button(format!("编辑\"{}\"", chr)).clicked() {
                                        self.editor_window = Some(StrucEditing::from_struc(
                                            name.clone(),
                                            &run_data
                                                .user_data()
                                                .components
                                                .get(name.as_str())
                                                .cloned()
                                                .unwrap_or_default(),
                                        ));
                                        ui.close_menu();
                                    }
                                    if ui.button(format!("复制\"{}\"", chr)).clicked() {
                                        ui.output_mut(|o| o.copied_text = name.clone());
                                        ui.close_menu();
                                    }
                                } else {
                                    let list: BTreeSet<String> =
                                        construct::requirements(*chr, &core_data.construction)
                                            .into_iter()
                                            .collect();
                                    match list.len() {
                                        0 => {}
                                        _ => {
                                            if ui.button(format!("复制\"{}\"", chr)).clicked() {
                                                ui.output_mut(|o| o.copied_text = name.clone());
                                                ui.close_menu();
                                            }
                                            ui.menu_button("编辑", |ui| {
                                                list.iter().for_each(|name| {
                                                    if ui.button(name).clicked() {
                                                        self.editor_window =
                                                            Some(StrucEditing::from_struc(
                                                                name.clone(),
                                                                &run_data
                                                                    .user_data()
                                                                    .components
                                                                    .get(name.as_str())
                                                                    .cloned()
                                                                    .unwrap_or_default(),
                                                            ));
                                                        ui.close_menu();
                                                    }
                                                });
                                            });
                                            ui.menu_button("复制", |ui| {
                                                list.iter().for_each(|name| {
                                                    if ui.button(name).clicked() {
                                                        ui.output_mut(|o| {
                                                            o.copied_text = name.clone()
                                                        });
                                                        ui.close_menu();
                                                    }
                                                });
                                            });
                                        }
                                    }
                                }
                            });
                        },
                    );
                });
                count
            },
        )
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
        run_data: &mut RunData,
    ) {
        if !self.selected.is_empty() {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                self.selected.clear();
            }
        }
        if input.key_pressed(egui::Key::S) && input.modifiers.ctrl {
            run_data.user_data_mut().config = self.config.clone();
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
        self.config = run_data.user_data().config.clone();
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

                self.main_panel(ui, core_data, run_data);
            });

        if let Some(mut editor_window) = self.editor_window.take() {
            editor_window.update_ui(ui, frame, core_data, run_data);
            let attrs = run_data.get_comp_attrs(editor_window.name.as_str());
            run_data
                .requests_cache
                .insert(editor_window.name.clone(), attrs.clone());
            self.cache.remove(editor_window.name.as_str());

            if editor_window.run {
                self.editor_window = Some(editor_window);
            }
        }
    }
}
