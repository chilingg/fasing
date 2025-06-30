use crate::{
    algorithm as al,
    axis::*,
    component::{
        attrs,
        struc::StrucProto,
        view::{StandardEdge, StrucView, ViewLines},
    },
    config::{setting, Config, WhiteArea},
    construct::{space::*, CharTree, CstError, CstTable, CstType},
    fas::FasFile,
};

use std::collections::HashMap;

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
        comb_name: String,
        intervals: Vec<AssignVal>,
        intervals_alloc: Vec<usize>,
        edges: Vec<[StandardEdge; 2]>,
    },
}

#[derive(serde::Serialize, Clone, Default)]
pub struct CharInfo {
    pub comb_name: String,
    pub levels: DataHV<usize>,
    pub base_size: DataHV<usize>,
    pub scales: DataHV<f32>,
    pub center: WorkPoint,
    pub comp_infos: Vec<CompInfo>,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct AssignVal {
    pub base: f32,
    pub excess: f32,
}

impl std::cmp::PartialEq for AssignVal {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.excess == other.excess
    }
}

impl std::cmp::PartialOrd for AssignVal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.base.partial_cmp(&other.base) {
            Some(std::cmp::Ordering::Equal) => self.excess.partial_cmp(&other.excess),
            cmp => cmp,
        }
    }
}

impl std::ops::Add for AssignVal {
    type Output = AssignVal;

    fn add(self, rhs: AssignVal) -> Self::Output {
        AssignVal {
            base: self.base + rhs.base,
            excess: self.excess + rhs.excess,
        }
    }
}

impl std::ops::Sub for AssignVal {
    type Output = AssignVal;

    fn sub(self, rhs: Self) -> Self::Output {
        AssignVal {
            base: self.base - rhs.base,
            excess: self.excess - rhs.excess,
        }
    }
}

impl std::iter::Sum for AssignVal {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| AssignVal {
            base: a.base + b.base,
            excess: a.excess + b.excess,
        })
    }
}
impl<'a> std::iter::Sum<&'a AssignVal> for AssignVal {
    fn sum<I: Iterator<Item = &'a AssignVal>>(iter: I) -> Self {
        iter.fold(Self::default(), |a, b| AssignVal {
            base: a.base + b.base,
            excess: a.excess + b.excess,
        })
    }
}

impl AssignVal {
    pub fn new(base: f32, excess: f32) -> Self {
        Self { base, excess }
    }

    pub fn total(&self) -> f32 {
        self.base + self.excess
    }
}

