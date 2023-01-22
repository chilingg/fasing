use super::app::AppState;

pub mod we {
    pub use winit::event::*;
}

// pub enum Event {
//     Start,
//     Finish,
//     Update,
//     Process,
//     Render,
// }

pub struct StartEvent<'a> {
    pub app_state: &'a mut AppState,
}

pub struct FinishEvent<'a> {
    pub app_state: &'a mut AppState,
}

pub struct UpdateEvent<'a> {
    pub egui_ctx: &'a egui::Context,
}

pub struct ProcessEvent<'a> {
    pub window_event: &'a winit::event::WindowEvent<'a>,
    pub app_state: &'a mut AppState,
}
