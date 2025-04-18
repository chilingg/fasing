use crate::{
    algorithm as al,
    axis::*,
    component::{
        attrs,
        struc::StrucProto,
        view::{StandardEdge, StrucView, ViewLines},
    },
    config::Config,
    construct::{space::*, CstError, CstType},
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

impl std::ops::Add for AssignVal {
    type Output = AssignVal;

    fn add(self, rhs: AssignVal) -> Self::Output {
        AssignVal {
            base: self.base + rhs.base,
            excess: self.excess + rhs.excess,
        }
    }
}

impl std::iter::Sum for AssignVal {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Default::default(), |a, b| a + b)
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

    pub fn get_comb_name(&self) -> String {
        match self {
            Self::Single { name, .. } => name.clone(),
            Self::Complex { tp, combs, .. } => {
                format!(
                    "{}({})",
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

    pub fn get_bases_length(&self, axis: Axis) -> usize {
        match self {
            Self::Single { proto, .. } => proto.allocation_space().hv_get(axis).iter().sum(),
            Self::Complex {
                tp,
                intervals_alloc,
                combs,
                ..
            } => {
                match *tp {
                    CstType::Scale(comb_axis) => {
                        let list = combs.iter().map(|c| c.get_bases_length(axis));
                        if axis == comb_axis {
                            list.chain(intervals_alloc.iter().copied()).sum()
                        } else {
                            list.max().unwrap()
                        }
                    }
                    CstType::Surround(_) => {
                        todo!(); // bases_length surround
                    }
                    CstType::Single => unreachable!(),
                }
            }
        }
    }

    pub fn get_assign_length(&self, axis: Axis) -> AssignVal {
        match self {
            Self::Single { assign_vals, .. } => assign_vals.hv_get(axis).iter().copied().sum(),
            Self::Complex {
                tp,
                intervals,
                combs,
                ..
            } => {
                match *tp {
                    CstType::Scale(comb_axis) => {
                        let list = combs.iter().map(|c| c.get_assign_length(axis));
                        if axis == comb_axis {
                            list.chain(intervals.iter().copied()).sum()
                        } else {
                            list.max_by(|a, b| match a.base.partial_cmp(&b.base).unwrap() {
                                std::cmp::Ordering::Equal => {
                                    a.excess.partial_cmp(&b.excess).unwrap()
                                }
                                cmp => cmp,
                            })
                            .unwrap()
                        }
                    }
                    CstType::Surround(_) => {
                        todo!(); // bases_length surround
                    }
                    CstType::Single => unreachable!(),
                }
            }
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
            } => {
                match *tp {
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
                                    for _ in 0..intervals_alloc[i2 - 1] {
                                        lines1.add_gap(Place::End);
                                    }
                                    lines1.connect(lines2);
                                    (usize::MAX, lines1)
                                })
                                .unwrap()
                                .1
                        }
                    }
                    CstType::Surround(_) => {
                        todo!(); // edge Surround
                    }
                    CstType::Single => unreachable!(),
                }
            }
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
                    lines[1].add_gap(place);
                } else {
                    lines[0].add_gap(place);
                }
            }
        }
        lines.map(|l| l.to_standard_edge(dot_val))
    }

    fn set_allocs_in_edge(
        &mut self,
        edge: &StandardEdge,
        axis: Axis,
        place: Place,
    ) -> Option<(bool, usize)> {
        let mut ok = None;
        if let Self::Single { view, proto, .. } = self {
            if let Some(data) = proto.attrs.get::<attrs::IntervalAlloc>() {
                if let Some(data) = data.get(&axis) {
                    if let Some(interval_alloc) = data.get(&place) {
                        let rules = &interval_alloc.rules;
                        let allocs = &interval_alloc.allocs;
                        let mut fiexd = proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();

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
        ok
    }

    pub fn init_edges(&mut self, cfg: &Config) -> DataHV<usize> {
        match self {
            Self::Single { proto, .. } => {
                proto.allocation_values().map(|allocs| allocs.iter().sum())
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
                            len_list.extend(combs.iter_mut().map(|c| c.init_edges(cfg)));

                            {
                                // todo!() // Set edge alignment
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

                        Axis::hv().into_map(|axis| {
                            if axis == comb_axis {
                                len_list.iter().map(|cl| *cl.hv_get(axis)).sum::<usize>()
                                    + intervals_alloc.iter().sum::<usize>()
                            } else {
                                len_list.iter().map(|cl| *cl.hv_get(axis)).max().unwrap()
                            }
                        })
                    }
                    CstType::Surround(_surround) => {
                        todo!() // init edges surround
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

        let mut base_len_list = self.init_edges(cfg);
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
                            if scale < *cfg.reduce_trigger.hv_get(axis) && self.reduce_space(axis) {
                                *check_state.hv_get_mut(axis.inverse()) = false;
                                base_len_list = self.init_edges(cfg);
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

        Ok((assign, offsets, levels, scales))
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
                                let mut white_allocated = [0.; 2];
                                let comp_white = cfg.comp_white.hv_get(comb_axis.inverse());
                                let sub_axis_ofs = offsets.hv_get_mut(comb_axis.inverse());

                                for (j, place) in Place::start_and_end().iter().enumerate() {
                                    if !edge_main.hv_get(comb_axis.inverse())[place][i]
                                        || base_total == 0.0
                                    {
                                        white_allocated[j] = comp_white.allocated;
                                        let mut offset_val = sub_axis_ofs[j]
                                            .unwrap_or(ofs.hv_get(comb_axis.inverse())[j]);

                                        if base_total != 0.0 {
                                            offset_val.base += offset_val.excess + comp_white.fixed;
                                            offset_val.excess = comp_white.value;
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
                                    let scale = *val
                                        / (white_allocated[0] + base_total + white_allocated[1]);
                                    for j in 0..2 {
                                        sub_axis_ofs[j].as_mut().map(|sub_ofs| {
                                            sub_ofs.excess += white_allocated[j] * scale;
                                        });
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
                    CstType::Surround(_) => {
                        todo!(); // assign surround
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
                } => {
                    for c in combs.iter_mut() {
                        c.process_space(levels, cfg);
                    }

                    match *tp {
                        CstType::Scale(comb_axis) => {
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

                                    if j < intervals.len() {
                                        *start.hv_get_mut(comb_axis) += intervals[j].total();
                                        vlist.push(intervals[j].total());
                                        bases.push(intervals[j].base);
                                    }
                                }

                                let center =
                                    al::visual_center_length(paths, min_len, cfg.strok_width);
                                let center_opt = cfg.comp_center.hv_get(comb_axis);
                                let new_vals = al::center_correction(
                                    &vlist,
                                    &bases,
                                    *center.hv_get(comb_axis),
                                    center_opt.operation,
                                    center_opt.execution,
                                );

                                for j in 0..=i {
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
                        CstType::Surround(_) => todo!(), // process space surround
                        CstType::Single => unreachable!(),
                    }
                }
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
                    let add_val = (corrected[0] + corrected[1]) / assign_val.len() as f32;
                    assign_val.iter_mut().for_each(|val| val.excess += add_val);
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
        self.edge_aligment(&fas.config);

        if gen_info {
            let min_len = fas.config.min_val.h[levels.h].min(fas.config.min_val.v[levels.v]);
            let mut comp_infos = vec![];
            self.get_comp_info(&mut comp_infos, fas.config.strok_width);

            Ok(Some(CharInfo {
                comb_name: self.get_name().to_string(),
                levels,
                white_areas: offsets.map(|offset| offset.map(|av| av.total())),
                scales,
                center: self.get_visual_center(min_len, fas.config.strok_width),
                comp_infos,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn reduce_space(&mut self, axis: Axis) -> bool {
        match self {
            Self::Single { view, proto, .. } => {
                if proto.reduce(axis) {
                    *view = StrucView::new(proto);
                    true
                } else {
                    false
                }
            }
            Self::Complex { tp, combs, .. } => {
                match *tp {
                    CstType::Scale(comb_axis) => {
                        let mut ok = false;
                        let mut size_list: Vec<(usize, usize)> = combs
                            .iter()
                            .map(|c| c.get_bases_length(axis))
                            .enumerate()
                            .collect();
                        let max_size = size_list.iter().max_by_key(|(_, s)| *s).unwrap().1;

                        if comb_axis == axis {
                            size_list.sort_by(|(_, s1), (_, s2)| s1.cmp(s2));
                            let mut target = None;
                            for (i, s) in size_list {
                                match target {
                                    None => {
                                        ok |= combs[i].reduce_space(axis);
                                        if ok {
                                            target = Some(s)
                                        }
                                    }
                                    Some(target) => {
                                        if s < target {
                                            break;
                                        } else {
                                            combs[i].reduce_space(axis);
                                        }
                                    }
                                }
                            }
                        } else {
                            size_list.into_iter().for_each(|(i, s)| {
                                if max_size == s {
                                    ok |= combs[i].reduce_space(axis);
                                }
                            });
                        }

                        ok
                    }
                    CstType::Surround(_) => todo!(), // reduce Surround
                    CstType::Single => unreachable!(),
                }
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
            Self::Complex { tp, combs, .. } => {
                match *tp {
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
                    CstType::Surround(_) => todo!(), // mergo to Surround
                    CstType::Single => unreachable!(),
                }
            }
        }
    }
}
