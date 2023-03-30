use super::gui::widget::Widget;
use super::widgets::{MainWidget, MessagePanel};
use fasing::{
    construct,
    fas_file::{self, FasFile},
    struc::{self, attribute::StrucAttributes},
};

use std::collections::{BTreeMap, BTreeSet, HashMap};

pub struct CoreData {
    pub construction: construct::Table,
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
    pub tags: BTreeMap<String, BTreeSet<String>>,

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
            tags: BTreeMap::from([
                ("top".to_string(), BTreeSet::<String>::default()),
                ("bottom".to_string(), BTreeSet::<String>::default()),
                ("left".to_string(), BTreeSet::<String>::default()),
                ("right".to_string(), BTreeSet::<String>::default()),
                ("middle".to_string(), BTreeSet::<String>::default()),
            ]),
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

    pub fn remove_comp_tag(&mut self, name: String, tag: String) {
        self.tags.entry(tag.clone()).and_modify(|items| {
            items.remove(&name);
        });
        self.user_data_mut()
            .components
            .entry(name)
            .and_modify(|comp| {
                comp.tags.remove(&tag);
            });
    }

    pub fn modify_tag_name(&mut self, new_tag: String, old_tag: &str) {
        if let Some(items) = self.tags.remove(old_tag).take() {
            self.tags.insert(new_tag, items);
        }
    }

    pub fn sync_comp_tags(&mut self, name: &str, tags: &BTreeSet<String>) {
        self.tags.extend(
            tags.iter()
                .filter_map(|tag| {
                    if self.tags.contains_key(tag.as_str()) {
                        None
                    } else {
                        Some(tag.to_string())
                    }
                })
                .zip(std::iter::repeat(BTreeSet::new()))
                .collect::<Vec<(String, BTreeSet<_>)>>(),
        );
        self.tags
            .iter_mut()
            .for_each(|(tag, items)| match tags.contains(tag) {
                true => {
                    items.insert(name.to_string());
                }
                false => {
                    items.remove(name);
                }
            });
    }

    pub fn save_comp_data(&mut self, name: String, proto: struc::StrucProto) {
        self.sync_comp_tags(name.as_str(), &proto.tags);
        self.user_data_mut().components.insert(name.clone(), proto);
        let attrs = self.get_comp_attrs(name.as_str());
        self.requests_cache.insert(name, attrs);
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
        self.run_data
            .tags
            .append(&mut self.run_data.user_data().components.iter().fold(
                BTreeMap::new(),
                |mut tags, (name, struc)| {
                    struc.tags.iter().for_each(|tag| {
                        tags.entry(tag.to_string())
                            .or_default()
                            .insert(name.clone());
                    });
                    tags
                },
            ));

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
