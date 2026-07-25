use crate::{
    base::*,
    combination::{StrucProto, StrucView, attrs, view},
    config::EdgeCheck,
    construct::{CstError, CstType},
    service::algorithm as al,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
        intervals: DataHV<[usize; 2]>,
        intervals_val: DataHV<[f32; 2]>,
        base_size: DataHV<usize>,
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
        intervals: DataHV<[usize; 2]>,
        intervals_val: DataHV<[AssignVal; 2]>,
    },
}

pub mod scale {
    use super::*;

    pub enum Record {
        None,
        Name {
            name: String,
            blanks: DataHV<[AssignVal; 2]>,
            attrs: attrs::CompAttrs,
            sub_recard: Vec<Record>,
        },
    }

    pub fn flatten(
        comps: &mut Vec<StrucComb>,
        intervals: &mut Vec<usize>,
        intervals_val: &mut Vec<AssignVal>,
        axis: Axis,
    ) -> Vec<Record> {
        let mut temp_comps = vec![];
        let mut temp_intervals = vec![];
        let mut temp_intervals_val = vec![];
        let mut records = vec![];

        let mut i_iter = intervals.drain(..);
        let mut iv_iter = intervals_val.drain(..);
        for comp in comps.drain(..) {
            match comp.cdata {
                CompData::Scale {
                    axis: c_axis,
                    mut comps,
                    mut intervals,
                    mut intervals_val,
                } if c_axis == axis => {
                    let new_records = flatten(&mut comps, &mut intervals, &mut intervals_val, axis);

                    records.push(Record::Name {
                        name: comp.name,
                        blanks: comp.blanks,
                        attrs: comp.attrs,
                        sub_recard: new_records,
                    });
                    temp_comps.append(&mut comps);
                    temp_intervals.append(&mut intervals);
                    temp_intervals_val.append(&mut intervals_val);
                }
                _ => {
                    records.push(Record::None);
                    temp_comps.push(comp);
                }
            }
            if let Some(interval) = i_iter.next() {
                temp_intervals.push(interval);
            }
            if let Some(interval) = iv_iter.next() {
                temp_intervals_val.push(interval);
            }
        }
        assert_eq!(i_iter.count(), 0);
        assert_eq!(iv_iter.count(), 0);

        *comps = temp_comps;
        *intervals = temp_intervals;
        *intervals_val = temp_intervals_val;
        records
    }

    pub fn restore(
        records: Vec<Record>,
        axis: Axis,
        comps: &mut Vec<StrucComb>,
        intervals: &mut Vec<usize>,
        intervals_val: &mut Vec<AssignVal>,
    ) {
        fn r(
            records: Vec<Record>,
            axis: Axis,
            comps: &mut Vec<StrucComb>,
            intervals: &mut Vec<usize>,
            intervals_val: &mut Vec<AssignVal>,
        ) -> (Vec<StrucComb>, Vec<usize>, Vec<AssignVal>) {
            let len = records.len();
            let mut new_comps = Vec::with_capacity(len);
            let mut new_intervals = vec![];
            let mut new_intervals_val = vec![];

            for (i, record) in records.into_iter().enumerate() {
                match record {
                    Record::None => {
                        new_comps.push(comps.pop().unwrap());
                    }
                    Record::Name {
                        name,
                        blanks,
                        attrs,
                        sub_recard,
                    } => {
                        let (sub_comps, sub_intervals, sub_intervals_val) =
                            r(sub_recard, axis, comps, intervals, intervals_val);
                        let sub_comp = StrucComb {
                            name,
                            blanks,
                            cdata: CompData::Scale {
                                axis,
                                comps: sub_comps,
                                intervals: sub_intervals,
                                intervals_val: sub_intervals_val,
                            },
                            attrs,
                        };

                        new_comps.push(sub_comp);
                    }
                }
                if i + 1 != len {
                    if let Some(i) = intervals.pop() {
                        new_intervals.push(i);
                    }
                    if let Some(i) = intervals_val.pop() {
                        new_intervals_val.push(i);
                    }
                }
            }
            (new_comps, new_intervals, new_intervals_val)
        }

        comps.reverse();
        intervals.reverse();
        intervals_val.reverse();

        (*comps, *intervals, *intervals_val) = r(records, axis, comps, intervals, intervals_val);
    }
}

pub mod surround {
    use super::*;

    pub fn check_surround(primary: &StrucComb, surround: &DataHV<Section>) -> Result<(), CstError> {
        match &primary.cdata {
            CompData::Single { view, .. } => match view.surround_area(*surround) {
                Some(_) => Ok(()),
                None => Err(CstError::Surround {
                    tp: CstType::Surround(*surround).symbol(),
                    comp: primary.name.clone(),
                }),
            },
            CompData::Scale { axis, .. } => Err(CstError::Surround {
                tp: CstType::Surround(*surround).symbol(),
                comp: CstType::Scale(*axis).symbol().to_string(),
            }),
            CompData::Surround {
                surround: c_surr, ..
            } => Err(CstError::Surround {
                tp: CstType::Surround(*surround).symbol(),
                comp: CstType::Surround(*c_surr).symbol().to_string(),
            }), // TODO surround in other type
        }
    }