#[derive(Clone)]
pub enum StrucComb {
    Single {
        name: String,
        offsets: DataHV<[AssignVal; 2]>,
        assign_vals: DataHV<Vec<AssignVal>>,
        view: StrucView,
        proto: StrucProto,
    },
    Complex {
        name: String,
        tp: CstType,
        offsets: DataHV<[AssignVal; 2]>,
        intervals_alloc: Vec<usize>,
        intervals: Vec<AssignVal>, // hs he vs ve
        edge_main: DataHV<HashMap<Place, Vec<bool>>>,
        combs: Vec<StrucComb>,
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
            tp,
            offsets: Default::default(),
            intervals_alloc: Default::default(),
            intervals: Default::default(),
            edge_main: Default::default(),
            combs,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Self::Single { name, .. } => name,
            Self::Complex { name, .. } => name,
        }
    }

    pub fn get_char_tree(&self) -> CharTree {
        match self {
            Self::Single { name, .. } => CharTree {
                name: name.to_string(),
                tp: CstType::Single,
                children: vec![],
            },
            Self::Complex {
                name, combs, tp, ..
            } => {
                let children = combs.iter().map(|c| c.get_char_tree()).collect();
                CharTree {
                    name: name.to_string(),
                    tp: *tp,
                    children,
                }
            }
        }
    }

    pub fn get_comb_name(&self) -> String {
        match self {
            Self::Single { name, .. } => name.clone(),
            Self::Complex {
                tp, combs, name, ..
            } => {
                format!(
                    "{} {}({})",
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

    pub fn get_comp_info(&self, list: &mut Vec<CompInfo>, dot_val: f32) {
        match self {
            Self::Single {
                assign_vals,
                proto,
                offsets,
                name,
                ..
            } => {
                list.push(CompInfo::Single {
                    name: name.to_string(),
                    offsets: offsets.map(|vals| vals.map(|v| v.total())),
                    allocs: proto.allocation_space(),
                    assign: assign_vals.clone(),
                });
            }
            Self::Complex {
                combs,
                intervals_alloc: ia,
                intervals: inter,
                name,
                tp,
                edge_main,
                ..
            } => {
                let edges = match *tp {
                    CstType::Scale(axis) => combs
                        .iter()
                        .zip(combs.iter().skip(1))
                        .enumerate()
                        .map(|(i, (c1, c2))| {
                            Self::gen_edges_in_scale(
                                c1,
                                c2,
                                edge_main
                                    .hv_get(axis.inverse())
                                    .iter()
                                    .map(|(k, v)| (*k, &v[i..=i + 1]))
                                    .collect(),
                                axis,
                            )
                        })
                        .collect(),
                    CstType::Surround(_) => vec![],
                    CstType::Single => unreachable!(),
                };
                list.push(CompInfo::Complex {
                    name: name.to_string(),
                    comb_name: self.get_comb_name(),
                    intervals: inter.clone(),
                    intervals_alloc: ia.clone(),
                    edges,
                });
                combs.iter().for_each(|c| c.get_comp_info(list, dot_val));
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

    pub fn get_main_base_length(&self, axis: Axis) -> usize {
        match self {
            Self::Single { proto, .. } => proto.allocation_space().hv_get(axis).iter().sum(),
            Self::Complex { tp, combs, .. } => match *tp {
                CstType::Scale(comb_axis) => {
                    if axis == comb_axis {
                        combs
                            .iter()
                            .map(|c| c.get_main_base_length(axis))
                            .max()
                            .unwrap()
                    } else {
                        let list: Vec<usize> =
                            combs.iter().map(|c| c.get_bases_length(axis)).collect();
                        let max = *list.iter().max().unwrap();
                        list.into_iter()
                            .enumerate()
                            .filter_map(|(i, s)| {
                                if s == max {
                                    Some(combs[i].get_main_base_length(axis))
                                } else {
                                    None
                                }
                            })
                            .max()
                            .unwrap()
                    }
                }
                CstType::Surround(_) => combs
                    .iter()
                    .map(|c| c.get_main_base_length(axis))
                    .max()
                    .unwrap(),
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_line_length(&self, scale: DataHV<f32>) -> f32 {
        match self {
            Self::Single { proto, .. } => proto.line_length(scale),
            Self::Complex {
                tp,
                combs,
                intervals_alloc,
                ..
            } => match *tp {
                CstType::Scale(comb_axis) => {
                    let max_len = self.get_bases_length(comb_axis.inverse()) as f32;

                    combs
                        .iter()
                        .map(|c| {
                            let mut c_scale = scale;

                            if max_len == 0.0 {
                                *c_scale.hv_get_mut(comb_axis.inverse()) = 1.0;
                            } else {
                                *c_scale.hv_get_mut(comb_axis.inverse()) = max_len
                                    / c.get_bases_length(comb_axis.inverse()) as f32
                                    * scale.hv_get(comb_axis.inverse());
                            }
                            c.get_line_length(c_scale)
                        })
                        .sum()
                }
                CstType::Surround(surround) => {
                    let mut p_scale = scale;
                    let mut s_scale = scale;
                    let sur_area = combs[0].get_surround_area(surround).unwrap();

                    for axis in Axis::list() {
                        let sur_len = sur_area.hv_get(axis)[1] - sur_area.hv_get(axis)[0];
                        let secondary_len = combs[1].get_bases_length(axis);
                        let interval_len: usize = match axis {
                            Axis::Horizontal => intervals_alloc[..2].iter().sum(),
                            Axis::Vertical => intervals_alloc[2..].iter().sum(),
                        };

                        if sur_len > interval_len + secondary_len {
                            if secondary_len != 0 {
                                *s_scale.hv_get_mut(axis) *=
                                    (sur_len - interval_len) as f32 / secondary_len as f32;
                            }
                        } else {
                            let p_len = combs[0].get_bases_length(axis);
                            *p_scale.hv_get_mut(axis) *=
                                (p_len - sur_len + interval_len + secondary_len) as f32
                                    / p_len as f32;
                        }
                    }

                    let p_length = combs[0].get_line_length(p_scale);
                    let s_length = combs[1].get_line_length(s_scale);

                    p_length + s_length
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_bases_length(&self, axis: Axis) -> usize {
        match self {
            Self::Single { proto, .. } => proto.allocation_space().hv_get(axis).iter().sum(),
            Self::Complex {
                tp,
                intervals_alloc,
                combs,
                ..
            } => match *tp {
                CstType::Scale(comb_axis) => {
                    let list = combs.iter().map(|c| c.get_bases_length(axis));
                    if axis == comb_axis {
                        list.chain(intervals_alloc.iter().copied()).sum()
                    } else {
                        list.max().unwrap()
                    }
                }
                CstType::Surround(surround) => {
                    let sur_area_len = match &combs[0] {
                        Self::Single { view, .. } => {
                            let area = *view.surround_area(surround).unwrap().hv_get(axis);
                            area[1] - area[0]
                        }
                        Self::Complex { .. } => unreachable!(),
                    };

                    let primary_len = combs[0].get_bases_length(axis);
                    let secondary_len = combs[1].get_bases_length(axis);
                    let interval_len: usize = match axis {
                        Axis::Horizontal => intervals_alloc[..2].iter().sum(),
                        Axis::Vertical => intervals_alloc[2..].iter().sum(),
                    };

                    if sur_area_len > interval_len + secondary_len {
                        primary_len
                    } else {
                        primary_len - sur_area_len + interval_len + secondary_len
                    }
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_assign_length(&self, axis: Axis) -> AssignVal {
        match self {
            Self::Single { assign_vals, .. } => assign_vals.hv_get(axis).iter().sum(),
            Self::Complex {
                tp,
                intervals,
                combs,
                ..
            } => match *tp {
                CstType::Scale(comb_axis) => {
                    let list = combs.iter().map(|c| c.get_assign_length(axis));
                    if axis == comb_axis {
                        list.chain(intervals.iter().copied()).sum()
                    } else {
                        list.max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
                    }
                }
                CstType::Surround(surround) => {
                    let primary_len = if let Self::Single {
                        view,
                        proto,
                        assign_vals,
                        ..
                    } = &combs[0]
                    {
                        let area = *view.surround_area(surround).unwrap().hv_get(axis);
                        let indexes = proto.value_index_in_axis(&area, axis);

                        [
                            assign_vals.hv_get(axis)[..indexes[0]].iter().sum(),
                            assign_vals.hv_get(axis)[indexes[0]..indexes[1]]
                                .iter()
                                .sum(),
                            assign_vals.hv_get(axis)[indexes[1]..].iter().sum(),
                        ]
                    } else {
                        panic!()
                    };

                    let secondary_len = combs[1].get_assign_length(axis);
                    let interval_len: AssignVal = match axis {
                        Axis::Horizontal => intervals[..2].iter().sum(),
                        Axis::Vertical => intervals[2..].iter().sum(),
                    };

                    if primary_len[1] < interval_len + secondary_len {
                        primary_len[0] + interval_len + secondary_len + primary_len[2]
                    } else {
                        primary_len.iter().sum()
                    }
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_offsets(&self) -> DataHV<[AssignVal; 2]> {
        match self {
            Self::Single { offsets, .. } => *offsets,
            Self::Complex { offsets, .. } => *offsets,
        }
    }

    fn get_surround_comp_gray(
        &self,
        surround: DataHV<Place>,
        axis: Axis,
        place: Place,
        dot_val: f32,
    ) -> f32 {
        if let Self::Single { view, .. } = self {
            let area = *view.surround_area(surround).unwrap().hv_get(axis.inverse());
            let view_size = *view.size().into_map(|s| s - 1).hv_get(axis.inverse());
            let lines = view.read_lines(axis, place);

            if *surround.hv_get(axis) == place.inverse() {
                let gray1 = lines.slice(0, area[0]).to_edge().gray_scale(dot_val);
                let gray2 = lines
                    .slice(area[1], view_size)
                    .to_edge()
                    .gray_scale(dot_val);
                gray1.max(gray2)
            } else {
                lines.to_edge().gray_scale(dot_val)
            }
        } else {
            unreachable!()
        }
    }

    pub fn get_type(&self) -> CstType {
        match self {
            Self::Single { .. } => CstType::Single,
            Self::Complex { tp, .. } => *tp,
        }
    }

    pub fn get_min_gray(&self, axis: Axis, place: Place, dot_val: f32) -> f32 {
        match self {
            Self::Single { view, .. } => view.read_lines(axis, place).to_edge().gray_scale(dot_val),
            Self::Complex {
                tp,
                edge_main,
                combs,
                ..
            } => match tp {
                CstType::Scale(comb_axis) => {
                    if axis == *comb_axis {
                        let idx = if place == Place::Start {
                            0
                        } else {
                            combs.len() - 1
                        };
                        combs[idx].get_min_gray(axis, place, dot_val)
                    } else {
                        (0..combs.len() - 1)
                            .filter(|i| edge_main.hv_get(axis)[&place][*i])
                            .map(|i| combs[i].get_min_gray(axis, place, dot_val))
                            .reduce(f32::min)
                            .unwrap()
                    }
                }
                CstType::Surround(surround) => {
                    if *surround.hv_get(axis) == place.inverse() {
                        combs[0]
                            .get_surround_comp_gray(*surround, axis, place, dot_val)
                            .min(combs[1].get_min_gray(axis, place, dot_val))
                    } else {
                        combs[0].get_min_gray(axis, place, dot_val)
                    }
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    fn get_adjacency(&self) -> DataHV<[bool; 2]> {
        match self {
            Self::Single { proto, .. } => proto.attrs.get::<attrs::Adjacencies>().unwrap(),
            Self::Complex { tp, combs, .. } => match *tp {
                CstType::Scale(comb_axis) => {
                    let mut a1 = combs[0].get_adjacency();
                    let a2 = combs.last().unwrap().get_adjacency();
                    a1.hv_get_mut(comb_axis)[1] = a2.hv_get(comb_axis)[1];

                    a1
                }
                CstType::Surround(_) => combs[0].get_adjacency(),
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_comb_lines(&self, axis: Axis, place: Place) -> ViewLines {
        match self {
            Self::Single { view, .. } => view.read_lines(axis, place),
            Self::Complex {
                tp,
                intervals_alloc,
                edge_main,
                combs,
                ..
            } => match *tp {
                CstType::Scale(comb_axis) => {
                    if comb_axis == axis {
                        match place {
                            Place::Start => combs[0].get_comb_lines(axis, place),
                            Place::End => combs.last().unwrap().get_comb_lines(axis, place),
                            _ => panic!("Parameter cannot be {:?}", place),
                        }
                    } else {
                        let edge_main = edge_main.hv_get(axis);
                        combs
                            .iter()
                            .map(|c| c.get_comb_lines(axis, place))
                            .enumerate()
                            .reduce(|(i1, mut lines1), (i2, mut lines2)| {
                                if i1 != usize::MAX && !edge_main[&place][i1] {
                                    lines1.backspace();
                                }
                                if !edge_main[&place][i2] {
                                    lines2.backspace();
                                }

                                match intervals_alloc[i2 - 1] {
                                    0 => {
                                        let mut temp = lines1.l.pop().unwrap();
                                        lines2.l[0][0].append(&mut temp[0]);
                                        lines2.l[0][1].append(&mut temp[1]);
                                    }
                                    n => lines1.add_gap(Place::End, n - 1),
                                }
                                lines1.connect(lines2);
                                (usize::MAX, lines1)
                            })
                            .unwrap()
                            .1
                    }
                }
                CstType::Surround(surround) => {
                    if *surround.hv_get(axis) != place.inverse() {
                        combs[0].get_comb_lines(axis, place)
                    } else {
                        match &combs[0] {
                            Self::Single { view, .. } => {
                                let edge_main = edge_main.hv_get(axis).get(&place).unwrap();
                                let surround_place = *surround.hv_get(axis.inverse());
                                let area =
                                    *view.surround_area(surround).unwrap().hv_get(axis.inverse());
                                let view_size = view.size().into_map(|val| val - 1);
                                let segment = match place {
                                    Place::Start => 0,
                                    Place::End => *view_size.hv_get(axis),
                                    Place::Middle => unreachable!(),
                                };
                                let mut lines = ViewLines {
                                    l: Default::default(),
                                    place,
                                    axis,
                                };

                                if surround_place != Place::End {
                                    let mut a_lines =
                                        view.read_lines_in(axis, 0, area[0], segment, place);
                                    if !edge_main[0] {
                                        a_lines.backspace();
                                    }
                                    lines.connect(a_lines);
                                }

                                {
                                    let intervals_alloc = match axis {
                                        Axis::Horizontal => &intervals_alloc[2..4],
                                        Axis::Vertical => &intervals_alloc[0..2],
                                    };
                                    lines.add_gap(Place::End, intervals_alloc[0]);
                                    let mut a_lines = combs[1].get_comb_lines(axis, place);
                                    if !edge_main[1] {
                                        a_lines.backspace();
                                    }
                                    lines.connect(a_lines);
                                    lines.add_gap(Place::End, intervals_alloc[1]);
                                }

                                if surround_place != Place::Start {
                                    let mut a_lines = view.read_lines_in(
                                        axis,
                                        area[1],
                                        *view_size.hv_get(axis.inverse()),
                                        segment,
                                        place,
                                    );
                                    if !edge_main[0] {
                                        a_lines.backspace();
                                    }
                                    lines.connect(a_lines);
                                }

                                lines
                            }
                            Self::Complex { .. } => unreachable!(),
                        }
                    }
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    pub fn get_visual_center(&self, min_len: f32, stroke_width: f32) -> WorkPoint {
        let paths = self.to_paths();
        let mut center = al::visual_center_length(paths, min_len, stroke_width);

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

    pub fn get_surround_area(
        &self,
        surround: DataHV<Place>,
    ) -> Result<DataHV<[usize; 2]>, CstError> {
        match self {
            Self::Single { view, name, .. } => {
                view.surround_area(surround).ok_or(CstError::Surround {
                    tp: CstType::Surround(surround).symbol(),
                    comp: name.to_string(),
                })
            }
            Self::Complex { .. } => panic!(),
        }
    }

    fn gen_edges_in_scale(
        c1: &StrucComb,
        c2: &StrucComb,
        edge_main: HashMap<Place, &[bool]>,
        axis: Axis,
    ) -> [StandardEdge; 2] {
        let mut lines = [
            c1.get_comb_lines(axis, Place::End),
            c2.get_comb_lines(axis, Place::Start),
        ];
        for place in Place::start_and_end() {
            if edge_main[&place][0] != edge_main[&place][1] {
                if edge_main[&place][0] {
                    lines[1].add_gap(place, 1);
                } else {
                    lines[0].add_gap(place, 1);
                }
            }
        }
        lines.map(|l| l.to_standard_edge())
    }

    fn gen_edges_in_surround(
        c1: &StrucComb,
        c2: &StrucComb,
        axis: Axis,
        area: DataHV<[usize; 2]>,
        edge_main: &DataHV<HashMap<Place, Vec<bool>>>,
        surround: DataHV<Place>,
    ) -> [Option<[StandardEdge; 2]>; 2] {
        match c1 {
            Self::Single { view, .. } => {
                let surround_main = *surround.hv_get(axis);

                let mut lines1 = if surround_main != Place::End {
                    Some([
                        view.read_lines_in(
                            axis,
                            area.hv_get(axis.inverse())[0],
                            area.hv_get(axis.inverse())[1],
                            area.hv_get(axis)[0],
                            Place::End,
                        ),
                        c2.get_comb_lines(axis, Place::Start),
                    ])
                } else {
                    None
                };
                let mut lines2 = if surround_main != Place::Start {
                    Some([
                        c2.get_comb_lines(axis, Place::End),
                        view.read_lines_in(
                            axis,
                            area.hv_get(axis.inverse())[0],
                            area.hv_get(axis.inverse())[1],
                            area.hv_get(axis)[1],
                            Place::Start,
                        ),
                    ])
                } else {
                    None
                };

                for place in Place::start_and_end() {
                    if *surround.hv_get(axis.inverse()) == place {
                        let edge_main = edge_main.hv_get(axis.inverse());
                        if !edge_main.get(&place.inverse()).unwrap()[0] {
                            if let Some([l, _]) = lines1.as_mut() {
                                l.add_gap(place.inverse(), 1);
                            }
                            if let Some([_, l]) = lines2.as_mut() {
                                l.add_gap(place.inverse(), 1);
                            }
                        }
                        if !edge_main.get(&place.inverse()).unwrap()[1] {
                            if let Some([_, l]) = lines1.as_mut() {
                                l.add_gap(place.inverse(), 1);
                            }
                            if let Some([l, _]) = lines2.as_mut() {
                                l.add_gap(place.inverse(), 1);
                            }
                        }
                    }
                }
                [
                    lines1.map(|l| l.map(|l| l.to_standard_edge())),
                    lines2.map(|l| l.map(|l| l.to_standard_edge())),
                ]
            }
            Self::Complex { .. } => unreachable!(),
        }
    }

    fn set_allocs_in_edge(
        &mut self,
        edge: &StandardEdge,
        axis: Axis,
        place: Place,
    ) -> Option<(bool, usize)> {
        let mut ok = None;
        match self {
            Self::Single { view, proto, .. } => {
                if let Some(data) = proto.attrs.get::<attrs::IntervalAlloc>() {
                    if let Some(data) = data.get(&axis) {
                        if let Some(interval_alloc) = data.get(&place) {
                            let rules = &interval_alloc.rules;
                            let allocs = &interval_alloc.allocs;
                            let mut fiexd =
                                proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();

                            if rules.iter().find(|rule| rule.match_edge(edge)).is_some() {
                                let mut modified = false;
                                let mut allocs_proto = proto.allocation_values();
                                Axis::list().into_iter().for_each(|axis| {
                                    allocs_proto
                                        .hv_get_mut(axis)
                                        .iter_mut()
                                        .zip(allocs.hv_get(axis))
                                        .enumerate()
                                        .for_each(|(i, (val, &exp))| {
                                            if exp >= 0 {
                                                let exp = exp as usize;
                                                if *val != exp {
                                                    *val = exp;
                                                    modified = true;
                                                } else {
                                                    fiexd.hv_get_mut(axis).insert(i);
                                                }
                                            }
                                        })
                                });
                                proto.attrs.set::<attrs::Allocs>(&allocs_proto);
                                proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
                                *view = StrucView::new(&proto);
                                ok = Some((modified, interval_alloc.interval));
                            }
                        }
                    }
                }
            }
            Self::Complex { tp, combs, .. } => match *tp {
                CstType::Scale(comb_axis) if comb_axis == axis => {
                    let i = match place {
                        Place::Start => 0,
                        Place::End => combs.len() - 1,
                        _ => panic!(),
                    };
                    ok = combs[i].set_allocs_in_edge(edge, axis, place);
                }
                CstType::Surround(surround) if *surround.hv_get(axis) != place.inverse() => {
                    ok = combs[0].set_allocs_in_edge(edge, axis, place)
                }
                _ => {}
            },
        }
        ok
    }

    pub fn init_edges(&mut self, cfg: &Config) -> Result<DataHV<usize>, CstError> {
        match self {
            Self::Single { proto, .. } => {
                proto.attrs.set::<attrs::FixedAlloc>(&Default::default());
                Ok(proto.allocation_values().map(|allocs| allocs.iter().sum()))
            }
            Self::Complex {
                tp,
                combs,
                intervals_alloc,
                edge_main,
                ..
            } => {
                match *tp {
                    CstType::Scale(comb_axis) => {
                        let mut len_list: Vec<DataHV<usize>> = Vec::with_capacity(combs.len());

                        'outer: loop {
                            len_list.clear();
                            for c in combs.iter_mut() {
                                len_list.push(c.init_edges(cfg)?);
                            }

                            {
                                // todo!() // Set edge alignment in Scale
                                // max_len - len_list[i] > 1
                                let axis = comb_axis.inverse();
                                let list: Vec<bool> =
                                    len_list.iter().map(|len| *len.hv_get(axis) != 0).collect();
                                *edge_main.hv_get_mut(axis) = HashMap::from([
                                    (Place::Start, list.clone()),
                                    (Place::End, list),
                                ]);
                            }

                            *intervals_alloc = vec![0; combs.len() - 1];
                            for i in 0..combs.len() - 1 {
                                let edges = Self::gen_edges_in_scale(
                                    &combs[i],
                                    &combs[i + 1],
                                    edge_main
                                        .hv_get(comb_axis.inverse())
                                        .iter()
                                        .map(|(k, v)| (*k, &v[i..=i + 1]))
                                        .collect(),
                                    comb_axis,
                                );

                                let r = combs[i]
                                    .set_allocs_in_edge(&edges[1], comb_axis, Place::End)
                                    .or_else(|| {
                                        combs[i + 1].set_allocs_in_edge(
                                            &edges[0],
                                            comb_axis,
                                            Place::Start,
                                        )
                                    });
                                if let Some((modified, i_val)) = r {
                                    if modified {
                                        continue 'outer;
                                    } else {
                                        intervals_alloc[i] = i_val;
                                    }
                                } else {
                                    if let Some(val) = cfg.interval.rules.iter().find_map(|rule| {
                                        rule.is_match(&edges[0], &edges[1], comb_axis)
                                    }) {
                                        intervals_alloc[i] = val;
                                    };
                                }
                            }
                            break;
                        }

                        Ok(Axis::hv().into_map(|axis| {
                            if axis == comb_axis {
                                len_list.iter().map(|cl| *cl.hv_get(axis)).sum::<usize>()
                                    + intervals_alloc.iter().sum::<usize>()
                            } else {
                                len_list.iter().map(|cl| *cl.hv_get(axis)).max().unwrap()
                            }
                        }))
                    }
                    CstType::Surround(surround) => {
                        let mut len_list: Vec<DataHV<usize>> = Vec::with_capacity(combs.len());

                        'outer: loop {
                            len_list.clear();
                            for c in combs.iter_mut() {
                                len_list.push(c.init_edges(cfg)?);
                            }

                            {
                                // todo!() // Set edge alignment in Surround
                                let area = combs[0].get_surround_area(surround)?;
                                for axis in Axis::list() {
                                    let place = *surround.hv_get(axis);
                                    let area = area.hv_get(axis);
                                    if place != Place::Middle {
                                        let mut list = vec![true; 2];
                                        if *len_list[1].hv_get(axis) == 0 && area[1] - area[0] > 2 {
                                            list[1] = false;
                                        }
                                        *edge_main.hv_get_mut(axis) =
                                            HashMap::from([(place.inverse(), list)]);
                                    }
                                }
                            }

                            *intervals_alloc = vec![0; 4];
                            match &combs[0] {
                                Self::Single { view, name, .. } => {
                                    let area =
                                        view.surround_area(surround).ok_or(CstError::Surround {
                                            tp: tp.symbol(),
                                            comp: name.to_string(),
                                        })?;

                                    for axis in Axis::list() {
                                        let edges = Self::gen_edges_in_surround(
                                            &combs[0], &combs[1], axis, area, edge_main, surround,
                                        );
                                        let r = [
                                            edges[0].as_ref().and_then(|edges| {
                                                combs[1].set_allocs_in_edge(
                                                    &edges[0],
                                                    axis,
                                                    Place::Start,
                                                )
                                            }),
                                            edges[1].as_ref().and_then(|edges| {
                                                combs[1].set_allocs_in_edge(
                                                    &edges[1],
                                                    axis,
                                                    Place::End,
                                                )
                                            }),
                                        ];

                                        let ofs = match axis {
                                            Axis::Horizontal => 0,
                                            Axis::Vertical => 2,
                                        };
                                        let s_b_len = combs[1].get_bases_length(axis);
                                        for i in 0..2 {
                                            if let Some((modified, i_val)) = r[i] {
                                                if modified {
                                                    continue 'outer;
                                                } else {
                                                    intervals_alloc[i + ofs] = i_val;
                                                }
                                            } else if let Some(edges) = &edges[i] {
                                                if let Some(val) =
                                                    cfg.interval.rules.iter().find_map(|rule| {
                                                        rule.is_match(&edges[0], &edges[1], axis)
                                                    })
                                                {
                                                    intervals_alloc[i + ofs] = val;
                                                };
                                                if s_b_len == 0 {
                                                    intervals_alloc[i + ofs] =
                                                        intervals_alloc[i + ofs].min(2);
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                                Self::Complex { .. } => unreachable!(),
                            }
                        }
                        Ok(Axis::hv().into_map(|axis| self.get_bases_length(axis)))
                    }
                    CstType::Single => unreachable!(),
                }
            }
        }
    }

    fn check_space(
        &mut self,
        fas: &FasFile,
        cst_table: &CstTable,
    ) -> Result<
        (
            DataHV<f32>,
            DataHV<[AssignVal; 2]>,
            DataHV<usize>,
            DataHV<f32>,
        ),
        CstError,
    > {
        let cfg = &fas.config;
        let char_box = self.get_char_box();
        let size = cfg
            .size
            .zip(char_box.size().to_hv_data())
            .into_map(|(a, b)| a * b);

        let mut assign = DataHV::default();
        let mut offsets: DataHV<[AssignVal; 2]> = DataHV::default();
        let mut levels: DataHV<usize> = DataHV::default();
        let mut scales: DataHV<f32> = DataHV::default();

        let mut base_len_list = self.init_edges(cfg)?;
        let mut check_state: Vec<Axis> = Axis::list().into();
        let mut first = true;

        while !check_state.is_empty() {
            loop {
                let axis = check_state[0];
                let white = cfg.white.hv_get(axis);

                let length = *size.hv_get(axis) - (white.fixed + white.value) * 2.0;
                let base_len = *base_len_list.hv_get(axis);

                let base_total = base_len as f32 + white.allocated * 2.;
                let ok =
                    cfg.min_val
                        .hv_get(axis)
                        .iter()
                        .enumerate()
                        .find_map(|(i, &min)| {
                            match min - al::NORMAL_OFFSET < length / base_total {
                                true => Some(i),
                                false => None,
                            }
                        });
                match ok {
                    Some(level) => {
                        if base_len != 0 {
                            let scale = length / base_total;
                            if scale < *cfg.reduce_trigger.hv_get(axis)
                                && self.reduce_space(axis, false)
                            // && (self.reduce_space(axis, false)
                            //     || self.reduce_replace(axis, fas)?)
                            {
                                if check_state.len() == 1 {
                                    check_state.push(axis.inverse());
                                }
                                base_len_list = self.init_edges(cfg)?;
                                continue;
                            }
                            (0..=1).into_iter().for_each(|i| {
                                offsets.hv_get_mut(axis)[i] = AssignVal::new(
                                    white.fixed,
                                    white.allocated * scale + white.value,
                                );
                            });
                            *assign.hv_get_mut(axis) = base_len as f32 * scale;
                            *scales.hv_get_mut(axis) = scale;
                        } else {
                            let half = *size.hv_get(axis) / 2.0 - white.fixed;
                            *offsets.hv_get_mut(axis) = [AssignVal::new(white.fixed, half); 2];
                            *assign.hv_get_mut(axis) = 0.0;
                        }
                        *levels.hv_get_mut(axis) = level;
                        break;
                    }
                    None => {
                        if self.reduce_space(axis, false)
                            || self.reduce_replace(axis, fas, cst_table)?
                        {
                            if check_state.len() == 1 {
                                check_state.push(axis.inverse());
                            }
                            base_len_list = self.init_edges(cfg)?;
                            continue;
                        } else if first {
                            check_state.swap(0, 1);
                            first = false;
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
            first = false;
            check_state.remove(0);
        }

        Ok((assign, offsets, levels, scales))
    }

    fn assign_space_in_surround(
        combs: &mut Vec<StrucComb>,
        intervals: &mut Vec<AssignVal>,
        ofs: &DataHV<[AssignVal; 2]>,
        intervals_alloc: &Vec<usize>,
        edge_main: &DataHV<HashMap<Place, Vec<bool>>>,
        surround: DataHV<Place>,
        assign: DataHV<Option<f32>>,
        min_val: DataHV<f32>,
        offsets: DataHV<[Option<AssignVal>; 2]>,
        levels: DataHV<usize>,
        cfg: &Config,
    ) {
        let s_b_len = Axis::hv().into_map(|axis| combs[1].get_bases_length(axis));
        if intervals.is_empty() {
            *intervals = vec![Default::default(); 4];
        }

        if let Self::Single {
            offsets: p_ofs,
            assign_vals,
            view,
            proto,
            ..
        } = &mut combs[0]
        {
            let area = view.surround_area(surround).unwrap();
            let p_allocs = proto.allocation_space();

            let mut s_assign = assign;
            let mut s_offsets: DataHV<[AssignVal; 2]> = Default::default();
            for axis in Axis::list() {
                if let Some(assign) = assign.hv_get(axis) {
                    let min_val = *min_val.hv_get(axis);
                    let s_b_len = *s_b_len.hv_get(axis);
                    let edge_main = edge_main.hv_get(axis);
                    let comp_white = cfg.comp_white.hv_get(axis);
                    let offsets = offsets.hv_get(axis);
                    let area_idx = proto.value_index_in_axis(area.hv_get(axis), axis);
                    let p_allocs = p_allocs.hv_get(axis);
                    let i_ofs = match axis {
                        Axis::Horizontal => 0,
                        Axis::Vertical => 2,
                    };
                    let s_surr_b_len =
                        s_b_len + intervals_alloc[0 + i_ofs..2 + i_ofs].iter().sum::<usize>();
                    let p_surr_b_len = p_allocs[area_idx[0]..area_idx[1]].iter().sum::<usize>();

                    let (base_total, surr_assign) = if s_surr_b_len > p_surr_b_len {
                        let base_total = (p_allocs[..area_idx[0]]
                            .iter()
                            .chain(p_allocs[area_idx[1]..].iter())
                            .sum::<usize>()
                            + s_surr_b_len) as f32;
                        let surr_assign = assign / base_total * (s_surr_b_len as f32);
                        (base_total, surr_assign)
                    } else {
                        let base_total = p_allocs.iter().sum::<usize>() as f32;
                        let surr_assign = assign / base_total * (p_surr_b_len as f32);
                        (base_total, surr_assign)
                    };

                    for i in 0..2 {
                        let scale = assign / base_total;
                        let alloc = intervals_alloc[i + i_ofs] as f32;
                        let base = alloc * min_val;
                        intervals[i + i_ofs] = AssignVal::new(
                            base,
                            ((alloc * scale).min(cfg.interval.limit) - base).max(0.0),
                        );
                    }

                    let mut white = [[WhiteArea::default(); 2]; 2];
                    for place in Place::start_and_end() {
                        let place_idx = match place {
                            Place::Start => 0,
                            Place::End => 1,
                            _ => unreachable!(),
                        };

                        if edge_main.get(&place).is_some() {
                            if edge_main[&place][0] {
                                if let Some(offset) = offsets[place_idx] {
                                    p_ofs.hv_get_mut(axis)[place_idx] = offset
                                }
                            } else {
                                p_ofs.hv_get_mut(axis)[place_idx] =
                                    AssignVal::new(ofs.hv_get(axis)[place_idx].total(), 0.0);
                                white[0][place_idx] = *comp_white;
                            }

                            if !edge_main[&place][1] {
                                white[1][place_idx] = *comp_white;
                            }
                        } else {
                            if let Some(offset) = offsets[place_idx] {
                                p_ofs.hv_get_mut(axis)[place_idx] = offset
                            }
                        }
                    }

                    {
                        // primary
                        let is_main = white[0][0].is_zero() && white[0][1].is_zero();
                        let total = match is_main {
                            true => base_total,
                            false => p_allocs.iter().sum::<usize>() as f32,
                        };
                        let scale = (assign
                            - white[0].iter().map(|a| a.fixed + a.value).sum::<f32>())
                            / (white[0][0].allocated + total + white[0][1].allocated);
                        let surr_scale = match is_main {
                            false => scale,
                            true => surr_assign / p_surr_b_len as f32,
                        };

                        let surr_range = area_idx[0]..area_idx[1];
                        let p_assign = assign_vals.hv_get_mut(axis);
                        p_assign.clear();
                        for i in 0..p_allocs.len() {
                            let scale = if surr_range.contains(&i) {
                                surr_scale
                            } else {
                                scale
                            };
                            let alloc = p_allocs[i] as f32;
                            p_assign
                                .push(AssignVal::new(alloc * min_val, alloc * (scale - min_val)));
                        }

                        for i in 0..2 {
                            let ofs_val = &mut p_ofs.hv_get_mut(axis)[i];
                            ofs_val.base += white[0][i].fixed;
                            ofs_val.excess +=
                                white[0][i].value + white[0][i].allocated * surr_scale;

                            let (place, space) = match i {
                                1 => (Place::Start, area_idx[1] != p_allocs.len()),
                                _ => (Place::End, area_idx[0] != 0),
                            };
                            if *surround.hv_get(axis) != place && !space {
                                ofs_val.base += ofs_val.excess;
                                ofs_val.excess = 0.0;
                            }
                        }
                    }

                    {
                        // sencondary
                        let scale = (surr_assign
                            - intervals[0 + i_ofs..2 + i_ofs]
                                .iter()
                                .sum::<AssignVal>()
                                .total()
                            - white[1].iter().map(|a| a.fixed + a.value).sum::<f32>())
                            / (white[1][0].allocated + s_b_len as f32 + white[1][1].allocated);

                        *s_assign.hv_get_mut(axis) = Some(s_b_len as f32 * scale);
                        for i in 0..2 {
                            let (place, range) = match i {
                                0 => (Place::End, 0..area_idx[0]),
                                _ => (Place::Start, area_idx[1]..p_allocs.len()),
                            };

                            let mut ofs_val = ofs.hv_get(axis)[i];
                            if s_b_len == 0 {
                                let half = surr_assign / 2.0;
                                if *surround.hv_get(axis) == place {
                                    if !white[1][i].is_zero() {
                                        ofs_val.excess += half;
                                    };
                                } else {
                                    ofs_val.base = ofs_val.total();
                                    ofs_val.excess = 0.0;

                                    if white[1][(i + 1) % 2].is_zero() {
                                        intervals[i + i_ofs].excess =
                                            surr_assign - intervals[i + i_ofs].base;
                                    } else {
                                        intervals[i + i_ofs].excess =
                                            half - intervals[i + i_ofs].base;
                                    }
                                }
                            } else {
                                if *surround.hv_get(axis) == place {
                                    if !white[1][i].is_zero() {
                                        ofs_val.base = ofs_val.total() + white[1][i].fixed;
                                        ofs_val.excess =
                                            white[1][i].value + white[1][i].allocated * scale;
                                    }
                                } else {
                                    ofs_val.base = ofs_val.total();
                                    ofs_val.excess = 0.0;
                                }
                            }
                            let p_assign = assign_vals.hv_get_mut(axis);
                            s_offsets.hv_get_mut(axis)[i] = AssignVal::new(
                                ofs_val.base
                                    + p_assign[range].iter().sum::<AssignVal>().total()
                                    + intervals[i + i_ofs].base
                                    + intervals[i + i_ofs].excess / 2.0,
                                ofs_val.excess + intervals[i + i_ofs].excess / 2.0,
                            );
                        }
                    }
                }
            }
            combs[1].assign_space(
                s_assign,
                s_offsets.map(|ofs| ofs.map(|v| Some(v))),
                levels,
                cfg,
            );
        } else {
            unreachable!()
        }
    }

    pub fn assign_space(
        &mut self,
        assign: DataHV<Option<f32>>,
        offsets: DataHV<[Option<AssignVal>; 2]>,
        levels: DataHV<usize>,
        cfg: &Config,
    ) {
        let min_val = Axis::hv().into_map(|axis| cfg.min_val.hv_get(axis)[*levels.hv_get(axis)]);

        match self {
            Self::Single {
                assign_vals: asg,
                proto,
                offsets: ofs,
                ..
            } => {
                ofs.as_mut()
                    .zip(offsets)
                    .into_iter()
                    .for_each(|(ofs, offsets)| {
                        for i in 0..2 {
                            if let Some(av) = offsets[i] {
                                ofs[i] = av;
                            }
                        }
                    });

                let allocs = proto.allocation_space();
                for axis in Axis::list() {
                    if let &Some(assign) = assign.hv_get(axis) {
                        let allocs = allocs.hv_get(axis);
                        let min_val = *min_val.hv_get(axis);

                        let alloc_total = allocs.iter().sum::<usize>() as f32;
                        *asg.hv_get_mut(axis) = if alloc_total == 0.0 {
                            vec![Default::default(); allocs.len()]
                        } else {
                            let scale = assign / alloc_total;
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
                    }
                }
            }
            Self::Complex {
                tp,
                intervals,
                intervals_alloc,
                edge_main,
                combs,
                offsets: ofs,
                ..
            } => {
                ofs.as_mut()
                    .zip(offsets)
                    .into_iter()
                    .for_each(|(ofs, offsets)| {
                        for i in 0..2 {
                            if let Some(av) = offsets[i] {
                                ofs[i] = av;
                            }
                        }
                    });

                match *tp {
                    CstType::Scale(comb_axis) => {
                        let c_assigns = if let &Some(assign) = assign.hv_get(comb_axis) {
                            let size_list: Vec<_> = combs
                                .iter()
                                .map(|c| c.get_bases_length(comb_axis))
                                .collect();
                            let axis_len = size_list
                                .iter()
                                .chain(intervals_alloc.iter())
                                .sum::<usize>();

                            let scale = assign / axis_len as f32;
                            let mut excess_totall = 0.0;

                            let target_count = size_list.iter().filter(|s| **s != 0).count();
                            *intervals = intervals_alloc
                                .iter()
                                .map(|&ia| {
                                    let alloc = ia as f32;
                                    let space = alloc * scale;
                                    let base = alloc * min_val.hv_get(comb_axis);
                                    let mut excess = space - base;
                                    let limit = cfg.interval.limit;

                                    if base + excess > limit && axis_len != ia {
                                        if base > limit && target_count != 0 {
                                            excess_totall += excess;
                                            excess = 0.;
                                        } else {
                                            excess_totall += excess + base - limit;
                                            excess = limit - base;
                                        }
                                    }

                                    AssignVal { base, excess }
                                })
                                .collect();

                            (0..combs.len())
                                .map(|i| {
                                    if size_list[i] == 0 {
                                        0.0
                                    } else {
                                        size_list[i] as f32 * scale
                                            + excess_totall / target_count as f32
                                    }
                                })
                                .collect()
                        } else {
                            vec![]
                        };

                        for i in 0..combs.len() {
                            let mut assign = assign;
                            let mut offsets = offsets;

                            if let Some(val) = assign.hv_get_mut(comb_axis) {
                                *val = c_assigns[i];
                                if i != 0 {
                                    offsets.hv_get_mut(comb_axis)[0] = Some(AssignVal {
                                        base: intervals[i - 1].base / 2.,
                                        excess: intervals[i - 1].excess / 2.,
                                    });
                                }
                                if i + 1 != combs.len() {
                                    offsets.hv_get_mut(comb_axis)[1] = Some(AssignVal {
                                        base: intervals[i].base / 2.,
                                        excess: intervals[i].excess / 2.,
                                    });
                                }
                            } else {
                                *offsets.hv_get_mut(comb_axis) = [None; 2];
                            }

                            if let Some(val) = assign.hv_get_mut(comb_axis.inverse()) {
                                let base_total =
                                    combs[i].get_bases_length(comb_axis.inverse()) as f32;
                                let c_min_len = base_total * min_val.hv_get(comb_axis.inverse());
                                let mut white_area = [WhiteArea::default(); 2];
                                let comp_white = cfg.comp_white.hv_get(comb_axis.inverse());
                                let sub_axis_ofs = offsets.hv_get_mut(comb_axis.inverse());

                                for (j, place) in Place::start_and_end().iter().enumerate() {
                                    if !edge_main.hv_get(comb_axis.inverse())[place][i]
                                        || base_total == 0.0
                                    {
                                        white_area[j] = *comp_white;
                                        let mut offset_val = sub_axis_ofs[j]
                                            .unwrap_or(ofs.hv_get(comb_axis.inverse())[j]);

                                        if base_total != 0.0 {
                                            offset_val.base += offset_val.excess;
                                            offset_val.excess = 0.0;
                                        } else {
                                            offset_val.base += offset_val.excess + *val / 2.;
                                            offset_val.excess = 0.0;
                                        }
                                        sub_axis_ofs[j] = Some(offset_val);
                                    }
                                }

                                if base_total == 0.0 {
                                    *val = 0.0;
                                } else {
                                    let scale = (*val
                                        - white_area
                                            .iter()
                                            .map(|a| a.fixed + a.value)
                                            .sum::<f32>())
                                        / (white_area[0].allocated
                                            + base_total
                                            + white_area[1].allocated);
                                    for j in 0..2 {
                                        if !white_area[j].is_zero() {
                                            let sub_ofs = sub_axis_ofs[j].as_mut().unwrap();
                                            sub_ofs.base += white_area[j].fixed;
                                            sub_ofs.excess = white_area[j].value
                                                + white_area[j].allocated * scale;
                                        }
                                    }
                                    *val = base_total * scale;
                                }

                                debug_assert!(*val > c_min_len - al::NORMAL_OFFSET);
                            } else {
                                *offsets.hv_get_mut(comb_axis.inverse()) = [None; 2];
                            }

                            combs[i].assign_space(assign, offsets, levels, cfg);
                        }
                    }
                    CstType::Surround(surround) => {
                        Self::assign_space_in_surround(
                            combs,
                            intervals,
                            ofs,
                            &intervals_alloc,
                            &edge_main,
                            surround,
                            assign,
                            min_val,
                            offsets,
                            levels,
                            cfg,
                        );
                    }
                    CstType::Single => unreachable!(),
                }
            }
        }
    }

    pub fn scale_space(
        &mut self,
        assign: DataHV<Option<f32>>,
        offsets: DataHV<[Option<AssignVal>; 2]>,
    ) {
        match self {
            Self::Single {
                assign_vals,
                offsets: ofs,
                ..
            } => {
                ofs.as_mut()
                    .zip(offsets)
                    .into_iter()
                    .for_each(|(ofs, offsets)| {
                        for i in 0..2 {
                            if let Some(av) = offsets[i] {
                                ofs[i] = av;
                            }
                        }
                    });
                for axis in Axis::list() {
                    if let Some(assign) = *assign.hv_get(axis) {
                        al::scale_correction(assign_vals.hv_get_mut(axis), assign);
                    }
                }
            }
            Self::Complex {
                tp,
                offsets: ofs,
                intervals_alloc,
                intervals,
                edge_main,
                combs,
                ..
            } => {
                let old_ofs = ofs.clone();
                ofs.as_mut()
                    .zip(offsets)
                    .into_iter()
                    .for_each(|(ofs, offsets)| {
                        for i in 0..2 {
                            if let Some(av) = offsets[i] {
                                ofs[i] = av;
                            }
                        }
                    });

                match *tp {
                    CstType::Scale(comb_axis) => {
                        let mut c_assigns: Vec<DataHV<Option<f32>>> =
                            vec![Default::default(); combs.len()];
                        let mut c_offsets: Vec<DataHV<[Option<AssignVal>; 2]>> =
                            vec![Default::default(); combs.len()];

                        for axis in Axis::list() {
                            if let &Some(assign) = assign.hv_get(axis) {
                                if axis == comb_axis {
                                    let mut vlist = Vec::with_capacity(combs.len() * 2 - 1);
                                    for i in 0..combs.len() {
                                        vlist.push(combs[i].get_assign_length(axis));
                                        if i < intervals.len() {
                                            vlist.push(intervals[i]);
                                        }
                                    }
                                    al::scale_correction(&mut vlist, assign);

                                    for i in 0..combs.len() {
                                        *c_assigns[i].hv_get_mut(axis) = Some(vlist[i * 2].total());

                                        if i < intervals.len() {
                                            intervals[i] = vlist[i * 2 + 1];
                                        }

                                        if i == 0 {
                                            c_offsets[i].hv_get_mut(axis)[0] =
                                                offsets.hv_get(axis)[0];
                                        } else {
                                            let val = intervals[i - 1];
                                            c_offsets[i].hv_get_mut(axis)[0] = Some(
                                                AssignVal::new(val.base / 2.0, val.excess / 2.0),
                                            );
                                        }

                                        if i == intervals.len() {
                                            c_offsets[i].hv_get_mut(axis)[1] =
                                                offsets.hv_get(axis)[1];
                                        } else {
                                            let val = intervals[i];
                                            c_offsets[i].hv_get_mut(axis)[1] = Some(
                                                AssignVal::new(val.base / 2.0, val.excess / 2.0),
                                            );
                                        }
                                    }
                                } else {
                                    let edge_main = edge_main.hv_get(axis);
                                    for i in 0..combs.len() {
                                        let mut edge_state = [false; 2];
                                        for (j, place) in [(0, Place::Start), (1, Place::End)] {
                                            edge_state[j] = edge_main[&place][i];
                                        }

                                        if edge_state[0] && edge_state[1] {
                                            *c_assigns[i].hv_get_mut(axis) = Some(assign);
                                            *c_offsets[i].hv_get_mut(axis) = *offsets.hv_get(axis);
                                        } else {
                                            let [ofs1, ofs2] = [0, 1].map(|j| {
                                                if edge_state[j] {
                                                    AssignVal::default()
                                                } else {
                                                    let mut c_ofs =
                                                        combs[i].get_offsets().hv_get(axis)[j];
                                                    c_ofs.base -= ofs.hv_get(axis)[j].total();
                                                    c_ofs
                                                }
                                            });
                                            let c_asg = combs[i].get_assign_length(axis);
                                            let mut vlist = vec![ofs1, c_asg, ofs2];
                                            al::scale_correction(&mut vlist, assign);
                                            *c_assigns[i].hv_get_mut(axis) = Some(vlist[1].total());
                                            for j in 0..2 {
                                                c_offsets[i].hv_get_mut(axis)[j] = if edge_state[j]
                                                {
                                                    None
                                                } else {
                                                    let mut val = vlist[j * 2];
                                                    val.base += ofs.hv_get(axis)[j].total();
                                                    Some(val)
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        for i in 0..combs.len() {
                            combs[i].scale_space(c_assigns[i], c_offsets[i]);
                        }
                    }
                    CstType::Surround(surround) => {
                        let s_b_len = Axis::hv().into_map(|axis| combs[1].get_bases_length(axis));
                        let s_old_assign =
                            Axis::hv().into_map(|axis| combs[1].get_assign_length(axis));
                        let s_old_ofs = combs[1].get_offsets();

                        if let Self::Single {
                            assign_vals,
                            view,
                            proto,
                            offsets: p_ofs,
                            ..
                        } = &mut combs[0]
                        {
                            let area = view.surround_area(surround).unwrap();
                            let p_allocs = proto.allocation_space();

                            let mut s_assign: DataHV<Option<f32>> = Default::default();
                            let mut s_offsets: DataHV<[Option<AssignVal>; 2]> = Default::default();
                            for axis in Axis::list() {
                                if let Some(assign) = *assign.hv_get(axis) {
                                    let area_idx =
                                        proto.value_index_in_axis(area.hv_get(axis), axis);
                                    let assign_vals = assign_vals.hv_get_mut(axis);
                                    let s_old_assign = *s_old_assign.hv_get(axis);
                                    let i_ofs = match axis {
                                        Axis::Horizontal => 0,
                                        Axis::Vertical => 2,
                                    };
                                    let edge_main = edge_main.hv_get(axis);

                                    let is_p = {
                                        let s_surr_b_len = s_b_len.hv_get(axis)
                                            + intervals_alloc[0 + i_ofs..2 + i_ofs]
                                                .iter()
                                                .sum::<usize>();
                                        let p_surr_b_len = p_allocs.hv_get(axis)
                                            [area_idx[0]..area_idx[1]]
                                            .iter()
                                            .sum::<usize>();
                                        p_surr_b_len > s_surr_b_len
                                    };

                                    if is_p {
                                        p_ofs.as_mut().zip(offsets).into_iter().for_each(
                                            |(ofs, offsets)| {
                                                for i in 0..2 {
                                                    if let Some(av) = offsets[i] {
                                                        ofs[i] = av;
                                                    }
                                                }
                                            },
                                        );

                                        al::scale_correction(assign_vals, assign);

                                        let edge_state = [(0, Place::Start), (1, Place::End)].map(
                                            |(i, place)| {
                                                if edge_main
                                                    .get(&place)
                                                    .map(|ep| ep[1])
                                                    .unwrap_or(true)
                                                {
                                                    AssignVal::default()
                                                } else {
                                                    let mut c_ofs = s_old_ofs.hv_get(axis)[i];
                                                    c_ofs.base -= old_ofs.hv_get(axis)[i].total();
                                                    c_ofs
                                                }
                                            },
                                        );

                                        let surr_assign = assign_vals[area_idx[0]..area_idx[1]]
                                            .iter()
                                            .sum::<AssignVal>()
                                            .total();
                                        let mut vlist = vec![
                                            intervals[0 + i_ofs],
                                            edge_state[0],
                                            s_old_assign,
                                            intervals[1 + i_ofs],
                                            edge_state[1],
                                        ];
                                        al::scale_correction(&mut vlist, surr_assign);

                                        for i in 0..2 {
                                            intervals[i + i_ofs] = vlist[i * 3];

                                            let (place, range) = match i {
                                                0 => (Place::End, 0..area_idx[0]),
                                                _ => (Place::Start, area_idx[1]..assign_vals.len()),
                                            };

                                            if *surround.hv_get(axis) == place {
                                                s_offsets.hv_get_mut(axis)[i] =
                                                    offsets.hv_get(axis)[i];
                                                if edge_state[i].base != 0.0 {
                                                    s_offsets.hv_get_mut(axis)[i] =
                                                        Some(AssignVal::new(
                                                            ofs.hv_get(axis)[i].total()
                                                                + vlist[i * 3 + 1].base,
                                                            vlist[i * 3 + 1].excess,
                                                        ));
                                                }
                                            } else {
                                                s_offsets.hv_get_mut(axis)[i] =
                                                    Some(AssignVal::new(
                                                        assign_vals[range]
                                                            .iter()
                                                            .sum::<AssignVal>()
                                                            .total()
                                                            + ofs.hv_get(axis)[i].total()
                                                            + intervals[i + i_ofs].base
                                                            + intervals[i + i_ofs].excess / 2.0,
                                                        intervals[i + i_ofs].excess / 2.0,
                                                    ));
                                            }
                                        }
                                        *s_assign.hv_get_mut(axis) = Some(vlist[2].total());
                                    } else {
                                        let mut vlist: Vec<_> = assign_vals[..area_idx[0]]
                                            .iter()
                                            .copied()
                                            .chain([
                                                intervals[0 + i_ofs],
                                                s_old_assign,
                                                intervals[1 + i_ofs],
                                            ])
                                            .chain(assign_vals[area_idx[1]..].iter().copied())
                                            .collect();
                                        al::scale_correction(&mut vlist, assign);

                                        {
                                            // p_surround
                                            let p_surr_assign = vlist[area_idx[0]..area_idx[0] + 3]
                                                .iter()
                                                .sum::<AssignVal>()
                                                .total();

                                            let edge_state = [(0, Place::Start), (1, Place::End)]
                                                .map(|(i, place)| {
                                                    if edge_main
                                                        .get(&place)
                                                        .map(|ep| ep[0])
                                                        .unwrap_or(true)
                                                    {
                                                        AssignVal::default()
                                                    } else {
                                                        let mut c_ofs = p_ofs.hv_get(axis)[i];
                                                        c_ofs.base -=
                                                            old_ofs.hv_get(axis)[i].total();
                                                        c_ofs
                                                    }
                                                });

                                            let mut vlist: Vec<AssignVal> = assign_vals
                                                [area_idx[0]..area_idx[1]]
                                                .iter()
                                                .chain(edge_state.iter())
                                                .copied()
                                                .collect();
                                            al::scale_correction(&mut vlist, p_surr_assign);
                                            assign_vals[area_idx[0]..area_idx[1]]
                                                .iter_mut()
                                                .zip(vlist.iter())
                                                .for_each(|(av, v)| *av = *v);

                                            for i in 0..2 {
                                                if edge_state[i].base != 0.0 {
                                                    p_ofs.hv_get_mut(axis)[i] = AssignVal::new(
                                                        ofs.hv_get(axis)[i].total()
                                                            + vlist[area_idx[1] + i].base,
                                                        vlist[area_idx[1] + i].excess,
                                                    )
                                                } else {
                                                    p_ofs.hv_get_mut(axis)[i] = ofs.hv_get(axis)[i];
                                                }
                                            }
                                        }

                                        for i in 0..2 {
                                            intervals[i + i_ofs] = vlist[area_idx[0] + i * 2];

                                            let (place, range, vofs) = match i {
                                                0 => (Place::End, 0..area_idx[0], 0),
                                                _ => (
                                                    Place::Start,
                                                    area_idx[1]..assign_vals.len(),
                                                    area_idx[0] + 3, // 崎
                                                ),
                                            };

                                            assign_vals[range.clone()]
                                                .iter_mut()
                                                .zip(vlist.iter().skip(vofs))
                                                .for_each(|(av, v)| {
                                                    *av = *v;
                                                });

                                            if *surround.hv_get(axis) == place {
                                                s_offsets.hv_get_mut(axis)[i] =
                                                    offsets.hv_get(axis)[i];
                                            } else {
                                                s_offsets.hv_get_mut(axis)[i] =
                                                    Some(AssignVal::new(
                                                        assign_vals[range]
                                                            .iter()
                                                            .sum::<AssignVal>()
                                                            .total()
                                                            + ofs.hv_get(axis)[i].total()
                                                            + intervals[i + i_ofs].base
                                                            + intervals[i + i_ofs].excess / 2.0,
                                                        intervals[i + i_ofs].excess / 2.0,
                                                    ));
                                            }
                                        }
                                        *s_assign.hv_get_mut(axis) =
                                            Some(vlist[area_idx[0] + 1].total());
                                    }
                                }
                            }

                            combs[1].scale_space(s_assign, s_offsets);
                        } else {
                            unreachable!()
                        }
                    }
                    CstType::Single => unreachable!(),
                }
            }
        }
    }

    pub fn process_space(&mut self, cfg: &Config) {
        cfg.process_control
            .iter()
            .for_each(|ctrl| ctrl.process_space(self, cfg));
    }

    pub fn edge_aligment(&mut self, cfg: &Config) {
        match self.get_type() {
            CstType::Single => {
                for axis in Axis::list() {
                    let edges = [Place::Start, Place::End].map(|place| {
                        let mut edge = self.get_comb_lines(axis, place).to_edge();
                        if cfg.setting.contains(setting::DOT_FACE) {
                            if !edge.faces.contains(&1.0)
                                && edge.dots.iter().filter(|b| **b).count() < 3
                            {
                                edge.faces.fill(0.0);
                            } else {
                                edge.faces.fill(1.0);
                                edge.dots.fill(false);
                            }
                        }

                        edge
                    });
                    let edge_corr = edges.clone().map(|edge| edge.gray_scale(cfg.strok_width));

                    if let Self::Single {
                        assign_vals,
                        offsets,
                        ..
                    } = self
                    {
                        if assign_vals.hv_get(axis).is_empty() {
                            continue;
                        }

                        let mut corrected =
                            [0, 1].map(|i| offsets.hv_get(axis)[i].excess * (1.0 - edge_corr[i]));
                        if axis == Axis::Horizontal && edges.iter().all(|e| e.faces.contains(&1.0))
                        {
                            let val = corrected[0].min(corrected[1]);
                            corrected.fill(val);
                        }

                        (0..2).for_each(|i| {
                            offsets.hv_get_mut(axis)[i].excess -= corrected[i];
                        });
                        let assign_val = assign_vals.hv_get_mut(axis);

                        // 1
                        // let add_val = (corrected[0] + corrected[1]) / assign_val.len() as f32;
                        // assign_val.iter_mut().for_each(|val| val.excess += add_val);

                        // 2
                        if let Some(val) = assign_val.first_mut() {
                            val.excess += corrected[0]
                        }
                        if let Some(val) = assign_val.last_mut() {
                            val.excess += corrected[1]
                        }
                    }
                }
            }
            CstType::Scale(_) => {
                if let Self::Complex { combs, .. } = self {
                    // todo!(); // 1
                    combs.iter_mut().for_each(|c| c.edge_aligment(cfg));
                }
            }
            CstType::Surround(surround) => {
                if let Self::Complex { combs, .. } = self {
                    let gray_vals = Axis::hv().into_map(|axis| {
                        Place::start_and_end().map(|place| {
                            combs[0].get_surround_comp_gray(surround, axis, place, cfg.strok_width)
                        })
                    });

                    if let Self::Single {
                        offsets,
                        assign_vals,
                        ..
                    } = &mut combs[0]
                    {
                        for axis in Axis::list() {
                            let corrected: Vec<_> = (0..2)
                                .into_iter()
                                .map(|i| {
                                    let ofs = &mut offsets.hv_get_mut(axis)[i];
                                    let gray = gray_vals.hv_get(axis)[i];

                                    let old_val = ofs.excess;
                                    ofs.excess *= gray;
                                    old_val * (1. - gray)
                                })
                                .collect();

                            let assign_val = assign_vals.hv_get_mut(axis);
                            if let Some(val) = assign_val.first_mut() {
                                val.excess += corrected[0]
                            }
                            if let Some(val) = assign_val.last_mut() {
                                val.excess += corrected[1]
                            }
                        }
                    } else {
                        unreachable!()
                    }

                    combs[1].edge_aligment(cfg);
                }
            }
        }
    }

    pub fn expand_comb_proto(
        &mut self,
        fas: &FasFile,
        cst_table: &CstTable,
        gen_info: bool,
    ) -> Result<Option<CharInfo>, CstError> {
        let (assign, offsets, levels, scales) = self.check_space(fas, cst_table)?;
        self.assign_space(
            assign.into_map(|val| Some(val)),
            offsets.into_map(|ofs| ofs.map(|val| Some(val))),
            levels,
            &fas.config,
        );

        self.process_space(&fas.config);
        debug_assert!(
            (self.get_assign_length(Axis::Horizontal).total() - assign.h).abs() < al::NORMAL_OFFSET,
            "`{}` size changed in process_space! {} -> {}",
            self.get_name(),
            assign.h,
            self.get_assign_length(Axis::Horizontal).total()
        );
        debug_assert!(
            (self.get_assign_length(Axis::Vertical).total() - assign.v).abs() < al::NORMAL_OFFSET,
            "`{}` size changed in process_space! {} -> {}",
            self.get_name(),
            assign.v,
            self.get_assign_length(Axis::Vertical).total()
        );

        self.edge_aligment(&fas.config);

        if gen_info {
            let min_len = fas.config.min_val.h[levels.h].min(fas.config.min_val.v[levels.v]);
            let mut comp_infos = vec![];
            self.get_comp_info(&mut comp_infos, fas.config.strok_width);

            Ok(Some(CharInfo {
                comb_name: self.get_name().to_string(),
                base_size: Axis::hv().into_map(|axis| self.get_bases_length(axis)),
                levels,
                scales,
                center: self.get_visual_center(min_len, fas.config.strok_width),
                comp_infos,
            }))
        } else {
            Ok(None)
        }
    }

    fn line_weight_list_in_axis(
        tp: CstType,
        combs: &Vec<StrucComb>,
        sort: bool,
    ) -> Vec<(usize, f32)> {
        match tp {
            CstType::Scale(comb_axis) => {
                let size_list: Vec<_> = combs
                    .iter()
                    .map(|c| Axis::hv().into_map(|axis| c.get_bases_length(axis)))
                    .collect();
                let max_len = size_list
                    .iter()
                    .map(|size| *size.hv_get(comb_axis.inverse()))
                    .max()
                    .unwrap();

                let mut weight_list: Vec<(usize, f32)> = combs
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mut size = size_list[i];
                        let mut scale = DataHV::splat(1.0);
                        *scale.hv_get_mut(comb_axis.inverse()) =
                            max_len as f32 / *size.hv_get(comb_axis.inverse()) as f32;

                        let line_length = c.get_line_length(scale);
                        *size.hv_get_mut(comb_axis.inverse()) = max_len;
                        (i, line_length / ((size.h + 1) * (size.v + 1)) as f32)
                    })
                    .collect();
                if sort {
                    weight_list.sort_by(|(_, s1), (_, s2)| s1.partial_cmp(s2).unwrap());
                }

                weight_list
            }
            _ => panic!(),
        }
    }

    pub fn reduce_space(&mut self, axis: Axis, is_check: bool) -> bool {
        match self {
            Self::Single { view, proto, .. } => {
                if proto.reduce(axis, is_check) {
                    if !is_check {
                        *view = StrucView::new(proto);
                    }
                    true
                } else {
                    false
                }
            }
            Self::Complex {
                tp,
                combs,
                intervals_alloc,
                ..
            } => {
                let mut ok = false;
                match *tp {
                    CstType::Scale(comb_axis) => {
                        if comb_axis == axis {
                            let weight_list = Self::line_weight_list_in_axis(*tp, combs, true);
                            for (i, _) in weight_list.iter().copied() {
                                ok |= combs[i].reduce_space(axis, is_check);
                                if ok {
                                    if !is_check {
                                        for j in i + 1..combs.len() {
                                            if combs[i].get_name() == combs[j].get_name() {
                                                combs[j].reduce_space(axis, false);
                                            }
                                        }
                                    }

                                    break;
                                }
                            }
                        } else {
                            let size_list: Vec<(usize, usize)> = combs
                                .iter()
                                .map(|c| c.get_bases_length(axis))
                                .enumerate()
                                .collect();
                            let max_size = size_list.iter().max_by_key(|(_, s)| *s).unwrap().1;

                            let mut sub_is_check = true;
                            ok = true;

                            for _ in 0..2 {
                                size_list.iter().for_each(|&(i, s)| {
                                    if max_size == s {
                                        ok &= combs[i].reduce_space(axis, sub_is_check);
                                    }
                                });

                                if ok && !is_check {
                                    sub_is_check = false;
                                } else {
                                    break;
                                }
                            }
                        }
                        ok
                    }
                    CstType::Surround(surround) => {
                        let area = if let Self::Single { view, .. } = &combs[0] {
                            *view.surround_area(surround).unwrap().hv_get(axis)
                        } else {
                            unreachable!()
                        };
                        let p_surr_len = area[1] - area[0];
                        let i_ofs = match axis {
                            Axis::Horizontal => 0,
                            Axis::Vertical => 2,
                        };
                        let s_surr_len = combs[1].get_bases_length(axis)
                            + intervals_alloc[0 + i_ofs..2 + i_ofs].iter().sum::<usize>();

                        match p_surr_len.cmp(&s_surr_len) {
                            std::cmp::Ordering::Equal => {
                                let mut sub_is_check = true;
                                ok = true;

                                for _ in 0..2 {
                                    combs.iter_mut().for_each(|c| {
                                        ok &= c.reduce_space(axis, sub_is_check);
                                    });

                                    if ok && !is_check {
                                        sub_is_check = false;
                                    } else {
                                        break;
                                    }
                                }
                                ok
                            }
                            std::cmp::Ordering::Greater => combs[0].reduce_space(axis, is_check),
                            std::cmp::Ordering::Less => {
                                let p_len = combs[0].get_bases_length(axis);
                                let list = if area[0] > s_surr_len || p_len - area[1] > s_surr_len {
                                    [0, 1]
                                } else {
                                    [1, 0]
                                };
                                list.into_iter()
                                    .find(|i| combs[*i].reduce_space(axis, is_check))
                                    .is_some()
                            }
                        }
                    }
                    CstType::Single => panic!(),
                }
            }
        }
    }

    pub fn reduce_replace(
        &mut self,
        axis: Axis,
        fas: &FasFile,
        cst_table: &CstTable,
    ) -> Result<bool, CstError> {
        fn replace(
            comb: &mut StrucComb,
            axis: Axis,
            fas: &FasFile,
            cst_table: &CstTable,
        ) -> Result<bool, CstError> {
            let cfg = &fas.config;
            match cfg.reduce_replace_name(axis, comb.get_name()) {
                Some(new_name) => {
                    use crate::service::combination::{gen_comb_proto_in, get_char_tree};

                    let new_tree = get_char_tree(new_name.to_string(), cst_table, &fas.config);
                    let mut new_comb =
                        gen_comb_proto_in(new_tree, comb.get_adjacency(), cst_table, fas)?;

                    let old_len = comb.get_bases_length(axis);
                    loop {
                        let new_len = *new_comb.init_edges(cfg)?.hv_get(axis);
                        if new_len <= old_len || !new_comb.reduce_space(axis, false) {
                            break;
                        }
                    }

                    if let Some(target) = comb
                        .get_proto_attr::<attrs::ReduceTarget>()
                        .and_then(|data| *data.hv_get(axis.inverse()))
                    {
                        loop {
                            let new_len = new_comb.get_bases_length(axis.inverse());
                            if new_len <= target || !new_comb.reduce_space(axis.inverse(), false) {
                                break;
                            }
                        }
                    }

                    *comb = new_comb;
                    Ok(true)
                }
                None => Ok(false),
            }
        }

        match self {
            Self::Single { .. } => replace(self, axis, fas, cst_table),
            Self::Complex {
                tp,
                intervals_alloc,
                combs,
                ..
            } => {
                const STANDARD: f32 = 0.64;
                let mut ok = false;

                match *tp {
                    CstType::Scale(comb_axis) => {
                        if comb_axis == axis {
                            let mut weight_list = Self::line_weight_list_in_axis(*tp, combs, false);
                            weight_list.reverse();
                            weight_list.sort_by(|a, b| {
                                (a.1 - STANDARD)
                                    .abs()
                                    .partial_cmp(&(b.1 - STANDARD).abs())
                                    .unwrap()
                            });
                            weight_list.reverse();

                            for (i, _) in weight_list.iter().copied() {
                                ok |= combs[i].reduce_replace(axis, fas, cst_table)?;
                                if ok {
                                    break;
                                }
                            }
                        } else {
                            let size_list: Vec<(usize, usize)> = combs
                                .iter()
                                .map(|c| c.get_bases_length(axis))
                                .enumerate()
                                .collect();
                            let max_size = size_list.iter().max_by_key(|(_, s)| *s).unwrap().1;

                            let targets: Vec<_> = size_list
                                .iter()
                                .filter_map(|&(i, s)| {
                                    if max_size == s && !combs[i].reduce_space(axis, true) {
                                        return Some(i);
                                    }
                                    None
                                })
                                .collect();

                            ok = true;
                            let temp = combs.clone();
                            for i in targets {
                                ok &= combs[i].reduce_replace(axis, fas, cst_table)?;

                                if fas.config.setting.contains(setting::SAME_HORIZONTAL)
                                    && axis == Axis::Vertical
                                    && combs[i].get_type() == CstType::Single
                                {
                                    for j in 0..combs.len() {
                                        if j != i && combs[j].get_name() == combs[i].get_name() {
                                            let len = combs[j].get_bases_length(axis.inverse());
                                            while combs[i].get_bases_length(axis.inverse()) > len {
                                                if !combs[i].reduce_space(axis.inverse(), false) {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if !ok {
                                *combs = temp;
                            }
                        }
                    }
                    CstType::Surround(surround) => {
                        let area = if let Self::Single { view, .. } = &combs[0] {
                            *view.surround_area(surround).unwrap().hv_get(axis)
                        } else {
                            unreachable!()
                        };
                        let p_surr_len = area[1] - area[0];
                        let i_ofs = match axis {
                            Axis::Horizontal => 0,
                            Axis::Vertical => 2,
                        };
                        let s_surr_len = combs[1].get_bases_length(axis)
                            + intervals_alloc[0 + i_ofs..2 + i_ofs].iter().sum::<usize>();

                        match p_surr_len.cmp(&s_surr_len) {
                            std::cmp::Ordering::Equal => {
                                ok = true;
                                for i in 0..2 {
                                    ok &= combs[i].reduce_replace(axis, fas, cst_table)?;
                                }
                            }
                            std::cmp::Ordering::Greater => {
                                ok |= combs[0].reduce_replace(axis, fas, cst_table)?
                            }
                            std::cmp::Ordering::Less => {
                                let p_len = combs[0].get_bases_length(axis);
                                let p_sub_len = area[0].max(p_len - area[1]);
                                let s_main_len = combs[1].get_main_base_length(axis);

                                let list = if p_sub_len > s_main_len {
                                    [0, 1]
                                } else {
                                    [0, 1] // [1, 0]
                                };
                                for i in list {
                                    if combs[i].reduce_replace(axis, fas, cst_table)? {
                                        ok = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    CstType::Single => panic!(),
                }

                if !ok {
                    ok = replace(self, axis, fas, cst_table)?
                }

                Ok(ok)
            }
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
                proto,
                offsets,
                ..
            } => {
                let new_start = WorkPoint::new(
                    start.x + offsets.h[0].total(),
                    start.y + offsets.v[0].total(),
                );
                let assigns =
                    assign_vals.map(|assign| assign.iter().map(|av| av.base + av.excess).collect());
                let size = assigns.map(|assigns: &Vec<f32>| assigns.iter().sum::<f32>());
                paths.extend(proto.to_paths(new_start, assigns));
                WorkSize::new(
                    offsets.h[0].total() + size.h + offsets.h[1].total(),
                    offsets.v[0].total() + size.v + offsets.v[1].total(),
                )
            }
            Self::Complex { tp, combs, .. } => match *tp {
                CstType::Scale(comb_axis) => {
                    let mut new_start = start;
                    let mut size = WorkSize::new(0., 0.);

                    for i in 0..combs.len() {
                        let c_size = combs[i].merge_to(new_start, paths);
                        *size.hv_get_mut(comb_axis.inverse()) = size
                            .hv_get_mut(comb_axis.inverse())
                            .max(*c_size.hv_get(comb_axis.inverse()));
                        *size.hv_get_mut(comb_axis) += *c_size.hv_get(comb_axis);
                        *new_start.hv_get_mut(comb_axis) += *c_size.hv_get(comb_axis);
                    }
                    size
                }
                CstType::Surround(_) => {
                    combs[1].merge_to(start, paths);
                    combs[0].merge_to(start, paths)
                }
                CstType::Single => unreachable!(),
            },
        }
    }
}
