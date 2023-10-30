use crate::{
    axis::*,
    component::{
        comb::{StrucComb, TransformValue},
        view::*,
    },
    construct::{self, Component, Error, Table},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct InPlace(pub [Option<bool>; 4]);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InSurround([Place; 2]);

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchValue<T = usize> {
    #[serde(with = "serde_regex")]
    pub regex: Regex,
    pub val: T,
    pub note: String,
}

impl<T> MatchValue<T> {
    pub fn from_str(regex: &str, val: T) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: Regex::new(regex)?,
            val,
            note: Default::default(),
        })
    }

    pub fn new(regex: Regex, val: T) -> Self {
        Self {
            regex,
            val,
            note: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub min_values: DataHV<Vec<f32>>,
    pub base_values: DataHV<Vec<f32>>,

    pub correction_table: Table,
    pub place_replace: BTreeMap<String, Vec<(InPlace, Component)>>,
    pub surround_replace: BTreeMap<InSurround, BTreeMap<String, Component>>,
    pub interval_rule: Vec<MatchValue>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DataHV::splat(1.0),
            min_values: DataHV::splat(vec![Self::DEFAULT_MIN_VALUE]),
            base_values: DataHV::splat(vec![1.0]),

            correction_table: Table::empty(),
            place_replace: Default::default(),
            surround_replace: Default::default(),
            interval_rule: Default::default(),
        }
    }
}

impl Config {
    pub const DEFAULT_MIN_VALUE: f32 = 0.05;

    pub fn place_replace_name(
        &self,
        name: &str,
        in_place: DataHV<[bool; 2]>,
    ) -> Option<&Component> {
        self.place_replace.get(name).and_then(|pm| {
            pm.iter().find_map(|(ip, comp)| {
                match ip
                    .0
                    .iter()
                    .zip(in_place.hv_iter().flatten())
                    .all(|(p1, p2)| p1.is_none() || p1.unwrap().eq(p2))
                {
                    true => Some(comp),
                    false => None,
                }
            })
        })
    }

    pub fn surround_replace_name(
        &self,
        name: &str,
        in_surround: DataHV<Place>,
    ) -> Option<&Component> {
        self.surround_replace
            .get(&InSurround(in_surround.to_array()))
            .and_then(|sm| sm.get(name))
    }

    pub fn get_alloc_base(&self, axis: Axis, alloc: usize) -> f32 {
        let bases = self.base_values.hv_get(axis);
        if alloc == 0 {
            0.0
        } else {
            bases
                .get(alloc)
                .or(bases.last())
                .cloned()
                .unwrap_or(Self::DEFAULT_MIN_VALUE)
        }
    }

    pub fn get_allocs_base_total<T: IntoIterator<Item = usize>>(
        &self,
        axis: Axis,
        allocs: T,
    ) -> f32 {
        let bases = self.base_values.hv_get(axis);
        allocs
            .into_iter()
            .map(|val| match val {
                0 => 0.0,
                n => bases
                    .get(n)
                    .or(bases.last())
                    .cloned()
                    .unwrap_or(Self::DEFAULT_MIN_VALUE),
            })
            .sum()
    }

    pub fn compare_allocs(
        &self,
        axis: Axis,
        allocs1: &Vec<usize>,
        allocs2: &Vec<usize>,
    ) -> std::cmp::Ordering {
        let [lv1, lv2] = [allocs1, allocs2].map(|list| {
            list.iter()
                .map(|l| self.get_alloc_base(axis, *l))
                .sum::<f32>()
        });
        lv1.partial_cmp(&lv2).unwrap()
    }

    pub fn primary_allocs(
        &self,
        allocs1: Vec<usize>,
        allocs2: Vec<usize>,
        axis: Axis,
    ) -> Vec<usize> {
        let [lv1, lv2] = [&allocs1, &allocs2].map(|list| {
            list.iter()
                .map(|l| self.get_alloc_base(axis, *l))
                .sum::<f32>()
        });
        if lv1 > lv2 {
            allocs1
        } else {
            allocs2
        }
    }

    pub fn primary_allocs_and_intervals(
        &self,
        list1: (Vec<usize>, Vec<usize>),
        list2: (Vec<usize>, Vec<usize>),
        axis: Axis,
    ) -> (Vec<usize>, Vec<usize>) {
        let [lv1, lv2] = [&list1, &list2].map(|(a, i)| {
            a.iter()
                .chain(i.iter())
                .map(|l| self.get_alloc_base(axis, *l))
                .sum::<f32>()
        });
        if lv1 > lv2 {
            list1
        } else {
            list2
        }
    }

