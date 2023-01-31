use crate::gui;

pub mod prelude {
    pub use super::gui::prelude::*;
}

mod main_widget;
pub use main_widget::MainWidget;

mod sidebar;
mod center;
mod query_window;