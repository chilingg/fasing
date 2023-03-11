use super::gui::widget::Widget;
use super::widgets::{MainWidget, MessagePanel};
use fasing::{
    construct,
    fas_file::{self, FasFile},
    struc::{self, StrucAttributes},
};

use std::collections::HashMap;

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

pub type RequestCache = HashMap<String, StrucAttributes>;

pub struct RunData {
    pub messages: MessagePanel,

    pub file_path: String,
    pub requests_cache: RequestCache,

    fas_file: FasFile,
    changed: bool,
}

impl Default for RunData {
    fn default() -> Self {
        Self {
            fas_file: FasFile::default(),

            file_path: Default::default(),
            changed: false,
            messages: Default::default(),
            requests_cache: Default::default(),
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

    pub fn save_user_data(&mut self) -> std::io::Result<usize> {
        self.changed = false;
        self.fas_file.save(self.file_path.as_str())
    }

    pub fn save_user_data_as(&mut self, path: &str) -> std::io::Result<usize> {
        self.changed = false;
        self.file_path = path.to_string();

        self.fas_file.save(path)
    }

    pub fn new_user_data_from(&mut self, path: &str) -> Result<(), fas_file::Error> {
        self.fas_file = FasFile::from_file(path)?;
        self.file_path = path.to_string();
        Ok(())
    }

    pub fn new_user_data(&mut self) {
        self.fas_file = FasFile::default();
        self.file_path.clear();
    }

    pub fn is_user_data_changed(&self) -> bool {
        self.changed
    }

    pub fn create_requestes_cache(&mut self, core_data: &CoreData) {
        self.requests_cache = fasing::construct::all_requirements(&core_data.construction)
            .into_iter()
            .map(|name| (name, Default::default()))
            .collect();
        self.update_requestes_cache();
    }

    pub fn update_requestes_cache(&mut self) {
        let mut cache = Default::default();
        std::mem::swap(&mut self.requests_cache, &mut cache);
        cache.iter_mut().for_each(|(name, attr)| {
            *attr = self.get_comp_attrs(name);
        });
        std::mem::swap(&mut self.requests_cache, &mut cache);
    }

    pub fn get_comp_attrs(&self, name: &str) -> StrucAttributes {
        self.fas_file
            .components
            .get(name)
            .get_or_insert(&Default::default())
            .attributes()
            .or_else(|e| {
                eprintln!("StrucAttributes Error `{}`: {}", name, e.msg);
                Ok::<StrucAttributes, struc::Error>(Default::default())
            })
            .unwrap()
    }
}

pub struct App {
    pub core_data: CoreData,
    pub run_data: RunData,
    pub root: MainWidget,
}

pub fn recursion_start(
    widget: &mut dyn Widget<CoreData, RunData>,
    context: &eframe::CreationContext,
    core_data: &CoreData,
    run_data: &mut RunData,
) {
    widget.start(context, core_data, run_data);
    widget
        .children()
        .iter_mut()
        .for_each(|widget| recursion_start(**widget, context, core_data, run_data));
}

impl App {
    pub fn new() -> Self {
        Self {
            core_data: Default::default(),
            run_data: Default::default(),
            root: MainWidget::new(),
        }
    }

    pub fn start<'a>(&'a mut self, context: &eframe::CreationContext<'a>) {
        if let Some(path) = context.storage.unwrap().get_string("file_path") {
            if let Err(e) = self.run_data.new_user_data_from(path.as_str()) {
                self.run_data.messages.add_error(e.to_string());
            }
        } else {
            // Development stage start
            self.run_data
                .new_user_data_from("tmp/user_data.json")
                .unwrap();
        }

        self.run_data.create_requestes_cache(&self.core_data);

        recursion_start(&mut self.root, context, &self.core_data, &mut self.run_data)
    }

    pub fn finish(&mut self) {
        pub fn recursion_finish(
            widget: &mut dyn Widget<CoreData, RunData>,
            core_data: &CoreData,
            run_data: &mut RunData,
        ) {
            widget
                .children()
                .iter_mut()
                .for_each(|widget| recursion_finish(**widget, core_data, run_data));
            widget.finished(core_data, run_data);
        }

        recursion_finish(&mut self.root, &self.core_data, &mut self.run_data)
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        fn recursion_input_process(
            widget: &mut dyn Widget<CoreData, RunData>,
            input: &mut egui::InputState,
            core_data: &CoreData,
            run_data: &mut RunData,
        ) {
            widget
                .children()
                .iter_mut()
                .for_each(|widget| recursion_input_process(**widget, input, core_data, run_data));
            widget.input_process(input, core_data, run_data);
        }

        ctx.input_mut(|input| {
            recursion_input_process(&mut self.root, input, &self.core_data, &mut self.run_data)
        });

        self.root
            .update(ctx, frame, &self.core_data, &mut self.run_data);

        self.run_data.messages.update(ctx);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        fn recursion_save(
            widget: &mut dyn Widget<CoreData, RunData>,
            storage: &mut dyn eframe::Storage,
        ) {
            widget.save(storage);
            widget
                .children()
                .iter_mut()
                .for_each(|widget| recursion_save(**widget, storage));
        }

        recursion_save(&mut self.root, storage);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.finish();
    }
}
