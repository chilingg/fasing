#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde_json::json;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};

use fasing::struc::{attribute::StrucAttributes, StrucProto, StrucWork};

type Context = Arc<Mutex<fasing_editor::Context>>;
type Service = Arc<Mutex<fasing::Service>>;

#[derive(serde::Serialize, serde::Deserialize)]
struct WindowState {
    maximized: bool,
    size: tauri::LogicalSize<u32>,
    pos: tauri::LogicalPosition<u32>,
}

impl WindowState {
    pub fn new(window: &tauri::Window) -> tauri::Result<Self> {
        let factor = window.scale_factor()?;

        Ok(Self {
            maximized: window.is_maximized()?,
            size: window.outer_size()?.to_logical::<u32>(factor),
            pos: window.outer_position()?.to_logical::<u32>(factor),
        })
    }
}

fn exit_save(context: &Context, window: &tauri::Window) {
    let mut guard = context.lock();
    let context = guard.as_mut().unwrap();
    if let Ok(win_state) = WindowState::new(window) {
        context.set(json!("window"), serde_json::to_value(win_state).unwrap());
    }

    if let Err(e) = context.save() {
        eprintln!("{:?}: {}", e, "Save context error!");
    };
}

fn gen_service_info(service: &fasing::Service, path: &str) -> ServiceInfo {
    let source = service.source().unwrap();
    ServiceInfo {
        file_name: Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        name: source.name.clone(),
        major_version: source.major_version,
        minor_version: source.minor_version,
    }
}

fn set_window_title_as_serviceinfo(window: &tauri::Window, info: &ServiceInfo) {
    window
        .set_title(
            format!(
                "{} - {} {}.{} - 繁星",
                info.file_name, info.name, info.major_version, info.minor_version
            )
            .as_str(),
        )
        .expect("Unable to set title!");
}

fn set_window_title_in_change(window: &tauri::Window, changed: bool) {
    match window.title() {
        Ok(title) => {
            let mut title_chars = title.chars();
            if let Some(first) = title_chars.next() {
                if changed && first != '*' {
                    window.set_title(format!("*{}", title).as_str()).unwrap();
                }
                if !changed && first == '*' {
                    window
                        .set_title(title_chars.collect::<String>().as_str())
                        .unwrap();
                }
            }
        }
        Err(e) => eprintln!("{}", e),
    }
}

struct ServiceInfo {
    file_name: String,
    name: String,
    major_version: u32,
    minor_version: u32,
}

#[derive(Clone, serde::Serialize)]
struct SourcePayload {
    event: &'static str,
}

#[tauri::command]
fn reload(service: State<Service>, context: State<Context>, window: tauri::Window) {
    match context.lock().unwrap().get(json!("source")) {
        Some(path) if path.is_string() => {
            let mut service = service.lock().unwrap();
            if service.is_changed() {
                set_window_title_in_change(&window, false);
            }
            service.reload(path.as_str().unwrap());
            window
                .emit("source", SourcePayload { event: "load" })
                .expect("Emit event `source_load` error!");
        }
        _ => {}
    }
}