    pub fn get_area_line_weight(
        proto: &StrucProto,
        assigns: &DataHV<Vec<f32>>,
        surr_area: DataHV<[usize; 2]>,
        target_axis: Axis,
    ) -> [f32; 3] {
        use view::Direction;

        let values_map = proto.values_map(true);
        let mut main_weights = vec![0.0; assigns.hv_get(target_axis).len()];
        let mut cross_weights = [0.0; 3];
        let main_area = surr_area.hv_get(target_axis);
        let main_ranges = [
            0..=main_area[0],
            main_area[0]..=main_area[1],
            main_area[1]..=assigns.hv_get(target_axis).len(),
        ];

        proto.paths.iter().for_each(|path| {
            let iter = path
                .kpoints
                .iter()
                .filter(|pos| !pos.is_mark())
                .map(|kp| IndexPoint::new(values_map.h[&kp.pos.x], values_map.v[&kp.pos.y]));
            iter.clone().zip(iter.skip(1)).for_each(|(p1, p2)| {
                let min = p1.min(p2);
                let max = p1.max(p2);

                let mut set_weights = |axis| {
                    let assigns = assigns.hv_get(axis);
                    if axis == target_axis {
                        for i in *min.hv_get(axis)..*max.hv_get(axis) {
                            main_weights[i] += assigns[i]
                        }
                    } else {
                        for j in [1, 0, 2] {
                            if main_ranges[j].contains(min.hv_get(axis.inverse()))
                                && main_ranges[j].contains(max.hv_get(axis.inverse()))
                            {
                                let min = *min.hv_get(axis);
                                let max = *max.hv_get(axis);
                                cross_weights[j] += assigns[min..max].iter().sum::<f32>();
                                break;
                            }
                        }
                    }
                };

                match Direction::new(p1, Some(p2)) {
                    Direction::Above | Direction::Below => set_weights(Axis::Vertical),
                    Direction::Left | Direction::Right => set_weights(Axis::Horizontal),
                    Direction::None => {}
                    _ => Axis::list().into_iter().for_each(|axis| set_weights(axis)),
                }
            });
        });

        let cross_len = assigns.hv_get(target_axis).iter().sum::<f32>();
        [0, 1, 2].map(|i| {
            let range = &main_ranges[i];
            let start = *range.start();
            let end = *range.end();
            if start == end || cross_len == 0.0 {
                0.0
            } else {
                main_weights[start..end].iter().sum::<f32>()
                    // / assigns.hv_get(target_axis)[start..end].iter().sum::<f32>()
                    + cross_weights[i]
                //  / cross_len
            }
        })
    }

    pub fn get_area_base_len(
        primary: &StrucComb,
        surround: &DataHV<Section>,
        axis: Axis,
    ) -> [usize; 3] {
        let mut areas = match &primary.cdata {
            CompData::Single { view, .. } => {
                let area = *view.surround_area(*surround).unwrap().hv_get(axis);
                let allocs = view.allocs.hv_get(axis);
                [0..area[0], area[0]..area[1], area[1]..allocs.len()]
                    .map(|range| allocs[range].iter().sum::<usize>())
            }
            _ => todo!(), // surround in
        };

        let blank = primary.get_blank_base(axis);
        if Section::End.eq(surround.hv_get(axis)) {
            areas[1] += blank[0];
        } else {
            areas[0] += blank[0];
        }
        if Section::Start.eq(surround.hv_get(axis)) {
            areas[1] += blank[1];
        } else {
            areas[2] += blank[1];
        }

        areas
    }

    pub fn primary_is_main(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
    ) -> bool {
        let [_, pm, _] = surround::get_area_base_len(primary, surround, axis);
        let secondary_len = secondary.get_bases_length(axis, true);
        let interval_len: usize = intervals.hv_get(axis).iter().sum();
        pm >= interval_len + secondary_len
    }

    pub fn get_bases_length(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
    ) -> usize {
        let [pf, _, pb] = surround::get_area_base_len(primary, surround, axis);
        let p_len = primary.get_bases_length(axis, true);

        let secondary_len = secondary.get_bases_length(axis, true);
        let interval_len: usize = intervals.hv_get(axis).iter().sum();
        p_len.max(pf + interval_len + secondary_len + pb)
    }

    pub fn get_area_complexity(
        view: &StrucView,
        surround: &DataHV<Section>,
        axis: Axis,
    ) -> [usize; 3] {
        let area = view.surround_area(*surround).unwrap().hv_get_into(axis);
        [
            view.get_complexity_in(axis, 0..=area[0]),
            view.get_complexity_in(axis, area[0]..=area[1]),
            view.get_complexity_in(axis, area[1]..),
        ]
    }

    pub fn get_complexity(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
    ) -> usize {
        let [pf, pm, pb] = match &primary.cdata {
            CompData::Single { view, .. } => get_area_complexity(view, surround, axis),
            _ => todo!(), // surround in
        };
        let secondary_len = secondary.get_complexity(axis);
        let interval_len: usize = intervals
            .hv_get(axis)
            .iter()
            .copied()
            .map(al::zero_one)
            .sum();

        pf + pb
            + if secondary_len + interval_len > pm {
                secondary_len + interval_len
            } else {
                pm
            }
    }

    fn get_area_assign(
        view: &StrucView,
        assigns: &Vec<AssignVal>,
        blank: &[AssignVal; 2],
        surround: &DataHV<Section>,
        axis: Axis,
    ) -> [AssignVal; 3] {
        let area = *view.surround_area(*surround).unwrap().hv_get(axis);
        let mut areas = [0..area[0], area[0]..area[1], area[1]..assigns.len()]
            .map(|range| assigns[range].iter().sum());

        if Section::End.eq(surround.hv_get(axis)) {
            areas[1] += blank[0];
        } else {
            areas[0] += blank[0];
        }
        if Section::Start.eq(surround.hv_get(axis)) {
            areas[1] += blank[1];
        } else {
            areas[2] += blank[1];
        }

        areas
    }

    pub fn get_area_assign_fb(
        primary: &StrucComb,
        surround: &DataHV<Section>,
    ) -> DataHV<[AssignVal; 3]> {
        match &primary.cdata {
            CompData::Single { view, assigns, .. } => Axis::hv().into_map(|axis| {
                surround::get_area_assign(
                    view,
                    assigns.hv_get(axis),
                    primary.blanks.hv_get(axis),
                    surround,
                    axis,
                )
            }),
            _ => todo!(), // surround in
        }
    }

    pub fn get_area_assign_all(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[AssignVal; 2]>,
        axis: Axis,
    ) -> [AssignVal; 3] {
        let [pf, pm, pb] = surround::get_area_assign_fb(primary, surround).hv_get_into(axis);
        let secondary_len = secondary.get_assign_value(axis, true);
        let interval_len: AssignVal = intervals.hv_get(axis).iter().sum();

        let s_len = interval_len + secondary_len;
        if pm.base > s_len.base {
            [pf, pm, pb]
        } else {
            [pf, s_len, pb]
        }
    }

