use super::{StrucAllocates, StrucAttributes, StrucProto};
use crate::{construct, fas_file::AllocateTable};

use std::collections::HashMap;

#[derive(Default)]
pub struct StrucVarietys {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub format_allocs: HashMap<construct::Format, StrucVarietys>,
}

impl StrucVarietys {
    pub fn from_attrs(
        proto: StrucProto,
        attrs: StrucAttributes,
        alloc_tab: &AllocateTable,
    ) -> Self {
        Self {
            proto,
            allocs: attrs.get_space_allocates(alloc_tab),
            attrs,
            format_allocs: Default::default(),
        }
    }
}