    pub fn get_comb_bases_total(&self, comb: &StrucComb, axis: Axis) -> f32 {
        match self.get_comb_allocs(comb, axis) {
            Ok((allocs, intervals)) => allocs
                .into_iter()
                .chain(intervals)
                .map(|l| self.get_alloc_base(axis, l))
                .sum(),
            Err(_) => f32::INFINITY,
        }
    }

    pub fn assign_base_trans_value(
        &self,
        allocs: &Vec<usize>,
        limit: f32,
        axis: Axis,
        min_level: usize,
    ) -> Result<(TransformValue, usize), Error> {
        let mins = self.min_values.hv_get(axis);
        let bases = self.base_values.hv_get(axis);
        let mut level = 0;

        let alloc_bases: Vec<f32> = allocs
            .iter()
            .map(|&l| self.get_alloc_base(axis, l))
            .collect();
        let base_total: f32 = alloc_bases.iter().sum::<f32>();
        if let Some(r) = mins
            .iter()
            .enumerate()
            .skip(min_level)
            .find_map(|(i, &min)| {
                if base_total * min < limit + 0.001 {
                    level = i;
                    Some(
                        allocs
                            .iter()
                            .copied()
                            .zip(alloc_bases.iter().map(|&v| v * min))
                            .collect(),
                    )
                } else {
                    None
                }
            })
        {
            Ok((TransformValue { assign: r }, level))
        } else {
            return Err(Error::AxisTransform {
                axis,
                length: limit,
                bases: bases.clone(),
            });
        }
    }

    // This is test function
    pub fn assign_comb_space(&self, comb: &mut StrucComb, level: DataHV<usize>, size: DataHV<f32>) {
        let min_val =
            Axis::hv_data().into_map(|axis| self.min_values.hv_get(axis)[*level.hv_get(axis)]);
        match comb {
            StrucComb::Single { proto, trans, .. } => {
                let proto_allocs = proto.get_allocs();
                let tvs = Axis::hv_data().map(|&axis| {
                    let allocs = proto_allocs.hv_get(axis);
                    if allocs.len() > 0 {
                        let base = size.hv_get(axis) / allocs.len() as f32;
                        let assign = allocs.iter().map(|&l| (l, base)).collect();
                        TransformValue { assign }
                    } else {
                        TransformValue::default()
                    }
                });
                *trans = Some(tvs);
            }
            StrucComb::Complex {
                tp,
                combs,
                intervals,
                assign_intervals,
                ..
            } => match tp {
                construct::Type::Scale(axis) => {
                    *intervals = self.get_combs_axis_intervals(combs, *axis).unwrap();
                    *assign_intervals = intervals
                        .iter()
                        .map(|i| self.get_alloc_base(*axis, *i) * *min_val.hv_get(*axis))
                        .collect();

                    let (comb_allocs, comb_intervals): (Vec<Vec<usize>>, Vec<Vec<f32>>) =
                        combs.iter().fold((vec![], vec![]), |(mut al, mut il), c| {
                            let (a, i) = self.get_comb_allocs(c, *axis).unwrap();
                            al.push(a);
                            il.push(
                                i.into_iter()
                                    .map(|i| self.get_alloc_base(*axis, i) * *min_val.hv_get(*axis))
                                    .collect(),
                            );
                            (al, il)
                        });
                    let axis_allocs_base = (*size.hv_get(*axis)
                        - comb_intervals.iter().flatten().sum::<f32>()
                        - assign_intervals.iter().sum::<f32>())
                        / comb_allocs.iter().map(|ca| ca.len() as f32).sum::<f32>();

                    combs
                        .iter_mut()
                        .zip(comb_allocs)
                        .zip(comb_intervals)
                        .for_each(|((c, a_list), i_list)| {
                            let mut c_size = size;
                            *c_size.hv_get_mut(*axis) = a_list.len() as f32 * axis_allocs_base
                                + i_list.into_iter().sum::<f32>();
                            self.assign_comb_space(c, level, c_size)
                        });
                }
                construct::Type::Surround(surround) => {
                    todo!()
                    // let mut secondary = combs.remove(1);
                    // let mut primary = combs.remove(0);
                    // let s_intervals = self.assign_surround_comb_space(
                    //     &mut primary,
                    //     &mut secondary,
                    //     *surround,
                    //     level,
                    //     size,
                    // );
                    // *combs = vec![primary, secondary];

                    // intervals.clear();
                    // assign_intervals.clear();
                    // Axis::list().for_each(|axis| {
                    //     s_intervals.hv_get(axis).into_iter().for_each(|i| {
                    //         let ia = i.unwrap_or_default();
                    //         intervals.push(ia);
                    //         assign_intervals.push(self.get_alloc_base(axis, ia))
                    //     });
                    // });
                }
                construct::Type::Single => unreachable!(),
            },
        }
    }

