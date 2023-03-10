mod gui;
mod widgets;

mod prelude {
    pub use super::app::{CoreData, RequestCache, RunData};
    pub use super::gui::widget::Widget;
    pub type Children<'a> = super::gui::widget::Children<'a, CoreData, RunData>;
}

mod app;
pub use app::App;
