mod gui;
pub mod paint;
mod widgets;

mod app;
pub use app::App;

mod prelude {
    pub use super::app::{CoreData, RequestCache, RunData};
    pub use super::gui::widget::Widget;
    pub use super::paint;
    pub type Children<'a> = super::gui::widget::Children<'a, CoreData, RunData>;
}
