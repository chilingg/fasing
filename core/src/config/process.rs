use crate::{
    algorithm as al,
    axis::*,
    component::{
        attrs,
        comb::{AssignVal, StrucComb},
        struc::StrucProto,
        view::StrucView,
    },
    construct::CstType,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Operation<O, E> {
    pub operation: O,
    pub execution: E,
}

impl<O, E> Operation<O, E> {
    pub fn new(operation: O, execution: E) -> Self {
        Self {
            operation,
            execution,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SpaceProcess {
    Center(DataHV<Operation<f32, f32>>),
    CompCenter(DataHV<Operation<f32, f32>>),
    CenterArea(DataHV<Operation<f32, f32>>),
    EdgeShrink(DataHV<[bool; 2]>),
}

impl SpaceProcess {
    pub fn process_space(&self, comb: &mut StrucComb, cfg: &super::Config) {
        match self {
            SpaceProcess::EdgeShrink(setting) => {
                fn process(
                    assign_vals: &mut DataHV<Vec<AssignVal>>,
                    proto: &StrucProto,
                    view: &StrucView,
                    surround: Option<DataHV<Place>>,
                    setting: &DataHV<[bool; 2]>,
                ) -> DataHV<bool> {
                    let states = proto
                        .attrs
                        .get::<attrs::EdgeShrink>()
                        .unwrap_or(DataHV::splat([true; 2]));

                    let allocs = proto.allocation_space();
                    let surround = surround.unwrap_or(DataHV::splat(Place::Middle));

                    Axis::hv().into_map(|axis| {
                        let shrink = setting.hv_get(axis);
                        let allocs = allocs.hv_get(axis);
                        let assign = assign_vals.hv_get_mut(axis);

                        if allocs.len() > shrink.iter().filter(|b| **b).count() {
                            let targets = [0, 1].map(|mut i| {
                                let mut idx = None;
                                if shrink[i] && states.hv_get(axis)[i] {
                                    let place = if i == 0 {
                                        Place::Start
                                    } else {
                                        i = allocs.len() - 1;
                                        Place::End
                                    };

                                    if *surround.hv_get(axis) != place.inverse() && allocs[i] == 1 {
                                        let edge = view.read_lines(axis, place).to_edge();
                                        if edge.faces.iter().find(|f| **f == 1.0).is_none()
                                            && edge.dots.iter().filter(|d| **d).count() < 4
                                        {
                                            idx = Some(i)
                                        }
                                    }
                                }
                                idx
                            });

                            let excess = targets.iter().fold(0.0, |mut e, t| {
                                if let Some(i) = *t {
                                    e += assign[i].excess;
                                    assign[i].excess = 0.0;
                                }
                                e
                            });
                            let range = targets[0].map(|i| i + 1).unwrap_or(0)
                                ..=targets[1].map(|i| i - 1).unwrap_or(allocs.len() - 1);
                            let total = assign[range.clone()].iter().sum::<AssignVal>().total();
                            assign[range]
                                .iter_mut()
                                .for_each(|v| v.excess += v.total() / total * excess);

                            excess != 0.0
                        } else {
                            false
                        }
                    })
                }

                match comb {
                    StrucComb::Single {
                        assign_vals,
                        proto,
                        view,
                        ..
                    } => {
                        process(assign_vals, proto, view, None, setting);
                    }
                    StrucComb::Complex {
                        combs,
                        tp,
                        offsets,
                        intervals,
                        ..
                    } => match tp {
                        CstType::Scale(_) => {
                            combs.iter_mut().for_each(|c| self.process_space(c, cfg))
                        }
                        CstType::Surround(surround) => {
                            let s_ofs = combs[1].get_offsets();

                            if let StrucComb::Single {
                                assign_vals,
                                view,
                                proto,
                                offsets: p_ofs,
                                ..
                            } = &mut combs[0]
                            {
                                let mut s_assign = DataHV::splat(None);
                                let mut s_offsets = DataHV::splat([None; 2]);

                                let area = view.surround_area(*surround).unwrap();
                                let ok =
                                    process(assign_vals, proto, view, Some(*surround), setting);
                                for axis in Axis::list() {
                                    if *ok.hv_get(axis) {
                                        let area =
                                            proto.value_index_in_axis(area.hv_get(axis), axis);
                                        let assign_vals = assign_vals.hv_get(axis);
                                        let i_ofs = match axis {
                                            Axis::Horizontal => 0,
                                            Axis::Vertical => 2,
                                        };

                                        let mut surr_assign = assign_vals[area[0]..area[1]]
                                            .iter()
                                            .sum::<AssignVal>()
                                            .total()
                                            - intervals[i_ofs..i_ofs + 2]
                                                .iter()
                                                .sum::<AssignVal>()
                                                .total()
                                            + offsets
                                                .hv_get(axis)
                                                .iter()
                                                .zip(p_ofs.hv_get(axis).iter())
                                                .map(|(a, b)| b.total() - a.total())
                                                .sum::<f32>();
                                        for i in 0..2 {
                                            let place = match i {
                                                0 => Place::End,
                                                _ => Place::Start,
                                            };
                                            if *surround.hv_get(axis) == place {
                                                surr_assign -= s_ofs.hv_get(axis)[i].total()
                                                    - offsets.hv_get(axis)[i].total();
                                            } else {
                                                let ofs = match i {
                                                    0 => assign_vals[..area[0]]
                                                        .iter()
                                                        .sum::<AssignVal>()
                                                        .total(),
                                                    _ => assign_vals[area[1]..]
                                                        .iter()
                                                        .sum::<AssignVal>()
                                                        .total(),
                                                };
                                                s_offsets.hv_get_mut(axis)[i] =
                                                    Some(AssignVal::new(
                                                        offsets.hv_get(axis)[i].total()
                                                            + ofs
                                                            + intervals[i + i_ofs].base
                                                            + intervals[i + i_ofs].excess / 2.0,
                                                        intervals[i + i_ofs].excess / 2.0,
                                                    ))
                                            }
                                        }
                                        *s_assign.hv_get_mut(axis) = Some(surr_assign);
                                    }
                                }
                                combs[1].scale_space(s_assign, s_offsets);
                            }

                            self.process_space(&mut combs[1], cfg);
                        }
                        CstType::Single => unreachable!(),
                    },
                }
            }
            SpaceProcess::Center(operation) => match comb {
                StrucComb::Single { .. } => {
                    let center = comb.get_visual_center(al::NORMAL_OFFSET, cfg.strok_width);
                    if let StrucComb::Single { assign_vals, .. } = comb {
                        for axis in Axis::list() {
                            let assign_vals = assign_vals.hv_get_mut(axis);
                            let center_opt = operation.hv_get(axis);
                            let new_vals = al::center_correction(
                                &assign_vals.iter().map(|av| av.total()).collect(),
                                &assign_vals.iter().map(|av| av.base).collect(),
                                *center.hv_get(axis),
                                center_opt.operation,
                                center_opt.execution,
                            );
                            new_vals.into_iter().zip(assign_vals.iter_mut()).for_each(
                                |(nval, aval)| {
                                    aval.excess = nval;
                                },
                            );
                        }
                    }
                }
                StrucComb::Complex {
                    combs,
                    tp,
                    intervals_alloc,
                    ..
                } => match tp {
                    CstType::Surround(_) => self.process_space(&mut combs[1], cfg),
                    _ => combs.iter_mut().enumerate().for_each(|(i, c)| {
                        if intervals_alloc[i.checked_sub(1).unwrap_or(0)] != 0
                            && intervals_alloc.get(i).map(|v| *v != 0).unwrap_or(true)
                        {
                            self.process_space(c, cfg)
                        }
                    }),
                },
            },
            SpaceProcess::CompCenter(operation) => match comb {
                StrucComb::Complex {
                    tp,
                    intervals,
                    combs,
                    edge_main,
                    offsets: ofs,
                    ..
                } => match *tp {
                    CstType::Scale(comb_axis) => {
                        combs.iter_mut().for_each(|c| self.process_space(c, cfg));

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

                            let center =
                                al::visual_center_length(paths, al::NORMAL_OFFSET, cfg.strok_width);
                            let center_opt = operation.hv_get(comb_axis);
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

                                combs[j].scale_space(assign, offsets);
                            }
                        }
                    }
                    CstType::Surround(surround) => {
                        self.process_space(&mut combs[1], cfg);

                        let mut paths = vec![];
                        combs.iter().for_each(|c| {
                            c.merge_to(Default::default(), &mut paths);
                        });
                        let center =
                            al::visual_center_length(paths, al::NORMAL_OFFSET, cfg.strok_width);
                        let secondary_len =
                            Axis::hv().into_map(|axis| combs[1].get_assign_length(axis));

                        let mut s_offsets = combs[1].get_offsets();
                        let s_assign = Axis::hv().into_map(|axis| {
                            if let StrucComb::Single {
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

                                let center_opt = operation.hv_get(axis);
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
                                }

                                let edge_main = edge_main.hv_get(axis);
                                for i in 0..2 {
                                    let (place, range) = match i {
                                        0 => (Place::End, 0..area_idx[0]),
                                        _ => (
                                            Place::Start,
                                            area_idx[1]..assign_vals.hv_get(axis).len(),
                                        ),
                                    };

                                    let s_white =
                                        if edge_main.get(&place).map(|ep| ep[1]).unwrap_or(true) {
                                            AssignVal::default()
                                        } else {
                                            let mut c_ofs = s_offsets.hv_get(axis)[i];
                                            c_ofs.base -= ofs.hv_get(axis)[i].total();
                                            c_ofs.excess =
                                                (c_ofs.total() * scale - c_ofs.base).max(0.0);
                                            c_ofs
                                        };

                                    if *surround.hv_get(axis) == place {
                                        s_offsets.hv_get_mut(axis)[i] =
                                            s_white + s_offsets.hv_get(axis)[i];
                                    } else {
                                        s_offsets.hv_get_mut(axis)[i] = AssignVal::new(
                                            assign_vals.hv_get(axis)[range]
                                                .iter()
                                                .sum::<AssignVal>()
                                                .total()
                                                + ofs.hv_get(axis)[i].total()
                                                + intervals[i + i_ofs].base
                                                + intervals[i + i_ofs].excess / 2.0,
                                            intervals[i + i_ofs].excess / 2.0,
                                        );
                                    }

                                    new_surr -= intervals[i + i_ofs].total() + s_white.total();
                                }
                                Some(new_surr)
                            } else {
                                panic!()
                            }
                        });

                        combs[1]
                            .scale_space(s_assign, s_offsets.into_map(|ofs| ofs.map(|v| Some(v))));
                    }
                    CstType::Single => unreachable!(),
                },
                StrucComb::Single { .. } => {}
            },
            _ => {}
        }
    }
}
