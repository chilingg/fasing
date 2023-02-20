use super::*;
use crate::{gui::theme, prelude::*};

use anyhow::Result;
use eframe::egui;

pub struct MainWidget {
    style_editor: theme::StyleEditor,
    query_window: QueryWindow,
    sidbar: Sidebar,
    center: Center,
}

impl MainWidget {
    pub fn new() -> Self {
        Self {
            style_editor: theme::StyleEditor::new(false, "style.json".to_string()),
            query_window: QueryWindow::default(),
            sidbar: Sidebar::default(),
            center: Center::default(),
        }
    }
}

pub fn get_fonts<P>(font_key: String, path: P) -> Result<egui::FontDefinitions>
where
    P: AsRef<std::path::Path>,
{
    let font_data = std::fs::read(path)?;

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert(font_key.clone(), egui::FontData::from_owned(font_data));

    // Put my font first (highest priority):
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, font_key.clone());

    // Put my font as last fallback for monospace:
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push(font_key);

    Ok(fonts)
}

impl Widget<CoreData, RunData> for MainWidget {
    fn start(
        &mut self,
        context: &eframe::CreationContext,
        _core_data: &CoreData,
        _run_data: &mut RunData,
    ) {
        context.egui_ctx.set_style(theme::default_style());

        let font_path = "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc";
        context.egui_ctx.set_fonts(
            get_fonts("Fasing Font".to_string(), font_path)
                .expect(format!("Failed to set font `{font_path}`").as_str()),
        );
    }

    fn children(&mut self) -> Children {
        vec![
            Box::new(&mut self.style_editor),
            Box::new(&mut self.query_window),
            Box::new(&mut self.sidbar),
            Box::new(&mut self.center),
        ]
    }

    fn input_process(
        &mut self,
        input: &mut egui::InputState,
        _core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        if input.key_pressed(egui::Key::F12) {
            self.style_editor.open = !self.style_editor.open;
            input.keys_down.remove(&egui::Key::F12);
        }
        if input.key_pressed(egui::Key::F5) {
            self.query_window.open = !self.query_window.open;
            input.keys_down.remove(&egui::Key::F5);
        }
        if input.key_pressed(egui::Key::S) && input.modifiers.ctrl {
            const PATH: &str = "tmp/user_data.json";

            if run_data.is_user_data_changed() {
                match run_data.save_user_data(PATH) {
                    Ok(size) => {
                        run_data
                            .messages
                            .add_info(format!("[{}]文件已保存: {}.", size, PATH));
                    }
                    Err(e) => eprintln!("Save failed: {}", e),
                }
            }
            input.keys_down.remove(&egui::Key::S);
        }
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        core_data: &CoreData,
        run_data: &mut RunData,
    ) {
        self.style_editor.update(ctx, frame, core_data, run_data);
        self.query_window.update(ctx, frame, core_data, run_data);
        self.sidbar.update(ctx, frame, core_data, run_data);
        self.center.update(ctx, frame, core_data, run_data);
    }

    fn finished(&self, _core_data: &CoreData, run_data: &mut RunData) {
        if let Err(e) = run_data.save_user_data("tmp/backup_user_data.json") {
            eprintln!("Auto save failed: {}", e);
        }
    }
}