    pub fn get_original_assign_value(
        view: &StrucView,
        proto: &StrucProto,
        assigns: &DataHV<Vec<AssignVal>>,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        intervals_val: &DataHV<[AssignVal; 2]>,
        blanks: &DataHV<[AssignVal; 2]>,
    ) -> DataHV<Vec<AssignVal>> {
        let area = view.surround_area(*surround).unwrap();
        let allocs = proto.allocation_space();

        Axis::hv().into_map(|axis| {
            let area = area.hv_get(axis);
            let allocs = allocs.hv_get(axis);
            let blanks = blanks.hv_get(axis);

            let surr_b_len = secondary.get_bases_length(axis, true)
                + intervals.hv_get(axis).iter().sum::<usize>();
            let surr_len =
                secondary.get_assign_value(axis, true) + intervals_val.hv_get(axis).iter().sum();
            let p_area_len = allocs[area[0]..area[1]]
                .iter()
                .chain(blanks.iter().map(|v| match v.base {
                    0.0 => &0,
                    _ => &1,
                }))
                .sum::<usize>();

            let extra = if p_area_len >= surr_b_len {
                0.0
            } else {
                let extra_b = surr_b_len - p_area_len;
                let extra = surr_len.total() / surr_b_len as f32 * extra_b as f32;
                0.0_f32.max(extra - blanks.iter().sum::<AssignVal>().total())
            };

            let mut assigns = assigns.hv_get(axis).clone();
            let area_total = assigns[area[0]..area[1]].iter().sum::<AssignVal>().total();

            al::reassign(assigns[area[0]..area[1]].iter_mut(), area_total - extra).unwrap();
            assigns
        })
    }

    pub fn get_assign_weights(
        proto: &StrucProto,
        assigns: &DataHV<Vec<AssignVal>>,
        original: &DataHV<Vec<AssignVal>>,
    ) -> DataHV<Vec<f32>> {
        Axis::hv().into_map(|axis| {
            let mut size = DataHV::default();
            *size.hv_get_mut(axis) = assigns.hv_get(axis).iter().map(|v| v.total()).sum::<f32>();
            *size.hv_get_mut(axis.inverse()) = original
                .hv_get(axis.inverse())
                .iter()
                .map(|v| v.total())
                .sum::<f32>();
            proto.subarea_weight(size).hv_get_into(axis)
        })
    }

    pub fn get_assign_value(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[AssignVal; 2]>,
        axis: Axis,
    ) -> AssignVal {
        get_area_assign_all(primary, secondary, surround, intervals, axis)
            .into_iter()
            .sum()
    }

    pub fn get_primary_area_edge(
        primary: &StrucComb,
        surround: &DataHV<Section>,
        axis: Axis,
        from: Side,
    ) -> (Option<view::Edge>, view::Edge, Option<view::Edge>) {
        let mut edges = match &primary.cdata {
            CompData::Single { view, .. } => {
                let surr_area = view
                    .surround_area(*surround)
                    .unwrap()
                    .hv_get_into(axis.inverse());
                let surround = *surround.hv_get(axis.inverse());

                let edge1 = if surround != Section::End {
                    Some(view.get_edge_range(axis, from, ..=surr_area[0]))
                } else {
                    None
                };
                let edge2 = view.get_edge_range(axis, from, surr_area[0]..=surr_area[1]);
                let edge3 = if surround != Section::Start {
                    Some(view.get_edge_range(axis, from, surr_area[1]..))
                } else {
                    None
                };

                (edge1, edge2, edge3)
            }
            _ => todo!(), // surround in
        };

        if primary.blanks.hv_get(axis)[from.n()].base != 0.0 {
            [edges.0.as_mut(), edges.2.as_mut()]
                .iter_mut()
                .for_each(|edge| {
                    if edge.is_some() {
                        edge.as_mut().unwrap().backspace();
                    }
                });
            edges.1.backspace();
        }
        if primary.blanks.hv_get(axis.inverse())[0].base != 0.0 {
            if let Some(edge) = edges.0.as_mut() {
                edge.add_head()
            };
        }
        if primary.blanks.hv_get(axis.inverse())[1].base != 0.0 {
            if let Some(edge) = edges.2.as_mut() {
                edge.add()
            };
        }
        edges
    }

    pub fn get_surround_inside_edge(
        primary: &StrucComb,
        surround: &DataHV<Section>,
    ) -> DataHV<[Option<view::Edge>; 2]> {
        match &primary.cdata {
            CompData::Single { view, .. } => {
                let surr_area = view.surround_area(*surround).unwrap();
                Axis::hv().into_map(|axis| {
                    Side::fb().map(|side| {
                        let surround = *surround.hv_get(axis);
                        if surround != side.inverse().to_section() {
                            let cross_area = surr_area.hv_get(axis.inverse());
                            let mut edge = view.get_edge_in(
                                axis,
                                side.inverse(),
                                surr_area.hv_get(axis)[side.n()],
                                cross_area[0]..=cross_area[1],
                            );

                            if primary.blanks.hv_get(axis.inverse())[0].base != 0.0 {
                                edge.add_head();
                            }
                            if primary.blanks.hv_get(axis.inverse())[1].base != 0.0 {
                                edge.add();
                            }
                            Some(edge)
                        } else {
                            None
                        }
                    })
                })
            }
            _ => todo!(), // surround in
        }
    }