#[tauri::command]
fn new_service_from_file(
    service: State<Service>,
    path: &str,
    context: State<Context>,
    window: tauri::Window,
) -> Result<(), String> {
    match fasing::Service::new(path) {
        Ok(new_service) => {
            let info = gen_service_info(&new_service, path);
            set_window_title_as_serviceinfo(&window, &info);

            window
                .emit("source", SourcePayload { event: "load" })
                .expect("Emit event `source_load` error!");

            context.lock().unwrap().set(json!("source"), json!(path));
            *service.lock().unwrap() = new_service;

            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn get_struc_proto(service: State<Service>, name: &str) -> StrucProto {
    service.lock().unwrap().get_struc_proto(name)
}

#[tauri::command]
fn get_struc_proto_all(service: State<Service>) -> std::collections::BTreeMap<String, StrucProto> {
    service.lock().unwrap().get_struc_proto_all()
}

#[tauri::command]
fn get_struc_attribute(service: State<Service>, name: &str) -> StrucAttributes {
    service.lock().unwrap().get_struc_proto(name).attributes()
}

#[tauri::command]
fn get_struc_attributes(
    service: State<Service>,
    names: Vec<String>,
) -> BTreeMap<String, StrucAttributes> {
    let service = service.lock().unwrap();
    names
        .into_iter()
        .map(|name| {
            let attrs = service.get_struc_proto(&name).attributes();
            (name, attrs)
        })
        .collect()
}

#[tauri::command]
fn get_allocate_table(service: State<Service>) -> fasing::fas_file::AllocateTable {
    match service.lock().unwrap().source() {
        Some(source) => source.alloc_tab.clone(),
        None => Default::default(),
    }
}

#[tauri::command]
fn get_construct_table(service: State<Service>) -> fasing::construct::Table {
    service.lock().unwrap().construct_table.clone()
}

#[tauri::command]
fn get_struc_comb(service: State<Service>, name: char) -> Result<StrucWork, String> {
    service
        .lock()
        .unwrap()
        .get_struc_comb(name)
        .map_err(|e| e.to_string())
}

type StrucEditorData = Arc<Mutex<Option<String>>>;

#[tauri::command(async)]
fn open_struc_editor(
    handle: tauri::AppHandle,
    name: Option<String>,
    data: State<StrucEditorData>,
) -> Result<(), String> {
    tauri::WindowBuilder::new(
        &handle,
        "struc-editor",
        tauri::WindowUrl::App("/editor".into()),
    )
    .title(name.as_ref().map_or("untitle", |n| n.as_str()))
    .center()
    .build()
    .map_err(|e| e.to_string())?;

    *data.lock().unwrap() = name;

    Ok(())
}

#[tauri::command]
fn get_struc_editor_data(
    data: State<StrucEditorData>,
    service: State<Service>,
) -> (Option<String>, StrucWork) {
    let name = data.lock().unwrap().clone();
    let struc = match &name {
        Some(name) => service.lock().unwrap().get_struc_proto(name),
        None => Default::default(),
    };

    (name, struc.to_normal())
}

#[tauri::command]
fn fiter_attribute(service: State<Service>, regex: &str) -> Result<Vec<String>, String> {
    match service.lock().unwrap().source() {
        Some(source) => match regex::Regex::new(regex) {
            Ok(rgx) => Ok(source
                .components
                .iter()
                .fold(vec![], |mut list, (name, struc)| {
                    let attrs = struc.attributes();
                    let is_match = attrs
                        .into_iter()
                        .find(|attrs| attrs.iter().find(|attr| rgx.is_match(attr)).is_some())
                        .is_some();
                    if is_match {
                        list.push(name.to_string());
                    }

                    list
                })),
            Err(e) => Err(e.to_string()),
        },
        None => Ok(vec![]),
    }
}

#[tauri::command]
fn normalization(struc: StrucWork, offset: f32) -> StrucWork {
    fasing::Service::normalization(&struc, offset)
}

#[tauri::command]
fn save_struc(service: State<Service>, handle: tauri::AppHandle, name: String, struc: StrucWork) {
    let mut service = service.lock().unwrap();
    let main_window = handle.get_window("main").unwrap();

    if !service.is_changed() {
        set_window_title_in_change(&main_window, true);
    }
    service.save_struc(name.clone(), &struc);

    main_window.emit("struc_change", name).unwrap();
}

#[tauri::command]
fn save_service_file(service: State<Service>, context: State<Context>, window: tauri::Window) {
    let mut service_data = service.lock().unwrap();

    if service_data.source().is_some() {
        match context.lock().unwrap().get(json!("source")) {
            Some(path) if path.is_string() => {
                let before_state = service_data.is_changed();
                let result = service_data.save(path.as_str().unwrap());
                if result.is_ok() && before_state {
                    set_window_title_in_change(&window, false);
                } else if let Err(e) = result {
                    eprintln!("{}", e);
                }
            }
            _ => {
                drop(service_data);
                let service = Arc::clone(&*service);
                tauri::api::dialog::FileDialogBuilder::new().pick_file(move |file_path| {
                    if let Some(path) = file_path {
                        let mut service_data = service.lock().unwrap();
                        let before_state = service_data.is_changed();
                        let result = service_data.save(path.to_str().unwrap());
                        if result.is_ok() && before_state {
                            set_window_title_in_change(&window, false);
                        } else if let Err(e) = result {
                            eprintln!("{}", e);
                        }
                    }
                })
            }
        }
    }
}

fn main() {
    let (context, win_data, source) = {
        let context = fasing_editor::Context::default();
        let win_data = context.get(json!("window"));
        let source = context.get(json!("source"));
        (Arc::new(Mutex::new(context)), win_data, source)
    };
    let (service, service_info) = match source {
        Some(path) if path.is_string() => match fasing::Service::new(path.as_str().unwrap()) {
            Ok(service) => {
                let info = gen_service_info(&service, path.as_str().unwrap());

                (Arc::new(Mutex::new(service)), Some(info))
            }
            Err(e) => {
                eprintln!("Failed open service: {:?}", e);
                (Service::default(), None)
            }
        },
        _ => (Service::default(), None),
    };

    tauri::Builder::default()
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();

            if let Some(data) = win_data {
                let win_state: WindowState = serde_json::from_value(data).unwrap();
                main_window
                    .set_size(win_state.size)
                    .expect("Unable to set size!");
                main_window
                    .set_position(win_state.pos)
                    .expect("Unable to set position!");
                if win_state.maximized {
                    main_window.maximize().expect("Unable to set maximize!");
                }
            }
            if let Some(info) = service_info {
                set_window_title_as_serviceinfo(&main_window, &info);
            }

            Ok(())
        })
        .manage(Arc::clone(&service))
        .manage(Arc::clone(&context))
        .manage(StrucEditorData::default())
        .on_window_event(move |event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let window = event.window().clone();
                if window.label() == "main" {
                    let service = service.lock().unwrap();
                    if service.is_changed() {
                        api.prevent_close();
                        let context = Arc::clone(&context);
                        tauri::api::dialog::confirm(
                            Some(&event.window()),
                            service.source().unwrap().name.clone(),
                            "文件未保存，确定是否关闭当前应用",
                            move |answer| {
                                if answer {
                                    exit_save(&context, &window);
                                    window.close().unwrap();
                                }
                            },
                        )
                    } else {
                        exit_save(&context, event.window());
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            new_service_from_file,
            reload,
            get_struc_proto,
            get_struc_proto_all,
            get_struc_comb,
            get_struc_attribute,
            get_struc_attributes,
            get_allocate_table,
            get_construct_table,
            open_struc_editor,
            get_struc_editor_data,
            fiter_attribute,
            normalization,
            save_struc,
            save_service_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
