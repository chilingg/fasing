use crate::{
    algorithm as al,
    axis::*,
    component::{
        attrs,
        struc::StrucProto,
        view::{Edge, StrucView},
    },
    config::Config,
    construct::{space::*, CstError, CstType},
    fas::FasFile,
};

#[derive(serde::Serialize, Clone)]
pub enum CompInfo {
    Single {
        name: String,
        offsets: DataHV<[f32; 2]>,
        allocs: DataHV<Vec<usize>>,
        assign: DataHV<Vec<AssignVal>>,
    },
    Complex {
        name: String,
        offset: DataHV<[f32; 2]>,
        intervals: Vec<AssignVal>,
        intervals_alloc: Vec<usize>,
        intervals_attrs: Vec<String>,
    },
}

#[derive(serde::Serialize, Clone, Default)]
pub struct CharInfo {
    pub comb_name: String,
    pub levels: DataHV<usize>,
    pub white_areas: DataHV<[f32; 2]>,
    pub scales: DataHV<f32>,
    pub center: WorkPoint,
    pub comp_infos: Vec<CompInfo>,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct AssignVal {
    pub base: f32,
    pub excess: f32,
}

#[derive(Clone)]
pub enum StrucComb {
    Single {
        name: String,
        offsets: DataHV<[f32; 2]>,

        assign_vals: DataHV<Vec<AssignVal>>,
        view: StrucView,
        proto: StrucProto,
    },
    Complex {
        name: String,
        offsets: DataHV<[f32; 2]>,

        tp: CstType,
        combs: Vec<StrucComb>,
        intervals_alloc: Vec<usize>,
        intervals: Vec<AssignVal>, // hs he vs ve
    },
}

impl StrucComb {
    pub fn new_single(name: String, proto: StrucProto) -> Self {
        Self::Single {
            name,
            offsets: Default::default(),
            assign_vals: Default::default(),
            view: StrucView::new(&proto),
            proto,
        }
    }

    pub fn new_complex(name: String, tp: CstType, combs: Vec<StrucComb>) -> Self {
        Self::Complex {
            name,
            offsets: Default::default(),
            tp,
            combs,
            intervals_alloc: Default::default(),
            intervals: Default::default(),
        }
    }

    pub fn get_comb_name(&self) -> String {
        match self {
            Self::Single { name, .. } => name.clone(),
            Self::Complex {
                name, tp, combs, ..
            } => {
                format!(
                    "{}:{}({})",
                    name,
                    tp.symbol(),
                    combs
                        .iter()
                        .map(|c| c.get_comb_name())
                        .collect::<Vec<String>>()
                        .join("+")
                )
            }
        }
    }

    pub fn get_children_comp(&self) -> Vec<&StrucComb> {
        match self {
            Self::Single { .. } => vec![],
            Self::Complex { combs, .. } => combs.iter().collect(),
        }
    }

    pub fn get_comp_info(&self, list: &mut Vec<CompInfo>) {
        let name = self.get_comb_name();
        match self {
            Self::Single {
                offsets: ofts,
                assign_vals,
                proto,
                ..
            } => list.push(CompInfo::Single {
                name,
                offsets: ofts.clone(),
                allocs: proto.allocation_space(),
                assign: assign_vals.clone(),
            }),
            Self::Complex {
                offsets: ofts,
                combs,
                intervals_alloc: ia,
                intervals: inter,
                ..
            } => {
                list.push(CompInfo::Complex {
                    name,
                    offset: ofts.clone(),
                    intervals: inter.clone(),
                    intervals_alloc: ia.clone(),
                    intervals_attrs: Default::default(),
                });
                combs.iter().for_each(|c| c.get_comp_info(list));
            }
        }
    }

    pub fn get_proto_attr<T: attrs::CompAttrData>(&self) -> Option<<T as attrs::CompAttrData>::Data>
    where
        <T as attrs::CompAttrData>::Data: serde::de::DeserializeOwned,
    {
        match self {
            Self::Single { proto, .. } => proto.attrs.get::<T>(),
            Self::Complex { .. } => None,
        }
    }

