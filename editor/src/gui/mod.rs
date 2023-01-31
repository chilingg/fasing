pub mod app;
pub mod theme;

pub mod prelude {
    pub use super::app::{
        AppState,
        RootWidget,
        Widget,
        WidgetData,
        CoreData,
        Children,
        Task,
        we,
        widget_box,
    };
}