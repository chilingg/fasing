use crate::{
    base::*,
    combination::{SharpnessModel, StrucProto, StrucView, attrs},
    construct::{CharTree, CstType},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CompTree {
    pub name: String,
    pub tp: CstType,
    pub paths: Vec<WorkKeyPath>,
    pub comps: Vec<CompTree>,
}

pub enum CompData {
    Single {
        proto: StrucProto,
        view: StrucView,
        assigns: DataHV<Vec<AssignVal>>,
    },
    Complex {
        tp: CstType,
        comps: Vec<StrucComb>,
    },
}

pub struct StrucComb {
    pub name: String,
    pub offsets: DataHV<[AssignVal; 2]>,
    pub blank: DataHV<[Option<f32>; 2]>,
    pub cdata: CompData,
    pub attrs: attrs::CompAttrs,
}

impl StrucComb {
    pub fn new_single(name: String, proto: StrucProto) -> Self {
        Self {
            name,
            offsets: Default::default(),
            blank: Default::default(),
            cdata: CompData::Single {
                view: StrucView::new(&proto),
                proto,
                assigns: Default::default(),
            },
            attrs: Default::default(),
        }
    }

    pub fn get_char_tree(&self) -> CharTree {
        match &self.cdata {
            CompData::Single { .. } => CharTree::new_single(self.name.clone()),
            CompData::Complex {
                comps: combs, tp, ..
            } => CharTree {
                name: self.name.clone(),
                tp: *tp,
                children: combs.iter().map(|c| c.get_char_tree()).collect(),
            },
        }
    }

    pub fn get_char_box(&self) -> WorkBox {
        match &self.cdata {
            CompData::Single { proto, .. } => proto.attrs.get::<attrs::CharBox>(),
            _ => None,
        }
        .unwrap_or(WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0)))
    }

    pub fn get_bases_length(&self, axis: Axis) -> usize {
        match &self.cdata {
            CompData::Single { view, .. } => *view.space_size().hv_get(axis),
            CompData::Complex { .. } => todo!(), // complex
        }
    }

    pub fn get_paths(&self) -> CompTree {
        let (tree, _) = self.get_paths_in(self.get_char_box().min);
        tree
    }

    pub fn get_paths_in(&self, start: WorkPoint) -> (CompTree, WorkSize) {
        match &self.cdata {
            CompData::Single { proto, assigns, .. } => {
                let offsets = &self.offsets;
                let new_start = WorkPoint::new(
                    start.x + offsets.h[0].total(),
                    start.y + offsets.v[0].total(),
                );
                let assigns = assigns.map(|assign| assign.iter().map(|av| av.total()).collect());
                let c_size = assigns.map(|assigns: &Vec<f32>| assigns.iter().sum::<f32>());
                let paths = proto.get_paths(new_start, assigns);

                let size = WorkSize::new(
                    offsets.h[0].total() + c_size.h + offsets.h[1].total(),
                    offsets.v[0].total() + c_size.v + offsets.v[1].total(),
                );
                let tree = CompTree {
                    name: self.name.clone(),
                    tp: CstType::Single,
                    paths,
                    comps: vec![],
                };
                (tree, size)
            }
            CompData::Complex { .. } => todo!(), // complex
        }
    }

    pub fn edge_sharpness(&self, axis: Axis, place: Place, model: SharpnessModel) -> f32 {
        match &self.cdata {
            CompData::Single { view, .. } => view.edge_sharpness(axis, place, model),
            _ => todo!(), // complex
        }
    }

    pub fn reduce_space(&mut self, axis: Axis, is_check: bool) -> bool {
        let new_length = match &mut self.cdata {
            CompData::Single { proto, view, .. } => {
                if proto.reduce(axis, is_check) {
                    if !is_check {
                        *view = StrucView::new(proto);
                    }
                    Some(*view.space_size().hv_get(axis))
                } else {
                    None
                }
            }
            CompData::Complex { .. } => todo!(), // complex
        };

        if let Some(new_len) = new_length {
            if !is_check {
                let mut r_target = self.attrs.get::<attrs::ReduceTarget>().unwrap_or_default();
                *r_target.hv_get_mut(axis) = Some(new_len);
                self.attrs.set::<attrs::ReduceTarget>(&r_target);
            }
            true
        } else {
            false
        }
    }
}
