mod context;
use fasing::{
    component::struc::StrucProto,
    construct::{space::WorkPoint, CharTree, CstError, CstTable},
    service::LocalService,
};

use serde_json::json;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

type Context = Mutex<context::Context>;
type Service = Mutex<LocalService>;

mod signal {
    pub const SOURCE: &'static str = "source";
    pub const CHANGED: &'static str = "changed";
    pub const SAVED: &'static str = "saved";

    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Payload<T> {
        target: String,
        value: T,
    }

    impl<T> Payload<T> {
        pub fn new(target: String, value: T) -> Self {
            Self { target, value }
        }
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

#[tauri::command]
fn new_source(
    path: &str,
    service: State<Service>,
    context: State<Context>,
    app: AppHandle,
) -> Result<(), String> {
    let mut service = service.lock().unwrap();
    match service.load_file(path) {
        Ok(_) => {
            set_window_title_as_serviceinfo(
                &app.get_webview_window("main").unwrap(),
                &ServiceInfo::new(&service, path),
            );
            app.emit(
                signal::SOURCE,
                signal::Payload::new("open".to_string(), path.to_string()),
            )
            .expect("Emit event `source_load` error!");
            context.lock().unwrap().set(json!("source"), json!(path));

            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn reload(service: State<Service>, context: State<Context>, app: AppHandle) {
    match context.lock().unwrap().get(json!("source")) {
        Some(path) if path.is_string() => {
            let mut service = service.lock().unwrap();
            if service.is_changed() {
                if !app
                    .dialog()
                    .message("文件未保存，是否重新载入？")
                    .title("Warning!")
                    .blocking_show()
                {
                    return;
                }
            }
            let _ = service.load_file(path.as_str().unwrap());
            app.emit(
                signal::SOURCE,
                signal::Payload::new("reload".to_string(), path.to_string()),
            )
            .expect("Emit event `source_load` error!");
        }
        _ => {}
    }
}

#[tauri::command]
fn target_chars(service: State<Service>) -> Vec<char> {
    let mut list = vec![];
    let s = service.lock().unwrap();
    if s.source().is_some() {
        list = s.construct_table.target_chars();
        list.sort();
    }
    list
}

#[tauri::command]
fn get_char_tree(service: State<Service>, name: String) -> CharTree {
    service.lock().unwrap().gen_char_tree(name)
}

#[tauri::command]
fn get_cst_table(service: State<Service>) -> CstTable {
    service.lock().unwrap().construct_table.clone()
}

#[tauri::command]
fn gen_comp_path(
    service: State<Service>,
    target: CharTree,
) -> Result<(Vec<Vec<WorkPoint>>, CharTree), CstError> {
    service.lock().unwrap().gen_comp_visible_path(target)
}

#[tauri::command(async)]
fn open_struc_editor(app: tauri::AppHandle, name: String) {
    tauri::WebviewWindowBuilder::new(
        &app,
        "struc-editor",
        tauri::WebviewUrl::App("editor.html".into()),
    )
    .title(&name)
    .center()
    .inner_size(800.0, 600.0)
    .build()
    .expect("Unable to create editing window!");
}

#[tauri::command]
fn get_struc_editor_data(app: AppHandle) -> (String, StrucProto) {
    let name = app
        .get_webview_window("struc-editor")
        .unwrap()
        .title()
        .unwrap();
    let service = app.state::<Service>();
    let proto = service.lock().unwrap().get_struc_proto(&name);
    (name, proto)
}

#[tauri::command]
fn save_struc(service: State<Service>, handle: tauri::AppHandle, name: String, struc: StrucProto) {
    let mut service = service.lock().unwrap();
    service.save_struc(name.clone(), struc);

    let main_window = handle.get_webview_window("main").unwrap();
    main_window
        .emit(
            signal::CHANGED,
            signal::Payload::new("struc".to_string(), name),
        )
        .unwrap();
}

#[tauri::command]
fn save_fas_file(service: State<Service>, context: State<Context>, window: tauri::Window) {
    use tauri_plugin_dialog::DialogExt;

    let mut service_data = service.lock().unwrap();

    let r = match context.lock().unwrap().get(json!("source")) {
        Some(path) if path.is_string() => service_data.save(path.as_str().unwrap()),
        _ => {
            if let Some(path) = window.dialog().file().blocking_pick_file() {
                service_data.save(&path.to_string())
            } else {
                Ok(())
            }
        }
    };

    match r {
        Ok(_) => window.emit(signal::SAVED, 0).unwrap(),
        Err(e) => eprintln!("{}", e),
    }
}

#[tauri::command]
fn is_changed(service: State<Service>) -> bool {
    service.lock().unwrap().is_changed()
}

#[tauri::command]
fn get_config(service: State<Service>) -> Option<fasing::config::Config> {
    service.lock().unwrap().source().map(|s| s.config.clone())
}

#[tauri::command]
fn set_config(service: State<Service>, cfg: fasing::config::Config, window: tauri::Window) {
    let mut service = service.lock().unwrap();
    service.set_config(cfg);

    window.emit(signal::CHANGED, "config").unwrap();
}

#[tauri::command]
fn get_char_info(
    service: State<Service>,
    name: String,
) -> Result<fasing::component::comb::CharInfo, CstError> {
    service.lock().unwrap().gen_char_info(name)
}

#[tauri::command]
fn export_chars(
    service: State<Service>,
    list: Vec<char>,
    width: usize,
    height: usize,
    path: &str,
) -> Vec<String> {
    service
        .lock()
        .unwrap()
        .export_chars(list, width, height, path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (context, service, win_state, service_info) = init();

    tauri::Builder::default()
        .setup(|app| {
            app.manage(service);
            app.manage(context);

            let main_window = app.get_webview_window("main").unwrap();

            if let Some(win_state) = win_state {
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
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    let app = window.app_handle();
                    let service = app.state::<Service>();
                    let context = app.state::<Context>();
                    if service.lock().unwrap().is_changed() {
                        api.prevent_close();
                        if window
                            .dialog()
                            .message("文件未保存，确定是否关闭当前应用？")
                            .kind(MessageDialogKind::Warning)
                            .blocking_show()
                        {
                            exit_save(context, &window);
                            window.destroy().unwrap();
                        }
                    } else {
                        exit_save(context, &window);
                    }
                }
            }
            _ => {}
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            new_source,
            reload,
            target_chars,
            get_char_tree,
            get_cst_table,
            gen_comp_path,
            open_struc_editor,
            get_struc_editor_data,
            save_struc,
            save_fas_file,
            is_changed,
            get_config,
            set_config,
            get_char_info,
            export_chars
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct ServiceInfo {
    file_name: String,
    name: String,
    major_version: u32,
    minor_version: u32,
}

impl ServiceInfo {
    pub fn new(service: &LocalService, path: &str) -> ServiceInfo {
        let source = service.source().unwrap();
        let versions = {
            let mut iter = source
                .version
                .split('.')
                .map(|n| n.parse::<u32>().unwrap_or_default());
            let mut versions = vec![];
            for _ in 0..2 {
                versions.push(iter.next().unwrap_or_default())
            }
            versions
        };
        ServiceInfo {
            file_name: std::path::Path::new(path)
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
}

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
            size: window.inner_size()?.to_logical::<u32>(factor),
            pos: window.outer_position()?.to_logical::<u32>(factor),
        })
    }
}

fn init() -> (Context, Service, Option<WindowState>, Option<ServiceInfo>) {
    let mut service = LocalService::new();
    let context = context::Context::default();

    let sinfo = context
        .get(json!("source"))
        .and_then(|path| match path {
            serde_json::Value::String(s) => Some(s),
            _ => None,
        })
        .and_then(|path| match service.load_file(&path) {
            Err(e) => {
                eprintln!("Failed open service source: {:?}", e);
                None
            }
            Ok(_) => Some(ServiceInfo::new(&service, &path)),
        });

    let wstate = context
        .get(json!("window"))
        .and_then(|data| serde_json::from_value::<WindowState>(data).ok());

    (Mutex::new(context), Mutex::new(service), wstate, sinfo)
}

fn exit_save(context: State<'_, Context>, window: &tauri::Window) {
    let mut guard = context.lock();
    let context = guard.as_mut().unwrap();
    if let Ok(win_state) = WindowState::new(window) {
        context.set(json!("window"), serde_json::to_value(win_state).unwrap());
    }

    if let Err(e) = context.save() {
        eprintln!("{:?}: {}", e, "Save context error!");
    };
}

fn set_window_title_as_serviceinfo(window: &tauri::WebviewWindow, info: &ServiceInfo) {
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