    // fn assign_surround_comb_space(
    //     &self,
    //     primary: &mut StrucComb,
    //     secondary: &mut StrucComb,
    //     surround: DataHV<Place>,
    //     level: DataHV<usize>,
    //     size: DataHV<f32>,
    // ) -> DataHV<[Option<usize>; 2]> {
    //     let min_val =
    //         Axis::hv_data().into_map(|axis| self.min_values.hv_get(axis)[*level.hv_get(axis)]);
    //     match primary {
    //         StrucComb::Single {
    //             proto, view, trans, ..
    //         } => {
    //             let area = view.surround_area(surround).unwrap();
    //             let mut p_tvs = DataHV::<TransformValue>::default();
    //             let mut s_size = DataHV::splat(0.0);

    //             let r = Axis::hv_data()
    //                 .zip(proto.get_allocs())
    //                 .into_map(|(axis, mut allocs1)| {
    //                     let area = area.hv_get(axis);
    //                     let allocs2 = allocs1.split_off(area[1]);
    //                     let sub_area = allocs1.split_off(area[0]);

    //                     let (secondary_allocs, secondary_intervals) =
    //                         self.get_comb_allocs(secondary, axis).unwrap();
    //                     let [s_intervals1, s_intervals2] = self
    //                         .get_comps_surround_intervals(
    //                             view.read_surround_edge(surround, axis).unwrap(),
    //                             secondary,
    //                             axis,
    //                         )
    //                         .unwrap()
    //                         .map(|i| match i {
    //                             Some(i) => vec![i],
    //                             None => vec![],
    //                         });

    //                     let p_allocs: Vec<usize> = allocs1
    //                         .iter()
    //                         .chain(sub_area.iter())
    //                         .chain(allocs2.iter())
    //                         .copied()
    //                         .collect();
    //                     let p_val = self.get_allocs_base_total(axis, p_allocs.iter().copied());
    //                     let all_intervals: Vec<usize> = s_intervals1
    //                         .iter()
    //                         .chain(secondary_intervals.iter())
    //                         .chain(s_intervals2.iter())
    //                         .copied()
    //                         .collect();
    //                     let s_val = self.get_allocs_base_total(
    //                         axis,
    //                         allocs1
    //                             .iter()
    //                             .chain(all_intervals.iter())
    //                             .chain(secondary_allocs.iter())
    //                             .chain(allocs2.iter())
    //                             .copied(),
    //                     );

    //                     let min_val = min_val.hv_get(axis);

    //                     if p_val > s_val {
    //                         let a_val = *size.hv_get(axis) / p_allocs.len() as f32;

    //                         *p_tvs.hv_get_mut(axis) = TransformValue {
    //                             assign: p_allocs.into_iter().map(|a| (a, a_val)).collect(),
    //                         };
    //                         *s_size.hv_get_mut(axis) = a_val * sub_area.len() as f32
    //                             - s_intervals1
    //                                 .into_iter()
    //                                 .chain(s_intervals2)
    //                                 .map(|i| self.get_alloc_base(axis, i) * min_val)
    //                                 .sum::<f32>();
    //                     } else {
    //                         let len = *size.hv_get(axis)
    //                             - all_intervals
    //                                 .iter()
    //                                 .map(|i| self.get_alloc_base(axis, *i) * min_val)
    //                                 .sum::<f32>();
    //                         let a_val = len
    //                             / (allocs1.len() + secondary_allocs.len() + allocs2.len()) as f32;

