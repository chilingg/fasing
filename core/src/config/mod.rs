use crate::{
    algorithm,
    axis::*,
    component::{
        attrs,
        comb::{StrucComb, TransformValue},
        strategy,
        view::*,
    },
    construct::{self, Component, Error, Table},
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InSurround([Place; 2]);

#[derive(Serialize, Deserialize, Clone)]
pub struct MatchValue<T = i32> {
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

#[derive(serde::Serialize, Clone)]
pub struct CompInfo {
    name: String,
    tp: construct::Type,
    bases: DataHV<Vec<i32>>,
    i_attr: DataHV<Vec<String>>,
    i_notes: DataHV<Vec<String>>,
    assign: DataHV<Vec<f32>>,
    offset: DataHV<[f32; 2]>,
}

impl CompInfo {
    pub fn new(name: String, tp: construct::Type) -> Self {
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
    level: DataHV<usize>,
    white_areas: DataHV<[f32; 2]>,
    scale: DataHV<f32>,
    center: [DataHV<f32>; 2],
    comp_infos: Vec<CompInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub min_values: DataHV<Vec<f32>>,

    pub correction_table: Table,
    pub place_replace: BTreeMap<char, BTreeMap<Place, BTreeMap<String, Component>>>,
    pub interval_rule: Vec<MatchValue>,

    pub white_area: DataHV<f32>,
    pub white_weights: BTreeMap<Element, f32>,

    pub center: DataHV<f32>,
    pub center_correction: DataHV<f32>,
    pub central_correction: DataHV<f32>,
    pub peripheral_correction: DataHV<f32>,

    pub place_main_strategy: DataHV<BTreeSet<strategy::PlaceMain>>,

    pub align_edge: DataHV<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DataHV::splat(1.0),
            min_values: DataHV::splat(vec![Self::DEFAULT_MIN_VALUE]),

            correction_table: Table::empty(),
            place_replace: Default::default(),
            interval_rule: Default::default(),

            white_area: DataHV::splat(0.0),
            white_weights: Default::default(),
            center: DataHV::splat(0.5),
            center_correction: DataHV::splat(2.0),
            central_correction: DataHV::splat(1.0),
            peripheral_correction: DataHV::splat(1.0),
            place_main_strategy: Default::default(),
            align_edge: DataHV::default(),
        }
    }
}

impl Config {
    pub const DEFAULT_MIN_VALUE: f32 = 0.05;

    pub fn place_replace_name(
        &self,
        name: &str,
        tp: construct::Type,
        in_tp: Place,
    ) -> Option<Component> {
        self.place_replace
            .get(&tp.symbol())
            .and_then(|pm| pm.get(&in_tp).and_then(|map| map.get(name)))
            .cloned()
    }

    pub fn assign_base_trans_value(
        &self,
        base_length: usize,
        limit: f32,
        edge: [f32; 2],
        axis: Axis,
        min_level: usize,
    ) -> Result<usize, Error> {
        let mins = self.min_values.hv_get(axis);
        let base_len_f = base_length as f32 + edge[0] + edge[1];

        if let Some(r) = mins
            .iter()
            .enumerate()
            .skip(min_level)
            .find_map(|(i, &min)| {
                if base_len_f * min < limit + 0.001 {
                    Some(i)
                } else {
                    None
                }
            })
        {
            Ok(r)
        } else {
            return Err(Error::AxisTransform {
                axis,
                length: limit,
                base_length,
            });
        }
    }

    pub fn get_white_area_weight(&self, elements: &Vec<Element>) -> f32 {
        if elements.is_empty() {
            0.5
        } else if elements.iter().all(|el| *el == Element::Face) {
            1.0
        } else {
            1.0 - elements.iter().fold(1.0, |weight, el| {
                weight * self.white_weights.get(el).unwrap_or(&0.0)
            })
        }
    }

    pub fn get_min_len(&self, levels: DataHV<usize>) -> f32 {
        let unit = self
            .min_values
            .as_ref()
            .zip(levels)
            .into_map(|(list, l)| list.get(l).or(list.last()).unwrap());
        unit.h.min(*unit.v)
    }

    pub fn expand_comb_proto(
        &self,
        comb: &mut StrucComb,
        size: DataHV<f32>,
    ) -> Result<CharInfo, Error> {
        let mut char_info = CharInfo::default();
        let mut min_vals = DataHV::default();
        let mut offsets = DataHV::default();
        let mut assign = DataHV::default();

        for axis in Axis::list() {
            loop {
                let base_length = self.get_comb_bases_length(comb, axis)?;

                let edge_correction = [Place::Start, Place::End].map(|place| {
                    self.get_white_area_weight(
                        &self
                            .get_comb_edge(comb, axis, place)
                            .unwrap()
                            .to_elements(axis, place),
                    )
                });
                let edge_base = edge_correction.map(|c| c * *self.white_area.hv_get(axis));

                match self.assign_base_trans_value(
                    base_length,
                    *self.size.hv_get(axis),
                    edge_base,
                    axis,
                    0,
                ) {
                    Ok(level) => {
                        if level != 0 {
                            println!(
                                "`{}` level as {} in length {}",
                                comb.name(),
                                level,
                                size.hv_get(axis)
                            );
                        }

                        let scale =
                            size.hv_get(axis) / (base_length as f32 + edge_base[0] + edge_base[1]);

                        *char_info.level.hv_get_mut(axis) = level;
                        *char_info.scale.hv_get_mut(axis) = scale;
                        *char_info.white_areas.hv_get_mut(axis) = edge_correction;

                        *min_vals.hv_get_mut(axis) = self.min_values.hv_get(axis)[level];
                        *offsets.hv_get_mut(axis) = edge_base.map(|c| c * scale);
                        *assign.hv_get_mut(axis) = base_length as f32 * scale;

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

        char_info.comb_info = comb.comb_info();
        self.init_comb_space(comb, min_vals, assign, offsets, &mut char_info);

        let min_len = min_vals
            .clone()
            .into_iter()
            .reduce(|a, b| a.min(b))
            .unwrap();
        let center = comb.visual_center(min_len, true).to_hv_data();
        char_info.center[0] = center;

        // let min_len_square = min_val.h.powi(2) + min_val.v.powi(2);
        // Axis::hv_data()
        //     .zip(
        //         proto
        //             .visual_center_in_assign(
        //                 &tvs.map(|t| t.assigns()),
        //                 Default::default(),
        //                 min_len_square,
        //             )
        //             .to_hv_data(),
        //     )
        //     .into_iter()
        //     .for_each(|(axis, center)| {
        //         tvs.hv_get_mut(axis).allowances = algorithm::central_unit_correction(
        //             &tvs.hv_get(axis).allowances,
        //             center,
        //             *self.central_correction.hv_get(axis),
        //         );
        //     });

        // Axis::hv_data()
        //     .zip(
        //         proto
        //             .visual_center_in_assign(
        //                 &tvs.map(|t| t.assigns()),
        //                 Default::default(),
        //                 min_len_square,
        //             )
        //             .to_hv_data(),
        //     )
        //     .into_iter()
        //     .for_each(|(axis, center)| {
        //         tvs.hv_get_mut(axis).allowances = algorithm::peripheral_correction(
        //             &tvs.hv_get(axis).allowances,
        //             center,
        //             *self.peripheral_correction.hv_get(axis),
        //         );
        //     });

        Ok(char_info)
    }

    pub fn init_comb_space(
        &self,
        comb: &mut StrucComb,
        min_val: DataHV<f32>,
        assign: DataHV<f32>,
        offsets: DataHV<[f32; 2]>,
        char_info: &mut CharInfo,
    ) {
        match comb {
            StrucComb::Single {
                proto, trans, name, ..
            } => {
                let tvs = assign
                    .zip(min_val)
                    .zip(proto.get_allocs())
                    .zip(offsets)
                    .into_map(|(((size, min_val), allocs), offset)| {
                        let mut base_total = 0.0;
                        let bases: Vec<f32> = allocs
                            .iter()
                            .map(|&l| {
                                let base_val = l as f32 * min_val;
                                base_total += base_val;
                                base_val
                            })
                            .collect();
                        if base_total == 0.0 {
                            let middle = size / 2.0;
                            TransformValue {
                                allowances: vec![0.0; allocs.len()],
                                bases: vec![0.0; allocs.len()],
                                allocs,
                                offset: offset.map(|v| v + middle),
                            }
                        } else {
                            let scale = size / base_total;
                            let allowances: Vec<f32> =
                                bases.iter().map(|&b| (scale - 1.0) * b).collect();

                            TransformValue {
                                allocs,
                                allowances,
                                bases,
                                offset,
                            }
                        }
                    });

                let mut comp_info = CompInfo::new(name.clone(), construct::Type::Single);
                let allocs = proto.get_allocs();
                Axis::list().into_iter().for_each(|axis| {
                    *comp_info.assign.hv_get_mut(axis) = tvs.hv_get(axis).assigns();
                    *comp_info.bases.hv_get_mut(axis) =
                        allocs.hv_get(axis).iter().map(|v| *v as i32).collect();
                });
                char_info.comp_infos.push(comp_info);

                *trans = Some(tvs);
            }
            StrucComb::Complex {
                tp,
                combs,

                intervals,
                i_allowances,
                i_bases,
                offset: c_offset,
                name,
            } => {
                let mut comp_info = CompInfo::new(name.clone(), *tp);

                match tp {
                    construct::Type::Scale(c_axis) => {
                        *c_offset = offsets;

                        // premary axis
                        let primary_assign_lens: Vec<f32> = {
                            let axis = *c_axis;
                            let min_val = *min_val.hv_get(axis);
                            let length = *assign.hv_get(axis);

                            let comb_base_lengths: Vec<f32> = combs
                                .iter()
                                .map(|comb| {
                                    self.get_comb_bases_length(comb, axis).unwrap() as f32 * min_val
                                })
                                .collect();

                            let (c_intervals, i_attrs, i_notes) =
                                self.get_combs_axis_intervals(combs, axis).unwrap();
                            *comp_info.bases.hv_get_mut(axis) = c_intervals.clone();
                            *comp_info.i_attr.hv_get_mut(axis) = i_attrs;
                            *comp_info.i_notes.hv_get_mut(axis) = i_notes;

                            let i_bases = i_bases.hv_get_mut(axis);
                            *i_bases = c_intervals.iter().map(|&i| i as f32 * min_val).collect();
                            *intervals.hv_get_mut(axis) = c_intervals;

                            let base_len =
                                comb_base_lengths.iter().chain(i_bases.iter()).sum::<f32>();
                            let scale = length / base_len;
                            *i_allowances.hv_get_mut(axis) =
                                i_bases.iter().map(|&b| (scale - 1.0) * b).collect();

                            comb_base_lengths.into_iter().map(|v| v * scale).collect()
                        };

                        // secondary axis
                        let secondary_offsets: Vec<[f32; 2]> = {
                            let axis = c_axis.inverse();
                            let assign = *assign.hv_get(axis);
                            let align_edge = self.align_edge.hv_get(axis);

                            let (lengths, max_b_len) =
                                self.get_axis_combs_lengths(combs, axis).unwrap();
                            let [sinfos, einfos] = [Place::Start, Place::End].map(|place| {
                                self.get_axis_place_info(combs, axis, place, &lengths, max_b_len)
                                    .unwrap()
                            });

                            let align_corr = {
                                let [fc, bc] = [sinfos.iter(), einfos.iter()].map(|iter| {
                                    iter.filter(|i| i.1)
                                        .map(|i| i.3)
                                        .reduce(|a, b| a * b)
                                        .unwrap_or_default()
                                });
                                if fc > bc {
                                    [0.0, fc - bc]
                                } else {
                                    [bc - fc, 0.0]
                                }
                            };

                            sinfos
                                .into_iter()
                                .zip(einfos)
                                .map(|(s, e)| {
                                    let corr_vals = [
                                        match s.1 {
                                            true => 0.0,
                                            false => s.3 + align_corr[0],
                                        },
                                        match e.1 {
                                            true => 0.0,
                                            false => e.3 + align_corr[1],
                                        },
                                    ];

                                    let corr_total = corr_vals[0] + corr_vals[1];

                                    if corr_total == 0.0 {
                                        [0.0; 2]
                                    } else {
                                        let min_val = min_val.hv_get(axis);
                                        let max_len =
                                            (assign - min_val).max(lengths[s.0] as f32 * min_val);

                                        let white_base = self.white_area.hv_get(axis);

                                        // base_len <= max_len < max_b_len
                                        let base_len = {
                                            let start = match 2 {
                                                0 => {
                                                    let assign_ratio =
                                                        assign / (max_b_len + 2) as f32;
                                                    lengths[s.0] as f32 * assign_ratio
                                                }
                                                1 => {
                                                    let base_total = max_len
                                                        + white_base
                                                            * (corr_vals[0] + corr_vals[1]);
                                                    max_len.powi(2) / base_total
                                                }
                                                2 => {
                                                    let base_total = white_base
                                                        * assign
                                                        * (corr_vals[0] + corr_vals[1])
                                                        + lengths[s.0] as f32;
                                                    lengths[s.0] as f32 * assign / base_total
                                                }
                                                _ => unreachable!(),
                                            }
                                            .min(max_len);

                                            start * (1.0 - align_edge) + max_len * align_edge
                                        };

                                        let scale: f32 = (assign - base_len) / corr_total;
                                        corr_vals.map(|val| val * scale)
                                    }
                                })
                                .collect()
                        };

                        combs
                            .iter_mut()
                            .zip(primary_assign_lens)
                            .zip(secondary_offsets)
                            .for_each(|((comb, p_len), s_offset)| {
                                let mut c_offset = DataHV::splat([0.0; 2]);
                                *c_offset.hv_get_mut(c_axis.inverse()) = s_offset;

                                let mut assign = assign.clone();
                                *assign.hv_get_mut(*c_axis) = p_len;
                                *assign.hv_get_mut(c_axis.inverse()) -= s_offset[0] + s_offset[1];

                                self.init_comb_space(comb, min_val, assign, c_offset, char_info)
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
                }
                char_info.comp_infos.push(comp_info);
            }
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
                            .map(|c| (self.get_comb_bases_length(c, axis).unwrap(), c))
                            .collect();
                        list.sort_by(|(v1, _), (v2, _)| v1.cmp(v2));
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

    pub fn get_comb_bases_length(&self, comb: &StrucComb, axis: Axis) -> Result<usize, Error> {
        fn lengths_and_intervals(lengths: &Vec<usize>, intervals: &Vec<i32>) -> usize {
            let inter = intervals.iter().sum::<i32>();
            let val = lengths.iter().sum::<usize>() as i32 + inter;

            assert!(val >= 0);
            val as usize
        }

        match comb {
            StrucComb::Single { proto, .. } => Ok(proto.get_axis_allocs(axis).iter().sum()),
            StrucComb::Complex { tp, combs, .. } => match tp {
                construct::Type::Scale(c_axis) => {
                    let mut lengths = vec![];
                    for c in combs {
                        let length = self.get_comb_bases_length(c, axis)?;
                        lengths.push(length);
                    }

                    if *c_axis == axis {
                        Ok(lengths_and_intervals(
                            &lengths,
                            &self.get_combs_axis_intervals(combs, axis)?.0,
                        ))
                    } else {
                        Ok(lengths.into_iter().max().unwrap())
                    }
                }
                construct::Type::Surround(place) => todo!(),
                construct::Type::Single => unreachable!(),
            },
        }
    }

    // pub fn get_comb_allocs_in_surround(
    //     &self,
    //     primary: &StrucComb,
    //     secondary: &StrucComb,
    //     surround: DataHV<Place>,
    //     axis: Axis,
    // ) -> Result<(Vec<usize>, Vec<i32>), Error> {
    //     match primary {
    //         StrucComb::Single {
    //             name, proto, view, ..
    //         } => {
    //             let area = *view
    //                 .surround_area(surround)
    //                 .ok_or(Error::Surround {
    //                     place: surround,
    //                     comp: name.clone(),
    //                 })?
    //                 .hv_get(axis);

    //             let mut allocs1 = proto.get_allocs().hv_get(axis).to_owned();
    //             let allocs2 = allocs1.split_off(area[1]);
    //             let sub_area = allocs1.split_off(area[0]);

    //             let (secondary_allocs, secondary_intervals) =
    //                 self.get_comb_allocs(secondary, axis)?;
    //             let [s_intervals1, s_intervals2] = self
    //                 .get_comps_surround_intervals(
    //                     view.read_surround_edge(surround, axis).unwrap(),
    //                     secondary,
    //                     axis,
    //                 )?
    //                 .map(|i| match i {
    //                     Some(i) => vec![i],
    //                     None => vec![],
    //                 });

    //             let p_val = self.get_allocs_base_total(
    //                 axis,
    //                 allocs1
    //                     .iter()
    //                     .chain(sub_area.iter())
    //                     .chain(allocs2.iter())
    //                     .copied(),
    //             );
    //             let s_val = self.get_allocs_base_total(
    //                 axis,
    //                 allocs1
    //                     .iter()
    //                     .chain(s_intervals1.iter())
    //                     .chain(secondary_allocs.iter())
    //                     .chain(secondary_intervals.iter())
    //                     .chain(s_intervals2.iter())
    //                     .chain(allocs2.iter())
    //                     .copied(),
    //             );

    //             let r = if p_val > s_val {
    //                 allocs1.extend(sub_area);
    //                 allocs1.extend(allocs2);
    //                 (allocs1, vec![])
    //             } else {
    //                 allocs1.extend(secondary_allocs);
    //                 allocs1.extend(allocs2);
    //                 (
    //                     allocs1,
    //                     s_intervals1
    //                         .into_iter()
    //                         .chain(secondary_intervals)
    //                         .chain(s_intervals2)
    //                         .collect(),
    //                 )
    //             };
    //             Ok(r)
    //         }
    //         StrucComb::Complex { tp, combs, .. } => match tp {
    //             construct::Type::Scale(c_axis) => {
    //                 if *c_axis == axis {
    //                     let mut axis_intervals =
    //                         self.get_combs_axis_intervals(combs, axis)?.into_iter();

    //                     if *surround.hv_get(axis.inverse()) == Place::End {
    //                         let (mut allocs, mut intervals) = self.get_comb_allocs_in_surround(
    //                             &combs[0], secondary, surround, axis,
    //                         )?;
    //                         for c in combs[1..].iter() {
    //                             let (al, il) = self.get_comb_allocs(c, axis)?;
    //                             allocs.extend(al);
    //                             intervals.push(axis_intervals.next().unwrap());
    //                             intervals.extend(il);
    //                         }

    //                         Ok((allocs, intervals))
    //                     } else {
    //                         let (mut allocs, mut intervals) = (vec![], vec![]);
    //                         for c in combs[..combs.len() - 1].iter() {
    //                             let (al, il) = self.get_comb_allocs(c, axis)?;
    //                             allocs.extend(al);
    //                             intervals.extend(il);
    //                             intervals.push(axis_intervals.next().unwrap());
    //                         }
    //                         let (pa, pi) = self.get_comb_allocs_in_surround(
    //                             &combs[1], secondary, surround, axis,
    //                         )?;

    //                         allocs.extend(pa);
    //                         intervals.extend(pi);

    //                         Ok((allocs, intervals))
    //                     }
    //                 } else {
    //                     let (p_index, other) = if *surround.hv_get(axis.inverse()) == Place::End {
    //                         (0, &combs[1..])
    //                     } else {
    //                         (combs.len() - 1, &combs[..combs.len() - 1])
    //                     };

    //                     let mut allocs = vec![self.get_comb_allocs_in_surround(
    //                         &combs[p_index],
    //                         secondary,
    //                         surround,
    //                         axis,
    //                     )?];
    //                     for c in other {
    //                         allocs.push(self.get_comb_allocs(c, axis)?)
    //                     }

    //                     Ok(allocs
    //                         .into_iter()
    //                         .max_by(|(a1, i1), (a2, i2)| {
    //                             self.get_allocs_base_total(
    //                                 axis,
    //                                 a1.iter().chain(i1.iter()).copied(),
    //                             )
    //                             .partial_cmp(&self.get_allocs_base_total(
    //                                 axis,
    //                                 a2.iter().chain(i2.iter()).copied(),
    //                             ))
    //                             .unwrap()
    //                         })
    //                         .unwrap())
    //                 }
    //             }
    //             construct::Type::Surround(c_surround) => {
    //                 if *c_surround == surround {
    //                     let merge_axis = match c_surround.v {
    //                         Place::Mind => Axis::Horizontal,
    //                         _ => Axis::Vertical,
    //                     };

    //                     if merge_axis == axis {
    //                         let secondary = StrucComb::new_complex(
    //                             "read_edge".to_string(),
    //                             construct::Type::Scale(merge_axis),
    //                             vec![combs[1].clone(), secondary.clone()],
    //                         );
    //                         self.get_comb_allocs_in_surround(&combs[0], &secondary, surround, axis)
    //                     } else {
    //                         let secondary = match self
    //                             .get_comb_bases_total(&combs[1], axis)
    //                             .partial_cmp(&self.get_comb_bases_total(secondary, axis))
    //                             .unwrap()
    //                         {
    //                             std::cmp::Ordering::Greater => &combs[1],
    //                             _ => secondary,
    //                         };
    //                         self.get_comb_allocs_in_surround(primary, secondary, surround, axis)
    //                     }
    //                 } else if *c_surround.hv_get(axis) == *surround.hv_get(axis) {
    //                     Ok(self.primary_allocs_and_intervals(
    //                         self.get_comb_allocs_in_surround(&combs[0], secondary, surround, axis)?,
    //                         self.get_comb_allocs_in_surround(&combs[0], &combs[1], surround, axis)?,
    //                         axis,
    //                     ))
    //                 } else {
    //                     // 飓
    //                     todo!()
    //                 }
    //             }
    //             construct::Type::Single => unreachable!(),
    //         },
    //     }
    // }

    fn get_axis_combs_lengths(
        &self,
        combs: &Vec<StrucComb>,
        axis: Axis,
    ) -> Result<(Vec<usize>, usize), Error> {
        let mut len_list = vec![];
        for c in combs {
            len_list.push(self.get_comb_bases_length(c, axis)?);
        }
        let max_len = *len_list.iter().max().unwrap();
        Ok((len_list, max_len))
    }

    // index, is_key, edge, corr_val
    fn get_axis_place_info(
        &self,
        combs: &Vec<StrucComb>,
        axis: Axis,
        place: Place,
        lengths: &Vec<usize>,
        max_len: usize,
    ) -> Result<Vec<(usize, bool, Edge, f32)>, Error> {
        fn process(
            cfg: &Config,
            combs: &Vec<StrucComb>,
            axis: Axis,
            place: Place,
            lengths: &Vec<usize>,
            max_len: usize,
        ) -> Result<Vec<(usize, bool, Edge, f32)>, Error> {
            let mut key_list = vec![];
            let mut non_key_list = vec![];
            let mut key_corr_val: f32 = 1.0;
            let mut is_container = BTreeMap::<usize, bool>::new();

            let strategy = cfg.place_main_strategy.hv_get(axis.inverse());
            let acute = 1.0
                - cfg
                    .white_weights
                    .values()
                    .copied()
                    .reduce(f32::max)
                    .unwrap_or(1.0);

            for (i, comb) in combs.iter().enumerate() {
                let edge = cfg.get_comb_edge(comb, axis, place)?;
                let corr_val = cfg.get_white_area_weight(&edge.to_elements(axis, place));

                if lengths[i] == max_len {
                    key_corr_val = key_corr_val.min(corr_val);
                    key_list.push((i, edge, corr_val));
                } else {
                    non_key_list.push((i, edge, corr_val));
                }
            }

            (0..non_key_list.len()).rev().for_each(|i| {
                if lengths[non_key_list[i].0] == 0 {
                    return;
                }

                let mut ok = non_key_list[i].2 < key_corr_val;

                if !ok && strategy.contains(&strategy::PlaceMain::Equal) {
                    if !strategy.contains(&strategy::PlaceMain::Zero) || key_corr_val != 1.0 {
                        ok |= non_key_list[i].2 == key_corr_val;
                    }
                }

                if !ok && strategy.contains(&strategy::PlaceMain::Acute) {
                    ok |= non_key_list[i].2 == acute;
                }

                if strategy.contains(&strategy::PlaceMain::Contain) {
                    let index = non_key_list[i].0;
                    let mut check_place = vec![];
                    let mut surrounded = false;

                    if index != 0 {
                        check_place.push(Place::Start);
                        let j = index - 1;
                        surrounded |= *is_container.entry(j).or_insert(
                            cfg.get_comb_edge(&combs[j], axis.inverse(), check_place[0].inverse())
                                .unwrap()
                                .is_container_edge(axis.inverse(), check_place[0].inverse()),
                        );
                    }
                    if index + 1 != combs.len() {
                        check_place.push(Place::End);
                        let j = index + 1;
                        surrounded |= *is_container.entry(j).or_insert(
                            cfg.get_comb_edge(&combs[j], axis.inverse(), Place::Start)
                                .unwrap()
                                .is_container_edge(axis.inverse(), Place::Start),
                        );
                    }

                    if surrounded {
                        ok = false;
                    }

                    if !ok {
                        for place in check_place {
                            if cfg
                                .get_comb_edge(&combs[index], axis.inverse(), place)
                                .unwrap()
                                .is_container_edge(axis.inverse(), place)
                            {
                                ok = true;
                                break;
                            }
                        }
                    }
                };

                if ok {
                    key_list.push(non_key_list.remove(i))
                }
            });

            let mut r: Vec<(usize, bool, Edge, f32)> = key_list
                .into_iter()
                .map(|(i, e, c)| (i, true, e, c))
                .chain(non_key_list.into_iter().map(|(i, e, c)| (i, false, e, c)))
                .collect();

            r.sort_by_key(|(i, _, _, _)| *i);

            Ok(r)
        }

        let mut r = process(self, combs, axis, place, lengths, max_len)?;

        if self
            .place_main_strategy
            .hv_get(axis.inverse())
            .contains(&strategy::PlaceMain::Both)
        {
            let other = process(self, combs, axis, place.inverse(), lengths, max_len)?;
            r.iter_mut().zip(other).for_each(|(a, o)| {
                if a.1 && !o.1 {
                    a.1 = false;
                }
            })
        }

        Ok(r)
    }

    pub fn get_comb_edge(&self, comb: &StrucComb, axis: Axis, place: Place) -> Result<Edge, Error> {
        match comb {
            StrucComb::Single { view, .. } => Ok(view.read_edge(axis, place)),
            StrucComb::Complex { tp, combs, .. } => match tp {
                construct::Type::Scale(c_axis) => {
                    if *c_axis == axis {
                        let c = match place {
                            Place::Start => &combs[0],
                            Place::End => combs.last().unwrap(),
                            Place::Mind => unreachable!(),
                        };
                        self.get_comb_edge(c, axis, place)
                    } else {
                        let (length, max_len) = self.get_axis_combs_lengths(combs, axis)?;
                        let edge = self
                            .get_axis_place_info(combs, axis, place, &length, max_len)?
                            .into_iter()
                            .filter_map(|(_, k, mut e, _)| match k {
                                true => Some(e),
                                false => {
                                    e.line.iter_mut().for_each(|line| {
                                        std::mem::swap(&mut line.0, &mut line.1);
                                        if place == Place::Start {
                                            line.0.clear();
                                        } else {
                                            line.1.clear();
                                        }
                                    });
                                    e.alloc = 1;
                                    e.real = [true, e.real[0]];
                                    Some(e)
                                }
                            })
                            .reduce(|a, b| a.connect(b))
                            .unwrap();
                        Ok(edge)
                    }
                }
                construct::Type::Surround(_) => todo!(),
                construct::Type::Single => unreachable!(),
            },
        }
    }

    pub fn get_combs_axis_intervals(
        &self,
        combs: &Vec<StrucComb>,
        axis: Axis,
    ) -> Result<(Vec<i32>, Vec<String>, Vec<String>), Error> {
        let mut intervals = vec![];
        let mut attrs = vec![];
        let mut notes = vec![];

        for (c1, c2) in combs.iter().zip(combs.iter().skip(1)) {
            let axis_symbol = match axis {
                Axis::Horizontal => 'h',
                Axis::Vertical => 'v',
            };
            let edge_attr = format!(
                "{axis_symbol};{}{}",
                self.get_comb_edge(c1, axis, Place::End)?,
                self.get_comb_edge(c2, axis, Place::Start)?
            );
            let (val, note) = self
                .interval_rule
                .iter()
                .find_map(|rule| {
                    if rule.regex.is_match(&edge_attr) {
                        Some((rule.val, rule.note.clone()))
                    } else {
                        None
                    }
                })
                .unwrap_or((0, "Default".to_string()));
            intervals.push(val);
            attrs.push(edge_attr);
            notes.push(note)
        }
        Ok((intervals, attrs, notes))
    }

    pub fn get_comps_surround_intervals(
        &self,
        surround_edge: [Option<Edge>; 2],
        secondary: &StrucComb,
        axis: Axis,
    ) -> Result<[Option<i32>; 2], Error> {
        let axis_symbol = match axis {
            Axis::Horizontal => 'h',
            Axis::Vertical => 'v',
        };

        let mut edges = vec![None; 2];
        if let Some(edge) = &surround_edge[0] {
            edges[0] = Some(format!(
                "{axis_symbol};{}{}",
                edge,
                self.get_comb_edge(secondary, axis, Place::Start)?
            ))
        }
        if let Some(edge) = &surround_edge[1] {
            edges[1] = Some(format!(
                "{axis_symbol};{}{}",
                self.get_comb_edge(secondary, axis, Place::End)?,
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