    pub fn get_edge(
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
        from: Side,
    ) -> view::Edge {
        fn connect(
            edge1: Option<view::Edge>,
            edge2: view::Edge,
            edge3: Option<view::Edge>,
            intervals: &[usize; 2],
        ) -> view::Edge {
            let mut edge = if let Some(mut surr_edge) = edge1 {
                surr_edge.connect(edge2, intervals[0]);
                surr_edge
            } else {
                edge2
            };
            if let Some(surr_edge) = edge3 {
                edge.connect(surr_edge, intervals[1]);
            }

            edge
        }

        let c_surr = *surround.hv_get(axis);
        let from_section = from.to_section();

        if c_surr != from_section.inverse() {
            if intervals.hv_get(axis)[from.n()] == 0 {
                if get_area_base_len(primary, surround, axis)[from_section.n()] == 0 {
                    let (edge1, m_edge, edge3) =
                        get_primary_area_edge(primary, surround, axis, from);
                    if !m_edge.contain_black() {
                        return connect(
                            edge1,
                            secondary.get_edge(axis, from, true),
                            edge3,
                            intervals.hv_get(axis.inverse()),
                        );
                    }
                }
            }

            match &primary.cdata {
                CompData::Single { .. } => {
                    let mut edge = primary.get_edge(axis, from, true);
                    let s_len = intervals.hv_get(axis.inverse()).iter().sum::<usize>()
                        + secondary.get_bases_length(axis, true);
                    let [p1, p_len, _] = get_area_base_len(primary, surround, axis.inverse());
                    if p_len < s_len {
                        edge.add_in(p1 + 1, s_len - p_len); // TODO average
                    }

                    edge
                }
                _ => todo!(), // surround in
            }
        } else {
            let (edge1, _, edge3) = get_primary_area_edge(primary, surround, axis, from);
            connect(
                edge1,
                secondary.get_edge(axis, from, true),
                edge3,
                intervals.hv_get(axis.inverse()),
            )
        }
    }

    pub fn axis_main_comp<'a, F>(
        primary: &'a mut StrucComb,
        secondary: &'a mut StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
        mut f: F,
    ) where
        F: FnMut(Vec<(&'a mut StrucComb, bool)>, bool), // bool = all
    {
        let [pf, pm, pb] = surround::get_area_base_len(primary, surround, axis);
        let p_len = primary.get_bases_length(axis, true);

        let secondary_len = secondary.get_bases_length(axis, true);
        let interval_len: usize = intervals.hv_get(axis).iter().sum();

        let p_blank = pm >= secondary_len + interval_len;

        match (secondary_len + interval_len + pf + pb).cmp(&p_len) {
            std::cmp::Ordering::Equal => f(vec![(primary, p_blank), (secondary, true)], true),
            std::cmp::Ordering::Less => f(vec![(primary, p_blank)], false),
            std::cmp::Ordering::Greater => f(vec![(secondary, true), (primary, p_blank)], false),
        }
    }

    pub fn reduce_space(
        primary: &mut StrucComb,
        secondary: &mut StrucComb,
        surround: &DataHV<Section>,
        intervals: &DataHV<[usize; 2]>,
        axis: Axis,
        is_check: bool,
    ) -> bool {
        let mut ok = false;
        axis_main_comp(
            primary,
            secondary,
            surround,
            intervals,
            axis,
            |mut list, all| {
                if list.len() == 1 {
                    let blank = list[0].1;
                    ok = list[0].0.reduce_space(axis, is_check, blank)
                } else if all {
                    let mut sub_is_check = true;
                    ok = true;

                    for _ in 0..2 {
                        ok = list.iter_mut().fold(true, |b, (c, blank)| {
                            b & c.reduce_space(axis, sub_is_check, *blank)
                        });
                        if ok && !is_check {
                            sub_is_check = false;
                        } else {
                            break;
                        }
                    }
                } else {
                    for (c, blank) in list {
                        if c.reduce_space(axis, is_check, blank) {
                            ok = true;
                            break;
                        }
                    }
                }
            },
        );
        ok
    }

    pub fn reassign_space(
        primary: &mut StrucComb,
        secondary: &mut StrucComb,
        surround: &DataHV<Section>,
        _intervals: &DataHV<[usize; 2]>,
        intervals_val: &mut DataHV<[AssignVal; 2]>,
        new_val: f32,
        axis: Axis,
        blank: bool,
    ) {
        let mut surr_area =
            get_area_assign_all(primary, secondary, surround, intervals_val, axis)[1];
        match &mut primary.cdata {
            CompData::Single { view, assigns, .. } => {
                let area = view.surround_area(*surround).unwrap().hv_get_into(axis);
                let assigns = assigns.hv_get_mut(axis);
                let blanks = primary.blanks.hv_get_mut(axis);
                let mut s_assign = secondary.get_assign_value(axis, true);

                let range = area[0]..area[1];
                al::reassign(
                    assigns
                        .iter_mut()
                        .enumerate()
                        .filter_map(|(i, v)| match range.contains(&i) {
                            true => None,
                            false => Some(v),
                        })
                        .chain(std::iter::once(&mut surr_area)),
                    new_val,
                )
                .unwrap();

                let p_iter = assigns[range.clone()].iter_mut().chain(blanks.iter_mut());
                let s_iter = intervals_val
                    .hv_get_mut(axis)
                    .iter_mut()
                    .chain(std::iter::once(&mut s_assign));
                let new_val = surr_area.total();
                if blank {
                    al::reassign(p_iter, new_val).unwrap();
                    al::reassign(s_iter, new_val).unwrap();
                } else {
                    let mut change = vec![true; range.len()];
                    change.extend([false; 2].into_iter());
                    al::reassign_in(p_iter, &change, new_val).unwrap();

                    al::reassign_in(s_iter, &[false, false, true], new_val).unwrap();
                }

                secondary.reassign_space(s_assign.total(), blank, axis);
            }
            _ => todo!(), // surround in
        };
    }

    pub fn for_each_primary<F>(primary: &mut StrucComb, surround: &DataHV<Section>, mut f: F)
    where
        F: FnMut(&mut StrucComb),
    {
        f(primary);
        match &mut primary.cdata {
            CompData::Single { .. } => {}
            CompData::Scale { axis, comps, .. } => match surround.hv_get_into(*axis) {
                Section::End => for_each_primary(&mut comps[0], surround, f),
                Section::Start => for_each_primary(comps.last_mut().unwrap(), surround, f),
                Section::Middle if comps.len() == 3 => for_each_primary(&mut comps[1], surround, f),
                _ => panic!(),
            },
            _ => todo!(), // surround in
        };
    }

    pub fn for_each_primary_rev<F>(primary: &mut StrucComb, surround: &DataHV<Section>, mut f: F)
    where
        F: FnMut(&mut StrucComb),
    {
        match &mut primary.cdata {
            CompData::Single { .. } => {}
            CompData::Scale { axis, comps, .. } => match surround.hv_get_into(*axis) {
                Section::End => for_each_primary(&mut comps[0], surround, &mut f),
                Section::Start => for_each_primary(comps.last_mut().unwrap(), surround, &mut f),
                Section::Middle if comps.len() == 3 => {
                    for_each_primary(&mut comps[1], surround, &mut f)
                }
                _ => panic!(),
            },
            _ => todo!(), // surround in
        };
        f(primary);
    }
}

pub struct SurroundPrimeryIter<'a> {
    comb: &'a StrucComb,
    _surround: DataHV<Section>,
    record: usize,
}