    pub fn get_char_box(&self) -> WorkBox {
        if let Some(cbox) = self.get_proto_attr::<attrs::CharBox>() {
            cbox
        } else {
            WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0))
        }
    }

    pub fn get_comb_bases_length(&self, axis: Axis, _cfg: &Config) -> usize {
        match self {
            StrucComb::Single { proto, .. } => proto.allocation_space().hv_get(axis).iter().sum(),
            Self::Complex { .. } => todo!(),
        }
    }

    pub fn get_comb_edge(&self, axis: Axis, place: Place) -> Edge {
        match self {
            StrucComb::Single { view, .. } => view.read_edge(axis, place),
            StrucComb::Complex { .. } => todo!(),
        }
    }

    pub fn get_visual_center(&self, min_len: f32) -> WorkPoint {
        let mut paths = self.to_paths();
        al::split_intersect(&mut paths, min_len);
        let mut center = al::visual_center(&paths);

        if let Some(preset) = self.get_proto_attr::<attrs::PresetCenter>() {
            if let Some(val) = preset.h {
                center.x = val.clamp(0.0, 1.0);
            }
            if let Some(val) = preset.v {
                center.y = val.clamp(0.0, 1.0);
            }
        }
        center
    }

    pub fn init_edges(&mut self, _cfg: &Config) -> DataHV<usize> {
        match self {
            Self::Single { proto, .. } => {
                proto.allocation_values().map(|allocs| allocs.iter().sum())
            }
            Self::Complex { .. } => {
                todo!()
                // match tp {
                //     CstType::Scale(axis) => {
                //         combs
                //             .iter()
                //             .zip(combs.iter().skip(1))
                //             .for_each(|(c1, c2)| {});
                //     }
                //     CstType::Surround(surround) => {}
                //     CstType::Single => unreachable!(),
                // }
            }
        }
    }

    fn check_space(
        &mut self,
        fas: &FasFile,
    ) -> Result<(DataHV<f32>, DataHV<[f32; 2]>, DataHV<usize>, DataHV<f32>), CstError> {
        let cfg = &fas.config;
        let char_box = self.get_char_box();
        let size = cfg
            .size
            .zip(char_box.size().to_hv_data())
            .into_map(|(a, b)| a * b);

        let mut assign = DataHV::default();
        let mut offsets: DataHV<[f32; 2]> = DataHV::default();
        let mut levels: DataHV<usize> = DataHV::default();
        let mut scales: DataHV<f32> = DataHV::default();

        let mut base_len_list = self.init_edges(cfg);
        let mut check_state = DataHV::splat(false);

        while !(check_state.h & check_state.v) {
            let axis = check_state.in_axis(|state| !state).unwrap();
            let white = cfg.white.hv_get(axis);

            loop {
                let edge_corr = [Place::Start, Place::End]
                    .map(|place| self.get_comb_edge(axis, place).gray_scale(cfg.strok_width));
                let length = *size.hv_get(axis)
                    - white.fixed * 2.0
                    - white.value * (edge_corr[0] + edge_corr[1]);
                let edge_base = edge_corr.map(|v| v * white.allocated);
                let base_len = *base_len_list.hv_get(axis);

                let base_total = base_len as f32 + edge_base[0] + edge_base[1];
                let ok = cfg
                    .min_val
                    .hv_get(axis)
                    .iter()
                    .enumerate()
                    .find_map(|(i, &min)| match base_total * min < length + 0.0001 {
                        true => Some(i),
                        false => None,
                    });
                match ok {
                    Some(level) => {
                        if base_len != 0 {
                            let scale = length / base_total;
                            if scale < *cfg.reduce_trigger.hv_get(axis) && self.reduce_space(axis) {
                                *check_state.hv_get_mut(axis.inverse()) = false;
                                base_len_list = self.init_edges(cfg);
                                continue;
                            }
                            (0..=1).into_iter().for_each(|i| {
                                offsets.hv_get_mut(axis)[i] =
                                    edge_base[i] * scale + edge_corr[i] * white.value + white.fixed
                            });
                            *assign.hv_get_mut(axis) = base_len as f32 * scale;
                            *scales.hv_get_mut(axis) = scale;
                        } else {
                            let helf = *size.hv_get(axis) / 2.0;
                            *offsets.hv_get_mut(axis) = [helf; 2];
                            *assign.hv_get_mut(axis) = 0.0;
                        }
                        *levels.hv_get_mut(axis) = level;
                        break;
                    }
                    None => {
                        if self.reduce_space(axis) {
                            *check_state.hv_get_mut(axis.inverse()) = false;
                            base_len_list = self.init_edges(cfg);
                            continue;
                        }
                        return Err(CstError::AxisTransform {
                            axis,
                            length,
                            base_len,
                        });
                    }
                }
            }
            *check_state.hv_get_mut(axis) = true;
        }

        Ok((
            assign,
            offsets
                .zip(char_box.min.to_hv_data())
                .into_map(|(a, b)| [a[0] + b, a[1]]),
            levels,
            scales,
        ))
    }

    fn assign_space(&mut self, assign: DataHV<f32>, offsets: DataHV<[f32; 2]>, cfg: &Config) {
        match self {
            StrucComb::Single {
                assign_vals: asg,
                offsets: ofs,
                proto,
                ..
            } => {
                *ofs = offsets;

                let allocs = proto.allocation_space();
                let assign_vals = Axis::hv().into_map(|axis| {
                    let allocs = allocs.hv_get(axis);
                    let assign = *assign.hv_get(axis);

                    let alloc_total = allocs.iter().sum::<usize>() as f32;
                    if alloc_total == 0.0 {
                        vec![Default::default(); allocs.len()]
                    } else {
                        let scale = assign / alloc_total;
                        let min_val = *cfg
                            .min_val
                            .hv_get(axis)
                            .iter()
                            .find(|&&n| n < scale)
                            .unwrap();

                        allocs
                            .iter()
                            .map(|&n| {
                                let n = n as f32;
                                let space = n * scale;
                                let base = n * min_val;
                                AssignVal {
                                    base,
                                    excess: space - base,
                                }
                            })
                            .collect()
                    }
                });
                *asg = assign_vals;
            }
            StrucComb::Complex { .. } => todo!(), // assign complex
        }
    }

    pub fn process_space(&mut self, levels: DataHV<usize>, cfg: &Config) {
        let min_len = cfg.min_val.h[levels.h].min(cfg.min_val.v[levels.v]);

        if matches!(self, Self::Single { .. }) {
            let center = self.get_visual_center(min_len);

            if let Self::Single { assign_vals, .. } = self {
                for axis in Axis::list() {
                    let assign_vals = assign_vals.hv_get_mut(axis);
                    let center_opt = &cfg.center.hv_get(axis);
                    let new_vals = al::center_correction(
                        &assign_vals.iter().map(|av| av.base + av.excess).collect(),
                        &assign_vals.iter().map(|av| av.base).collect(),
                        *center.hv_get(axis),
                        center_opt.operation,
                        center_opt.execution,
                    );
                    new_vals
                        .into_iter()
                        .zip(assign_vals.iter_mut())
                        .for_each(|(nval, aval)| {
                            aval.excess = nval;
                        });
                }
            }
        } else {
            todo!(); // process space with Complex
        }
    }

    pub fn expand_comb_proto(
        &mut self,
        fas: &FasFile,
        gen_info: bool,
    ) -> Result<Option<CharInfo>, CstError> {
        let (assign, offsets, levels, scales) = self.check_space(fas)?;
        self.assign_space(assign, offsets, &fas.config);
        self.process_space(levels, &fas.config);

        if gen_info {
            let min_len = fas.config.min_val.h[levels.h].min(fas.config.min_val.v[levels.v]);
            let mut comp_infos = vec![];
            self.get_comp_info(&mut comp_infos);

            Ok(Some(CharInfo {
                comb_name: self.get_comb_name(),
                levels,
                white_areas: offsets,
                scales,
                center: self.get_visual_center(min_len),
                comp_infos,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn reduce_space(&mut self, axis: Axis) -> bool {
        match self {
            StrucComb::Single { view, proto, .. } => {
                if proto.reduce(axis) {
                    *view = StrucView::new(proto);
                    true
                } else {
                    false
                }
            }
            Self::Complex { .. } => todo!(), // reduce complex
        }
    }

    pub fn to_paths(&self) -> Vec<KeyWorkPath> {
        let mut paths = vec![];
        self.merge_to(self.get_char_box().min, &mut paths);
        paths
    }

    pub fn merge_to(&self, start: WorkPoint, paths: &mut Vec<KeyWorkPath>) -> WorkSize {
        match self {
            Self::Single {
                assign_vals,
                offsets,
                proto,
                ..
            } => {
                let new_start = WorkPoint::new(start.x + offsets.h[0], start.y + offsets.v[0]);
                let assigns =
                    assign_vals.map(|assign| assign.iter().map(|av| av.base + av.excess).collect());
                let size = assigns.map(|assigns: &Vec<f32>| assigns.iter().sum::<f32>());
                paths.extend(proto.to_paths(new_start, assigns));
                WorkSize::new(
                    new_start.x + size.h + offsets.h[1],
                    new_start.y + size.v + offsets.v[1],
                )
            }
            StrucComb::Complex { .. } => todo!(), // mergo to Complex
        }
    }
}
