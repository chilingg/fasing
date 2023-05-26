#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde_json::json;
use std::{
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
fn get_allocate_table(service: State<Service>) -> fasing::fas_file::AllocateTable {
    match service.lock().unwrap().source() {
        Some(source) => source.alloc_tab.clone(),
        None => Default::default(),
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
        .on_window_event(move |event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let service = service.lock().unwrap();
                if service.is_changed() {
                    api.prevent_close();
                    let window = event.window().clone();
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
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            new_service_from_file,
            get_struc_proto,
            get_struc_proto_all,
            get_struc_attribute,
            get_allocate_table,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