impl<'a> SurroundPrimeryIter<'a> {
    pub fn new(comb: &'a StrucComb, surround: DataHV<Section>) -> Self {
        Self {
            comb,
            _surround: surround,
            record: 0,
        }
    }
}

impl<'a> std::iter::Iterator for SurroundPrimeryIter<'a> {
    type Item = &'a StrucComb;

    fn next(&mut self) -> Option<Self::Item> {
        fn recursion<'a>(
            comb: &'a StrucComb,
            record: usize,
            pointer: usize,
        ) -> Option<&'a StrucComb> {
            if pointer == record {
                Some(comb)
            } else {
                match &comb.cdata {
                    CompData::Single { .. } => None,
                    _ => todo!(), // surround in
                }
            }
        }

        self.record += 1;
        recursion(self.comb, self.record, 1)
    }
}

// pub struct SurroundPrimeryIterMut<'a> {
//     comb: Option<*mut StrucComb>,
//     surround: DataHV<Section>,
//     _marker: std::marker::PhantomData<&'a mut StrucComb>,
// }

// impl<'a> SurroundPrimeryIterMut<'a> {
//     pub fn new(comb: &'a mut StrucComb, surround: DataHV<Section>) -> Self {
//         Self {
//             comb: Some(comb),
//             surround,
//             _marker: std::marker::PhantomData,
//         }
//     }
// }

// impl<'a> std::iter::Iterator for SurroundPrimeryIterMut<'a> {
//     type Item = &'a mut StrucComb;

//     fn next(&mut self) -> Option<Self::Item> {
//         let current_ptr = self.comb?;

//         unsafe {
//             let node = &mut *current_ptr; // 临时创建可变引用

//             self.comb = match &mut node.cdata {
//                 CompData::Single { .. } => None,
//                 CompData::Scale { axis, comps, .. } => match self.surround.hv_get_into(*axis) {
//                     Section::End => Some(&mut comps[0]),
//                     Section::Start => Some(comps.last_mut().unwrap()),
//                     Section::Middle if comps.len() == 3 => Some(&mut comps[1]),
//                     _ => panic!(),
//                 },
//                 _ => None,
//             };