    //                         let a_sub_val = (*size.hv_get(axis)
    //                             - (allocs1.len() + allocs2.len()) as f32 * a_val)
    //                             / sub_area.len() as f32;
    //                         let mut assign: Vec<_> =
    //                             allocs1.into_iter().map(|a| (a, a_val)).collect();
    //                         assign.extend(sub_area.into_iter().map(|a| (a, a_sub_val)));
    //                         assign.extend(allocs2.into_iter().map(|a| (a, a_val)));

    //                         *p_tvs.hv_get_mut(axis) = TransformValue { assign };

    //                         *s_size.hv_get_mut(axis) = a_val * secondary_allocs.len() as f32
    //                             + secondary_intervals
    //                                 .into_iter()
    //                                 .map(|i| self.get_alloc_base(axis, i) * min_val)
    //                                 .sum::<f32>();
    //                     }

    //                     self.get_comps_surround_intervals(
    //                         view.read_surround_edge(surround, axis).unwrap(),
    //                         secondary,
    //                         axis,
    //                     )
    //                     .unwrap()
    //                 });

    //             self.assign_comb_space(secondary, level, s_size);
    //             *trans = Some(p_tvs);
    //             r
    //         }
    //         StrucComb::Complex {
    //             tp,
    //             combs,
    //             intervals,
    //             assign_intervals,
    //             ..
    //         } => match tp {
    //             construct::Type::Scale(c_axis) => {
    //                 *intervals = self.get_combs_axis_intervals(combs, *axis).unwrap();
    //                 *assign_intervals = intervals
    //                     .iter()
    //                     .map(|i| self.get_alloc_base(*axis, *i) * *min_val.hv_get(*axis))
    //                     .collect();

    //                 Axis::list().into_iter().for_each(|axis| {

    //                 })

    //                 if *surround.hv_get(axis.inverse()) == Place::End {
    //                     let (p_allocs, p_intervals) = self.get_comb_allocs_in_surround(&combs[0], secondary, surround, axis)
    //                 }

    //                 todo!()
    //             }
    //         },
    //     }
    // }

