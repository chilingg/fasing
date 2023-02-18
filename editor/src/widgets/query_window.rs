use fasing::construct;
use crate::gui::prelude::*;

#[derive(Default)]
pub struct QueryWindow {
    core_data: Option<std::rc::Rc<CoreData>>,
    filter_str: String,
    filters: Vec<char>,
    widget_data: WidgetData,
}

impl Widget for QueryWindow {
    fn widget_data(&mut self) -> Option<&mut WidgetData> {
        Some(&mut self.widget_data)
    }

    fn update(&mut self, ctx: &egui::Context, _queue: &mut Vec<Task>) {
        let table = &self.core_data.as_mut().unwrap().construction;

        egui::Window::new("Query struct")
            .open(&mut self.widget_data.open)
            .show(ctx, |ui| {
                ui.label(format!("fasing 1.0 ({})", table.len()));

                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    if ui.add(
                        egui::TextEdit::singleline(&mut self.filter_str)//.desired_width(100.0)
                    ).changed() {
                        self.filters = self.filter_str.trim().chars().collect();
                    }
                    if ui.button("ｘ").clicked() {
                        self.filter_str.clear();
                        self.filters.clear();
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    fn char_info_panel(chr: char, ui: &mut egui::Ui, table: &construct::Table) {
                        fn attr_info(attr: &construct::Attrs, ui: &mut egui::Ui, table: &construct::Table) {
                            attr.components.iter().for_each(|comp| {
                                match comp {
                                    construct::Component::Char(str) => {
                                        let mut chars = str.chars();
                                        let chr = chars.next().unwrap();
                                        if chars.next().is_none() {
                                            char_info_panel(chr, ui, table);
                                        } else {
                                            let button = egui::Button::new(
                                                egui::RichText::new(str),
                                            ).frame(false);
                                            if ui.add(button).clicked() {
                                                ui.output_mut(|input| input.copied_text = str.clone());
                                            }
                                        }
                                    },
                                    construct::Component::Complex(ref attr) => attr_info(&attr, ui, table),
                                }
                            })
                        }

                        ui.horizontal(|ui| {
                            let button = egui::Button::new(
                                egui::RichText::new(chr.to_string()),
                            ).frame(false);
                            if ui.add(button).clicked() {
                                ui.output_mut(|input| input.copied_text = chr.to_string());
                            }
                            match table.get(&chr) {
                                Some(attrs) if attrs.format == construct::Format::Single => {},
                                Some(attrs) => {
                                    ui.collapsing(format!("{}", attrs), |ui| {
                                        attr_info(attrs, ui, table);
                                    });
                                },
                                None => { ui.label("未记录"); }
                            }
                        });
                    }

                    self.filters.iter().for_each(|chr| {
                        char_info_panel(*chr, ui, table);
                    });
                });
            });
    }

    fn start(&mut self, app_state: &mut AppState) {
        self.core_data = Some(app_state.core_data.clone());
    }

    fn children(&mut self) -> Children {
        vec![]
    }
}