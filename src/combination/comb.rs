use crate::{
    base::*,
    combination::{StrucProto, StrucView, attrs, view},
    construct::{CharTree, CstType},
    service::algorithm as al,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum CompInfoData {
    Single {
        allocs: DataHV<Vec<usize>>,
        assigns: DataHV<Vec<f32>>,
        level: DataHV<usize>,
    },
    Scale {
        axis: Axis,
        comps: Vec<StrucCombInfo>,
        intervals: Vec<usize>,
        intervals_val: Vec<f32>,
    },
    Surround {
        surround: DataHV<Section>,
        comps: Vec<StrucCombInfo>,
    },
}

#[derive(Serialize, Deserialize)]
pub struct StrucCombInfo {
    pub name: String,
    pub comb_name: String,
    pub blanks: DataHV<[f32; 2]>,
    pub cdata: CompInfoData,
}

#[derive(Serialize, Deserialize)]
pub struct CompTree {
    pub name: String,
    pub tp: CstType,
    pub paths: Vec<Vec<WorkKeyPoint>>,
    pub children: Vec<CompTree>,
}

pub enum CompData {
    Single {
        proto: StrucProto,
        view: StrucView,
        assigns: DataHV<Vec<AssignVal>>,
        level: DataHV<usize>,
    },
    Scale {
        axis: Axis,
        comps: Vec<StrucComb>,
        intervals: Vec<usize>,
        intervals_val: Vec<AssignVal>,
    },
    Surround {
        surround: DataHV<Section>,
        comps: Vec<StrucComb>,
    },
}

pub struct CompIter<'a> {
    comb: &'a StrucComb,
    stack: Vec<usize>,
}

impl<'a> CompIter<'a> {
    pub fn new(comb: &'a StrucComb) -> Self {
        Self {
            comb,
            stack: vec![0],
        }
    }
}

impl<'a> std::iter::Iterator for CompIter<'a> {
    type Item = &'a StrucComb;

    fn next(&mut self) -> Option<Self::Item> {
        fn recursion<'a>(
            comb: &'a StrucComb,
            stack: &mut Vec<usize>,
            pointer: usize,
        ) -> Option<&'a StrucComb> {
            if pointer == stack.len() {
                match &comb.cdata {
                    CompData::Single { .. } => *stack.get_mut(pointer - 1).unwrap() += 1,
                    _ => stack.push(0),
                }
                Some(comb)
            } else {
                match &comb.cdata {
                    CompData::Single { .. } => unreachable!(),
                    CompData::Scale { comps, .. } | CompData::Surround { comps, .. } => {
                        let c = comps.get(*stack.get(pointer).unwrap()).unwrap();
                        let r = recursion(c, stack, pointer + 1);

                        let idx = *stack.get(pointer).unwrap();
                        if idx == comps.len() {
                            stack.pop();
                            *stack.get_mut(pointer - 1).unwrap() += 1;
                        }
                        r
                    }
                }
            }
        }

        if self.stack[0] != 0 {
            None
        } else {
            recursion(self.comb, &mut self.stack, 1)
        }
    }
}

pub struct StrucComb {
    pub name: String,
    pub blanks: DataHV<[AssignVal; 2]>,
    pub cdata: CompData,
    pub attrs: attrs::CompAttrs,
}

impl StrucComb {
    pub fn new_single(name: String, proto: StrucProto) -> Self {
        Self {
            name,
            blanks: Default::default(),
            cdata: CompData::Single {
                view: StrucView::new(&proto),
                proto,
                assigns: Default::default(),
                level: Default::default(),
            },
            attrs: Default::default(),
        }
    }

    pub fn new_complex(name: String, tp: CstType, comps: Vec<StrucComb>) -> Self {
        let cdata = match tp {
            CstType::Scale(axis) => CompData::Scale {
                axis,
                comps,
                intervals: Default::default(),
                intervals_val: Default::default(),
            },
            CstType::Surround(surround) => CompData::Surround { surround, comps },
            CstType::Single => panic!("Construct Single in Complex!"),
        };

        Self {
            name,
            blanks: Default::default(),
            cdata,
            attrs: Default::default(),
        }
    }

