use crate::{
    algorithm,
    axis::*,
    component::{
        comb::{StrucComb, SurroundValue, TransformValue},
        strategy,
        view::*,
    },
    construct::{self, space::WorkPoint, Component, Error, Table},
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
    pub level: DataHV<usize>,
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
    pub interval_limit: DataHV<f32>,

    pub white_area: DataHV<f32>,
    pub white_weights: BTreeMap<Element, f32>,

    pub center: DataHV<Option<f32>>,
    pub center_correction: DataHV<f32>,

    pub comp_center: DataHV<Option<f32>>,
    pub comp_center_correction: DataHV<f32>,

    pub central_correction: DataHV<f32>,
    pub peripheral_correction: DataHV<f32>,
    pub cp_trigger: bool,
    pub cp_blacklist: DataHV<BTreeSet<String>>,

    pub place_strategy: DataHV<BTreeMap<Place, BTreeMap<Place, BTreeSet<strategy::PlaceMain>>>>,

    pub surround_main_strategy: BTreeMap<char, DataHV<BTreeSet<strategy::PlaceMain>>>,

    pub align_edge: DataHV<f32>,
    pub surround_scale: DataHV<f32>,

    pub reduce_trigger: DataHV<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DataHV::splat(1.0),
            min_values: DataHV::splat(vec![Self::DEFAULT_MIN_VALUE]),

            correction_table: Table::empty(),
            place_replace: Default::default(),
            interval_rule: Default::default(),
            interval_limit: DataHV::splat(1.0),

            white_area: DataHV::splat(0.0),
            white_weights: Default::default(),
            center: DataHV::splat(None),
            center_correction: DataHV::splat(1.0),
            comp_center: DataHV::splat(None),
            comp_center_correction: DataHV::splat(1.0),
            central_correction: DataHV::splat(1.0),
            peripheral_correction: DataHV::splat(1.0),
            cp_trigger: true,
            cp_blacklist: Default::default(),
            place_strategy: Default::default(),
            surround_main_strategy: Default::default(),
            align_edge: DataHV::default(),
            surround_scale: DataHV::default(),
            reduce_trigger: DataHV::splat(0.0),
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
                if base_len_f * min < limit + algorithm::NORMAL_OFFSET {
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
                    *size.hv_get(axis),
                    edge_base,
                    axis,
                    0,
                ) {
                    Ok(level) => {
                        let scale =
                            size.hv_get(axis) / (base_length as f32 + edge_base[0] + edge_base[1]);
                        if scale <= *self.reduce_trigger.hv_get(axis)
                            && self.reduce_comb(comb, axis)
                        {
                            continue;
                        }

                        // if level != 0 {
                        //     println!(
                        //         "`{}` level as {} in length {}",
                        //         comb.name(),
                        //         level,
                        //         size.hv_get(axis)
                        //     );
                        // }

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
                        }
                        return Err(e);
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

        self.process_comb_space(comb, min_vals);

        Ok(char_info)
    }

    pub fn process_comb_space(&self, comb: &mut StrucComb, min_vals: DataHV<f32>) {
        fn process(
            comb: &mut StrucComb,
            central_corr: &DataHV<f32>,
            peripheral_corr: &DataHV<f32>,
            min_len: f32,
            blacklist: &DataHV<BTreeSet<String>>,
        ) {
            match comb {
                StrucComb::Single {
                    name, proto, trans, ..
                } => {
                    let tvs = trans.as_mut().unwrap();
                    let center = proto
                        .to_work_in_assign(
                            DataHV::new(&tvs.h.assigns(), &tvs.v.assigns()),
                            DataHV::splat(min_len),
                            WorkPoint::splat(0.0),
                        )
                        .visual_center(min_len)
                        .0;

                    Axis::list()
                        .into_iter()
                        .filter(|axis| !blacklist.hv_get(*axis).contains(name))
                        .for_each(|axis| {
                            let center = *center.hv_get(axis);
                            // // let test1 = tvs.hv_get(axis).length();
                            // tvs.hv_get_mut(axis).allowances = algorithm::central_unit_correction(
                            //     &tvs.hv_get(axis).assigns(),
                            //     &tvs.hv_get(axis).bases,
                            //     center,
                            //     *central_corr.hv_get(axis),
                            // );
                            // // let test2 = tvs.hv_get(axis).length();
                            // tvs.hv_get_mut(axis).allowances = algorithm::peripheral_correction(
                            //     &tvs.hv_get(axis).assigns(),
                            //     &tvs.hv_get(axis).bases,
                            //     center,
                            //     *peripheral_corr.hv_get(axis),
                            // );
                            // // let test3 = tvs.hv_get(axis).length();
                            // // let wait = 1 + 2 + 3;
                            tvs.hv_get_mut(axis).allowances = algorithm::peripheral_and_central(
                                &tvs.hv_get(axis).assigns(),
                                &tvs.hv_get(axis).bases,
                                center,
                                *peripheral_corr.hv_get(axis),
                                *central_corr.hv_get(axis),
                            );
                        })
                }
                StrucComb::Complex { tp, combs, .. } => match tp {
                    construct::Type::Scale(_) => combs.iter_mut().for_each(|c| {
                        process(c, central_corr, peripheral_corr, min_len, blacklist)
                    }),
                    construct::Type::Surround(splace) => {
                        process(
                            &mut combs[1],
                            central_corr,
                            peripheral_corr,
                            min_len,
                            blacklist,
                        );
                        match &mut combs[0] {
                            StrucComb::Single { .. } => {}
                            StrucComb::Complex { combs, tp, .. } => match &tp {
                                construct::Type::Scale(c_axis) => {
                                    let index =
                                        StrucComb::axis_surround_comb_in(*c_axis, *splace, combs);
                                    combs
                                        .iter_mut()
                                        .enumerate()
                                        .filter(|(i, _)| *i == index)
                                        .for_each(|(_, c)| {
                                            process(
                                                c,
                                                central_corr,
                                                peripheral_corr,
                                                min_len,
                                                blacklist,
                                            );
                                        });
                                }
                                construct::Type::Surround(_) => process(
                                    &mut combs[1],
                                    central_corr,
                                    peripheral_corr,
                                    min_len,
                                    blacklist,
                                ),
                                construct::Type::Single => unreachable!(),
                            },
                        }
                    }
                    construct::Type::Single => unreachable!(),
                },
            }
        }

        let min_len = min_vals.h.min(min_vals.v);
        if self.cp_trigger {
            process(
                comb,
                &self.central_correction,
                &self.peripheral_correction,
                min_len,
                &self.cp_blacklist,
            )
        }

        comb.center_correction(
            self.center,
            self.center_correction,
            self.comp_center,
            self.comp_center_correction,
            min_vals,
            min_len,
            self.interval_limit,
        );
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
                            let mut scale = size / base_total;
                            if (scale - 1.0).abs() < algorithm::NORMAL_OFFSET {
                                scale = 1.0;
                            }
                            assert!(scale >= 1.0);
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
                *c_offset = offsets;

                match &tp {
                    construct::Type::Scale(c_axis) => {
                        let mut comp_info = CompInfo::new(name.clone(), *tp);

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
                            let mut scale = length / base_len;
                            if (scale - 1.0).abs() <= algorithm::NORMAL_OFFSET {
                                scale = 1.0;
                            }
                            debug_assert!(scale >= 1.0, "{scale}");
                            *i_allowances.hv_get_mut(axis) =
                                i_bases.iter().map(|&b| (scale - 1.0) * b).collect();

                            comb_base_lengths.into_iter().map(|v| v * scale).collect()
                        };

                        // secondary axis
                        let secondary_offsets: Vec<[f32; 2]> = {
                            let axis = c_axis.inverse();
                            let assign = *assign.hv_get(axis);
                            let align_edge = self.align_edge.hv_get(axis).clamp(-1.0, 1.0);

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
                                        let min_len = lengths[s.0] as f32 * min_val;
                                        let max_len = (assign
                                            - min_val
                                                * [s.1, e.1].iter().filter(|b| !**b).count()
                                                    as f32)
                                            .max(min_len);

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
                                            .min(max_len)
                                            .max(min_len);

                                            if align_edge < 0.0 {
                                                let align_edge = align_edge.abs();
                                                start * (1.0 - align_edge) + min_len * align_edge
                                            } else {
                                                start * (1.0 - align_edge) + max_len * align_edge
                                            }
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

                                self.init_comb_space(comb, min_val, assign, c_offset, char_info);
                            });
                        char_info.comp_infos.push(comp_info);
                    }
                    construct::Type::Surround(surround) => {
                        let mut secondary = combs.remove(1);
                        let mut primary = combs.remove(0);
                        let surround = *surround;

                        (*intervals, *i_allowances, *i_bases) = self.init_surround_combs_space(
                            name,
                            &mut primary,
                            &mut secondary,
                            surround,
                            min_val,
                            assign,
                            char_info,
                        );

                        *combs = vec![primary, secondary];
                    }
                    construct::Type::Single => unreachable!(),
                }
            }
        }
    }

    fn init_surround_combs_space(
        &self,
        name: &str,
        primary: &mut StrucComb,
        secondary: &mut StrucComb,
        surround: DataHV<Place>,
        min_val: DataHV<f32>,
        assign: DataHV<f32>,
        char_info: &mut CharInfo,
    ) -> (DataHV<Vec<i32>>, DataHV<Vec<f32>>, DataHV<Vec<f32>>) {
        let mut intervals = DataHV::splat(vec![]);
        let mut i_allowances = DataHV::splat(vec![]);
        let mut i_bases = DataHV::splat(vec![]);

        let mut comp_info = CompInfo::new(name.to_string(), construct::Type::Surround(surround));

        match primary {
            StrucComb::Single {
                proto,
                view,
                trans,
                name: p_name,
                ..
            } => {
                let surround_val = self
                    .get_surround_values(p_name, &proto.get_allocs(), view, surround, &secondary)
                    .unwrap();
                let mut p_tvs = DataHV::<TransformValue>::default();

                let (s_size, s_offset) = Axis::hv_data()
                    .zip(surround_val)
                    .into_map(|(axis, sval)| {
                        let min_val = *min_val.hv_get(axis);
                        let assign = *assign.hv_get(axis);

                        let SurroundValue {
                            p_allocs1,
                            sub_area,
                            p_allocs2,
                            interval_info,
                            s_base_len,
                            s_val,

                            p_edge_key,
                            p_edge,
                            s_edge_key,
                            s_edge,
                            ..
                        } = sval;

                        let p_allocs: Vec<usize> = p_allocs1
                            .iter()
                            .chain(sub_area.iter())
                            .chain(p_allocs2.iter())
                            .copied()
                            .collect();
                        let p_base_len = p_allocs.iter().sum::<usize>();

                        let f_num = p_allocs1.len();
                        let b_num = p_allocs2.len();

                        let intervals = intervals.hv_get_mut(axis);
                        *intervals = interval_info
                            .map(|i| match i {
                                Some((i, attr, note)) => {
                                    comp_info.bases.hv_get_mut(axis).push(i);
                                    comp_info.i_attr.hv_get_mut(axis).push(attr);
                                    comp_info.i_notes.hv_get_mut(axis).push(note);
                                    Some(i)
                                }
                                None => None,
                            })
                            .into_iter()
                            .filter_map(|x| x)
                            .collect();
                        let i_bases = i_bases.hv_get_mut(axis);
                        *i_bases = intervals.iter().map(|i| *i as f32 * min_val).collect();

                        let tvs = p_tvs.hv_get_mut(axis);
                        tvs.allocs = p_allocs;
                        tvs.bases = tvs.allocs.iter().map(|&v| v as f32 * min_val).collect();
                        let subarea_assign = if s_val > sub_area.iter().sum::<usize>() {
                            let base_total =
                                p_allocs1.iter().chain(p_allocs2.iter()).sum::<usize>() + s_val;
                            let scale = assign / base_total as f32;

                            let mut subarea_assign = s_val as f32 * scale;
                            let mut surr_scale =
                                subarea_assign / sub_area.iter().sum::<usize>() as f32;
                            if !p_edge_key {
                                match p_edge.iter().next() {
                                    Some((place, (_, _))) => {
                                        assert!(s_val > 1);
                                        subarea_assign = (s_val - 1) as f32 * scale;
                                        surr_scale =
                                            subarea_assign / sub_area.iter().sum::<usize>() as f32;

                                        let index = match place {
                                            Place::Start => 0,
                                            Place::End => 1,
                                            Place::Mind => unreachable!(),
                                        };
                                        tvs.offset[index] = 1.0 * scale;
                                    }
                                    None => {}
                                }
                            }

                            tvs.allowances = p_allocs1
                                .into_iter()
                                .map(|v| v as f32 * (scale - min_val))
                                .chain(
                                    sub_area
                                        .into_iter()
                                        .map(|v| v as f32 * (surr_scale - min_val)),
                                )
                                .chain(p_allocs2.into_iter().map(|v| v as f32 * (scale - min_val)))
                                .collect();

                            s_val as f32 * scale
                        } else {
                            let scale = assign / p_base_len as f32;
                            tvs.allowances = tvs
                                .allocs
                                .iter()
                                .map(|&v| v as f32 * (scale - min_val))
                                .collect();

                            sub_area.iter().sum::<usize>() as f32 * scale
                        };

                        let r = {
                            let surround_scale = *self.surround_scale.hv_get(axis);
                            let surround = *surround.hv_get(axis);
                            let mut corr_vals: Vec<f32> = s_edge.iter().map(|ei| ei.1 .1).collect();
                            match surround {
                                Place::Mind => intervals.iter().enumerate().for_each(|(i, b)| {
                                    if *b == 0 {
                                        corr_vals[i] = 0.0
                                    }
                                }),
                                Place::End => {
                                    if intervals[0] == 0 {
                                        corr_vals[1] = 0.0
                                    }
                                }
                                Place::Start => {
                                    if intervals[0] == 0 {
                                        corr_vals[0] = 0.0
                                    }
                                }
                            }

                            let corr_total = corr_vals[0] + corr_vals[1];
                            let mut max_len = subarea_assign - i_bases.iter().sum::<f32>();
                            let min_len = s_base_len as f32 * min_val;

                            let base_len = {
                                let offset_corr = if s_edge_key {
                                    0.0
                                } else {
                                    if surround != Place::Mind {
                                        max_len -= min_val;
                                    }
                                    match surround {
                                        Place::Mind => 0.0,
                                        Place::End => corr_vals[0],
                                        Place::Start => corr_vals[1],
                                    }
                                };

                                let base_total = s_base_len as f32
                                    + *self.white_area.hv_get(axis) * subarea_assign * offset_corr
                                    + intervals.iter().sum::<i32>() as f32;
                                let init_len = (s_base_len as f32 * subarea_assign / base_total)
                                    .min(max_len)
                                    .max(min_len);
                                init_len * (1.0 - surround_scale) + max_len * surround_scale
                            };

                            let s_allowance = max_len - base_len;
                            let mut s_offset = [0.0; 2];
                            let i_allowances = i_allowances.hv_get_mut(axis);
                            *i_allowances = vec![0.0; intervals.len()];

                            if surround == Place::Mind {
                                if corr_total != 0.0 {
                                    let scale = s_allowance / corr_total;
                                    i_allowances[0] = corr_vals[0] * scale;
                                    i_allowances[1] = corr_vals[1] * scale;
                                }
                            } else {
                                if s_edge_key {
                                    i_allowances[0] = s_allowance;
                                } else {
                                    let scale = s_allowance / corr_total;
                                    let front = corr_vals[0] * scale;
                                    let back = corr_vals[1] * scale;

                                    if surround == Place::End {
                                        s_offset[0] = front + min_val;
                                        i_allowances[0] = back;
                                    } else {
                                        i_allowances[0] = front;
                                        s_offset[1] = back + min_val;
                                    }
                                }
                            }

                            (base_len, s_offset)
                        };

                        if *surround.hv_get(axis) != Place::End {
                            let inter: usize = tvs.allocs.iter().take(f_num).sum();
                            let base: f32 = tvs.bases.iter().take(f_num).sum();
                            let allowance: f32 = tvs.allowances.iter().take(f_num).sum();

                            intervals.insert(0, inter as i32);
                            i_bases.insert(0, base);
                            i_allowances.hv_get_mut(axis).insert(0, allowance);
                        }
                        if *surround.hv_get(axis) != Place::Start {
                            let inter: usize = tvs.allocs.iter().rev().take(b_num).sum();
                            let base: f32 = tvs.bases.iter().rev().take(b_num).sum();
                            let allowance: f32 = tvs.allowances.iter().rev().take(b_num).sum();

                            intervals.push(inter as i32);
                            i_bases.push(base);
                            i_allowances.hv_get_mut(axis).push(allowance);
                        }

                        r
                    })
                    .unzip();

                // Test
                // let test1 = p_tvs.map(|t| t.length());
                // let test2 = i_bases
                //     .as_ref()
                //     .zip(i_allowances.as_ref())
                //     .zip(s_size.as_ref())
                //     .zip(s_offset.as_ref())
                //     .into_map(|(((a, b), c), d)| {
                //         a.iter().chain(b.iter()).chain(d.iter()).sum::<f32>() + *c
                //     });

                let mut p_comp_info = CompInfo::new(p_name.clone(), construct::Type::Single);
                let p_allocs = proto.get_allocs();
                Axis::list().into_iter().for_each(|axis| {
                    *p_comp_info.assign.hv_get_mut(axis) = p_tvs.hv_get(axis).assigns();
                    *p_comp_info.bases.hv_get_mut(axis) =
                        p_allocs.hv_get(axis).iter().map(|v| *v as i32).collect();
                });
                char_info.comp_infos.push(p_comp_info);
                *trans = Some(p_tvs);

                self.init_comb_space(secondary, min_val, s_size, s_offset, char_info);
            }
            StrucComb::Complex { combs: s_combs, .. } => todo!(),
        }

        char_info.comp_infos.push(comp_info);

        (intervals, i_allowances, i_bases)
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
                    let mut list: Vec<_> = combs
                        .iter_mut()
                        .map(|c| (self.get_comb_bases_length(c, axis).unwrap(), c))
                        .collect();

                    if axis == *c_axis {
                        list.sort_by(|(v1, _), (v2, _)| v1.cmp(v2));
                        let mut reduce_size = None;
                        for (csize, c) in list.into_iter().rev() {
                            if let Some(size) = reduce_size {
                                if csize == size {
                                    self.reduce_comb(c, axis);
                                } else {
                                    return true;
                                }
                            } else if self.reduce_comb(c, axis) {
                                reduce_size = Some(csize);
                            }
                        }
                        reduce_size.is_some()
                    } else {
                        let max = list
                            .iter()
                            .map(|(l, _)| *l)
                            .reduce(|a, b| a.max(b))
                            .unwrap_or_default();
                        list.retain(|(l, _)| *l == max);

                        let mut new_list: Vec<_> = list.iter().map(|(_, c)| (*c).clone()).collect();
                        for c in new_list.iter_mut() {
                            if !self.reduce_comb(c, axis) {
                                return false;
                            }
                        }
                        list.into_iter().zip(new_list).for_each(|((_, c), newc)| {
                            *c = newc;
                        });

                        // for (_, c) in list.iter_mut() {
                        //     if !self.reduce_comb(c, axis) {
                        //         return false;
                        //     }
                        // }
                        true
                    }
                }
                construct::Type::Surround(surround) => {
                    let mut secondary = combs.pop().unwrap();
                    match combs.pop().unwrap() {
                        StrucComb::Single {
                            name,
                            mut proto,
                            mut view,
                            ..
                        } => {
                            let area = *view.surround_area(*surround).unwrap().hv_get(axis);
                            let surr_len = proto.get_allocs().hv_get(axis)[area[0]..area[1]]
                                .iter()
                                .sum::<usize>();
                            let complex_len =
                                self.get_comb_bases_length(&secondary, axis).unwrap() + 1;

                            let ok = if complex_len >= surr_len {
                                // self.reduce_comb(&mut secondary, axis)
                                let s_ok = self.reduce_comb(&mut secondary, axis);
                                s_ok || {
                                    let p_ok = proto.reduce(axis);
                                    if p_ok {
                                        view = StrucView::new(&proto);
                                    }
                                    p_ok
                                }
                            } else {
                                let ok = proto.reduce(axis);
                                if ok {
                                    view = StrucView::new(&proto);
                                }
                                ok
                            };

                            combs.push(StrucComb::Single {
                                name,
                                proto,
                                view,
                                trans: None,
                            });
                            combs.push(secondary);

                            ok
                        }
                        StrucComb::Complex { tp, .. } => todo!(),
                    }
                }
                construct::Type::Single => unreachable!(),
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

        fn get_comb_bases_length_in_surround(
            config: &Config,
            primary: &StrucComb,
            secondary: &StrucComb,
            surround: DataHV<Place>,
            axis: Axis,
        ) -> Result<usize, Error> {
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

                    let mut allocs = proto.get_allocs().hv_get(axis).to_owned();
                    let allocs2: usize = allocs.split_off(area[1]).into_iter().sum();
                    let sub_area: usize = allocs.split_off(area[0]).into_iter().sum();
                    let allocs1: usize = allocs.into_iter().sum();

                    let intervals: Vec<i32> = config
                        .get_comps_surround_intervals(
                            view.read_surround_edge(surround, axis).unwrap(),
                            secondary,
                            axis,
                        )?
                        .into_iter()
                        .filter_map(|x| x.map(|x| x.0))
                        .collect();

                    let p_val = allocs1 + allocs2 + sub_area;
                    let s_length = config.get_comb_bases_length(secondary, axis)?;
                    let s_val =
                        lengths_and_intervals(&vec![allocs1, s_length, allocs2], &intervals);

                    Ok(p_val.max(s_val))
                }
                StrucComb::Complex { tp, combs, .. } => todo!(),
            }
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
                construct::Type::Surround(surround) => {
                    get_comb_bases_length_in_surround(self, &combs[0], &combs[1], *surround, axis)
                }
                construct::Type::Single => unreachable!(),
            },
        }
    }

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

    fn get_surround_values(
        &self,
        s_name: &str,
        allocs: &DataHV<Vec<usize>>,
        view: &StrucView,
        surround: DataHV<Place>,
        secondary: &StrucComb,
    ) -> Result<DataHV<SurroundValue>, Error> {
        let area = view.surround_area(surround).ok_or(Error::Surround {
            place: surround,
            comp: s_name.to_string(),
        })?;

        let mut surr_val = DataHV::default();
        for axis in Axis::list() {
            let area = area.hv_get(axis);

            let mut p_allocs1 = allocs.hv_get(axis).clone();
            let p_allocs2 = p_allocs1.split_off(area[1]);
            let sub_area = p_allocs1.split_off(area[0]);
            let sub_area_len = sub_area.iter().sum::<usize>();

            let interval_info = self.get_comps_surround_intervals(
                view.read_surround_edge(surround, axis).unwrap(),
                &secondary,
                axis,
            )?;
            let intervals: Vec<i32> = interval_info
                .clone()
                .map(|i| i.map(|i| i.0))
                .iter()
                .filter_map(|i| *i)
                .collect();

            let s_base_len = self.get_comb_bases_length(&secondary, axis).unwrap();
            let s_val = {
                let val = s_base_len as i32 + intervals.iter().sum::<i32>();
                assert!(val > 0);
                val as usize
            };

            let max_len = sub_area_len.max(s_val);
            let mut p_edge_key = sub_area_len == max_len;
            let mut s_edge_key = s_val == max_len;
            let p_edge: BTreeMap<Place, ([Option<Edge>; 2], f32)> = Place::start_and_end()
                .into_iter()
                .filter(|&place_state| *surround.hv_get(axis) == place_state.inverse())
                .map(|place| {
                    let p_edge = view.read_edge_in_surround(surround, axis, place).unwrap();

                    let p_edge_corr = self.get_white_area_weight(
                        &p_edge
                            .iter()
                            .filter_map(|e| e.clone())
                            .reduce(|a, b| a.connect(b))
                            .unwrap()
                            .to_elements(axis, place),
                    );
                    BTreeMap::from([(place, (p_edge, p_edge_corr))])
                })
                .next()
                .unwrap_or_default();
            let s_edge: BTreeMap<Place, (Edge, f32)> = Place::start_and_end()
                .into_iter()
                .map(|place| {
                    let edge = self.get_comb_edge(secondary, axis, place).unwrap();
                    let corr = self.get_white_area_weight(&edge.to_elements(axis, place));
                    (place, (edge, corr))
                })
                .collect();

            if p_edge_key ^ s_edge_key {
                Place::start_and_end().iter().for_each(|place| {
                    if p_edge.contains_key(&place) {
                        let (main_corr, sub_corr, sub_state) = if p_edge_key {
                            if s_base_len == 0 {
                                return;
                            }
                            (p_edge[place].1, s_edge[place].1, &mut s_edge_key)
                        } else {
                            (s_edge[place].1, p_edge[place].1, &mut p_edge_key)
                        };

                        if main_corr > sub_corr {
                            *sub_state = true;
                        }
                    }
                })
            }

            *surr_val.hv_get_mut(axis) = SurroundValue {
                p_allocs1,
                sub_area,
                p_allocs2,
                interval_info,
                s_base_len,
                s_val,
                p_edge_key,
                p_edge,
                s_edge_key,
                s_edge,
            };
        }

        Ok(surr_val)
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

            let plane = 1.0
                - cfg
                    .white_weights
                    .values()
                    .copied()
                    .reduce(f32::min)
                    .unwrap_or(0.0);

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

                let in_place = match non_key_list[i].0 {
                    0 => Place::Start,
                    n if n == combs.len() - 1 => Place::End,
                    _ => Place::Mind,
                };

                let strategy = cfg
                    .place_strategy
                    .hv_get(axis)
                    .get(&in_place)
                    .and_then(|data| data.get(&place))
                    .cloned()
                    .unwrap_or_default();
                let corr_val = non_key_list[i].2;
                let mut ok = false;

                if !strategy.contains(&strategy::PlaceMain::NoPlane) || corr_val < plane {
                    if !ok && !strategy.contains(&strategy::PlaceMain::NonLess) {
                        ok |= corr_val < key_corr_val;
                    }
                    if !ok && strategy.contains(&strategy::PlaceMain::Equal) {
                        ok |= corr_val == key_corr_val;
                    }
                    if !ok && strategy.contains(&strategy::PlaceMain::Acute) {
                        let acute = 1.0
                            - cfg
                                .white_weights
                                .values()
                                .copied()
                                .reduce(f32::max)
                                .unwrap_or(1.0);
                        ok |= non_key_list[i].2 == acute;
                    }
                }
                if !ok && strategy.contains(&strategy::PlaceMain::AlignPlane) {
                    ok |= key_corr_val >= plane;
                }
                if !ok && in_place != Place::End && strategy.contains(&strategy::PlaceMain::Contain)
                {
                    let index = non_key_list[i].0;
                    ok |= cfg
                        .get_comb_edge(&combs[index], axis.inverse(), Place::End)
                        .unwrap()
                        .is_container_edge(axis.inverse(), Place::End);
                }
                if ok
                    && in_place != Place::Start
                    && strategy.contains(&strategy::PlaceMain::InContain)
                {
                    let index = non_key_list[i].0 - 1;
                    ok = !cfg
                        .get_comb_edge(&combs[index], axis.inverse(), Place::End)
                        .unwrap()
                        .is_container_edge(axis.inverse(), Place::End);
                }

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

        let both_check: Vec<usize> = r
            .iter()
            .filter_map(|(i, ok, _, _)| {
                if *ok {
                    let in_place = match i {
                        0 => Place::Start,
                        n if *n == combs.len() - 1 => Place::End,
                        _ => Place::Mind,
                    };
                    let checked = self
                        .place_strategy
                        .hv_get(axis)
                        .get(&in_place)
                        .and_then(|data| {
                            data.get(&place)
                                .and_then(|set| set.get(&strategy::PlaceMain::Both))
                        })
                        .is_some();
                    if checked {
                        Some(*i)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        if !both_check.is_empty() {
            let other = process(self, combs, axis, place.inverse(), lengths, max_len)?;
            both_check.into_iter().for_each(|i| {
                if !other[i].1 {
                    r[i].1 = false
                }
            });
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
                construct::Type::Surround(surround) => {
                    if surround.hv_get(axis).inverse() == place {
                        match &combs[0] {
                            StrucComb::Single {
                                view, proto, name, ..
                            } => {
                                let surround_val = self.get_surround_values(
                                    name,
                                    &proto.get_allocs(),
                                    view,
                                    *surround,
                                    &combs[1],
                                )?;
                                let sval = surround_val.hv_get(axis);

                                let mut p_edge = sval.p_edge[&place].0.clone();
                                if !sval.p_edge_key {
                                    p_edge.iter_mut().for_each(|edge| {
                                        *edge = edge.take().map(|mut e| {
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
                                            e
                                        });
                                    });
                                }
                                let mut s_edge = sval.s_edge[&place].0.clone();
                                if !sval.s_edge_key {
                                    s_edge.line.iter_mut().for_each(|line| {
                                        std::mem::swap(&mut line.0, &mut line.1);
                                        if place == Place::Start {
                                            line.0.clear();
                                        } else {
                                            line.1.clear();
                                        }
                                    });
                                    s_edge.alloc = 1;
                                    s_edge.real = [true, s_edge.real[0]];
                                }

                                Ok(p_edge[0]
                                    .clone()
                                    .unwrap_or_default()
                                    .connect(s_edge)
                                    .connect(p_edge[1].clone().unwrap_or_default()))
                            }
                            StrucComb::Complex { name, .. } => todo!(),
                        }
                    } else {
                        self.get_comb_edge(&combs[0], axis, place)
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
    ) -> Result<[Option<(i32, String, String)>; 2], Error> {
        let axis_symbol = match axis {
            Axis::Horizontal => 'h',
            Axis::Vertical => 'v',
        };

        let mut edges = vec![None; 2];
        if let Some(edge) = &surround_edge[0] {
            edges[0] = Some(format!(
                "{axis_symbol};{}{}",
                edge,
                self.get_comb_edge(secondary, axis, Place::Start)?,
            ))
        }
        if let Some(edge) = &surround_edge[1] {
            edges[1] = Some(format!(
                "{axis_symbol};{}{}",
                self.get_comb_edge(secondary, axis, Place::End)?,
                edge,
            ))
        }
        let mut iter = edges.into_iter().map(|attr| {
            attr.map(|attr| {
                for mv in &self.interval_rule {
                    if mv.regex.is_match(&attr) {
                        return (mv.val, attr, mv.note.clone());
                    }
                }
                return (0, attr, "Default".to_string());
            })
        });

        Ok([iter.next().unwrap(), iter.next().unwrap()])
    }
}
