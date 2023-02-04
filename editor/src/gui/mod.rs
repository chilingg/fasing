pub mod app;
pub mod theme;

pub mod prelude {
    pub use super::app::{
        AppState,
        Widget,
        WidgetData,
        CoreData,
        UserData,
        Children,
        Task,
        we,
        widget_box,
    };
}