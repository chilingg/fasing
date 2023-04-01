use super::{view::StrucAllAttrView, StrucAllocates, StrucAttributes, StrucProto};
use crate::{fas_file::AllocateTable, Axis};

use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct StrucVarietys {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub view: StrucAllAttrView,
    pub sub_varietys: HashMap<Axis, Option<StrucVarietys>>,
}

impl StrucVarietys {
    pub fn from_attrs(
        proto: StrucProto,
        attrs: StrucAttributes,
        alloc_tab: &AllocateTable,
    ) -> Self {
        Self {
            view: StrucAllAttrView::new(&proto),
            proto,
            allocs: attrs.get_space_allocates(alloc_tab),
            attrs,
            sub_varietys: Default::default(),
        }
    }

    pub fn from_allocs(proto: StrucProto, allocs: StrucAllocates) -> Self {
        Self {
            view: StrucAllAttrView::new(&proto),
            attrs: proto.attributes(),
            proto,
            allocs,
            sub_varietys: Default::default(),
        }
    }

    pub fn can_reduce(&self, regex: &regex::Regex, axis: Axis) -> bool {
        self.attrs
            .get(axis)
            .iter()
            .find(|a| regex.is_match(a))
            .is_some()
    }

    pub fn get_reduce_horizontal(
        &mut self,
        regex: &regex::Regex,
        mut level: usize,
    ) -> Option<&StrucVarietys> {
        fn get_reduce(proto: &StrucVarietys, regex: &regex::Regex) -> Option<StrucVarietys> {
            let range: Vec<_> = (0..proto.attrs.h.len()).collect();
            let (front, back) = range.split_at(range.len() / 2);
            let front_reduce = front
                .iter()
                .find(|n| regex.is_match(proto.attrs.h[**n].as_str()))
                .copied();
            let back_reduce = back
                .iter()
                .rev()
                .find(|n| regex.is_match(proto.attrs.h[**n].as_str()))
                .copied();
            if front_reduce.is_none() && back_reduce.is_none() {
                None
            } else {
                let mut alloc = proto.allocs.clone();
                let mut sub_proto = proto.proto.clone();
                if let Some(re) = back_reduce {
                    sub_proto = sub_proto.reduce(Axis::Horizontal, re);
                    alloc.h.remove(re);
                }
                if let Some(re) = front_reduce {
                    sub_proto = sub_proto.reduce(Axis::Horizontal, re);
                    alloc.h.remove(re);
                }
                Some(StrucVarietys::from_allocs(sub_proto, alloc))
            }
        }

        let mut proto = self;
        loop {
            match level {
                0 => return Some(proto),
                _ => {
                    level -= 1;
                    if proto.sub_varietys.contains_key(&Axis::Horizontal) {
                        match proto.sub_varietys.get_mut(&Axis::Horizontal).unwrap() {
                            Some(p) => proto = p,
                            None => return None,
                        }
                    } else {
                        match get_reduce(proto, regex) {
                            Some(p) => {
                                proto.sub_varietys.insert(Axis::Horizontal, Some(p));
                                proto = proto
                                    .sub_varietys
                                    .get_mut(&Axis::Horizontal)
                                    .unwrap()
                                    .as_mut()
                                    .unwrap();
                            }
                            None => {
                                proto.sub_varietys.insert(Axis::Horizontal, None);
                                return None;
                            }
                        }
                    }
                }
            }
        }
    }
}
