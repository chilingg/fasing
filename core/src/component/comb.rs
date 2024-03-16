use crate::{
    algorithm,
    axis::*,
    component::{
        attrs,
        struc::*,
        view::{Edge, StrucView},
    },
    construct::{space::*, Type},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Default, Serialize)]
pub struct TransformValue {
    pub allocs: Vec<usize>,
    pub bases: Vec<f32>,
    pub allowances: Vec<f32>,
    pub offset: [f32; 2],
}

impl TransformValue {
    pub fn length(&self) -> f32 {
        self.bases
            .iter()
            .chain(self.allowances.iter())
            .chain(self.offset.iter())
            .sum()
    }

    pub fn allowance_length(&self) -> f32 {
        self.allowances.iter().sum()
    }

    pub fn assigns(&self) -> Vec<f32> {
        self.bases
            .iter()
            .zip(self.allowances.iter())
            .map(|(&b, &a)| a + b)
            .collect()
    }

    pub fn assigns_length(&self) -> f32 {
        self.assigns().iter().sum()
    }
}

#[derive(Default)]
pub struct SurroundValue {
    pub p_allocs1: Vec<usize>,
    pub sub_area: Vec<usize>,
    pub p_allocs2: Vec<usize>,

    pub interval_info: [Option<(i32, String, String)>; 2],

    pub s_base_len: usize,
    pub s_val: usize,

    pub p_edge_key: bool,
    pub p_edge: BTreeMap<Place, ([Option<Edge>; 2], f32)>,
    pub s_edge_key: bool,
    pub s_edge: BTreeMap<Place, (Edge, f32)>,
}

#[derive(Clone)]
pub enum StrucComb {
    Single {
        name: String,
        proto: StrucProto,
        view: StrucView,
        trans: Option<DataHV<TransformValue>>,
    },
    Complex {
        name: String,
        tp: Type,
        combs: Vec<StrucComb>,

        intervals: DataHV<Vec<i32>>,
        i_bases: DataHV<Vec<f32>>,
        i_allowances: DataHV<Vec<f32>>,
        offset: DataHV<[f32; 2]>,
    },
}

impl StrucComb {
    pub fn new_single(name: String, proto: StrucProto) -> Self {
        Self::Single {
            name,
            view: StrucView::new(&proto),
            proto,
            trans: None,
        }
    }

