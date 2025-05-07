use crate::{
    algorithm as al,
    axis::*,
    component::{
        attrs,
        struc::StrucProto,
        view::{StandardEdge, StrucView, ViewLines},
    },
    config::{Config, WhiteArea},
    construct::{space::*, CharTree, CstError, CstType},
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
                                dot_val,
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

                    if primary_len[1] > interval_len + secondary_len {
                        primary_len.iter().sum()
                    } else {
                        primary_len[0] + interval_len + secondary_len + primary_len[2]
                    }
                }
                CstType::Single => unreachable!(),
            },
        }
    }

    fn get_offsets(&self) -> DataHV<[AssignVal; 2]> {
        match self {
            Self::Single { offsets, .. } => *offsets,
            Self::Complex { offsets, .. } => *offsets,
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

                                lines1.add_gap(Place::End, intervals_alloc[i2 - 1]);
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

    fn gen_edges_in_scale(
        c1: &StrucComb,
        c2: &StrucComb,
        edge_main: HashMap<Place, &[bool]>,
        axis: Axis,
        dot_val: f32,
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
        lines.map(|l| l.to_standard_edge(dot_val))
    }

    fn gen_edges_in_surround(
        c1: &StrucComb,
        c2: &StrucComb,
        axis: Axis,
        area: DataHV<[usize; 2]>,
        edge_main: &DataHV<HashMap<Place, Vec<bool>>>,
        surround: DataHV<Place>,
        dot_val: f32,
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
                    lines1.map(|l| l.map(|l| l.to_standard_edge(dot_val))),
                    lines2.map(|l| l.map(|l| l.to_standard_edge(dot_val))),
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
                                            if exp > 0 {
                                                let exp = exp as usize;
                                                if *val != exp {
                                                    *val = exp;
                                                    fiexd.hv_get_mut(axis).insert(i);
                                                    modified = true;
                                                }
                                            }
                                        })
                                });
                                proto.attrs.set::<attrs::Allocs>(&allocs_proto);
                                proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
                                *view = StrucView::new(&proto);
                                ok = Some((modified, interval_alloc.interval));
                            } else {
                                proto.attrs.set::<attrs::FixedAlloc>(&Default::default());
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
                _ => {}
            },
        }
        ok
    }

    pub fn init_edges(&mut self, cfg: &Config) -> Result<DataHV<usize>, CstError> {
        match self {
            Self::Single { proto, .. } => {
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
                                    cfg.strok_width,
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
                                    for rule in &cfg.interval.rules {
                                        if (rule.axis.is_some() && rule.axis.unwrap() == comb_axis)
                                            || rule.axis.is_none()
                                        {
                                            if rule.rule1.match_edge(&edges[0])
                                                && rule.rule2.match_edge(&edges[1])
                                            {
                                                intervals_alloc[i] = rule.val;
                                                break;
                                            }
                                        }
                                    }
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
                                for axis in Axis::list() {
                                    let place = *surround.hv_get(axis);
                                    if place != Place::Middle {
                                        let list: Vec<bool> = len_list
                                            .iter()
                                            .map(|len| *len.hv_get(axis) != 0)
                                            .collect();
                                        *edge_main.hv_get_mut(axis) =
                                            HashMap::from([(place.inverse(), list.clone())]);
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
                                            &combs[0],
                                            &combs[1],
                                            axis,
                                            area,
                                            edge_main,
                                            surround,
                                            cfg.strok_width,
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
                                        for i in 0..2 {
                                            if let Some((modified, i_val)) = r[i] {
                                                if modified {
                                                    continue 'outer;
                                                } else {
                                                    intervals_alloc[i + ofs] = i_val;
                                                }
                                            } else if let Some(edges) = &edges[i] {
                                                for rule in &cfg.interval.rules {
                                                    if (rule.axis.is_some()
                                                        && rule.axis.unwrap() == axis)
                                                        || rule.axis.is_none()
                                                    {
                                                        if rule.rule1.match_edge(&edges[0])
                                                            && rule.rule2.match_edge(&edges[1])
                                                        {
                                                            intervals_alloc[i + ofs] = rule.val;
                                                            break;
                                                        }
                                                    }
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
        let mut check_state = DataHV::splat(false);

        while !(check_state.h & check_state.v) {
            let axis = check_state.in_axis(|state| !state).unwrap();
            let white = cfg.white.hv_get(axis);

            loop {
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
                                *check_state.hv_get_mut(axis.inverse()) = false;
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
                        if self.reduce_space(axis, false) || self.reduce_replace(axis, fas)? {
                            *check_state.hv_get_mut(axis.inverse()) = false;
                            base_len_list = self.init_edges(cfg)?;
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
            let mut s_offsets = offsets;
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
                                s_offsets.hv_get_mut(axis)[place_idx] =
                                    Some(AssignVal::new(ofs.hv_get(axis)[place_idx].total(), 0.0));
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

                            let place = match i {
                                1 => Place::Start,
                                _ => Place::End,
                            };
                            if *surround.hv_get(axis) != place {
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

                            if *surround.hv_get(axis) == place {
                                if let Some(ofs_val) = s_offsets.hv_get_mut(axis)[i].as_mut() {
                                    ofs_val.base += white[1][i].fixed;
                                    ofs_val.excess +=
                                        white[1][i].value + white[1][i].allocated * scale;
                                }
                            } else {
                                let p_assign = assign_vals.hv_get_mut(axis);
                                s_offsets.hv_get_mut(axis)[i] = Some(AssignVal::new(
                                    p_assign[range].iter().sum::<AssignVal>().total()
                                        + p_ofs.hv_get(axis)[i].total()
                                        + intervals[i + i_ofs].base
                                        + intervals[i + i_ofs].excess / 2.0,
                                    intervals[i + i_ofs].excess / 2.0,
                                ));
                            }
                        }
                    }
                }
            }
            combs[1].assign_space(s_assign, s_offsets, levels, cfg);
        } else {
            unreachable!()
        }
    }

    fn assign_space(
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

                            *intervals = intervals_alloc
                                .iter()
                                .map(|&ia| {
                                    let alloc = ia as f32;
                                    let space = alloc * scale;
                                    let base = alloc * min_val.hv_get(comb_axis);
                                    let mut excess = space - base;
                                    let limit = cfg.interval.limit;

                                    if base + excess > limit {
                                        if base > limit {
                                            excess_totall += excess;
                                            excess = 0.;
                                        } else {
                                            excess_totall = excess + base - limit;
                                            excess = limit - base;
                                        }
                                    }

                                    AssignVal { base, excess }
                                })
                                .collect();

                            (0..combs.len())
                                .map(|i| {
                                    size_list[i] as f32 * scale + excess_totall / combs.len() as f32
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

                                debug_assert!(*val >= c_min_len);
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

    pub fn process_space(&mut self, levels: DataHV<usize>, cfg: &Config) {
        let min_len = cfg.min_val.h[levels.h].min(cfg.min_val.v[levels.v]);

        if matches!(self, Self::Single { .. }) {
            let center = self.get_visual_center(min_len, cfg.strok_width);

            if let Self::Single { assign_vals, .. } = self {
                for axis in Axis::list() {
                    let assign_vals = assign_vals.hv_get_mut(axis);
                    let center_opt = &cfg.center.hv_get(axis);
                    let new_vals = al::center_correction(
                        &assign_vals.iter().map(|av| av.total()).collect(),
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
            match self {
                Self::Complex {
                    tp,
                    intervals,
                    combs,
                    ..
                } => match *tp {
                    CstType::Scale(comb_axis) => {
                        for c in combs.iter_mut() {
                            c.process_space(levels, cfg);
                        }

                        for i in 1..combs.len() {
                            let mut paths = vec![];
                            let mut start = Default::default();
                            let mut vlist = Vec::with_capacity(i * 2 + 1);
                            let mut bases = Vec::with_capacity(i * 2 + 1);

                            for j in 0..=i {
                                let size = combs[j].merge_to(start, &mut paths);
                                *start.hv_get_mut(comb_axis) += *size.hv_get(comb_axis);

                                let c_assgin = combs[j].get_assign_length(comb_axis);
                                vlist.push(c_assgin.total());
                                bases.push(c_assgin.base);

                                if j < i {
                                    *start.hv_get_mut(comb_axis) += intervals[j].total();
                                    vlist.push(intervals[j].total());
                                    bases.push(intervals[j].base);
                                }
                            }

                            let center = al::visual_center_length(paths, min_len, cfg.strok_width);
                            let center_opt = cfg.comp_center.hv_get(comb_axis);
                            let new_vals = al::center_correction(
                                &vlist,
                                &bases,
                                *center.hv_get(comb_axis),
                                center_opt.operation,
                                center_opt.execution,
                            );

                            for j in 0..=i {
                                if j != i {
                                    intervals[j].excess = new_vals[j * 2 + 1];
                                }

                                let mut assign: DataHV<Option<f32>> = Default::default();
                                let mut offsets: DataHV<[Option<AssignVal>; 2]> =
                                    Default::default();

                                *assign.hv_get_mut(comb_axis) =
                                    Some(new_vals[j * 2] + bases[j * 2]);
                                if j != 0 {
                                    offsets.hv_get_mut(comb_axis)[0] = Some(AssignVal {
                                        base: bases[j * 2 - 1] * 0.5,
                                        excess: new_vals[j * 2 - 1] * 0.5,
                                    });
                                }
                                if j != i {
                                    offsets.hv_get_mut(comb_axis)[1] = Some(AssignVal {
                                        base: bases[j * 2 + 1] * 0.5,
                                        excess: new_vals[j * 2 + 1] * 0.5,
                                    });
                                }

                                combs[j].assign_space(assign, offsets, levels, cfg);
                            }
                        }
                    }
                    CstType::Surround(surround) => {
                        combs[1].process_space(levels, cfg);

                        let mut paths = vec![];
                        combs.iter().for_each(|c| {
                            c.merge_to(Default::default(), &mut paths);
                        });
                        let center = al::visual_center_length(paths, min_len, cfg.strok_width);
                        let secondary_len =
                            Axis::hv().into_map(|axis| combs[1].get_assign_length(axis));

                        let old_intervals = intervals.clone();
                        let mut s_offsets = combs[1].get_offsets();
                        let s_assign = Axis::hv().into_map(|axis| {
                            if let Self::Single {
                                view,
                                proto,
                                assign_vals,
                                ..
                            } = &mut combs[0]
                            {
                                let surr_area = view.surround_area(surround).unwrap();
                                let area = *surr_area.hv_get(axis);
                                let area_idx = proto.value_index_in_axis(&area, axis);
                                let mut surr_idx = area_idx.clone();

                                let primary_len = [
                                    &assign_vals.hv_get(axis)[..area_idx[0]],
                                    &assign_vals.hv_get(axis)[area_idx[0]..area_idx[1]],
                                    &assign_vals.hv_get(axis)[area_idx[1]..],
                                ];

                                let secondary_len = *secondary_len.hv_get(axis);
                                let i_ofs = match axis {
                                    Axis::Horizontal => 0,
                                    Axis::Vertical => 2,
                                };
                                let interval_len = &intervals[0 + i_ofs..2 + i_ofs];

                                let (aval_list, is_p): (Vec<AssignVal>, bool) =
                                    if primary_len[1].iter().map(|val| val.base).sum::<f32>()
                                        > interval_len[0].base
                                            + secondary_len.base
                                            + interval_len[1].base
                                    {
                                        (
                                            primary_len
                                                .iter()
                                                .map(|slice| slice.iter())
                                                .flatten()
                                                .copied()
                                                .collect(),
                                            true,
                                        )
                                    } else {
                                        surr_idx[1] = surr_idx[0] + 3;
                                        (
                                            primary_len[0]
                                                .iter()
                                                .copied()
                                                .chain([
                                                    interval_len[0],
                                                    secondary_len,
                                                    interval_len[1],
                                                ])
                                                .chain(primary_len[2].iter().copied())
                                                .collect(),
                                            false,
                                        )
                                    };

                                let center_opt = cfg.comp_center.hv_get(axis);
                                let vlist = aval_list.iter().map(|av| av.total()).collect();
                                let bases = aval_list.iter().map(|av| av.base).collect();
                                let new_vals = al::center_correction(
                                    &vlist,
                                    &bases,
                                    *center.hv_get(axis),
                                    center_opt.operation,
                                    center_opt.execution,
                                );

                                let (mut new_surr, old_surr) = (surr_idx[0]..surr_idx[1])
                                    .map(|i| (new_vals[i] + bases[i], vlist[i]))
                                    .reduce(|a, b| (a.0 + b.0, a.1 + b.1))
                                    .unwrap();
                                let scale = new_surr / old_surr;

                                {
                                    if is_p {
                                        assign_vals
                                            .hv_get_mut(axis)
                                            .iter_mut()
                                            .zip(new_vals.iter())
                                            .for_each(|(old, new)| {
                                                old.excess = *new;
                                            });

                                        for i in i_ofs..i_ofs + 2 {
                                            intervals[i].excess = (intervals[i].total() * scale
                                                - intervals[i].base)
                                                .max(0.0);
                                        }
                                    } else {
                                        for i in 0..assign_vals.hv_get(axis).len() {
                                            let av = &mut assign_vals.hv_get_mut(axis)[i];
                                            if i < area_idx[0] {
                                                av.excess = new_vals[i];
                                            } else if i < area_idx[1] {
                                                av.excess = av.total() * scale - av.base;
                                            } else {
                                                av.excess = new_vals[i - area_idx[1] + surr_idx[1]];
                                            }
                                        }

                                        intervals[i_ofs].excess = new_vals[surr_idx[0]];
                                        intervals[i_ofs + 1].excess = new_vals[surr_idx[0] + 2];

                                        let mut surr_range = DataHV::splat(None);
                                        *surr_range.hv_get_mut(axis) = Some(area[0]..=area[1]);
                                        *surr_range.hv_get_mut(axis.inverse()) =
                                            Some(0..=surr_area.hv_get(axis.inverse())[0]);
                                        let mut surr_paths = proto.to_path_in(
                                            Default::default(),
                                            assign_vals.map(|list| {
                                                list.iter().map(|av| av.total()).collect()
                                            }),
                                            surr_range.clone(),
                                        );
                                        *surr_range.hv_get_mut(axis.inverse()) = Some(
                                            surr_area.hv_get(axis.inverse())[1]
                                                ..=view.size().hv_get(axis.inverse()) - 1,
                                        );
                                        surr_paths.extend(proto.to_path_in(
                                            Default::default(),
                                            assign_vals.map(|list| {
                                                list.iter().map(|av| av.total()).collect()
                                            }),
                                            surr_range.clone(),
                                        ));

                                        let surr_center = al::visual_center_length(
                                            surr_paths,
                                            min_len,
                                            cfg.strok_width,
                                        );
                                        let mut surr_assign_vals = assign_vals.clone();
                                        *surr_assign_vals.hv_get_mut(axis) =
                                            assign_vals.hv_get(axis)[area_idx[0]..area_idx[1]]
                                                .into();
                                        let surr_new_val = al::center_correction(
                                            &surr_assign_vals
                                                .hv_get(axis)
                                                .iter()
                                                .map(|av| av.total())
                                                .collect(),
                                            &surr_assign_vals
                                                .hv_get(axis)
                                                .iter()
                                                .map(|av| av.base)
                                                .collect(),
                                            *surr_center.hv_get(axis),
                                            cfg.center.hv_get(axis).operation,
                                            cfg.center.hv_get(axis).execution,
                                        );

                                        for i in area_idx[0]..area_idx[1] {
                                            assign_vals.hv_get_mut(axis)[i].excess =
                                                surr_new_val[i - area_idx[0]];
                                        }
                                    }
                                }

                                for i in i_ofs..i_ofs + 2 {
                                    new_surr -= intervals[i].total();

                                    s_offsets.hv_get_mut(axis)[i % 2].excess -=
                                        old_intervals[i].excess / 2.0;
                                    s_offsets.hv_get_mut(axis)[i % 2].excess +=
                                        intervals[i].excess / 2.0;
                                }
                                Some(new_surr)
                            } else {
                                panic!()
                            }
                        });

                        combs[1].assign_space(
                            s_assign,
                            s_offsets.into_map(|ofs| ofs.map(|v| Some(v))),
                            levels,
                            cfg,
                        );
                    }
                    CstType::Single => unreachable!(),
                },
                Self::Single { .. } => unreachable!(),
            }
        }
    }

    pub fn edge_aligment(&mut self, cfg: &Config) {
        if matches!(self, Self::Single { .. }) {
            for axis in Axis::list() {
                let edge_corr = [Place::Start, Place::End].map(|place| {
                    self.get_comb_lines(axis, place)
                        .to_edge()
                        .gray_scale(cfg.strok_width)
                });

                if let Self::Single {
                    assign_vals,
                    offsets,
                    ..
                } = self
                {
                    let corrected = [0, 1].map(|i| {
                        let old_val = offsets.hv_get(axis)[i].excess;
                        offsets.hv_get_mut(axis)[i].excess *= edge_corr[i];
                        old_val * (1. - edge_corr[i])
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
        } else if let Self::Complex { combs, .. } = self {
            combs.iter_mut().for_each(|c| c.edge_aligment(cfg));
        }
    }

    pub fn expand_comb_proto(
        &mut self,
        fas: &FasFile,
        gen_info: bool,
    ) -> Result<Option<CharInfo>, CstError> {
        let (assign, offsets, levels, scales) = self.check_space(fas)?;
        self.assign_space(
            assign.into_map(|val| Some(val)),
            offsets.into_map(|ofs| ofs.map(|val| Some(val))),
            levels,
            &fas.config,
        );

        self.process_space(levels, &fas.config);
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
                        let mut size_list: Vec<(usize, usize)> = combs
                            .iter()
                            .map(|c| c.get_main_base_length(axis))
                            .enumerate()
                            .collect();
                        let max_size = size_list.iter().max_by_key(|(_, s)| *s).unwrap().1;

                        if comb_axis == axis {
                            size_list.sort_by(|(_, s1), (_, s2)| s1.cmp(s2));
                            let mut target = None;
                            for (i, s) in size_list.iter().copied().rev() {
                                match target {
                                    None => {
                                        ok |= combs[i].reduce_space(axis, is_check);
                                        if ok {
                                            target = Some(s)
                                        }
                                    }
                                    Some(target) => {
                                        if s < target {
                                            break;
                                        } else {
                                            combs[i].reduce_space(axis, is_check);
                                        }
                                    }
                                }
                            }
                        } else {
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

    pub fn reduce_replace(&mut self, axis: Axis, fas: &FasFile) -> Result<bool, CstError> {
        let cfg = &fas.config;
        let components = &fas.components;
        let name = self.get_name();

        if let Some(new_name) = cfg.reduce_replace.hv_get(axis).get(name) {
            if let Some(mut new_proto) = components.get(new_name).cloned() {
                for axis in Axis::list() {
                    let old_len = self.get_bases_length(axis);
                    loop {
                        let new_len = new_proto
                            .allocation_space()
                            .hv_get(axis)
                            .iter()
                            .sum::<usize>();
                        if new_len <= old_len || !new_proto.reduce(axis, false) {
                            break;
                        }
                    }
                }

                *self = Self::new_single(new_name.to_string(), new_proto);
                Ok(true)
            } else {
                Err(CstError::Empty(new_name.to_string()))
            }
        } else {
            let mut ok = false;
            if let Self::Complex {
                tp,
                combs,
                intervals_alloc,
                ..
            } = self
            {
                match *tp {
                    CstType::Scale(comb_axis) => {
                        let mut size_list: Vec<(usize, usize)> = combs
                            .iter()
                            .map(|c| c.get_bases_length(axis))
                            .enumerate()
                            .collect();
                        let max_size = size_list.iter().max_by_key(|(_, s)| *s).unwrap().1;

                        if comb_axis == axis {
                            size_list.sort_by(|(_, s1), (_, s2)| s1.cmp(s2));
                            let mut target = None;
                            for (i, s) in size_list.iter().copied().rev() {
                                match target {
                                    None => {
                                        ok |= combs[i].reduce_replace(axis, fas)?;
                                        if ok {
                                            target = Some(s)
                                        }
                                    }
                                    Some(target) => {
                                        if s < target {
                                            break;
                                        } else {
                                            combs[i].reduce_replace(axis, fas)?;
                                        }
                                    }
                                }
                            }
                        } else {
                            ok = true;
                            for (i, s) in size_list {
                                if max_size == s {
                                    ok &= combs[i].reduce_replace(axis, fas)?;
                                }
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
                                    ok &= combs[i].reduce_replace(axis, fas)?;
                                }
                            }
                            std::cmp::Ordering::Greater => {
                                ok |= combs[0].reduce_replace(axis, fas)?
                            }
                            std::cmp::Ordering::Less => {
                                let p_len = combs[0].get_bases_length(axis);
                                let list = if area[0] > s_surr_len || p_len - area[1] > s_surr_len {
                                    [0, 1]
                                } else {
                                    [1, 0]
                                };
                                for i in list {
                                    if combs[i].reduce_replace(axis, fas)? {
                                        ok = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    CstType::Single => panic!(),
                }
            }
            Ok(ok)
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