//             Some(node)
//         }
//     }
// }

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
            CstType::Surround(surround) => CompData::Surround {
                surround,
                comps,
                intervals: Default::default(),
                intervals_val: Default::default(),
            },
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

    pub fn surround_primary_iter(&self, surround: DataHV<Section>) -> SurroundPrimeryIter<'_> {
        SurroundPrimeryIter::new(self, surround)
    }

    // pub fn surround_primary_iter_mut(
    //     &mut self,
    //     surround: DataHV<Section>,
    // ) -> SurroundPrimeryIterMut<'_> {
    //     SurroundPrimeryIterMut::new(self, surround)
    // }

    pub fn get_comb_name(&self) -> String {
        match &self.cdata {
            CompData::Single { .. } => self.name.clone(),
            CompData::Scale { axis, comps, .. } => {
                format!(
                    "{}({})",
                    CstType::Scale(*axis).symbol(),
                    comps
                        .iter()
                        .map(|c| c.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            CompData::Surround {
                surround, comps, ..
            } => {
                format!(
                    "{}({}, {})",
                    CstType::Surround(*surround).symbol(),
                    comps[0].name,
                    comps[1].name
                )
            }
        }
    }

    pub fn get_char_box(&self) -> WorkBox {
        match &self.cdata {
            CompData::Single { proto, .. } => proto.attrs.get::<attrs::CharBox>(),
            _ => None,
        }
        .unwrap_or(WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0)))
    }

    pub fn get_surr_interval_setting(
        &self,
    ) -> BTreeMap<String, BTreeMap<Axis, BTreeMap<Side, Vec<attrs::IntervalAlloc>>>> {
        match &self.cdata {
            CompData::Single { proto, .. } => proto
                .attrs
                .get::<attrs::IntervalSurrAlloc>()
                .unwrap_or_default(),
            _ => Default::default(),
        }
    }

    pub fn get_trend_setting(&self) -> Option<Vec<EdgeCheck<[Option<String>; 2]>>> {
        match &self.cdata {
            CompData::Single { proto, .. } => proto.attrs.get::<attrs::TrendSetting>(),
            _ => None,
        }
    }

    pub fn get_edge_area_setting(&self) -> Option<Vec<EdgeCheck<[Option<String>; 2]>>> {
        match &self.cdata {
            CompData::Single { proto, .. } => proto.attrs.get::<attrs::EdgeAreaSetting>(),
            _ => None,
        }
    }

    pub fn get_edge_main_comb(&self, axis: Axis, side: Side) -> Option<&Self> {
        match &self.cdata {
            CompData::Single { .. } => Some(self),
            CompData::Scale {
                axis: c_axis,
                comps,
                ..
            } => {
                if axis == *c_axis {
                    let idx = [0, comps.len() - 1][side.n()];
                    comps[idx].get_edge_main_comb(axis, side)
                } else {
                    None
                }
            }
            CompData::Surround {
                surround, comps, ..
            } => {
                if *surround.hv_get(axis) != side.inverse().to_section() {
                    comps[0].get_edge_main_comb(axis, side)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_edge_main_comb_mut<'a>(
        &'a mut self,
        axis: Axis,
        side: Side,
    ) -> Option<&'a mut Self> {
        match &self.cdata {
            CompData::Single { .. } => Some(self),
            _ => match &mut self.cdata {
                CompData::Single { .. } => unreachable!(),
                CompData::Scale {
                    axis: c_axis,
                    comps,
                    ..
                } => {
                    if axis == *c_axis {
                        let idx = [0, comps.len() - 1][side.n()];
                        comps[idx].get_edge_main_comb_mut(axis, side)
                    } else {
                        None
                    }
                }
                CompData::Surround {
                    surround, comps, ..
                } => {
                    if *surround.hv_get(axis) != side.inverse().to_section() {
                        comps[0].get_edge_main_comb_mut(axis, side)
                    } else {
                        None
                    }
                }
            },
        }
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
            CompData::Surround {
                surround,
                comps,
                intervals,
                ..
            } => surround::get_bases_length(&comps[0], &comps[1], surround, intervals, axis),
        };

        if blank {
            len + self.get_blank_base(axis).iter().sum::<usize>()
        } else {
            len
        }
    }

    pub fn get_complexity(&self, axis: Axis) -> usize {
        let len = match &self.cdata {
            // CompData::Single { view, .. } => view.get_complexity(axis),
            CompData::Single { view, .. } => view.get_struc_complexity(axis),
            CompData::Scale {
                axis: c_axis,
                comps,
                ..
            } => {
                if axis == *c_axis {
                    comps.iter().map(|c| c.get_complexity(axis)).sum::<usize>()
                } else {
                    comps.iter().map(|c| c.get_complexity(axis)).max().unwrap()
                }
            }
            CompData::Surround {
                surround,
                comps,
                intervals,
                ..
            } => surround::get_complexity(&comps[0], &comps[1], surround, intervals, axis),
        };

        len
    }

    pub fn get_line_weight(&self) -> DataHV<f32> {
        match &self.cdata {
            CompData::Single { proto, assigns, .. } => {
                let assign_vals = if assigns.map(|list| list.is_empty()).iter().all(|b| *b) {
                    proto
                        .allocation_space()
                        .into_map(|list| list.into_iter().map(|v| v as f32).collect())
                } else {
                    assigns.map(|list| list.iter().map(|v| v.total()).collect())
                };
                proto
                    .subarea_line_weight(&assign_vals, DataHV::splat(0.0))
                    .into_map(|list| list.iter().sum())
            }
            CompData::Scale { comps, .. } | CompData::Surround { comps, .. } => comps
                .iter()
                .map(|c| c.get_line_weight())
                .reduce(|a, b| a.zip(b).into_map(|(a, b)| a + b))
                .unwrap(),
        }
    }

    pub fn get_assign_value(&self, axis: Axis, blank: bool) -> AssignVal {
        let val = match &self.cdata {
            CompData::Single { assigns, .. } => assigns.hv_get(axis).iter().sum(),
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
                        .sum()
                } else {
                    comps
                        .iter()
                        .map(|c| c.get_assign_value(axis, true))
                        .max_by(|a, b| a.base.partial_cmp(&b.base).unwrap())
                        .unwrap()
                }
            }
            CompData::Surround {
                surround,
                comps,
                intervals_val,
                ..
            } => surround::get_assign_value(&comps[0], &comps[1], surround, intervals_val, axis),
        };

        if blank {
            val + self.blanks.hv_get(axis).iter().sum()
        } else {
            val
        }
    }

    pub fn get_length_limit(&self, size: DataHV<f32>, axis: Axis) -> Option<f32> {
        if let CompData::Single { proto, .. } = &self.cdata {
            if let Some(limit) = proto.attrs.get::<attrs::LengthLimit>() {
                return limit.limit(size, axis);
            }
        }

        None
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
        let blanks = self
            .blanks
            .map(|data: &[AssignVal; 2]| data.map(assign_to_f32));
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
            CompData::Surround {
                surround,
                comps,
                intervals,
                intervals_val,
                ..
            } => CompInfoData::Surround {
                surround: *surround,
                comps: comps.iter().map(|c| c.get_comb_info()).collect(),
                intervals: intervals.clone(),
                intervals_val: intervals_val.map(|list| list.map(|v| v.total())),
                base_size: Axis::hv().into_map(|axis| {
                    surround::get_bases_length(&comps[0], &comps[1], surround, intervals, axis)
                }),
            },
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
            CompData::Surround {
                surround,
                comps,
                intervals_val,
                ..
            } => {
                let (primary_tree, c_size) = comps[0].get_paths_in(new_start);
                size = c_size;

                let surr_advances = surround::get_area_assign_fb(&comps[0], surround)
                    .into_map(|arrary| arrary[0].total());
                for axis in Axis::list() {
                    *new_start.hv_get_mut(axis) +=
                        *surr_advances.hv_get(axis) + intervals_val.hv_get(axis)[0].total();
                }
                let (secondary_tree, _) = comps[1].get_paths_in(new_start);

                CompTree {
                    name: self.name.clone(),
                    tp: CstType::Surround(*surround),
                    paths: Default::default(),
                    children: vec![primary_tree, secondary_tree],
                }
            }
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

    pub fn get_edge(&self, axis: Axis, from: Side, blank: bool) -> view::Edge {
        let mut edge = match &self.cdata {
            CompData::Single { view, .. } => view.get_edge(axis, from),
            CompData::Scale {
                axis: c_axis,
                comps,
                intervals,
                ..
            } => {
                if axis == *c_axis {
                    match from {
                        Side::Front => comps[0].get_edge(axis, from, true),
                        Side::Back => comps.last().unwrap().get_edge(axis, from, true),
                    }
                } else {
                    let mut interval = intervals.iter();
                    comps
                        .iter()
                        .map(|c| c.get_edge(axis, from, true))
                        .reduce(|mut e1, e2| {
                            let n = interval.next().copied().unwrap_or_default();
                            e1.connect(e2, n);
                            e1
                        })
                        .unwrap()
                }
            }
            CompData::Surround {
                surround,
                comps,
                intervals,
                ..
            } => surround::get_edge(&comps[0], &comps[1], surround, intervals, axis, from),
        };

        if blank {
            if self.blanks.hv_get(axis)[from.n()].base != 0.0 {
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

    pub fn reduce_space(&mut self, axis: Axis, is_check: bool, blank: bool) -> bool {
        let r = match &mut self.cdata {
            CompData::Single { proto, view, .. } => {
                if proto.reduce(axis, is_check) {
                    if !is_check {
                        *view = StrucView::new(proto);
                    }
                    true
                } else {
                    false
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
                    // let mut new_lengths = lenghts.clone();
                    // new_lengths.last_mut().unwrap().1 = usize::MAX;
                    // new_lengths.sort_by_key(|(_, c)| *c);
                    let mut orders: Vec<_> = comps
                        .iter()
                        .zip(lenghts.iter())
                        .map(|(c, &(i, b))| match b {
                            0 => (i, 0.0),
                            // _ => (i, c.get_line_weight().hv_get_into(axis) / b as f32), // 猕
                            _ => (i, c.get_complexity(axis) as f32 / b as f32),
                        })
                        .collect();
                    orders.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                    let mut r = false;
                    for (i, _) in orders {
                        if comps[i].reduce_space(axis, is_check, true) {
                            r = true;
                            break;
                        }
                    }
                    r
                } else {
                    let max_len = lenghts.iter().max_by_key(|(_, l)| *l).unwrap().1;
                    if lenghts
                        .iter()
                        .all(|&(i, l)| l != max_len || comps[i].reduce_space(axis, true, true))
                    {
                        if !is_check {
                            lenghts
                                .iter()
                                .filter(|(_, l)| *l == max_len)
                                .for_each(|&(i, _)| {
                                    comps[i].reduce_space(axis, false, true);
                                });
                        }
                        true
                    } else {
                        false
                    }
                }
            }
            CompData::Surround {
                comps,
                intervals,
                surround,
                ..
            } => {
                let (l, r) = comps.split_at_mut(1);
                surround::reduce_space(&mut l[0], &mut r[0], surround, intervals, axis, is_check)
            }
        };

        if r {
            if !is_check {
                let mut r_target = self.attrs.get::<attrs::ReduceTarget>().unwrap_or_default();
                *r_target.hv_get_mut(axis) = Some(self.get_bases_length(axis, true));
                self.attrs.set::<attrs::ReduceTarget>(&r_target);

                let mut main_comp = self.attrs.get::<attrs::MainComp>().unwrap_or_default();
                *main_comp.hv_get_mut(axis) = false;
                self.attrs.set::<attrs::MainComp>(&main_comp);
            }
        } else if blank {
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
        }
        r
    }

    pub fn reassign_space(&mut self, mut new_val: f32, blank: bool, axis: Axis) {
        let mut old_assigns = self.get_assign_value(axis, false);
        let b_assigns = self.blanks.hv_get_mut(axis);
        let assign_list = b_assigns
            .iter_mut()
            .chain(std::iter::once(&mut old_assigns));
        if blank {
            al::reassign(assign_list, new_val).unwrap();
        } else {
            al::reassign_in(assign_list, &[false, false, true], new_val).unwrap();
        }
        new_val = old_assigns.total();

        match &mut self.cdata {
            CompData::Single { assigns, .. } => {
                al::reassign(assigns.hv_get_mut(axis).iter_mut(), new_val).unwrap()
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
                    al::reassign(assigns_list.iter_mut(), new_val).unwrap();

                    let mut iter = assigns_list.into_iter();
                    comps
                        .iter_mut()
                        .for_each(|c| c.reassign_space(iter.next().unwrap().total(), blank, axis));
                    intervals_val
                        .iter_mut()
                        .for_each(|av| *av = iter.next().unwrap());
                } else {
                    comps
                        .iter_mut()
                        .for_each(|c| c.reassign_space(new_val, blank, axis));
                }
            }
            CompData::Surround {
                comps,
                intervals,
                intervals_val,
                surround,
                ..
            } => {
                let (l, r) = comps.split_at_mut(1);
                surround::reassign_space(
                    &mut l[0],
                    &mut r[0],
                    surround,
                    intervals,
                    intervals_val,
                    new_val,
                    axis,
                    blank,
                );
            }
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
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 1), key_pos(1, 2)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 2), key_pos(3, 2)]),
            KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
        ]);
        let comb = StrucComb::new_single("name".to_string(), struc);
        let sharpness = Axis::hv()
            .into_map(|axis| Side::fb().map(|side| comb.get_edge(axis, side, false).gray_scale()));
        assert_eq!(sharpness.h[0], 1.0 / 2.0);
        assert_eq!(sharpness.h[1], 0.0);
        assert_eq!(sharpness.v[0], 0.0);
        assert_eq!(sharpness.v[1], 1.0 / 3.0);

        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 0),
            key_pos(2, 0),
            key_pos(2, 2),
            key_pos(0, 2),
            key_pos(0, 0),
        ])]);
        let comb = StrucComb::new_single("name".to_string(), struc);
        let sharpness = Axis::hv()
            .into_map(|axis| Side::fb().map(|side| comb.get_edge(axis, side, false).gray_scale()));
        assert_eq!(sharpness.h[0], 1.0);
        assert_eq!(sharpness.h[1], 1.0);
        assert_eq!(sharpness.v[0], 1.0);
        assert_eq!(sharpness.v[1], 1.0);
    }

    #[test]
    fn test_comp_edge() {
        use view::{Direction, ViewElement};

        let surround = DataHV::new(Section::Middle, Section::Start);
        // 冂
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 2),
            key_pos(0, 0),
            key_pos(2, 0),
            key_pos(2, 2),
        ])]);
        let p_comb = StrucComb::new_single("冂".to_string(), struc);
        let (edge1, edge2, edge3) =
            surround::get_primary_area_edge(&p_comb, &surround, Axis::Vertical, Side::Front);
        assert_eq!(
            edge1.unwrap().get_line().clone(),
            vec![Some(vec![ViewElement::D(Direction::Below),])]
        );
        assert_eq!(
            edge2.get_line().clone(),
            vec![
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                Some(vec![
                    ViewElement::D(Direction::Left),
                    ViewElement::D(Direction::Below)
                ]),
            ]
        );
        assert_eq!(
            edge3.unwrap().get_line().clone(),
            vec![Some(vec![ViewElement::D(Direction::Below),])]
        );
        // 口
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 2),
            key_pos(0, 0),
            key_pos(2, 0),
            key_pos(2, 2),
            key_pos(0, 2),
        ])]);
        let s_comb = StrucComb::new_single("口".to_string(), struc);
        let comb = StrucComb {
            name: "冋".to_string(),
            blanks: DataHV::new(
                Default::default(),
                [Default::default(), AssignVal::new(1.0, 0.0)],
            ),
            cdata: CompData::Surround {
                surround,
                comps: vec![p_comb, s_comb],
                intervals: DataHV::new([1, 2], [2, 0]),
                intervals_val: Default::default(),
            },
            attrs: Default::default(),
        };
        let edge = comb.get_edge(Axis::Vertical, Side::Front, true);
        assert_eq!(
            edge.get_line().clone(),
            vec![
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                None,
                None,
                None,
                Some(vec![
                    ViewElement::D(Direction::Left),
                    ViewElement::D(Direction::Below)
                ]),
            ]
        );
        let edge = comb.get_edge(Axis::Vertical, Side::Back, false);
        assert_eq!(
            edge.get_line().clone(),
            vec![
                Some(vec![ViewElement::D(Direction::Above),]),
                Some(vec![
                    ViewElement::D(Direction::Above),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                Some(vec![
                    ViewElement::D(Direction::Above),
                    ViewElement::D(Direction::Left)
                ]),
                None,
                Some(vec![ViewElement::D(Direction::Above)]),
            ]
        );

        let edge = comb.get_edge(Axis::Horizontal, Side::Front, false);
        assert_eq!(
            edge.get_line().clone(),
            vec![
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                None,
                None,
                Some(vec![ViewElement::D(Direction::Above),]),
            ]
        );
    }

    #[test]
    fn test_scale_flattn() {
        let c1 = StrucComb::new_single("1".to_string(), Default::default());
        let c2 = StrucComb::new_single("2".to_string(), Default::default());
        let c3 = StrucComb::new_single("3".to_string(), Default::default());
        let c4 = StrucComb::new_single("4".to_string(), Default::default());
        let c5 = StrucComb::new_single("5".to_string(), Default::default());

        let ca = StrucComb {
            name: "a".to_string(),
            blanks: Default::default(),
            attrs: Default::default(),
            cdata: CompData::Scale {
                axis: Axis::Vertical,
                comps: vec![c1, c2],
                intervals: vec![5],
                intervals_val: vec![],
            },
        };
        let cb = StrucComb {
            name: "b".to_string(),
            blanks: Default::default(),
            attrs: Default::default(),
            cdata: CompData::Scale {
                axis: Axis::Vertical,
                comps: vec![c4, c5],
                intervals: vec![3],
                intervals_val: vec![],
            },
        };

        let mut comps = vec![ca, c3, cb];
        let mut intervals = vec![1, 1];
        let mut intervals_val = vec![];
        let records = scale::flatten(
            &mut comps,
            &mut intervals,
            &mut intervals_val,
            Axis::Vertical,
        );

        let names = comps.iter().map(|c| c.name.as_str()).collect::<Vec<&str>>();
        assert_eq!(names, vec!["1", "2", "3", "4", "5"]);
        assert_eq!(intervals, vec![5, 1, 1, 3]);
        assert!(intervals_val.is_empty());

        scale::restore(
            records,
            Axis::Vertical,
            &mut comps,
            &mut intervals,
            &mut intervals_val,
        );

        assert_eq!(comps[1].name.as_str(), "3");
        assert_eq!(intervals, vec![1, 1]);
        assert!(intervals_val.is_empty());

        assert_eq!(comps[0].name.as_str(), "a");
        match &comps[0].cdata {
            CompData::Scale {
                axis: Axis::Vertical,
                comps,
                intervals,
                ..
            } => {
                assert_eq!(comps[0].name.as_str(), "1");
                assert_eq!(comps[1].name.as_str(), "2");
                assert_eq!(*intervals, vec![5]);
            }
            _ => panic!(),
        };
        assert_eq!(comps[2].name.as_str(), "b");
        match &comps[2].cdata {
            CompData::Scale {
                axis: Axis::Vertical,
                comps,
                intervals,
                ..
            } => {
                assert_eq!(comps[0].name.as_str(), "4");
                assert_eq!(comps[1].name.as_str(), "5");
                assert_eq!(*intervals, vec![3]);
            }
            _ => panic!(),
        };
    }

    #[test]
    fn test_comb_edge() {
        let c1 = StrucComb::new_single(
            "人".to_string(),
            StrucProto::from(vec![
                KeyPath::from([key_pos(1, 0), key_pos(0, 2)]),
                KeyPath::from([key_pos(1, 0), key_pos(2, 2)]),
            ]),
        );
        let mut c2 = StrucComb::new_single(
            "口".to_string(),
            StrucProto::from(vec![KeyPath::from([
                key_pos(0, 0),
                key_pos(2, 0),
                key_pos(2, 2),
                key_pos(0, 2),
                key_pos(0, 0),
                KeyPoint {
                    pos: IndexPoint::new(2, 2),
                    labels: vec!["mark".to_string()],
                },
            ])]),
        );
        c2.blanks.h = [AssignVal::new(1.0, 0.0); 2];
        let c3 = StrucComb::new_single(
            "㇆".to_string(),
            StrucProto::from(vec![KeyPath::from([
                key_pos(0, 0),
                key_pos(2, 0),
                key_pos(2, 5),
                KeyPoint {
                    pos: IndexPoint::new(1, 5),
                    labels: vec!["mark".to_string()],
                },
            ])]),
        );

        let comb = StrucComb {
            name: "a".to_string(),
            blanks: Default::default(),
            attrs: Default::default(),
            cdata: CompData::Scale {
                axis: Axis::Vertical,
                comps: vec![c1, c2, c3],
                intervals: vec![2, 2],
                intervals_val: vec![],
            },
        };

        let edge = comb.get_edge(Axis::Horizontal, Side::Back, true);
        assert_eq!(edge.get_line().len(), 14);
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [1, 0]);
        assert_eq!(
            shape.trend,
            [view::ShapeTrend::Acute, view::ShapeTrend::None]
        );
        assert_eq!(shape.middle, view::ShapeState::Breach);
    }
}