    pub fn new_complex(name: String, tp: Type, combs: Vec<StrucComb>) -> Self {
        Self::Complex {
            name,
            tp,
            combs,
            intervals: Default::default(),
            i_bases: Default::default(),
            i_allowances: Default::default(),
            offset: Default::default(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Single { name, .. } => name,
            Self::Complex { name, .. } => name,
        }
    }

    pub fn white_area(&self) -> DataHV<[f32; 2]> {
        match self {
            Self::Single { trans, .. } => trans.as_ref().unwrap().map(|t| t.offset),
            Self::Complex { offset, .. } => offset.clone(),
        }
    }

    pub fn get_char_box(&self) -> WorkBox {
        let mut char_box = WorkBox::new(WorkPoint::zero(), WorkPoint::splat(1.0));
        match self {
            Self::Single { proto, .. } => {
                if let Some([minx, miny, maxx, maxy]) = proto.get_attr::<attrs::CharBox>() {
                    char_box.min = char_box.min.max(WorkPoint::new(minx, miny));
                    char_box.max = char_box.max.min(WorkPoint::new(maxx, maxy));
                }
            }
            _ => {}
        }
        char_box
    }

    // pub fn get_edge(&self, axis: Axis, place: Place) -> Result<Edge, Error> {
    //     // pub fn get_edge_in_surround(
    //     //     &self,
    //     //     surround: DataHV<Place>,
    //     //     secondary: &StrucComb,
    //     //     axis: Axis,
    //     //     place: Place,
    //     // ) -> Result<Edge, Error> {
    //     //     match self {
    //     //         StrucComb::Single { view, name, .. } => {
    //     //             let area = *view
    //     //                 .surround_area(surround)
    //     //                 .ok_or(Error::Surround {
    //     //                     place: surround,
    //     //                     comp: name.clone(),
    //     //                 })?
    //     //                 .hv_get(axis.inverse());
    //     //             let surround = *surround.hv_get(axis.inverse());
    //     //             let max_index = view
    //     //                 .real_size()
    //     //                 .map(|i| i.checked_sub(1).unwrap_or_default());
    //     //             let segment = match place {
    //     //                 Place::Start => 0,
    //     //                 Place::End => *max_index.hv_get(axis),
    //     //                 _ => unreachable!(),
    //     //             };

    //     //             let edge1 = if surround != Place::End {
    //     //                 view.read_edge_in(axis, 0, area[0], segment, place)
    //     //             } else {
    //     //                 Default::default()
    //     //             };
    //     //             let edge2 = if surround != Place::Start {
    //     //                 view.read_edge_in(
    //     //                     axis,
    //     //                     area[1],
    //     //                     *max_index.hv_get(axis.inverse()),
    //     //                     segment,
    //     //                     place,
    //     //                 )
    //     //             } else {
    //     //                 Default::default()
    //     //             };

    //     //             Ok(edge1
    //     //                 .connect(secondary.get_edge(axis, place)?)
    //     //                 .connect(edge2))
    //     //         }
    //     //         Self::Complex { tp, combs, .. } => match tp {
    //     //             Type::Scale(c_axis) => {
    //     //                 if *c_axis == axis {
    //     //                     if *surround.hv_get(axis.inverse()) == Place::End {
    //     //                         combs[0].get_edge_in_surround(surround, secondary, axis, place)
    //     //                     } else {
    //     //                         combs
    //     //                             .last()
    //     //                             .unwrap()
    //     //                             .get_edge_in_surround(surround, secondary, axis, place)
    //     //                     }
    //     //                 } else {
    //     //                     if *surround.hv_get(axis.inverse()) == Place::End {
    //     //                         Ok(combs[0]
    //     //                             .get_edge_in_surround(surround, secondary, axis, place)?
    //     //                             .connect(
    //     //                                 combs[1..]
    //     //                                     .iter()
    //     //                                     .map(|c| c.get_edge(axis, place))
    //     //                                     .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //     //                                     .unwrap()?,
    //     //                             ))
    //     //                     } else {
    //     //                         Ok(combs[..combs.len() - 1]
    //     //                             .iter()
    //     //                             .map(|c| c.get_edge(axis, place))
    //     //                             .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //     //                             .unwrap()?
    //     //                             .connect(
    //     //                                 combs
    //     //                                     .last()
    //     //                                     .unwrap()
    //     //                                     .get_edge_in_surround(surround, secondary, axis, place)?,
    //     //                             ))
    //     //                     }
    //     //                 }
    //     //             }
    //     //             Type::Surround(c_surround) => {
    //     //                 if c_surround.hv_get(axis).inverse() == place
    //     //                     && *c_surround.hv_get(axis.inverse()) != Place::Mind
    //     //                     && axis == Axis::Horizontal
    //     //                 {
    //     //                     //  ↙X
    //     //                     // 十   Bug in c_surround != surround
    //     //                     //  ↖X
    //     //                     assert_eq!(surround, *c_surround);

    //     //                     let new_combs = if *c_surround.hv_get(axis.inverse()) == Place::Start {
    //     //                         vec![combs[1].clone(), secondary.clone()]
    //     //                     } else {
    //     //                         vec![secondary.clone(), combs[1].clone()]
    //     //                     };
    //     //                     let secondary = StrucComb::new_complex(
    //     //                         "read_edge".to_string(),
    //     //                         Type::Scale(Axis::Vertical),
    //     //                         new_combs,
    //     //                     );

    //     //                     combs[0].get_edge_in_surround(surround, &secondary, axis, place)
    //     //                 } else {
    //     //                     combs[0].get_edge_in_surround(surround, secondary, axis, place)
    //     //                 }
    //     //             }
    //     //             Type::Single => unreachable!(),
    //     //         },
    //     //     }
    //     // }

    //     match self {
    //         StrucComb::Single { view, .. } => Ok(view.read_edge(axis, place)),
    //         StrucComb::Complex { tp, combs, .. } => match tp {
    //             Type::Scale(c_axis) => {
    //                 if *c_axis == axis {
    //                     let c = match place {
    //                         Place::Start => &combs[0],
    //                         Place::End => combs.last().unwrap(),
    //                         Place::Mind => unreachable!(),
    //                     };
    //                     c.get_edge(axis, place)
    //                 } else {
    //                     combs
    //                         .iter()
    //                         .map(|c| c.get_edge(axis, place))
    //                         .reduce(|e1, e2| Edge::connect_result(e1, e2))
    //                         .unwrap()
    //                 }
    //             }
    //             Type::Surround(surround_place) => {
    //                 if surround_place.hv_get(axis).inverse() == place {
    //                     // combs[0].get_edge_in_surround(*surround_place, &combs[1], axis, place)
    //                     todo!()
    //                 } else {
    //                     combs[0].get_edge(axis, place)
    //                 }
    //             }
    //             Type::Single => unreachable!(),
    //         },
    //     }
    // }

    pub fn to_struc(&self, start: WorkPoint, min_vals: DataHV<f32>) -> StrucWork {
        let mut struc = Default::default();
        self.merge_to(&mut struc, start, min_vals);
        struc
    }

    pub fn merge_to(
        &self,
        struc: &mut StrucWork,
        start: WorkPoint,
        min_vals: DataHV<f32>,
    ) -> WorkSize {
        match self {
            Self::Single { proto, trans, .. } => {
                let trans = trans.as_ref().unwrap();
                let offset: WorkVec = trans.map(|t| t.offset[0]).to_array().into();
                struc.merge(proto.to_work_in_assign(
                    DataHV::new(&trans.h.assigns(), &trans.v.assigns()),
                    min_vals,
                    start + offset,
                ));
                WorkSize::new(trans.h.length(), trans.v.length())
            }
            Self::Complex {
                tp,
                combs,
                i_allowances,
                i_bases,
                offset,
                ..
            } => match tp {
                Type::Scale(axis) => {
                    let mut start_pos = start
                        .to_hv_data()
                        .zip(offset.as_ref())
                        .into_map(|(p, o)| p + o[0])
                        .to_array()
                        .into();

                    let mut interval = i_allowances
                        .hv_get(*axis)
                        .iter()
                        .zip(i_bases.hv_get(*axis).iter())
                        .map(|(a, i)| a + i);

                    combs.iter().fold(WorkSize::zero(), |mut size, c| {
                        let mut advance = c.merge_to(struc, start_pos, min_vals);
                        *advance.hv_get_mut(*axis) += interval.next().unwrap_or_default();

                        *size.hv_get_mut(*axis) += advance.hv_get(*axis);
                        *size.hv_get_mut(axis.inverse()) = size
                            .hv_get(axis.inverse())
                            .max(*advance.hv_get(axis.inverse()));
                        *start_pos.hv_get_mut(*axis) += *advance.hv_get(*axis);
                        size
                    })
                }
                Type::Surround(surround) => {
                    let mut start_pos = start
                        .to_hv_data()
                        .zip(offset.as_ref())
                        .into_map(|(p, o)| p + o[0])
                        .to_array()
                        .into();
                    let size = combs[0].merge_to(struc, start_pos, min_vals);

                    Axis::list().into_iter().for_each(|axis| {
                        if *surround.hv_get(axis) != Place::End {
                            *start_pos.hv_get_mut(axis) += i_bases
                                .hv_get(axis)
                                .iter()
                                .take(2)
                                .chain(i_allowances.hv_get(axis).iter().take(2))
                                .sum::<f32>();
                        }
                    });

                    let mut secondary_struc = StrucWork::default();
                    let subarea_size = combs[1].merge_to(&mut secondary_struc, start_pos, min_vals);
                    struc.marker_shrink(WorkRect::new(start_pos, subarea_size).to_box2d());
                    struc.merge(secondary_struc);

                    size
                }
                Type::Single => unreachable!(),
            },
        }
    }

    pub fn center_correction(
        &mut self,
        target: DataHV<Option<f32>>,
        correction: DataHV<f32>,
        between: DataHV<Option<f32>>,
        between_corr: DataHV<f32>,
        min_vals: DataHV<f32>,
        min_len: f32,
        interval_limit: DataHV<f32>,
    ) {
        match self {
            Self::Single { trans, proto, .. } => {
                let trans = trans.as_mut().unwrap();
                let center = proto
                    .to_work_in_assign(
                        DataHV::new(&trans.h.assigns(), &trans.v.assigns()),
                        DataHV::splat(0.06),
                        WorkPoint::splat(0.0),
                    )
                    .visual_center(min_len)
                    .0;
                target
                    .hv_axis_iter()
                    .filter(|(t, _)| t.is_some())
                    .for_each(|(target, axis)| {
                        let center_v = *center.hv_get(axis);
                        let trans_v = trans.hv_get_mut(axis);

                        trans_v.allowances = algorithm::center_correction(
                            &trans_v.assigns(),
                            &trans_v.bases,
                            center_v,
                            target.unwrap(),
                            *correction.hv_get(axis),
                        );
                    })
            }
            Self::Complex {
                combs,
                tp,
                i_allowances,
                i_bases,
                ..
            } => match tp {
                Type::Scale(axis) => {
                    combs.iter_mut().for_each(|c| {
                        c.center_correction(
                            target,
                            correction,
                            between,
                            between_corr,
                            min_vals,
                            min_len,
                            interval_limit,
                        )
                    });

                    if let Some(between) = between.hv_get(*axis) {
                        let sizes: Vec<(f32, f32)> = combs
                            .iter()
                            .map(|c| c.get_base_and_allowance(min_vals))
                            .map(|r| {
                                let (base, allowance) = *r.hv_get(*axis);
                                (base, base + allowance)
                            })
                            .collect();

                        let mut bases: Vec<f32> = sizes.iter().map(|(b, _)| *b).collect();
                        let mut assigns: Vec<f32> = sizes.iter().map(|(_, a)| *a).collect();
                        {
                            let mut i_b_iter = i_bases.hv_get(*axis).iter().rev();
                            let mut i_a_iter = i_allowances
                                .hv_get(*axis)
                                .iter()
                                .zip(i_bases.hv_get(*axis).iter())
                                .map(|(&b, &a)| b + a)
                                .rev();
                            (0..bases.len()).rev().for_each(|i| {
                                if i != 0 {
                                    let b = *i_b_iter.next().unwrap();
                                    bases.insert(i, b);
                                    assigns.insert(i, i_a_iter.next().unwrap());
                                }
                            });
                        }
                        let total = assigns.iter().sum::<f32>();

                        let struc_list: Vec<_> = combs
                            .iter()
                            .map(|c| {
                                let mut struc = Default::default();
                                let size = *c
                                    .merge_to(&mut struc, WorkPoint::zero(), min_vals)
                                    .hv_get(*axis);
                                (struc, size)
                            })
                            .collect();

                        (1..combs.len()).for_each(|i| {
                            let mut moved = WorkVec::zero();
                            let struc = struc_list[0..=i]
                                .iter()
                                .enumerate()
                                .map(|(index, (struc, size))| {
                                    let mut struc = struc.clone();
                                    let mut scale = WorkVec::splat(1.0);
                                    if *size != 0.0 {
                                        *scale.hv_get_mut(*axis) = assigns[index * 2] / size;
                                    }
                                    struc = struc.transform(scale, moved.clone());
                                    *moved.hv_get_mut(*axis) += assigns[index * 2]
                                        + assigns.get(index * 2 + 1).cloned().unwrap_or_default();

                                    struc
                                })
                                .reduce(|mut a, b| {
                                    a.merge(b);
                                    a
                                })
                                .unwrap();
                            let center = *struc.visual_center(min_len).0.hv_get(*axis);

                            let corr_vals = algorithm::center_correction(
                                &assigns[0..=i * 2],
                                &bases,
                                center,
                                *between,
                                *between_corr.hv_get(*axis),
                            );
                            assigns
                                .iter_mut()
                                .zip(bases.iter())
                                .zip(corr_vals)
                                .for_each(|((a, b), v)| {
                                    *a = b + v;
                                });
                        });

                        let i_allowances = i_allowances.hv_get_mut(*axis);
                        let interval_limit = total * *interval_limit.hv_get(*axis);
                        let mut limit_allow = 0.0;
                        *i_allowances = (0..i_allowances.len())
                            .map(|i| {
                                let limit = (interval_limit - i_bases.hv_get(*axis)[i]).max(0.0);
                                let val = assigns.remove(i + 1) - i_bases.hv_get(*axis)[i];
                                if val > limit {
                                    limit_allow += val - limit;
                                    limit
                                } else {
                                    val
                                }
                            })
                            .collect();
                        if limit_allow != 0.0 {
                            let total = assigns.iter().sum::<f32>();
                            let scale = (total + limit_allow) / total;
                            assigns.iter_mut().for_each(|v| *v *= scale);
                        }

                        // let scale_list = old_assigns
                        //     .into_iter()
                        //     .zip(assigns.iter())
                        //     .map(|(old, cur)| if old == 0.0 { 1.0 } else { cur / old });
                        combs.iter_mut().zip(assigns).for_each(|(c, assign)| {
                            let mut new_assign = DataHV::splat(None);
                            *new_assign.hv_get_mut(*axis) = Some(assign);
                            let r = c.assign_new_length(new_assign, min_vals);
                            debug_assert!(r.hv_get(*axis).unwrap() < algorithm::NORMAL_OFFSET);
                        })
                    }
                }
                Type::Surround(splace) => {
                    combs[1].center_correction(
                        target,
                        correction,
                        between,
                        between_corr,
                        min_vals,
                        min_len,
                        interval_limit,
                    );

                    let struc = {
                        let mut start_pos = Default::default();
                        let mut struc = combs[0].to_struc(start_pos, min_vals);
                        Axis::list().into_iter().for_each(|axis| {
                            if *splace.hv_get(axis) != Place::End {
                                *start_pos.hv_get_mut(axis) += i_bases
                                    .hv_get(axis)
                                    .iter()
                                    .take(2)
                                    .chain(i_allowances.hv_get(axis).iter().take(2))
                                    .sum::<f32>();
                            }
                        });
                        let mut secondary_struc = StrucWork::default();
                        let subarea_size =
                            combs[1].merge_to(&mut secondary_struc, start_pos, min_vals);
                        struc.marker_shrink(WorkRect::new(start_pos, subarea_size).to_box2d());
                        struc.merge(secondary_struc);

                        struc
                    };

                    let mut secondary = combs.remove(1);
                    let mut primary = combs.remove(0);
                    match &mut primary {
                        Self::Single { trans, view, .. } => {
                            let area = view.surround_area(*splace).unwrap();
                            let center = struc.visual_center(min_len).0;
                            let trans = trans.as_mut().unwrap();
                            let (s_base, s_allowance) =
                                secondary.get_base_and_allowance(min_vals).unzip();

                            let s_assign = Axis::hv_data().into_map(|axis| {
                                if let Some(between) = between.hv_get(axis) {
                                    let center = *center.hv_get(axis);
                                    let tvs = trans.hv_get_mut(axis);
                                    let min_val = *min_vals.hv_get(axis);
                                    let area = area.hv_get(axis);
                                    let s_base = *s_base.hv_get(axis);
                                    let mut s_allowance = *s_allowance.hv_get(axis);
                                    let i_bases = i_bases.hv_get(axis);
                                    let i_allowances = i_allowances.hv_get_mut(axis);

                                    let mut bases: Vec<f32> = vec![];
                                    bases.extend(tvs.bases[0..area[0]].iter());
                                    let mut allowances: Vec<f32> = vec![];
                                    allowances.extend(tvs.allowances[0..area[0]].iter());

                                    let (biter, aiter) = match splace.hv_get(axis) {
                                        Place::Start => (&i_bases[1..=1], &mut i_allowances[1..=1]),
                                        Place::Mind => (&i_bases[1..=2], &mut i_allowances[1..=2]),
                                        Place::End => (&i_bases[0..=0], &mut i_allowances[0..=0]),
                                    };
                                    let (sub_base, sub_allowance) = if s_base
                                        + i_bases.iter().sum::<f32>()
                                        > tvs.bases.iter().sum::<f32>()
                                    {
                                        (
                                            s_base + biter.iter().sum::<f32>(),
                                            s_allowance + aiter.iter().sum::<f32>(),
                                        )
                                    } else {
                                        (
                                            tvs.bases[area[0]..area[1]].iter().sum::<f32>(),
                                            tvs.allowances[area[0]..area[1]].iter().sum::<f32>(),
                                        )
                                    };
                                    bases.push(sub_base);
                                    allowances.push(sub_allowance);

                                    bases.extend(tvs.bases[area[1]..].iter());
                                    allowances.extend(tvs.allowances[area[1]..].iter());

                                    let vlist: Vec<f32> = bases
                                        .iter()
                                        .zip(allowances.iter())
                                        .map(|(b, a)| *b + *a)
                                        .collect();
                                    let mut corr_allo = algorithm::center_correction(
                                        &vlist,
                                        &bases,
                                        center,
                                        *between,
                                        *between_corr.hv_get(axis),
                                    );

                                    corr_allo
                                        .drain(0..area[0])
                                        .zip(tvs.allowances[0..area[0]].iter_mut())
                                        .for_each(|(ca, a)| *a = ca);

                                    let new_sub_assign = corr_allo.remove(0) + sub_base;
                                    {
                                        let mut o_vals =
                                            Self::offset_base_and_allowance(&tvs.offset, min_val)
                                                .unwrap_or_default();

                                        let p_sub_base: Vec<f32> = tvs.bases[area[0]..area[1]]
                                            .iter()
                                            .chain(o_vals.0.iter())
                                            .copied()
                                            .collect();
                                        let mut p_sub_allowance: Vec<&mut f32> = tvs.allowances
                                            [area[0]..area[1]]
                                            .iter_mut()
                                            .chain(o_vals.1.iter_mut())
                                            .collect();
                                        Self::assign_new_length_to(
                                            &mut p_sub_allowance,
                                            &p_sub_base,
                                            new_sub_assign,
                                        );
                                        tvs.offset =
                                            [o_vals.0[0] + o_vals.1[0], o_vals.0[1] + o_vals.1[1]];

                                        let s_sub_base: Vec<f32> = biter
                                            .iter()
                                            .chain(std::iter::once(&s_base))
                                            .copied()
                                            .collect();
                                        let mut s_sub_allowance: Vec<&mut f32> = aiter
                                            .iter_mut()
                                            .chain(std::iter::once(&mut s_allowance))
                                            .collect();
                                        Self::assign_new_length_to(
                                            &mut s_sub_allowance,
                                            &s_sub_base,
                                            new_sub_assign,
                                        );
                                    }

                                    corr_allo
                                        .drain(..)
                                        .zip(tvs.allowances[area[1]..].iter_mut())
                                        .for_each(|(ca, a)| *a = ca);
                                    if *splace.hv_get(axis) != Place::End {
                                        i_allowances[0] = tvs.allowances[..area[0]].iter().sum();
                                    }
                                    if *splace.hv_get(axis) != Place::Start {
                                        *i_allowances.last_mut().unwrap() =
                                            tvs.allowances[area[1]..].iter().sum();
                                    }

                                    Some(s_allowance + s_base)
                                } else {
                                    None
                                }
                            });

                            let r = secondary.assign_new_length(s_assign, min_vals);
                            debug_assert!(r.h.unwrap_or_default() < algorithm::NORMAL_OFFSET);
                            debug_assert!(r.v.unwrap_or_default() < algorithm::NORMAL_OFFSET);
                        }
                        Self::Complex { tp, .. } => todo!(),
                    }
                    *combs = vec![primary, secondary];
                }
                Type::Single => unreachable!(),
            },
        }
    }

    pub fn axis_surround_comb_in(
        axis: Axis,
        splace: DataHV<Place>,
        combs: &Vec<StrucComb>,
    ) -> usize {
        match splace.hv_get(axis) {
            Place::End => 0,
            _ => combs.len() - 1,
        }
    }

    fn assign_new_length_to(allowances: &mut Vec<&mut f32>, bases: &Vec<f32>, assign: f32) -> f32 {
        let bases_total = bases.iter().sum::<f32>();
        if (assign - bases_total).abs() <= algorithm::NORMAL_OFFSET {
            allowances.iter_mut().for_each(|a| **a = 0.0);
            bases_total - assign
        } else {
            let old_assign = allowances
                .iter()
                .map(|a| **a)
                .chain(bases.iter().copied())
                .sum::<f32>();
            debug_assert_ne!(old_assign, 0.0);
            let scale = assign / old_assign;
            let debt = allowances
                .iter_mut()
                .zip(bases.iter())
                .fold(0.0, |debt, (a, &b)| {
                    let val = (**a + b) * scale;
                    if val < b {
                        **a = 0.0;
                        debt + b - val
                    } else {
                        **a = val - b;
                        debt
                    }
                });

            if debt != 0.0 {
                let mut allow_total = allowances.iter().map(|v| **v).sum::<f32>();
                if (allow_total - debt).abs() < algorithm::NORMAL_OFFSET {
                    allow_total = debt;
                }
                debug_assert!(allow_total >= debt);
                let scale = (allow_total - debt) / allow_total;
                allowances.iter_mut().for_each(|a| **a *= scale);
            }

            0.0
        }
    }

    fn assign_new_length(
        &mut self,
        new_assign: DataHV<Option<f32>>,
        min_vals: DataHV<f32>,
    ) -> DataHV<Option<f32>> {
        match self {
            Self::Single { trans, .. } => Axis::hv_data().zip(new_assign).zip(min_vals).into_map(
                |((axis, assign), min_val)| {
                    let assign = assign?;
                    let trans = trans.as_mut().unwrap().hv_get_mut(axis);
                    let mut o_vals =
                        Self::offset_base_and_allowance(&trans.offset, min_val).unwrap_or_default();

                    let bases: Vec<f32> =
                        trans.bases.iter().chain(o_vals.0.iter()).copied().collect();
                    let mut allowances: Vec<&mut f32> = trans
                        .allowances
                        .iter_mut()
                        .chain(o_vals.1.iter_mut())
                        .collect();
                    let debt = Self::assign_new_length_to(&mut allowances, &bases, assign);

                    trans.offset = [o_vals.0[0] + o_vals.1[0], o_vals.0[1] + o_vals.1[1]];
                    Some(debt)
                },
            ),
            Self::Complex {
                combs,
                i_bases,
                i_allowances,
                offset,
                tp,
                ..
            } => match &tp {
                Type::Scale(c_axis) => {
                    let c_lengths: Vec<_> = combs
                        .iter()
                        .map(|c| c.get_base_and_allowance(min_vals))
                        .collect();

                    let mut c_assigns = vec![DataHV::splat(None); combs.len()];
                    let mut debt = DataHV::splat(None);

                    if let Some(assign) = *new_assign.hv_get(c_axis.inverse()) {
                        let axis = c_axis.inverse();
                        let (base, mut allowance) = c_lengths
                            .iter()
                            .map(|cl| *cl.hv_get(axis))
                            .reduce(|a, b| if a > b { a } else { b })
                            .unwrap();
                        let mut o_vals = Self::offset_base_and_allowance(
                            offset.hv_get(axis),
                            *min_vals.hv_get(axis),
                        )
                        .unwrap_or_default();

                        let bases: Vec<f32> = std::iter::once(&base)
                            .chain(o_vals.0.iter())
                            .chain(i_bases.hv_get(axis).iter())
                            .copied()
                            .collect();
                        let mut allowances: Vec<&mut f32> = std::iter::once(&mut allowance)
                            .chain(o_vals.1.iter_mut())
                            .chain(i_allowances.hv_get_mut(axis).iter_mut())
                            .collect();

                        *debt.hv_get_mut(axis) =
                            Some(Self::assign_new_length_to(&mut allowances, &bases, assign));
                        *offset.hv_get_mut(axis) =
                            [o_vals.0[0] + o_vals.1[0], o_vals.0[1] + o_vals.1[1]];

                        let c_assign = base + allowance;
                        c_assigns
                            .iter_mut()
                            .for_each(|ca| *ca.hv_get_mut(axis) = Some(c_assign));
                    }

                    if let Some(assign) = *new_assign.hv_get(*c_axis) {
                        let axis = *c_axis;
                        let mut c_lengths: Vec<_> =
                            c_lengths.iter().map(|cl| *cl.hv_get(axis)).collect();
                        let mut o_vals = Self::offset_base_and_allowance(
                            offset.hv_get(axis),
                            *min_vals.hv_get(axis),
                        )
                        .unwrap_or_default();

                        let bases: Vec<f32> = c_lengths
                            .iter()
                            .map(|cl| &cl.0)
                            .chain(o_vals.0.iter())
                            .chain(i_bases.hv_get(axis).iter())
                            .copied()
                            .collect();
                        let mut allowances: Vec<&mut f32> = c_lengths
                            .iter_mut()
                            .map(|cl| &mut cl.1)
                            .chain(o_vals.1.iter_mut())
                            .chain(i_allowances.hv_get_mut(axis).iter_mut())
                            .collect();
                        *debt.hv_get_mut(axis) =
                            Some(Self::assign_new_length_to(&mut allowances, &bases, assign));
                        *offset.hv_get_mut(axis) =
                            [o_vals.0[0] + o_vals.1[0], o_vals.0[1] + o_vals.1[1]];

                        c_assigns
                            .iter_mut()
                            .zip(c_lengths)
                            .for_each(|(ca, cl)| *ca.hv_get_mut(axis) = Some(cl.0 + cl.1));
                    }

                    combs.iter_mut().zip(c_assigns).for_each(|(c, assign)| {
                        c.assign_new_length(assign, min_vals);
                    });

                    debt
                }
                Type::Surround(c_surround) => {
                    let mut secondary = combs.remove(1);
                    let mut primary = combs.remove(0);
                    let r = match &mut primary {
                        StrucComb::Single { trans, view, .. } => {
                            let area = view.surround_area(*c_surround).unwrap();
                            let mut s_assigns = DataHV::splat(None);
                            let s_length = secondary.get_base_and_allowance(min_vals);

                            let debt = Axis::hv_data().into_map(|axis| {
                                let assign = (*new_assign.hv_get(axis))?;

                                let mut o_vals = Self::offset_base_and_allowance(
                                    offset.hv_get(axis),
                                    *min_vals.hv_get(axis),
                                )
                                .unwrap_or_default();

                                let area = area.hv_get(axis);
                                let trans = trans.as_mut().unwrap().hv_get_mut(axis);
                                let mut p_o_vals = Self::offset_base_and_allowance(
                                    &trans.offset,
                                    *min_vals.hv_get(axis),
                                )
                                .unwrap_or_default();
                                let p_subarea_base = trans.bases[area[0]..area[1]]
                                    .iter()
                                    .chain(p_o_vals.0.iter())
                                    .sum::<f32>();
                                let p_subarea_allow = trans.allowances[area[0]..area[1]]
                                    .iter()
                                    .chain(p_o_vals.1.iter())
                                    .sum::<f32>();

                                let i_bases = i_bases.hv_get(axis);
                                let i_allowances = i_allowances.hv_get_mut(axis);
                                let (s_base, mut s_allowance) = *s_length.hv_get(axis);
                                let surround = *c_surround.hv_get(axis);
                                let (s_i_b, mut s_i_a): (&[f32], Vec<&mut f32>) = match surround {
                                    Place::End => {
                                        (&i_bases[0..1], i_allowances[0..1].iter_mut().collect())
                                    }
                                    Place::Mind => {
                                        (&i_bases[1..=2], i_allowances[1..=2].iter_mut().collect())
                                    }
                                    Place::Start => {
                                        (&i_bases[1..2], i_allowances[1..2].iter_mut().collect())
                                    }
                                };

                                let (subarea_b, mut subarea_a) =
                                    if s_base + s_i_b.iter().sum::<f32>() < p_subarea_base {
                                        (p_subarea_base, p_subarea_allow)
                                    } else {
                                        (
                                            s_base + s_i_b.iter().sum::<f32>(),
                                            s_allowance + s_i_a.iter().map(|v| **v).sum::<f32>(),
                                        )
                                    };

                                let (bases, mut allowances): (Vec<f32>, Vec<&mut f32>) = (
                                    trans.bases[..area[0]]
                                        .iter()
                                        .chain(trans.bases[area[1]..].iter())
                                        .chain(std::iter::once(&subarea_b))
                                        .chain(o_vals.0.iter())
                                        .copied()
                                        .collect(),
                                    trans
                                        .allowances
                                        .iter_mut()
                                        .enumerate()
                                        .filter_map(|(i, v)| {
                                            if i < area[0] || i >= area[1] {
                                                Some(v)
                                            } else {
                                                None
                                            }
                                        })
                                        .chain(std::iter::once(&mut subarea_a))
                                        .chain(o_vals.1.iter_mut())
                                        .collect(),
                                );

                                let debt =
                                    Self::assign_new_length_to(&mut allowances, &bases, assign);

                                let sub_area = subarea_b + subarea_a;
                                let mut sub_allows: Vec<_> = trans.allowances[area[0]..area[1]]
                                    .iter_mut()
                                    .chain(p_o_vals.1.iter_mut())
                                    .collect();
                                Self::assign_new_length_to(
                                    &mut sub_allows,
                                    &trans.bases[area[0]..area[1]]
                                        .iter()
                                        .chain(p_o_vals.0.iter())
                                        .copied()
                                        .collect(),
                                    sub_area,
                                );
                                *offset.hv_get_mut(axis) =
                                    [o_vals.0[0] + o_vals.1[0], o_vals.0[1] + o_vals.1[1]];
                                trans.offset =
                                    [p_o_vals.0[0] + p_o_vals.1[0], p_o_vals.0[1] + p_o_vals.1[1]];

                                s_i_a.push(&mut s_allowance);
                                Self::assign_new_length_to(
                                    &mut s_i_a,
                                    &s_i_b
                                        .iter()
                                        .chain(std::iter::once(&s_base))
                                        .copied()
                                        .collect::<Vec<f32>>(),
                                    sub_area,
                                );
                                *s_assigns.hv_get_mut(axis) = Some(s_allowance + s_base);
                                if surround != Place::End {
                                    *i_allowances.first_mut().unwrap() =
                                        trans.allowances[..area[0]].iter().sum();
                                }
                                if surround != Place::Start {
                                    *i_allowances.last_mut().unwrap() =
                                        trans.allowances[area[1]..].iter().sum();
                                }

                                Some(debt)
                            });

                            secondary.assign_new_length(s_assigns, min_vals);

                            debt
                        }
                        StrucComb::Complex { tp, .. } => todo!(),
                    };

                    combs.push(primary);
                    combs.push(secondary);
                    r
                }
                Type::Single => unreachable!(),
            },
        }
    }

    // fn recombination_surround<F, R>(
    //     mut primary: StrucComb,
    //     mut secondary: StrucComb,
    //     surround: DataHV<Place>,
    //     intervals: &mut DataHV<Vec<i32>>,
    //     bases: &mut DataHV<Vec<f32>>,
    //     allowances: &mut DataHV<Vec<f32>>,
    //     offsets: &mut DataHV<[f32; 2]>,
    //     mut f: F,
    // ) -> (StrucComb, StrucComb, R)
    // where
    //     F: FnMut(&mut StrucComb) -> R,
    // {
    //     match primary {
    //         Self::Single { .. } => {
    //             let mut struc = StrucComb::Complex {
    //                 name: "recomb".to_string(),
    //                 tp: Type::Surround(surround),
    //                 combs: vec![primary, secondary],
    //                 intervals: intervals.clone(),
    //                 i_bases: bases.clone(),
    //                 i_allowances: allowances.clone(),
    //                 offset: offsets.clone(),
    //             };
    //             let r = f(&mut struc);

    //             let StrucComb::Complex {
    //                 mut combs,
    //                 intervals: c_intervals,
    //                 i_bases,
    //                 i_allowances,
    //                 offset,
    //                 ..
    //             } = struc
    //             else {
    //                 unreachable!()
    //             };

    //             let sc = combs.pop().unwrap();
    //             let pc = combs.pop().unwrap();
    //             *intervals = c_intervals;
    //             *bases = i_bases;
    //             *allowances = i_allowances;
    //             *offsets = offset;

    //             (pc, sc, r)
    //         }
    //         Self::Complex {
    //             tp,
    //             mut combs,
    //             intervals: mut c_intervals,
    //             mut i_bases,
    //             mut i_allowances,
    //             mut offset,
    //             name,
    //             ..
    //         } => match tp {
    //             Type::Scale(c_axis) => {
    //                 let p_index = Self::axis_surround_comb_in(c_axis, surround, &combs);
    //                 let new_struc = StrucComb::Complex {
    //                     name: "recomb".to_string(),
    //                     tp: Type::Surround(surround),
    //                     combs: vec![combs.remove(p_index), secondary],
    //                     intervals: intervals.clone(),
    //                     i_bases: bases.clone(),
    //                     i_allowances: allowances.clone(),
    //                     offset: offsets.clone(),
    //                 };
    //                 combs.insert(p_index, new_struc);

    //                 let mut struc = StrucComb::Complex {
    //                     name,
    //                     tp,
    //                     combs,
    //                     intervals: c_intervals,
    //                     i_bases,
    //                     i_allowances,
    //                     offset,
    //                 };

    //                 let r = f(&mut struc);

    //                 let StrucComb::Complex {
    //                     mut combs,
    //                     intervals: c_intervals,
    //                     i_bases,
    //                     i_allowances,
    //                     offset,
    //                     ..
    //                 } = struc
    //                 else {
    //                     unreachable!()
    //                 };

    //                 let StrucComb::Complex {
    //                     mut combs,
    //                     intervals: c_intervals,
    //                     i_bases,
    //                     i_allowances,
    //                     offset,
    //                     ..
    //                 } = combs.remove(p_index)
    //                 else {
    //                     unreachable!()
    //                 };

    //                 todo!()
    //             }
    //             Type::Surround(c_srround) => todo!(),
    //             Type::Single => unreachable!(),
    //         },
    //     }
    // }

    fn offset_base_and_allowance(offsets: &[f32; 2], min_val: f32) -> Option<([f32; 2], [f32; 2])> {
        let total = offsets[0] + offsets[1];
        if total == 0.0 {
            None
        } else {
            let bases = offsets.map(|v| v / total * min_val);
            let allows = [offsets[0] - bases[0], offsets[1] - bases[1]];
            Some((bases, allows))
        }
    }

    fn get_base_and_allowance(&self, min_vals: DataHV<f32>) -> DataHV<(f32, f32)> {
        match self {
            Self::Single { trans, .. } => {
                let tvs = trans.as_ref().unwrap();
                tvs.as_ref().zip(min_vals).into_map(|(t, min_val)| {
                    let o_vals =
                        Self::offset_base_and_allowance(&t.offset, min_val).unwrap_or_default();
                    (
                        t.bases.iter().chain(o_vals.0.iter()).sum(),
                        t.allowances.iter().chain(o_vals.1.iter()).sum(),
                    )
                })
            }
            Self::Complex {
                tp,
                combs,
                i_bases,
                i_allowances,
                offset,
                ..
            } => {
                let o_vals = offset.as_ref().zip(min_vals).into_map(|(o, min)| {
                    Self::offset_base_and_allowance(o, min).unwrap_or_default()
                });
                match tp {
                    Type::Scale(c_axis) => {
                        let mut r = combs
                            .iter()
                            .map(|c| c.get_base_and_allowance(min_vals))
                            .reduce(|a, b| {
                                a.zip(b).zip(Axis::hv_data()).into_map(
                                    |(((b1, a1), (b2, a2)), axis)| {
                                        if axis == *c_axis {
                                            (
                                                a.hv_get(axis).0 + b.hv_get(axis).0,
                                                a.hv_get(axis).1 + b.hv_get(axis).1,
                                            )
                                        } else {
                                            debug_assert!(
                                                (b1 + a1 - b2 - a2).abs()
                                                    < algorithm::NORMAL_OFFSET,
                                                "{} != {}",
                                                b1 + a1,
                                                b2 + a2
                                            );

                                            if b1 > b2 {
                                                (b1, a1)
                                            } else {
                                                (b2, a2)
                                            }
                                        }
                                    },
                                )
                            })
                            .unwrap();

                        r.hv_get_mut(*c_axis).0 += i_bases.hv_get(*c_axis).iter().sum::<f32>();
                        r.hv_get_mut(*c_axis).1 += i_allowances.hv_get(*c_axis).iter().sum::<f32>();
                        r.zip(o_vals)
                            .into_map(|(r, o)| (r.0 + o.0[0] + o.0[1], r.1 + o.1[0] + o.1[1]))
                    }
                    Type::Surround(_) => {
                        let p = combs[0].get_base_and_allowance(min_vals);
                        let mut s = combs[1].get_base_and_allowance(min_vals);
                        s = s.zip(i_bases.as_ref()).zip(i_allowances.as_ref()).into_map(
                            |((s, ib), ia)| {
                                (s.0 + ib.iter().sum::<f32>(), s.1 + ia.iter().sum::<f32>())
                            },
                        );

                        assert!(
                            (p.h.0 + p.h.1 - s.h.0 - s.h.1).abs() < algorithm::NORMAL_OFFSET,
                            "{} != {}",
                            p.h.0 + p.h.1,
                            s.h.0 + s.h.1
                        );
                        assert!(
                            (p.v.0 + p.v.1 - s.v.0 - s.v.1).abs() < algorithm::NORMAL_OFFSET,
                            "{} != {}",
                            p.v.0 + p.v.1,
                            s.v.0 + s.v.1
                        );

                        p.zip(s)
                            .into_map(|(p, s)| if p.0 > s.0 { p } else { s })
                            .zip(o_vals)
                            .map(|(r, o)| (r.0 + o.0[0] + o.0[1], r.1 + o.1[0] + o.1[1]))
                    }
                    Type::Single => unreachable!(),
                }
            }
        }
    }

    pub fn comb_info(&self) -> String {
        match self {
            StrucComb::Single { name, .. } => name.clone(),
            StrucComb::Complex {
                name, tp, combs, ..
            } => {
                format!(
                    "{name}{}({})",
                    tp.symbol(),
                    combs
                        .iter()
                        .map(|c| c.comb_info())
                        .collect::<Vec<String>>()
                        .join("+")
                )
            }
        }
    }

    pub fn visual_center(&self, min_len: f32, white_area: bool) -> WorkPoint {
        let struc = self.to_struc(WorkPoint::zero(), DataHV::splat(min_len));
        let (center, size) = struc.visual_center(min_len);

        if white_area {
            let white_area = self.white_area();
            center
                .to_hv_data()
                .zip(white_area)
                .zip(size.to_hv_data())
                .into_map(|((p, w), s)| (p * s + w[0]) / (w[0] + s + w[1]))
                .to_array()
                .into()
        } else {
            center
        }
    }

    pub fn name_list(&self) -> Vec<String> {
        let mut list = vec![];
        self.for_each_single(|name, _, _, _| list.push(name.to_string()));
        list
    }

    pub fn for_each_single<F>(&self, f: F)
    where
        F: FnMut(&str, &StrucProto, &StrucView, &Option<DataHV<TransformValue>>),
    {
        fn for_each<F>(comb: &StrucComb, mut f: F) -> F
        where
            F: FnMut(&str, &StrucProto, &StrucView, &Option<DataHV<TransformValue>>),
        {
            match comb {
                StrucComb::Single {
                    name,
                    proto,
                    view,
                    trans,
                } => {
                    f(name, proto, view, trans);
                    f
                }
                StrucComb::Complex { combs, .. } => combs.iter().fold(f, |f, c| for_each(c, f)),
            }
        }

        for_each(&self, f);
    }

    pub fn for_each_single_mut<F>(&mut self, f: F)
    where
        F: FnMut(&str, &StrucProto, &StrucView, &mut Option<DataHV<TransformValue>>),
    {
        fn for_each<F>(comb: &mut StrucComb, mut f: F) -> F
        where
            F: FnMut(&str, &StrucProto, &StrucView, &mut Option<DataHV<TransformValue>>),
        {
            match comb {
                StrucComb::Single {
                    name,
                    proto,
                    view,
                    trans,
                } => {
                    f(name, proto, view, trans);
                    f
                }
                StrucComb::Complex { combs, .. } => combs.iter_mut().fold(f, |f, c| for_each(c, f)),
            }
        }

        for_each(self, f);
    }
}
