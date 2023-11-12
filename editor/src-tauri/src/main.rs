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

use fasing::{
    axis::*,
    component::struc::*,
    service::{CharInfo, LocalService},
};

type Context = Arc<Mutex<fasing_editor::Context>>;
type Service = Arc<Mutex<LocalService>>;

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

fn gen_service_info(service: &LocalService, path: &str) -> ServiceInfo {
    let source = service.source().unwrap();
    let versions = {
        let mut iter = source
            .version
            .split(' ')
            .map(|n| n.parse::<u32>().unwrap_or_default());
        let mut versions = vec![];
        for _ in 0..2 {
            versions.push(iter.next().unwrap_or_default())
        }
        versions
    };
    ServiceInfo {
        file_name: Path::new(path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        name: source.name.clone(),
        major_version: versions[0],
        minor_version: versions[1],
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

type StrucEditorData = Arc<Mutex<Option<String>>>;

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
            let _ = service.load_file(path.as_str().unwrap());
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
    let mut service = service.lock().unwrap();
    match service.load_file(path) {
        Ok(_) => {
            let info = gen_service_info(&service, path);
            set_window_title_as_serviceinfo(&window, &info);

            window
                .emit("source", SourcePayload { event: "load" })
                .expect("Emit event `source_load` error!");

            context.lock().unwrap().set(json!("source"), json!(path));

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
fn get_char_info(service: State<Service>, name: &str) -> Result<CharInfo, String> {
    service
        .lock()
        .unwrap()
        .get_char_info(name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_struc_comb(service: State<Service>, name: &str) -> Result<(StrucWork, Vec<String>), String> {
    service
        .lock()
        .unwrap()
        .get_comb_struc(name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_construct_table(
    service: State<Service>,
) -> std::collections::HashMap<String, fasing::construct::Attrs> {
    service.lock().unwrap().construct_table.data.clone()
}

#[tauri::command]
fn get_config(service: State<Service>) -> Option<fasing::config::Config> {
    service.lock().unwrap().get_config()
}

#[tauri::command]
fn set_config(
    service: State<Service>,
    window: tauri::Window,
    config: fasing::config::Config,
) -> bool {
    let mut service = service.lock().unwrap();
    if !service.is_changed() {
        set_window_title_in_change(&window, true);
    }
    service.set_config(config)
}

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
    grid: fasing::construct::space::WorkSize,
) -> (
    Option<String>,
    StrucWork,
    fasing::construct::space::WorkSize,
) {
    let name = data.lock().unwrap().clone();
    let struc = match &name {
        Some(name) => service.lock().unwrap().get_struc_proto(name),
        None => Default::default(),
    };
    let size = struc.alloc_size().cast().cast_unit();
    let scale = fasing::construct::space::WorkVec::new(
        match size.width == 0.0 {
            true => 1.0,
            false => size.width / grid.width,
        },
        match size.height == 0.0 {
            true => 1.0,
            false => size.height / grid.height,
        },
    );
    let struc = struc
        .to_normal(size.to_hv_data().into_map(|v| 0.5 / v))
        .transform(scale, fasing::construct::space::WorkVec::zero());

    (name, struc, size)
}

#[tauri::command]
fn align_cells(mut struc: StrucWork, unit: fasing::construct::space::WorkSize) -> StrucWork {
    struc.align_cells(unit);
    struc
}

#[tauri::command]
fn save_struc_in_cells(
    service: State<Service>,
    handle: tauri::AppHandle,
    name: String,
    struc: StrucWork,
    unit: fasing::construct::space::WorkSize,
) {
    let mut service = service.lock().unwrap();
    let main_window = handle.get_window("main").unwrap();

    if !service.is_changed() {
        set_window_title_in_change(&main_window, true);
    }
    service.save_struc(name.clone(), struc.to_proto(unit));

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

#[tauri::command]
fn export_combs(service: State<Service>, list: Vec<String>, path: &str) {
    service.lock().unwrap().export_combs(&list, path)
}

fn main() {
    let (context, win_data, source) = {
        let context = fasing_editor::Context::default();
        let win_data = context.get(json!("window"));
        let source = context.get(json!("source"));
        (Arc::new(Mutex::new(context)), win_data, source)
    };
    let (service, service_info) = {
        let mut service = LocalService::new();
        let info = match source {
            Some(path) if path.is_string() => match service.load_file(path.as_str().unwrap()) {
                Ok(_) => {
                    let info = gen_service_info(&service, path.as_str().unwrap());
                    Some(info)
                }
                Err(e) => {
                    eprintln!("Failed open service: {:?}", e);
                    None
                }
            },
            _ => None,
        };
        (Arc::new(Mutex::new(service)), info)
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
            get_char_info,
            get_struc_comb,
            get_construct_table,
            get_config,
            set_config,
            open_struc_editor,
            get_struc_editor_data,
            align_cells,
            save_service_file,
            save_struc_in_cells,
            export_combs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
