use crate::{
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
pub struct CompInfo {
    name: String,
    tp: CstType,
    bases: DataHV<Vec<usize>>,
    i_attr: DataHV<Vec<String>>,
    i_notes: DataHV<Vec<String>>,
    assign: DataHV<Vec<f32>>,
    offset: DataHV<[f32; 2]>,
}

impl CompInfo {
    pub fn new(name: String, tp: CstType) -> Self {
        Self {
            name,
            tp,
            bases: Default::default(),
            i_attr: Default::default(),
            i_notes: Default::default(),
            assign: Default::default(),
            offset: Default::default(),
        }
    }
}

#[derive(serde::Serialize, Clone, Default)]
pub struct CharInfo {
    comb_info: String,
    pub level: DataHV<usize>,
    white_areas: DataHV<[f32; 2]>,
    scale: DataHV<f32>,
    center: [DataHV<f32>; 2],
    comp_infos: Vec<CompInfo>,
}

#[derive(Debug, Clone, Copy, Default)]
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
            intervals: Default::default(),
        }
    }

    pub fn get_char_box(&self) -> WorkBox {
        let mut char_box = WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0));
        if let Self::Single { proto, .. } = self {
            if let Some(cbox) = proto.attrs.get::<attrs::CharBox>() {
                let cbox = if let Ok(cbox) = serde_json::from_value::<WorkBox>(cbox.clone()) {
                    Some(cbox)
                } else if let Some(cbox_str) = cbox.as_str() {
                    match cbox_str {
                        "left" => Some(WorkBox::new(
                            WorkPoint::new(0.0, 0.0),
                            WorkPoint::new(0.5, 1.0),
                        )),
                        "right" => Some(WorkBox::new(
                            WorkPoint::new(0.5, 0.0),
                            WorkPoint::new(1.0, 1.0),
                        )),
                        "top" => Some(WorkBox::new(
                            WorkPoint::new(0.0, 0.0),
                            WorkPoint::new(1.0, 0.5),
                        )),
                        "bottom" => Some(WorkBox::new(
                            WorkPoint::new(0.0, 0.5),
                            WorkPoint::new(1.0, 1.0),
                        )),
                        _ => {
                            eprintln!("Unknown character box label: {}", cbox_str);
                            None
                        }
                    }
                } else {
                    None
                };

                if let Some(cbox) = cbox {
                    char_box.min = char_box.min.max(cbox.min);
                    char_box.max = char_box.max.min(cbox.max);
                }
            }
        }
        char_box
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

    fn check_space(&mut self, fas: &FasFile) -> Result<(DataHV<f32>, DataHV<[f32; 2]>), CstError> {
        let cfg = &fas.config;
        let char_box = self.get_char_box();
        let size = cfg
            .size
            .zip(char_box.size().to_hv_data())
            .into_map(|(a, b)| a * b);

        let mut assign = DataHV::default();
        let mut offsets: DataHV<[f32; 2]> = DataHV::default();

        let mut base_len_list = self.init_edges(cfg);
        let mut check_state = DataHV::splat(false);

        while !(check_state.h & check_state.v) {
            let axis = check_state.in_axis(|state| !state).unwrap();
            let white = cfg.white.hv_get(axis);

            loop {
                let edge_corr = [Place::Start, Place::End].map(|place| {
                    white.get_weight(&self.get_comb_edge(axis, place).to_elements(axis))
                });
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
                    .any(|&min| base_total * min < length + 0.0001);
                match ok {
                    true => {
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
                        } else {
                            let helf = *size.hv_get(axis) / 2.0;
                            *offsets.hv_get_mut(axis) = [helf; 2];
                            *assign.hv_get_mut(axis) = 0.0;
                        }
                        break;
                    }
                    false => {
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

    pub fn expand_comb_proto(&mut self, fas: &FasFile) -> Result<(), CstError> {
        let (assign, offsets) = self.check_space(fas)?;
        self.assign_space(assign, offsets, &fas.config);
        Ok(())
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

    pub fn to_paths(&self) -> Vec<Vec<WorkPoint>> {
        let mut paths = vec![];
        self.merge_to(self.get_char_box().min, &mut paths);
        paths
    }

    pub fn merge_to(&self, start: WorkPoint, paths: &mut Vec<Vec<WorkPoint>>) -> WorkSize {
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
