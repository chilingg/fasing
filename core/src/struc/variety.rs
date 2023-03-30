use super::{view::StrucAllAttrView, StrucAllocates, StrucAttributes, StrucProto};
use crate::{construct, fas_file::AllocateTable};

use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct StrucVarietys {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub view: StrucAllAttrView,
    pub sub_varietys: HashMap<construct::Format, StrucVarietys>,
}

impl StrucVarietys {
    pub fn from_attrs(
        proto: StrucProto,
        attrs: StrucAttributes,
        alloc_tab: &AllocateTable,
    ) -> Self {
        Self {
            view: StrucAllAttrView::new(&proto).unwrap_or_default(),
            proto,
            allocs: attrs.get_space_allocates(alloc_tab),
            attrs,
            sub_varietys: Default::default(),
        }
    }
}
