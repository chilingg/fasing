use super::gui::widget::Widget;
use super::widgets::MainWidget;
use fasing::{construct, fas_file::FasFile};

use std::path::Path;

pub struct CoreData {
    pub construction: construct::char_construct::Table,
}

impl Default for CoreData {
    fn default() -> Self {
        Self {
            construction: construct::fasing_1_0::generate_table(),
        }
    }
}

pub struct RunData {
    fas_file: FasFile,
    changed: bool,
}

impl Default for RunData {
    fn default() -> Self {
        Self {
            fas_file: FasFile::new_file("tmp/user_data.json").unwrap(),
            changed: false,
        }
    }
}

impl RunData {
    pub fn user_data(&self) -> &FasFile {
        &self.fas_file
    }

    pub fn user_data_mut(&mut self) -> &mut FasFile {
        self.changed = true;
        &mut self.fas_file
    }

    pub fn save_user_data<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<usize> {
        self.changed = false;
        self.fas_file.save(path)
    }
}

pub struct App {
    pub core_data: CoreData,
    pub run_data: RunData,
    pub root: MainWidget,
}

impl App {
    pub fn new() -> Self {
        Self {
            core_data: Default::default(),
            run_data: Default::default(),
            root: MainWidget::new(),
        }
    }

    pub fn start(&mut self, context: &eframe::CreationContext) {
        self.root
            .recursion_start(context, &self.core_data, &mut self.run_data)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        ctx.input_mut(|input| {
            self.root
                .recursion_input_process(input, &self.core_data, &mut self.run_data)
        });

        self.root
            .update(ctx, frame, &self.core_data, &mut self.run_data);
    }
}