    pub fn iter(&self) -> CompIter<'_> {
        CompIter::new(self)
    }

    pub fn get_comb_name(&self) -> String {
        match &self.cdata {
            CompData::Single { .. } => self.name.clone(),
            CompData::Scale { axis, comps, .. } => {
                format!(
                    "{}({})",
                    CstType::Scale(*axis).symbol(),
                    comps
                        .iter()
                        .map(|c| c.get_comb_name())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            CompData::Surround { surround, comps } => {
                format!(
                    "{}({}, {})",
                    CstType::Surround(*surround).symbol(),
                    comps[0].get_comb_name(),
                    comps[1].get_comb_name()
                )
            }
        }
    }

    pub fn get_char_tree(&self) -> CharTree {
        let name = self.name.clone();
        match &self.cdata {
            CompData::Single { .. } => CharTree::new_single(name),
            CompData::Scale { axis, comps, .. } => CharTree {
                name,
                tp: CstType::Scale(*axis),
                children: comps.iter().map(|c| c.get_char_tree()).collect(),
            },
            CompData::Surround { surround, comps } => CharTree {
                name: name,
                tp: CstType::Surround(surround.clone()),
                children: comps.iter().map(|c| c.get_char_tree()).collect(),
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

    pub fn set_white_area(&mut self, white: &DataHV<[f32; 2]>) {
        self.attrs.set::<attrs::WhiteArea>(&white);
    }

    pub fn get_white_area(&self) -> Option<DataHV<[f32; 2]>> {
        self.attrs.get::<attrs::WhiteArea>()
    }

    pub fn get_blank_base(&self, axis: Axis) -> [usize; 2] {
        self.blanks
            .hv_get(axis)
            .map(|v| if v.base == 0.0 { 0 } else { 1 })
    }

    pub fn get_bases_length(&self, axis: Axis, blank: bool) -> usize {
        let len = match &self.cdata {
            CompData::Single { proto, .. } => *proto.size().hv_get(axis),
            CompData::Scale {
                axis: c_axis,
                comps,
                intervals,
                ..
            } => {
                if axis == *c_axis {
                    comps
                        .iter()
                        .map(|c| c.get_bases_length(axis, true))
                        .chain(intervals.iter().copied())
                        .sum::<usize>()
                } else {
                    comps
                        .iter()
                        .map(|c| c.get_bases_length(axis, true))
                        .max()
                        .unwrap()
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        };

        if blank {
            len + self.get_blank_base(axis).iter().sum::<usize>()
        } else {
            len
        }
    }

    pub fn get_line_weight(&self) -> DataHV<f32> {
        match &self.cdata {
            CompData::Single { proto, assigns, .. } => proto
                .subarea_line_weight(
                    &assigns.map(|list| list.iter().map(|v| v.total()).collect()),
                    DataHV::splat(0.0),
                )
                .into_map(|list| list.iter().sum()),
            CompData::Scale { comps, .. } => comps
                .iter()
                .map(|c| c.get_line_weight())
                .reduce(|a, b| a.zip(b).into_map(|(a, b)| a + b))
                .unwrap(),
            CompData::Surround { .. } => todo!(), // surround
        }
    }

    pub fn get_assign_value(&self, axis: Axis, blank: bool) -> AssignVal {
        fn add(a: AssignVal, b: AssignVal) -> AssignVal {
            a + b
        }

        let val = match &self.cdata {
            CompData::Single { assigns, .. } => assigns
                .hv_get(axis)
                .iter()
                .copied()
                .reduce(add)
                .unwrap_or_default(),
            CompData::Scale {
                axis: c_axis,
                comps,
                intervals_val,
                ..
            } => {
                if *c_axis == axis {
                    comps
                        .iter()
                        .map(|c| c.get_assign_value(axis, true))
                        .chain(intervals_val.iter().copied())
                        .reduce(add)
                        .unwrap()
                } else {
                    comps
                        .iter()
                        .map(|c| c.get_assign_value(axis, true))
                        .max_by(|a, b| a.base.partial_cmp(&b.base).unwrap())
                        .unwrap()
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        };

        if blank {
            val + self
                .blanks
                .hv_get(axis)
                .iter()
                .copied()
                .reduce(add)
                .unwrap()
        } else {
            val
        }
    }

    pub fn get_comp_tree(&self) -> CompTree {
        let mut start = self.get_char_box().min;
        let offset = self.get_white_area().unwrap();
        start.x += offset.h[0];
        start.y += offset.v[0];

        let (tree, _) = self.get_paths_in(start);
        tree
    }

    pub fn get_comb_info(&self) -> StrucCombInfo {
        fn assign_to_f32(v: AssignVal) -> f32 {
            v.total()
        }

        let name = self.name.clone();
        let comb_name = self.get_comb_name();
        let blanks = self.blanks.map(|data| data.map(assign_to_f32));
        let cdata = match &self.cdata {
            CompData::Single {
                assigns,
                proto,
                level,
                ..
            } => CompInfoData::Single {
                allocs: proto.allocation_space(),
                assigns: assigns.map(|data| data.iter().map(AssignVal::total).collect()),
                level: *level,
            },
            CompData::Scale {
                axis,
                comps,
                intervals,
                intervals_val,
            } => CompInfoData::Scale {
                axis: *axis,
                comps: comps.iter().map(|c| c.get_comb_info()).collect(),
                intervals: intervals.clone(),
                intervals_val: intervals_val.iter().map(AssignVal::total).collect(),
            },
            CompData::Surround { .. } => todo!(), // surround
        };

        StrucCombInfo {
            name,
            comb_name,
            blanks,
            cdata,
        }
    }

    fn get_paths_in(&self, start: WorkPoint) -> (CompTree, DataHV<f32>) {
        let blanks = &self.blanks;
        let mut new_start =
            WorkPoint::new(start.x + blanks.h[0].total(), start.y + blanks.v[0].total());
        let mut size = DataHV::splat(0.0);

        let tree = match &self.cdata {
            CompData::Single { proto, assigns, .. } => {
                let assigns = assigns.map(|assign| assign.iter().map(|av| av.total()).collect());
                size = assigns.map(|assigns: &Vec<f32>| assigns.iter().sum::<f32>());
                let paths = proto.get_paths(new_start, &assigns);

                CompTree {
                    name: self.name.clone(),
                    tp: CstType::Single,
                    paths,
                    children: vec![],
                }
            }
            CompData::Scale {
                axis,
                comps,
                intervals_val,
                ..
            } => {
                let axis = *axis;
                let mut interval = intervals_val.iter().map(|v| v.total());
                let children: Vec<_> = comps
                    .iter()
                    .map(|c| {
                        let (c_tree, c_size) = c.get_paths_in(new_start);
                        let advance = *c_size.hv_get(axis) + interval.next().unwrap_or_default();
                        *new_start.hv_get_mut(axis) += advance;
                        *size.hv_get_mut(axis) += advance;
                        *size.hv_get_mut(axis.inverse()) = *c_size.hv_get(axis.inverse());

                        c_tree
                    })
                    .collect();

                CompTree {
                    name: self.name.clone(),
                    tp: CstType::Scale(axis),
                    paths: Default::default(),
                    children,
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        };

        for axis in Axis::list() {
            *size.hv_get_mut(axis) += blanks
                .hv_get(axis)
                .iter()
                .map(|ofs| ofs.total())
                .sum::<f32>();
        }

        (tree, size)
    }

    pub fn get_edge(&self, axis: Axis, side: Side, blank: bool) -> view::Edge {
        let mut edge = match &self.cdata {
            CompData::Single { view, .. } => view.get_edge(axis, side),
            CompData::Scale {
                axis: c_axis,
                comps,
                intervals,
                ..
            } => {
                if axis == *c_axis {
                    match side {
                        Side::Front => comps[0].get_edge(axis, side, true),
                        Side::Back => comps.last().unwrap().get_edge(axis, side, true),
                    }
                } else {
                    let mut interval = intervals.iter();
                    comps
                        .iter()
                        .map(|c| c.get_edge(axis, side, true))
                        .reduce(|mut e1, e2| {
                            let n = interval.next().copied().unwrap_or_default();
                            e1.connect(e2, n);
                            e1
                        })
                        .unwrap()
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        };

        if blank {
            if self.blanks.hv_get(axis)[side.n()].base != 0.0 {
                edge.backspace();
            }
            if self.blanks.hv_get(axis.inverse())[0].base != 0.0 {
                edge.add_head();
            }
            if self.blanks.hv_get(axis.inverse())[1].base != 0.0 {
                edge.add();
            }
        }
        edge
    }

    pub fn reduce_space(&mut self, axis: Axis, is_check: bool) -> bool {
        let new_length = match &mut self.cdata {
            CompData::Single { proto, view, .. } => {
                if proto.reduce(axis, is_check) {
                    if !is_check {
                        *view = StrucView::new(proto);
                    }
                    Some(*proto.size().hv_get(axis))
                } else {
                    None
                }
            }
            CompData::Scale {
                axis: c_axis,
                comps,
                ..
            } => {
                let lenghts: Vec<_> = comps
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (i, c.get_bases_length(axis, true)))
                    .collect();

                if axis == *c_axis {
                    let mut new_lengths = lenghts.clone();
                    new_lengths.last_mut().unwrap().1 = usize::MAX;
                    new_lengths.sort_by_key(|(_, c)| *c);

                    let mut r = None;
                    for (i, _) in new_lengths {
                        if comps[i].reduce_space(axis, is_check) {
                            r = Some(lenghts.iter().map(|(_, c)| c).sum::<usize>() - 1);
                            break;
                        }
                    }
                    r
                } else {
                    let max_len = lenghts.iter().max_by_key(|(_, l)| *l).unwrap().1;
                    let mut r = None;
                    if lenghts
                        .iter()
                        .all(|&(i, l)| l != max_len || comps[i].reduce_space(axis, true))
                    {
                        let mut len = 0;
                        for (i, l) in lenghts {
                            len += l;
                            if !is_check && l == max_len {
                                comps[i].reduce_space(axis, is_check);
                            }
                        }
                        r = Some(len - 1);
                    }
                    r
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        };

        if let Some(new_len) = new_length {
            if !is_check {
                let mut r_target = self.attrs.get::<attrs::ReduceTarget>().unwrap_or_default();
                *r_target.hv_get_mut(axis) = Some(new_len);
                self.attrs.set::<attrs::ReduceTarget>(&r_target);

                let mut main_comp = self.attrs.get::<attrs::MainComp>().unwrap_or_default();
                *main_comp.hv_get_mut(axis) = false;
                self.attrs.set::<attrs::MainComp>(&main_comp);
            }
            true
        } else {
            if self.blanks.hv_get(axis).iter().any(|v| v.base != 0.0) {
                if !matches!(&self.cdata, CompData::Single { view, .. } if *view.space_size().hv_get(axis) == 0)
                {
                    if !is_check {
                        let mut main_comp = self.attrs.get::<attrs::MainComp>().unwrap_or_default();
                        *main_comp.hv_get_mut(axis) = true;
                        self.attrs.set::<attrs::MainComp>(&main_comp);
                    }
                    return true;
                }
            }
            false
        }
    }

    pub fn set_edge_alloc(
        comb1: &mut StrucComb,
        edge1: &view::EdgeShape,
        comb2: &mut StrucComb,
        edge2: &view::EdgeShape,
        axis: Axis,
    ) -> Result<Option<usize>, ()> {
        fn get_edge_main_comb(
            comb: &mut StrucComb,
            axis: Axis,
            side: Side,
        ) -> Option<(&mut StrucProto, &mut StrucView)> {
            match &mut comb.cdata {
                CompData::Single { proto, view, .. } => Some((proto, view)),
                CompData::Scale {
                    axis: c_axis,
                    comps,
                    ..
                } => {
                    if axis == *c_axis {
                        let idx = [0, comps.len() - 1][side.n()];
                        get_edge_main_comb(&mut comps[idx], axis, side)
                    } else {
                        None
                    }
                }
                CompData::Surround { surround, comps } => {
                    if *surround.hv_get(axis) != side.inverse().to_section() {
                        get_edge_main_comb(&mut comps[0], axis, side)
                    } else {
                        None
                    }
                }
            }
        }

        fn match_rule(
            proto: &StrucProto,
            edge: &view::EdgeShape,
            blanks: [bool; 2],
            axis: Axis,
            side: Side,
        ) -> Option<(Option<usize>, Option<usize>, bool)> {
            let setting = proto.attrs.get::<attrs::IntervalAlloc>();
            let settings = setting
                .as_ref()
                .and_then(|data| data.get(&axis))
                .and_then(|data| data.get(&side));

            if let Some(settings) = settings {
                settings.iter().find_map(|setting| {
                    if setting
                        .rules
                        .iter()
                        .find(|rule| {
                            let mut b = true;
                            if let Some(query) = setting.blanks {
                                b = blanks[0] == query[0] && blanks[1] == query[1];
                            }
                            b && rule.is_match(edge)
                        })
                        .is_some()
                    {
                        Some((setting.interval, setting.allocs, setting.requist))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        }

        fn set_alloc(
            (proto, view): (&mut StrucProto, &mut StrucView),
            interval: Option<usize>,
            target: Option<usize>,
            side: Side,
            axis: Axis,
        ) -> Result<Option<usize>, ()> {
            let target = if let Some(target) = target {
                target
            } else {
                return Ok(interval);
            };

            let mut allocs_proto = proto.allocation_values();
            let allocs = allocs_proto.hv_get_mut(axis);
            let idx = [0, allocs.len() - 1][side.n()];

            if allocs[idx] != target {
                allocs[idx] = target;
                proto.attrs.set::<attrs::Allocs>(&allocs_proto);
                *view = StrucView::new(&proto);
                Err(())
            } else {
                let mut fiexd = proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();
                fiexd.hv_get_mut(axis).insert(idx);
                proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
                Ok(interval)
            }
        }

        let blank1 = comb1.blanks.hv_get(axis.inverse()).map(|v| v.base != 0.0);
        let blank2 = comb2.blanks.hv_get(axis.inverse()).map(|v| v.base != 0.0);
        let c1 = get_edge_main_comb(comb1, axis, Side::Back);
        let r1 = c1
            .as_ref()
            .and_then(|(p, _)| match_rule(p, edge2, blank1, axis, Side::Back));
        let c2 = get_edge_main_comb(comb2, axis, Side::Front);
        let r2 = c2
            .as_ref()
            .and_then(|(p, _)| match_rule(p, edge1, blank2, axis, Side::Front));

        match (r1, r2) {
            (Some(r1), Some(r2)) => match (r1.0, r2.0) {
                (Some(i1), Some(i2)) => {
                    if i1 != i2 {
                        return Ok(None);
                    } else {
                        set_alloc(c1.unwrap(), Some(i1), r1.1, Side::Back, axis).and(set_alloc(
                            c2.unwrap(),
                            Some(i2),
                            r2.1,
                            Side::Front,
                            axis,
                        ))
                    }
                }
                _ => {
                    let interval = r1.0.or(r2.0);
                    if !r1.2 {
                        set_alloc(c1.unwrap(), interval, r1.1, Side::Back, axis)
                    } else if !r2.2 {
                        set_alloc(c2.unwrap(), interval, r2.1, Side::Front, axis)
                    } else {
                        Ok(None)
                    }
                }
            },
            (Some((interval, alloc, requist)), None) if !requist => {
                set_alloc(c1.unwrap(), interval, alloc, Side::Back, axis)
            }
            (None, Some((interval, alloc, requist))) if !requist => {
                set_alloc(c2.unwrap(), interval, alloc, Side::Front, axis)
            }
            _ => Ok(None),
        }
    }

    pub fn reassign_space(&mut self, mut new_val: f32, blank: bool, axis: Axis) {
        if blank {
            let old_assigns = self.get_assign_value(axis, false);
            let b_assigns = self.blanks.hv_get_mut(axis);
            let mut assigns_list = [b_assigns[0], old_assigns, b_assigns[1]];
            al::reassign(&mut assigns_list, new_val).unwrap();

            *b_assigns = [assigns_list[0], assigns_list[2]];
            new_val = assigns_list[1].total();
        }

        match &mut self.cdata {
            CompData::Single { assigns, .. } => {
                al::reassign(assigns.hv_get_mut(axis), new_val).unwrap()
            }
            CompData::Scale {
                axis: c_axis,
                comps,
                intervals_val,
                ..
            } => {
                if *c_axis == axis {
                    let mut assigns_list: Vec<AssignVal> = comps
                        .iter()
                        .map(|c| c.get_assign_value(axis, true))
                        .collect();
                    assigns_list.extend(intervals_val.iter().copied());
                    al::reassign(&mut assigns_list, new_val).unwrap();

                    let mut iter = assigns_list.into_iter();
                    comps
                        .iter_mut()
                        .for_each(|c| c.reassign_space(iter.next().unwrap().total(), true, axis));
                    intervals_val
                        .iter_mut()
                        .for_each(|av| *av = iter.next().unwrap());
                } else {
                    comps
                        .iter_mut()
                        .for_each(|c| c.reassign_space(new_val, true, axis));
                }
            }
            CompData::Surround { .. } => todo!(), // surround
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comb_iter() {
        let comp = StrucComb::new_single("1".to_string(), Default::default());
        let names: Vec<&str> = comp.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["1"]);

        let comp = StrucComb::new_complex(
            "1".to_string(),
            CstType::Scale(Axis::Horizontal),
            vec![
                StrucComb::new_single("2".to_string(), Default::default()),
                StrucComb::new_complex(
                    "3".to_string(),
                    CstType::Surround(DataHV::splat(Section::Start)),
                    vec![
                        StrucComb::new_single("4".to_string(), Default::default()),
                        StrucComb::new_single("5".to_string(), Default::default()),
                    ],
                ),
            ],
        );
        let names: Vec<&str> = comp.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_edge_sharpness() {
        use view::SharpnessModel;

        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 1), key_pos(1, 2)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 2), key_pos(3, 2)]),
            KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
        ]);
        let comb = StrucComb::new_single("name".to_string(), struc);
        let sharpness = Axis::hv().into_map(|axis| {
            Side::fb().map(|side| {
                comb.get_edge(axis, side, false)
                    .sharpness(SharpnessModel::ZeroOne)
            })
        });
        assert_eq!(sharpness.h[0], 1.0);
        assert_eq!(sharpness.h[1], 0.0);
        assert_eq!(sharpness.v[0], 0.0);
        assert_eq!(sharpness.v[1], 1.0);

        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 0),
            key_pos(2, 0),
            key_pos(2, 2),
            key_pos(0, 2),
            key_pos(0, 0),
        ])]);
        let comb = StrucComb::new_single("name".to_string(), struc);
        let sharpness = Axis::hv().into_map(|axis| {
            Side::fb().map(|side| {
                comb.get_edge(axis, side, false)
                    .sharpness(SharpnessModel::ZeroOne)
            })
        });
        assert_eq!(sharpness.h[0], 1.0);
        assert_eq!(sharpness.h[1], 1.0);
        assert_eq!(sharpness.v[0], 1.0);
        assert_eq!(sharpness.v[1], 1.0);
    }
}
