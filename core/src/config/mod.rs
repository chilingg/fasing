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
    pub interval_limit: DataHV<f32>,

    pub white_area: DataHV<f32>,
    pub white_weights: BTreeMap<Element, f32>,

    pub center: DataHV<Option<f32>>,
    pub center_correction: DataHV<f32>,

    pub central_correction: DataHV<f32>,
    pub peripheral_correction: DataHV<f32>,
    pub cp_trigger: bool,
    pub cp_blacklist: DataHV<BTreeSet<String>>,

    pub place_main_strategy: DataHV<BTreeSet<strategy::PlaceMain>>,

    pub align_edge: DataHV<f32>,
    pub surround_scale: DataHV<f32>,
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
            center_correction: DataHV::splat(2.0),
            central_correction: DataHV::splat(1.0),
            peripheral_correction: DataHV::splat(1.0),
            cp_trigger: true,
            cp_blacklist: Default::default(),
            place_main_strategy: Default::default(),
            align_edge: DataHV::default(),
            surround_scale: DataHV::default(),
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
                            DataHV::splat(0.06),
                            WorkPoint::splat(0.0),
                        )
                        .visual_center(min_len)
                        .0;

                    Axis::list()
                        .into_iter()
                        .filter(|axis| !blacklist.hv_get(*axis).contains(name))
                        .for_each(|axis| {
                            let center = *center.hv_get(axis);
                            // let test1 = tvs.hv_get(axis).length();
                            tvs.hv_get_mut(axis).allowances = algorithm::central_unit_correction(
                                &tvs.hv_get(axis).assigns(),
                                &tvs.hv_get(axis).bases,
                                center,
                                *central_corr.hv_get(axis),
                            );
                            // let test2 = tvs.hv_get(axis).length();
                            tvs.hv_get_mut(axis).allowances = algorithm::peripheral_correction(
                                &tvs.hv_get(axis).assigns(),
                                &tvs.hv_get(axis).bases,
                                center,
                                *peripheral_corr.hv_get(axis),
                            );
                            // let test3 = tvs.hv_get(axis).length();
                            // let wait = 1 + 2 + 3;
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
                            StrucComb::Single { .. } => {}
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
                        let (subarea_assign, init_len) = if s_val > sub_area.iter().sum::<usize>() {
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

                            (s_val as f32 * scale, s_base_len as f32 * scale)
                        } else {
                            let scale = assign / p_base_len as f32;
                            tvs.allowances = tvs
                                .allocs
                                .iter()
                                .map(|&v| v as f32 * (scale - min_val))
                                .collect();

                            (
                                sub_area.iter().sum::<usize>() as f32 * scale,
                                s_base_len as f32 * scale,
                            )
                        };

                        let r = {
                            let surround_scale = *self.surround_scale.hv_get(axis);
                            let corr_vals: Vec<f32> = s_edge.iter().map(|ei| ei.1 .1).collect();
                            let corr_total = corr_vals[0] + corr_vals[1];
                            let max_len = subarea_assign - i_bases.iter().sum::<f32>();

                            let base_len =
                                init_len * (1.0 - surround_scale) + max_len * surround_scale;

                            let s_allowance = max_len - base_len;
                            let mut s_offset = [0.0; 2];
                            let i_allowances = i_allowances.hv_get_mut(axis);
                            *i_allowances = vec![0.0; intervals.len()];

                            let surround = *surround.hv_get(axis);
                            if surround == Place::Mind {
                                let scale = s_allowance / corr_total;
                                i_allowances[0] = corr_vals[0] * scale;
                                i_allowances[1] = corr_vals[1] * scale;
                            } else {
                                if s_edge_key {
                                    i_allowances[0] = s_allowance;
                                } else {
                                    let scale = s_allowance / corr_total;
                                    let front = corr_vals[0] * scale;
                                    let back = corr_vals[1] * scale;

                                    if surround == Place::End {
                                        s_offset[0] = front;
                                        i_allowances[0] = back;
                                    } else {
                                        i_allowances[0] = front;
                                        s_offset[1] = back;
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
                let test1 = p_tvs.map(|t| t.length());
                let test2 = i_bases
                    .as_ref()
                    .zip(i_allowances.as_ref())
                    .zip(s_size.as_ref())
                    .zip(s_offset.as_ref())
                    .into_map(|(((a, b), c), d)| {
                        a.iter().chain(b.iter()).chain(d.iter()).sum::<f32>() + *c
                    });

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
                construct::Type::Surround(surround) => {
                    let s_length = self.get_comb_bases_length(&combs[1], axis)?;

                    match &combs[0] {
                        StrucComb::Single {
                            name, proto, view, ..
                        } => {
                            let surround = *surround;
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

                            let intervals: Vec<i32> = self
                                .get_comps_surround_intervals(
                                    view.read_surround_edge(surround, axis).unwrap(),
                                    &combs[1],
                                    axis,
                                )?
                                .into_iter()
                                .filter_map(|x| x.map(|x| x.0))
                                .collect();

                            let p_val = allocs1 + allocs2 + sub_area;
                            let s_val = lengths_and_intervals(
                                &vec![allocs1, s_length, allocs2],
                                &intervals,
                            );

                            Ok(p_val.max(s_val))
                        }
                        StrucComb::Complex { tp, combs, .. } => todo!(),
                    }
                }
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
            let p_edge_key = sub_area_len == max_len;
            let s_edge_key = s_val == max_len;
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