    pub fn check_comb_proto(&self, comb: &mut StrucComb) -> Result<DataHV<usize>, Error> {
        let mut result = DataHV::default();
        for axis in Axis::list() {
            loop {
                let (allocs, intervals) = self.get_comb_allocs(comb, axis)?;
                match self.assign_base_trans_value(
                    &allocs.into_iter().chain(intervals).collect(),
                    1.0,
                    axis,
                    0,
                ) {
                    Ok((_, level)) => {
                        *result.hv_get_mut(axis) = level;
                        break;
                    }
                    Err(e) => {
                        if self.reduce_comb(comb, axis) {
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(result)
    }

    pub fn reduce_comb(&self, comb: &mut StrucComb, axis: Axis) -> bool {
        match comb {
            StrucComb::Single { proto, view, .. } => {
                if proto.reduce(axis) {
                    *view = StrucView::new(proto);
                    true
                } else {
                    false
                }
            }
            StrucComb::Complex { tp, combs, .. } => match tp {
                construct::Type::Scale(c_axis) => {
                    if axis == *c_axis {
                        for c in combs.iter_mut() {
                            if self.reduce_comb(c, axis) {
                                return true;
                            }
                        }
                        false
                    } else {
                        let mut list: Vec<_> = combs
                            .iter_mut()
                            .map(|c| (self.get_comb_bases_total(c, axis), c))
                            .collect();
                        list.sort_by(|(v1, _), (v2, _)| v1.partial_cmp(v2).unwrap());
                        for (_, c) in list.into_iter() {
                            if self.reduce_comb(c, axis) {
                                return true;
                            }
                        }
                        false
                    }
                }
                _ => {
                    for c in combs.iter_mut() {
                        if self.reduce_comb(c, axis) {
                            return true;
                        }
                    }
                    false
                }
            },
        }
    }

    pub fn get_comb_allocs(
        &self,
        comb: &StrucComb,
        axis: Axis,
    ) -> Result<(Vec<usize>, Vec<usize>), Error> {
        match comb {
            StrucComb::Single { proto, .. } => Ok((proto.get_axis_allocs(axis), vec![])),
            StrucComb::Complex { tp, combs, .. } => match tp {
                construct::Type::Scale(c_axis) => {
                    let mut allocs = vec![];
                    let mut intervals = vec![];
                    for c in combs {
                        let (a, i) = self.get_comb_allocs(c, axis)?;
                        allocs.push(a);
                        intervals.push(i);
                    }

                    if *c_axis == axis {
                        Ok((
                            allocs.into_iter().flatten().collect(),
                            intervals
                                .into_iter()
                                .flatten()
                                .chain(self.get_combs_axis_intervals(combs, axis)?)
                                .collect(),
                        ))
                    } else {
                        Ok(allocs
                            .into_iter()
                            .zip(intervals)
                            .max_by(|(a1, i1), (a2, i2)| {
                                self.get_allocs_base_total(
                                    axis,
                                    a1.iter().chain(i1.iter()).copied(),
                                )
                                .partial_cmp(&self.get_allocs_base_total(
                                    axis,
                                    a2.iter().chain(i2.iter()).copied(),
                                ))
                                .unwrap()
                            })
                            .unwrap())
                    }
                }
                construct::Type::Surround(surround) => {
                    self.get_comb_allocs_in_surround(&combs[0], &combs[1], *surround, axis)
                }
                construct::Type::Single => unreachable!(),
            },
        }
    }

    pub fn get_comb_allocs_in_surround(
        &self,
        primary: &StrucComb,
        secondary: &StrucComb,
        surround: DataHV<Place>,
        axis: Axis,
    ) -> Result<(Vec<usize>, Vec<usize>), Error> {
        match primary {
            StrucComb::Single {
                name, proto, view, ..
            } => {
                let area = *view
                    .surround_area(surround)
                    .ok_or(Error::Surround {
                        place: surround,
                        comp: name.clone(),
                    })?
                    .hv_get(axis);

                let mut allocs1 = proto.get_allocs().hv_get(axis).to_owned();
                let allocs2 = allocs1.split_off(area[1]);
                let sub_area = allocs1.split_off(area[0]);

                let (secondary_allocs, secondary_intervals) =
                    self.get_comb_allocs(secondary, axis)?;
                let [s_intervals1, s_intervals2] = self
                    .get_comps_surround_intervals(
                        view.read_surround_edge(surround, axis).unwrap(),
                        secondary,
                        axis,
                    )?
                    .map(|i| match i {
                        Some(i) => vec![i],
                        None => vec![],
                    });

                let p_val = self.get_allocs_base_total(
                    axis,
                    allocs1
                        .iter()
                        .chain(sub_area.iter())
                        .chain(allocs2.iter())
                        .copied(),
                );
                let s_val = self.get_allocs_base_total(
                    axis,
                    allocs1
                        .iter()
                        .chain(s_intervals1.iter())
                        .chain(secondary_allocs.iter())
                        .chain(secondary_intervals.iter())
                        .chain(s_intervals2.iter())
                        .chain(allocs2.iter())
                        .copied(),
                );

                let r = if p_val > s_val {
                    allocs1.extend(sub_area);
                    allocs1.extend(allocs2);
                    (allocs1, vec![])
                } else {
                    allocs1.extend(secondary_allocs);
                    allocs1.extend(allocs2);
                    (
                        allocs1,
                        s_intervals1
                            .into_iter()
                            .chain(secondary_intervals)
                            .chain(s_intervals2)
                            .collect(),
                    )
                };
                Ok(r)
            }
            StrucComb::Complex { tp, combs, .. } => match tp {
                construct::Type::Scale(c_axis) => {
                    if *c_axis == axis {
                        let mut axis_intervals =
                            self.get_combs_axis_intervals(combs, axis)?.into_iter();

                        if *surround.hv_get(axis.inverse()) == Place::End {
                            let (mut allocs, mut intervals) = self.get_comb_allocs_in_surround(
                                &combs[0], secondary, surround, axis,
                            )?;
                            for c in combs[1..].iter() {
                                let (al, il) = self.get_comb_allocs(c, axis)?;
                                allocs.extend(al);
                                intervals.push(axis_intervals.next().unwrap());
                                intervals.extend(il);
                            }

                            Ok((allocs, intervals))
                        } else {
                            let (mut allocs, mut intervals) = (vec![], vec![]);
                            for c in combs[..combs.len() - 1].iter() {
                                let (al, il) = self.get_comb_allocs(c, axis)?;
                                allocs.extend(al);
                                intervals.extend(il);
                                intervals.push(axis_intervals.next().unwrap());
                            }
                            let (pa, pi) = self.get_comb_allocs_in_surround(
                                &combs[1], secondary, surround, axis,
                            )?;

                            allocs.extend(pa);
                            intervals.extend(pi);

                            Ok((allocs, intervals))
                        }
                    } else {
                        let (p_index, other) = if *surround.hv_get(axis.inverse()) == Place::End {
                            (0, &combs[1..])
                        } else {
                            (combs.len() - 1, &combs[..combs.len() - 1])
                        };

                        let mut allocs = vec![self.get_comb_allocs_in_surround(
                            &combs[p_index],
                            secondary,
                            surround,
                            axis,
                        )?];
                        for c in other {
                            allocs.push(self.get_comb_allocs(c, axis)?)
                        }

                        Ok(allocs
                            .into_iter()
                            .max_by(|(a1, i1), (a2, i2)| {
                                self.get_allocs_base_total(
                                    axis,
                                    a1.iter().chain(i1.iter()).copied(),
                                )
                                .partial_cmp(&self.get_allocs_base_total(
                                    axis,
                                    a2.iter().chain(i2.iter()).copied(),
                                ))
                                .unwrap()
                            })
                            .unwrap())
                    }
                }
                construct::Type::Surround(c_surround) => {
                    if *c_surround == surround {
                        let merge_axis = match c_surround.v {
                            Place::Mind => Axis::Horizontal,
                            _ => Axis::Vertical,
                        };

                        if merge_axis == axis {
                            let secondary = StrucComb::new_complex(
                                "read_edge".to_string(),
                                construct::Type::Scale(merge_axis),
                                vec![combs[1].clone(), secondary.clone()],
                            );
                            self.get_comb_allocs_in_surround(&combs[0], &secondary, surround, axis)
                        } else {
                            let secondary = match self
                                .get_comb_bases_total(&combs[1], axis)
                                .partial_cmp(&self.get_comb_bases_total(secondary, axis))
                                .unwrap()
                            {
                                std::cmp::Ordering::Greater => &combs[1],
                                _ => secondary,
                            };
                            self.get_comb_allocs_in_surround(primary, secondary, surround, axis)
                        }
                    } else if *c_surround.hv_get(axis) == *surround.hv_get(axis) {
                        Ok(self.primary_allocs_and_intervals(
                            self.get_comb_allocs_in_surround(&combs[0], secondary, surround, axis)?,
                            self.get_comb_allocs_in_surround(&combs[0], &combs[1], surround, axis)?,
                            axis,
                        ))
                    } else {
                        // 飓
                        todo!()
                    }
                }
                construct::Type::Single => unreachable!(),
            },
        }
    }

    pub fn get_combs_axis_intervals(
        &self,
        combs: &Vec<StrucComb>,
        axis: Axis,
    ) -> Result<Vec<usize>, Error> {
        let mut intervals = vec![];
        for (c1, c2) in combs.iter().zip(combs.iter().skip(1)) {
            let axis_symbol = match axis {
                Axis::Horizontal => 'h',
                Axis::Vertical => 'v',
            };
            let edge_attr = format!(
                "{axis_symbol};{}{}",
                c1.get_edge(axis, Place::End)?,
                c2.get_edge(axis, Place::Start)?
            );
            for mv in self.interval_rule.iter() {
                if mv.regex.is_match(&edge_attr) {
                    intervals.push(mv.val);
                    break;
                }
            }
            intervals.push(0);
        }
        Ok(intervals)
    }

    pub fn get_comps_surround_intervals(
        &self,
        surround_edge: [Option<Edge>; 2],
        secondary: &StrucComb,
        axis: Axis,
    ) -> Result<[Option<usize>; 2], Error> {
        let axis_symbol = match axis {
            Axis::Horizontal => 'h',
            Axis::Vertical => 'v',
        };

        let mut edges = vec![None; 2];
        if let Some(edge) = &surround_edge[0] {
            edges[0] = Some(format!(
                "{axis_symbol};{}{}",
                edge,
                secondary.get_edge(axis, Place::Start)?
            ))
        }
        if let Some(edge) = &surround_edge[1] {
            edges[1] = Some(format!(
                "{axis_symbol};{}{}",
                secondary.get_edge(axis, Place::End)?,
                edge,
            ))
        }
        let mut iter = edges.iter().map(|attr| {
            attr.as_ref().map(|attr| {
                for mv in &self.interval_rule {
                    if mv.regex.is_match(&attr) {
                        return mv.val;
                    }
                }
                0
            })
        });

        Ok([iter.next().unwrap(), iter.next().unwrap()])
    }
}
