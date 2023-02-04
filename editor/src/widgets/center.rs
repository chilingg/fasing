use crate::gui::prelude::*;
use super::mete_comp_works::MeteCompWorks;

pub struct Center {
    current: usize,
    children: Vec<Box<dyn Widget>>,
}

impl std::default::Default for Center {
    fn default() -> Self {
        Self {
            current: 0,
            children: vec![
                widget_box(MeteCompWorks::default()),
            ],
        }
    }
}

impl Widget for Center {
    fn children(&mut self) -> Children {
        vec![&mut self.children[self.current]]
    }
}