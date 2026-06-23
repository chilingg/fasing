use super::algorithm as al;
use crate::{
    base::*,
    combination::{CompData, StrucComb, StrucProto, attrs, comb, view},
    config,
};

// use serde::{Deserialize, Serialize};
use serde_json as sj;
use std::collections::HashMap;

pub type CtrlEdgeAreaSettings = Vec<config::EdgeCheck<[Option<String>; 2]>>;

pub fn ctrl_edge_area(
    comb: &mut StrucComb,
    settings: &CtrlEdgeAreaSettings,
    ad_edges: &DataHV<[Option<view::EdgeShape>; 2]>,
    ad_interval: &DataHV<[Option<usize>; 2]>,
) {
    fn get_match<'a, T>(
        axis: Axis,
        edges: &DataHV<[view::EdgeShape; 2]>,
        adjacencies: &DataHV<[bool; 2]>,
        ad_edges: &DataHV<[Option<view::EdgeShape>; 2]>,
        ad_interval: &DataHV<[Option<usize>; 2]>,
        settings: T,
    ) -> [Option<config::Exps<'a>>; 2]
    where
        T: std::iter::IntoIterator<Item = &'a config::EdgeCheck<[Option<String>; 2]>>,
    {
        let mut result = [Option::<&String>::None; 2];
        for check in settings {
            if check
                .setup
                .iter()
                .zip(result.iter())
                .any(|(s, r)| s.is_some() && r.is_some())
            {
                continue;
            }

            match check.is_match(axis, edges, ad_edges, ad_interval, adjacencies, |_, _| None) {
                Ok(true) => {
                    for i in 0..2 {
                        if check.setup[i].is_some() {
                            result[i] = check.setup[i].as_ref();
                        }
                    }
                }
                Ok(false) => {}
                Err(e) => log::error!("In edge area ctrl setting: {e}"),
            }
        }

        result.map(|str| {
            str.and_then(|str| match config::Exps::try_from(str.as_str()) {
                Err(e) => {
                    log::error!("{e}");
                    None
                }
                Ok(exps) => Some(exps),
            })
        })
    }

    let c_setting = comb.get_edge_area_setting().unwrap_or_default();
    match &mut comb.cdata {
        CompData::Single {
            assigns,
            view,
            proto,
            ..
        } => {
            let edges = Axis::hv()
                .into_map(|axis| Side::fb().map(|from| view.get_edge(axis, from).to_shape()));
            let mut weights =
                proto.subarea_weight(assigns.map(|list| list.iter().map(|v| v.total()).sum()));
            let mut multipliers = weights.map(|w| vec![1.0; w.len()]);

            Axis::list().into_iter().for_each(|axis| {
                let assigns = assigns.hv_get_mut(axis);
                if assigns.len() < 2 {
                    return;
                }

                let setup = get_match(
                    axis,
                    &edges,
                    &comb.attrs.get::<attrs::Adjacencies>().unwrap(),
                    ad_edges,
                    ad_interval,
                    c_setting.iter().chain(settings.iter()),
                );
                let weights = weights.hv_get_mut(axis);
                let m_list = multipliers.hv_get_mut(axis);
                let mut weight_total = weights.iter().sum::<f32>();
                let mut vars = HashMap::from([
                    // ("wt", weights.iter().sum::<f32>() as f64),
                ]);

                let mut other_weight_total = weight_total;
                let mut is_null = vec![];
                for i in 0..2 {
                    let idx = [0, weights.len() - 1][i];
                    if let Some(setup) = &setup[i] {
                        let old_val = assigns[idx].total();
                        vars.insert("val", old_val as f64);
                        match setup.eval(&vars) {
                            Err(e) => log::error!("{e}"),
                            Ok(val) => {
                                weight_total -= weights[idx];
                                let ratio = val as f32 / old_val;
                                weights[idx] *= ratio;
                                m_list[idx] = ratio;
                                other_weight_total -= weights[idx];
                            }
                        };
                    } else {
                        is_null.push(idx);
                    }
                }
                let rario = other_weight_total.max(0.0) / weight_total;
                let n = weights.len() - 2;
                weights.iter_mut().skip(1).take(n).for_each(|w| *w *= rario);
                is_null.iter().for_each(|i| weights[*i] *= rario);

                if is_null.len() != 2 {
                    al::reallocate_on_weights(assigns.iter_mut(), &weights, 1.0);
                }
            });
            proto.set_area_weight_multipliers(&multipliers);
        }
        CompData::Scale {
            comps,
            axis,
            intervals,
            ..
        } => {
            let axis = *axis;
            for i in 0..comps.len() {
                let mut ad_edges = ad_edges.clone();
                let mut ad_interval = ad_interval.clone();
                *ad_interval.hv_get_mut(axis.inverse()) = [None; 2];
                if i != 0 {
                    ad_edges.hv_get_mut(axis)[0] =
                        Some(comps[i - 1].get_edge(axis, Side::Back, true).to_shape());
                    ad_interval.hv_get_mut(axis)[0] = Some(intervals[i - 1]);
                }
                if i + 1 != comps.len() {
                    ad_edges.hv_get_mut(axis)[1] =
                        Some(comps[i + 1].get_edge(axis, Side::Front, true).to_shape());
                    ad_interval.hv_get_mut(axis)[0] = Some(intervals[i]);
                }
                ctrl_edge_area(&mut comps[i], settings, &ad_edges, &ad_interval);
            }
        }
        CompData::Surround {
            comps,
            surround,
            intervals_val,
            intervals,
            ..
        } => {
            {
                let ad_interval =
                    surround
                        .zip(intervals.as_ref())
                        .map(|(surr, i_val)| match surr {
                            Section::Start => [Some(i_val[0]), None],
                            Section::Middle => [Some(i_val[0]), Some(i_val[1])],
                            Section::End => [None, Some(i_val[1])],
                        });
                ctrl_edge_area(&mut comps[1], settings, &Default::default(), &ad_interval);
            }

            let (primary, secondery) = comps.split_at_mut(1);
            let mut first = true;
            comb::surround::for_each_primary_rev(&mut primary[0], surround, |primary| {
                if first {
                    first = false;
                    match &mut primary.cdata {
                        CompData::Single {
                            view,
                            assigns,
                            proto,
                            ..
                        } => {
                            let edges = Axis::hv().into_map(|axis| {
                                Side::fb().map(|from| view.get_edge(axis, from).to_shape())
                            });
                            let area = view.surround_area(*surround).unwrap();
                            let mut multipliers = assigns.map(|w| vec![1.0; w.len()]);

                            Axis::list().into_iter().for_each(|axis| {
                                let area = area.hv_get(axis);
                                let assigns = assigns.hv_get_mut(axis);
                                let intervals_val = intervals_val.hv_get_mut(axis);
                                let multipliers = multipliers.hv_get_mut(axis);
                                let blanks = primary.blanks.hv_get_mut(axis);

                                if area[0] != 0 || area[1] + 1 != assigns.len() {
                                    let setup = get_match(
                                        axis,
                                        &edges,
                                        &comb.attrs.get::<attrs::Adjacencies>().unwrap(),
                                        &Default::default(),
                                        ad_interval,
                                        settings,
                                    );

                                    let area_p_assign = assigns[area[0]..area[1]]
                                        .iter()
                                        .chain(blanks.iter())
                                        .sum::<AssignVal>();
                                    let mut s_assign_len =
                                        secondery[0].get_assign_value(axis, true);
                                    let mut area_s_assign =
                                        s_assign_len + intervals_val.iter().sum::<AssignVal>();

                                    let mut weights: Vec<f32> =
                                        assigns.iter().map(|v| v.total()).collect();
                                    let mut vars = HashMap::from([
                                        // ("wt", weights.iter().sum::<f32>() as f64),
                                    ]);
                                    if area[0] != 0
                                        && let Some(setup) = &setup[0]
                                    {
                                        vars.insert("val", assigns[0].total() as f64);
                                        match setup.eval(&vars) {
                                            Err(e) => log::error!("{e}"),
                                            Ok(val) => {
                                                let ratio = val as f32 / assigns[0].total();
                                                weights[0] *= ratio;
                                                multipliers[0] = ratio;
                                            }
                                        };
                                    }
                                    if area[1] != assigns.len()
                                        && let Some(setup) = &setup[1]
                                    {
                                        let idx = weights.len() - 1;
                                        vars.insert("val", assigns[idx].total() as f64);
                                        match setup.eval(&vars) {
                                            Err(e) => log::error!("{e}"),
                                            Ok(val) => {
                                                let ratio = val as f32 / assigns[idx].total();
                                                weights[idx] *= ratio;
                                                multipliers[idx] = ratio;
                                            }
                                        };
                                    }

                                    let new_len = if area_p_assign.base > area_s_assign.base {
                                        al::reallocate_on_weights(
                                            assigns.iter_mut(),
                                            &weights,
                                            1.0,
                                        );
                                        assigns[area[0]..area[1]].iter().sum::<AssignVal>().total()
                                            + blanks.iter().sum::<AssignVal>().total()
                                    } else {
                                        weights.drain(area[0]..area[1]);
                                        weights.push(area_s_assign.total());
                                        let range = area[0]..area[1];
                                        let new_assigns: Vec<_> = assigns
                                            .iter_mut()
                                            .enumerate()
                                            .filter_map(|(i, v)| match range.contains(&i) {
                                                true => None,
                                                false => Some(v),
                                            })
                                            .chain(std::iter::once(&mut area_s_assign))
                                            .collect();
                                        al::reallocate_on_weights(
                                            new_assigns.into_iter(),
                                            &weights,
                                            1.0,
                                        );

                                        let mut changes = vec![true; range.len()];
                                        changes.extend([false; 2].into_iter());
                                        al::reassign_in(
                                            assigns[range].iter_mut().chain(blanks.iter_mut()),
                                            &changes,
                                            area_s_assign.total(),
                                        )
                                        .unwrap();
                                        area_s_assign.total()
                                    };
                                    al::reassign_in(
                                        intervals_val
                                            .iter_mut()
                                            .chain(std::iter::once(&mut s_assign_len)),
                                        &[false, false, true],
                                        new_len,
                                    )
                                    .unwrap();
                                    secondery[0].reassign_space(s_assign_len.total(), false, axis);
                                }
                            });
                            proto.set_area_weight_multipliers(&multipliers);
                        }
                        _ => todo!(), // surround in
                    }
                } else {
                    // ctrl_edge_area(primary, settings);
                    unreachable!()
                }
            });
        }
    }
}

pub fn ctrl_subarea(comb: &mut StrucComb, value: &sj::Value) {
    fn set_area_weight_multipliers(
        proto: &mut StrucProto,
        factor: DataHV<Option<f32>>,
        zero: DataHV<f32>,
        area_n: DataHV<usize>,
    ) {
        let mut line_weights = proto.subarea_line_weight(&area_n.into_map(|n| vec![1.0; n]), zero);
        for axis in Axis::list() {
            let factor = factor.hv_get(axis).unwrap_or(0.0);
            line_weights
                .hv_get_mut(axis)
                .iter_mut()
                .for_each(|v| *v = (*v - 1.0) * factor + 1.0);
        }
        proto.set_area_weight_multipliers(&line_weights);
    }

    match &mut comb.cdata {
        CompData::Single { proto, assigns, .. } => {
            let settings = config::get_axis_val(value).into_map(|val| match val {
                None => (None, 0.0),
                Some(val) => (
                    val.get("factor")
                        .and_then(|val| val.as_f64().map(|val| val as f32)),
                    val.get("zero").and_then(|val| val.as_f64()).unwrap_or(1.0) as f32,
                ),
            });
            let (factor, zero) = settings.unzip();

            set_area_weight_multipliers(proto, factor, zero, assigns.map(|list| list.len()));
            let weights =
                proto.subarea_weight(assigns.map(|list| list.iter().map(|v| v.total()).sum()));

            Axis::list().into_iter().for_each(|axis| {
                if factor.hv_get(axis).is_some() {
                    let assigns = assigns.hv_get_mut(axis);
                    let weights = weights.hv_get(axis);
                    al::reallocate_on_weights(assigns.iter_mut(), weights, 1.0);
                }
            });
        }
        CompData::Scale { comps, .. } => comps.iter_mut().for_each(|c| ctrl_subarea(c, value)),
        CompData::Surround {
            comps,
            surround,
            intervals,
            intervals_val,
            ..
        } => {
            let (primary, secondary) = comps.split_at_mut(1);

            let secondary = &mut secondary[0];
            let (surr_len, surr_b_len) = Axis::hv()
                .into_map(|axis| {
                    let val_len = secondary.get_assign_value(axis, true).total()
                        + intervals_val.hv_get(axis).iter().sum::<AssignVal>().total();
                    let b_len = secondary.get_bases_length(axis, true)
                        + intervals.hv_get(axis).iter().sum::<usize>();
                    (val_len, b_len)
                })
                .unzip();

            let mut first = true;
            comb::surround::for_each_primary_rev(&mut primary[0], surround, |primary| {
                if first {
                    first = false;
                    let blanks = Axis::hv().into_map(|axis| primary.get_blank_base(axis));

                    match &mut primary.cdata {
                        CompData::Single {
                            proto,
                            view,
                            assigns,
                            ..
                        } => {
                            let (factor, zero) = config::get_axis_val(value)
                                .into_map(|val| match val {
                                    None => (None, 0.0),
                                    Some(val) => (
                                        val.get("factor")
                                            .and_then(|val| val.as_f64().map(|val| val as f32)),
                                        val.get("zero").and_then(|val| val.as_f64()).unwrap_or(1.0)
                                            as f32,
                                    ),
                                })
                                .unzip();
                            set_area_weight_multipliers(
                                proto,
                                factor,
                                zero,
                                assigns.map(|list| list.len()),
                            );

                            let area = view.surround_area(*surround).unwrap();
                            let allocs = proto.allocation_space();
                            let extra = Axis::hv().into_map(|axis| {
                                let area = area.hv_get(axis);
                                let surr_b_len = *surr_b_len.hv_get(axis);
                                let allocs = allocs.hv_get(axis);
                                let surr_len = *surr_len.hv_get(axis);
                                let blanks = blanks.hv_get(axis);
                                let p_area_len = allocs[area[0]..area[1]]
                                    .iter()
                                    .chain(blanks.iter())
                                    .sum::<usize>();

                                if p_area_len >= surr_b_len {
                                    0.0
                                } else {
                                    let extra_b = surr_b_len - p_area_len;
                                    let extra = surr_len / surr_b_len as f32 * extra_b as f32;
                                    0.0_f32.max(
                                        extra
                                            - primary
                                                .blanks
                                                .hv_get(axis)
                                                .iter()
                                                .sum::<AssignVal>()
                                                .total(),
                                    )
                                }
                            });
                            let original_val = Axis::hv().into_map(|axis| {
                                let extra = *extra.hv_get(axis);
                                let area = area.hv_get(axis);
                                let mut assigns = assigns.hv_get(axis).clone();
                                let area_total =
                                    assigns[area[0]..area[1]].iter().sum::<AssignVal>().total();

                                al::reassign(
                                    assigns[area[0]..area[1]].iter_mut(),
                                    area_total - extra,
                                )
                                .unwrap();
                                assigns
                            });

                            let new_assigns = Axis::hv().into_map(|axis| {
                                if factor.hv_get(axis).is_some() {
                                    let area = area.hv_get(axis);
                                    let extra = *extra.hv_get(axis);

                                    let weights = Axis::hv().into_map(|axis| {
                                        let mut size = DataHV::default();
                                        *size.hv_get_mut(axis) = assigns
                                            .hv_get(axis)
                                            .iter()
                                            .map(|v| v.total())
                                            .sum::<f32>();
                                        *size.hv_get_mut(axis.inverse()) = original_val
                                            .hv_get(axis.inverse())
                                            .iter()
                                            .map(|v| v.total())
                                            .sum::<f32>();
                                        proto.subarea_weight(size).hv_get_into(axis)
                                    });
                                    let mut new_assigns = original_val.hv_get(axis).clone();
                                    al::reallocate_on_weights(
                                        new_assigns.iter_mut(),
                                        &weights.hv_get(axis),
                                        1.0,
                                    );
                                    let new_area_len = new_assigns[area[0]..area[1]]
                                        .iter()
                                        .sum::<AssignVal>()
                                        .total()
                                        + extra;
                                    al::reassign(
                                        new_assigns[area[0]..area[1]].iter_mut(),
                                        new_area_len,
                                    )
                                    .unwrap();
                                    al::reallocate_on_weights(
                                        new_assigns[area[0]..area[1]].iter_mut(),
                                        &weights.hv_get(axis)[area[0]..area[1]],
                                        1.0,
                                    );

                                    let intervals = intervals_val.hv_get_mut(axis);
                                    let mut s_assign = secondary.get_assign_value(axis, true);
                                    let iter =
                                        intervals.iter_mut().chain(std::iter::once(&mut s_assign));
                                    al::reassign_in(
                                        iter,
                                        &[false, false, true],
                                        new_area_len
                                            + primary
                                                .blanks
                                                .hv_get(axis)
                                                .iter()
                                                .sum::<AssignVal>()
                                                .total(),
                                    )
                                    .unwrap();
                                    secondary.reassign_space(s_assign.total(), true, axis);

                                    Some(new_assigns)
                                } else {
                                    None
                                }
                            });
                            assigns
                                .as_mut()
                                .zip(new_assigns)
                                .into_iter()
                                .for_each(|(a, n)| {
                                    if let Some(new) = n {
                                        *a = new
                                    }
                                });
                        }
                        _ => todo!(), // surround in
                    }
                } else {
                    unreachable!()
                }
            });
            ctrl_subarea(&mut comps[1], value);
        }
    }
}

pub fn ctrl_subcomp(comb: &mut StrucComb, value: &sj::Value) {
    static DEFAULT_SETTING: (f32, bool, [Option<String>; 3]) = (1.0, false, [const { None }; 3]);
    static DEFAULT_SURR_SETTING: (f32, [f32; 2]) = (1.0, [0.0; 2]);

    match &mut comb.cdata {
        CompData::Single { .. } => {}
        CompData::Scale {
            comps,
            axis,
            intervals,
            intervals_val,
            ..
        } => {
            let axis = *axis;
            let records = comb::scale::flatten(comps, intervals, intervals_val, axis);

            comps.iter_mut().for_each(|c| {
                ctrl_subcomp(c, value);
            });

            let settings = config::get_axis_val(value).into_map(|val| match val {
                None => DEFAULT_SETTING.clone(),
                Some(val) => (
                    val.get("factor")
                        .and_then(|val| val.as_f64().map(|val| val as f32))
                        .unwrap_or(DEFAULT_SETTING.0),
                    val.get("same")
                        .and_then(|val| val.as_bool())
                        .unwrap_or(DEFAULT_SETTING.1),
                    val.get("section")
                        .and_then(|val| sj::from_value::<[Option<String>; 3]>(val.clone()).ok())
                        .unwrap_or(DEFAULT_SETTING.2.clone()),
                ),
            });

            let (factor, same, section_weight) = settings.hv_get(axis);
            let mut comp_assigns: Vec<AssignVal> = vec![];

            let line_weights: Vec<f32> =
                if *same && comps.iter().skip(1).all(|c| c.name == comps[0].name) {
                    vec![1.0; comps.len()]
                } else {
                    comps
                        .iter()
                        .map(|c| {
                            let mut c_val = c.get_complexity(axis, true) as f32;
                            if c_val != 0.0 {
                                let c_size =
                                    Axis::hv().into_map(|axis| c.get_assign_value(axis, false));
                                comp_assigns.push(*c_size.hv_get(axis));

                                let line_len = c.get_line_weight();
                                let line_weight = line_len.zip(c_size).into_map(|(line, len)| {
                                    if len.total() == 0.0 {
                                        0.0
                                    } else {
                                        line / len.total()
                                    }
                                });

                                c_val *= line_weight.reduce(|w, h| w + h);
                                if let CompData::Single { proto, .. } = &c.cdata {
                                    let factors = proto
                                        .attrs
                                        .get::<attrs::LineWeight>()
                                        .unwrap_or(DataHV::splat(None));

                                    c_val *= factors.hv_get(axis).unwrap_or(1.0);
                                }
                            } else {
                                comp_assigns.push(c.get_assign_value(axis, false));
                            }
                            c_val
                        })
                        .collect()
                };

            al::reallocate_on_weights(comp_assigns.iter_mut(), &line_weights, *factor);

            let c_base_list: Vec<_> = comps
                .iter()
                .map(|c| c.get_bases_length(axis, false))
                .collect();
            let base_len = c_base_list.iter().sum::<usize>();
            let assigns_len = comp_assigns.iter().sum::<AssignVal>().total();
            let mut vars = HashMap::from([
                ("base_len", base_len as f64),
                ("assigns_len", assigns_len as f64),
            ]);

            let mut remaining = assigns_len;
            let mut other_total = 0.0;
            let mut weights = comp_assigns.iter().map(|v| v.total()).collect::<Vec<_>>();
            let idexes: Vec<usize> = weights
                .iter_mut()
                .enumerate()
                .filter_map(|(i, w)| {
                    if let Some(r) = section_weight[Section::from_idx(i, comps.len()).n()]
                        .as_ref()
                        .map(|str| -> anyhow::Result<f32> {
                            vars.insert("val", *w as f64);
                            vars.insert("base_val", c_base_list[i] as f64);
                            let exps = config::Exps::try_from(str.as_str())?;
                            Ok(exps.eval(&vars)? as f32)
                        })
                    {
                        match r {
                            Ok(r) => {
                                *w = r;
                                remaining -= r;
                                return None;
                            }
                            Err(e) => log::error!("{e}"),
                        }
                    }
                    other_total += *w;
                    Some(i)
                })
                .collect();
            let scale = remaining.max(0.0) / other_total;
            for i in idexes {
                weights[i] = weights[i] * scale;
            }
            al::reallocate_on_weights(comp_assigns.iter_mut(), &weights, 1.0);

            comps.iter_mut().zip(comp_assigns).for_each(|(c, v)| {
                c.reassign_space(v.total(), false, axis);
            });
            comb::scale::restore(records, axis, comps, intervals, intervals_val);
        }
        CompData::Surround {
            comps,
            surround,
            intervals_val,
            ..
        } => {
            ctrl_subcomp(&mut comps[1], value);

            let (l, r) = comps.split_at_mut(1);
            let secondery = &mut r[0];

            let mut first = true;
            comb::surround::for_each_primary_rev(&mut l[0], surround, |primary| {
                if first {
                    first = false;
                    let settings = value
                        .get("surround")
                        .map(|v| config::get_axis_val(v))
                        .unwrap_or_default()
                        .into_map(|val| match val {
                            None => DEFAULT_SURR_SETTING,
                            Some(val) => (
                                val.get("factor")
                                    .and_then(|val| val.as_f64().map(|val| val as f32))
                                    .unwrap_or(DEFAULT_SURR_SETTING.0),
                                val.get("comp")
                                    .and_then(|val| sj::from_value::<[f32; 2]>(val.clone()).ok())
                                    .unwrap_or(DEFAULT_SURR_SETTING.1),
                            ),
                        });

                    match &mut primary.cdata {
                        CompData::Single {
                            proto,
                            view,
                            assigns,
                            ..
                        } => {
                            let surr_area = view.surround_area(*surround).unwrap();
                            let assigns_f32 =
                                assigns.map(|a| a.iter().map(|a| a.total()).collect::<Vec<f32>>());
                            let s_size =
                                Axis::hv().into_map(|axis| secondery.get_assign_value(axis, false));
                            let s_line_w = secondery.get_line_weight();

                            for axis in Axis::list() {
                                if s_size.hv_get(axis).total() == 0.0 {
                                    continue;
                                }
                                let s_c_val = secondery.get_complexity(axis, true);
                                let s_line_w = s_line_w
                                    .zip(s_size)
                                    .into_map(|(line, len)| {
                                        if len.total() == 0.0 {
                                            0.0
                                        } else {
                                            line / len.total()
                                        }
                                    })
                                    .reduce(|w, h| w + h);

                                let p_c_val =
                                    comb::surround::get_area_complexity(view, surround, axis)
                                        .map(|v| v as f32);
                                let p_line_w = comb::surround::get_area_line_weight(
                                    proto,
                                    &assigns_f32,
                                    surr_area,
                                    axis,
                                );

                                let (factor, comp_weight) = settings.hv_get(axis);
                                let surr_area = surr_area.hv_get(axis);

                                let mut c_assigns = [
                                    assigns.hv_get(axis)[..surr_area[0]]
                                        .iter()
                                        .sum::<AssignVal>(),
                                    assigns.hv_get(axis)[surr_area[1]..]
                                        .iter()
                                        .sum::<AssignVal>(),
                                    secondery.get_assign_value(axis, false),
                                ];
                                let mut c_weights = [
                                    p_c_val[0] * p_line_w[0],
                                    p_c_val[2] * p_line_w[2],
                                    s_c_val as f32 * s_line_w + comp_weight[1],
                                ];
                                [0, 1].into_iter().for_each(|i| {
                                    if c_weights[i] != 0.0 {
                                        c_weights[i] += comp_weight[0];
                                    }
                                });
                                if let CompData::Single { proto, .. } = &secondery.cdata {
                                    let factors = proto
                                        .attrs
                                        .get::<attrs::LineWeight>()
                                        .unwrap_or(DataHV::splat(None));

                                    c_weights[2] *= factors.hv_get(axis).unwrap_or(1.0);
                                }

                                al::reallocate_on_weights(
                                    c_assigns.iter_mut(),
                                    &c_weights,
                                    *factor,
                                );
                                let s_new_len = c_assigns[2].total()
                                    + secondery
                                        .blanks
                                        .hv_get(axis)
                                        .iter()
                                        .sum::<AssignVal>()
                                        .total();

                                let mut changes = vec![true; surr_area[1] - surr_area[0]];
                                changes.extend([false, false]);
                                let new_surr_len = s_new_len
                                    + intervals_val.hv_get(axis).iter().sum::<AssignVal>().total();
                                al::reassign_in(
                                    assigns.hv_get_mut(axis)[surr_area[0]..surr_area[1]]
                                        .iter_mut()
                                        .chain(primary.blanks.hv_get_mut(axis).iter_mut()),
                                    &changes,
                                    new_surr_len,
                                )
                                .unwrap();
                                // al::reassign(
                                //     assigns.hv_get_mut(axis)[surr_area[0]..surr_area[1]].iter_mut(),
                                //     surr_area_assign - c_assigns[0].total() - c_assigns[1].total(),
                                // )
                                // .unwrap();

                                al::reassign(
                                    assigns.hv_get_mut(axis)[..surr_area[0]].iter_mut(),
                                    c_assigns[0].total(),
                                )
                                .unwrap();
                                al::reassign(
                                    assigns.hv_get_mut(axis)[surr_area[1]..].iter_mut(),
                                    c_assigns[1].total(),
                                )
                                .unwrap();

                                secondery.reassign_space(s_new_len, false, axis);
                            }
                        }
                        _ => todo!(), // surround in
                    }
                } else {
                    todo!(); // surround in
                }
            });
        }
    }
}

pub fn ctrl_trend(comb: &mut StrucComb, value: &sj::Value) {
    match &mut comb.cdata {
        CompData::Single { .. } => {}
        CompData::Scale {
            comps,
            axis: c_axis,
            intervals,
            intervals_val,
            ..
        } => {
            let records = comb::scale::flatten(comps, intervals, intervals_val, *c_axis);
            let axis = c_axis.inverse();
            let mut edge_datas: Vec<Vec<Option<view::EdgeShape>>> =
                vec![vec![None; 4]; comps.len()];
            fn get_edge(
                i: usize,
                axis: Axis,
                side: Side,
                edge_datas: &mut Vec<Vec<Option<view::EdgeShape>>>,
                comps: &Vec<StrucComb>,
            ) -> view::EdgeShape {
                let j = match (axis, side) {
                    (Axis::Horizontal, Side::Front) => 0,
                    (Axis::Horizontal, Side::Back) => 1,
                    (Axis::Vertical, Side::Front) => 2,
                    (Axis::Vertical, Side::Back) => 3,
                };
                edge_datas[i][j]
                    .get_or_insert_with(|| comps[i].get_edge(axis, side, false).to_shape())
                    .clone()
            }

            match value.get("scale").map(|val| {
                sj::from_value::<Vec<config::EdgeCheck<[Option<String>; 2]>>>(val.clone())
            }) {
                Some(Ok(settings)) => {
                    for i in 0..comps.len() {
                        let blanks = comps[i].get_blank_base(axis);
                        let c_setting = comps[i].get_trend_setting().unwrap_or_default();
                        for tcheck in c_setting.iter().chain(settings.iter()) {
                            if blanks
                                .iter()
                                .zip(tcheck.setup.iter())
                                .any(|(b, s)| s.is_some() && *b == 0)
                            {
                                continue;
                            }

                            let r = tcheck.is_match_scale(
                                axis,
                                i,
                                comps.len(),
                                |i, axis, side| get_edge(i, axis, side, &mut edge_datas, comps),
                                |k, _| Err(config::CheckError::UnknowKey(k.to_string())),
                            );

                            match r {
                                Ok(r) => {
                                    if r {
                                        let mut c_assign = comps[i].get_assign_value(axis, false);
                                        let assign_list: Vec<_> = comps[i]
                                            .blanks
                                            .hv_get_mut(axis)
                                            .iter_mut()
                                            .chain(std::iter::once(&mut c_assign))
                                            .collect();
                                        let mut b_weight: Vec<_> =
                                            assign_list.iter().map(|v| v.total()).collect();
                                        let total = b_weight.iter().sum::<f32>();

                                        let mut vars = HashMap::from([
                                            ("s_val", b_weight[0] as f64),
                                            ("e_val", b_weight[1] as f64),
                                            ("m_val", b_weight[2] as f64),
                                            ("total", total as f64),
                                        ]);
                                        for i in [0, 1] {
                                            vars.insert("val", b_weight[i] as f64);
                                            match tcheck.setup[i].as_ref().map(
                                                |str| -> anyhow::Result<f32> {
                                                    let exps =
                                                        config::Exps::try_from(str.as_str())?;
                                                    Ok(exps.eval(&vars)? as f32)
                                                },
                                            ) {
                                                Some(Ok(r)) => b_weight[i] = r,
                                                Some(Err(e)) => log::error!("{e}"),
                                                None => {}
                                            }
                                        }
                                        b_weight[2] = (total - b_weight[0] - b_weight[1]).max(0.0);

                                        al::reallocate_on_weights(
                                            assign_list.into_iter(),
                                            &b_weight,
                                            1.0,
                                        );
                                        let blank_val = comps[i]
                                            .blanks
                                            .hv_get(axis)
                                            .iter()
                                            .sum::<AssignVal>()
                                            .total();
                                        comps[i].reassign_space(
                                            c_assign.total() + blank_val,
                                            false,
                                            axis,
                                        );
                                        break;
                                    }
                                }
                                Err(e) => eprintln!("In trend setting: {e}"),
                            }
                        }
                    }
                }
                Some(Err(e)) => eprintln!("Error in Trend settings: {e}"),
                None => {}
            }

            comps.iter_mut().for_each(|c| ctrl_trend(c, value));
            comb::scale::restore(records, *c_axis, comps, intervals, intervals_val);
        }
        CompData::Surround {
            comps,
            surround,
            intervals,
            intervals_val,
            ..
        } => {
            let (l, r) = comps.split_at_mut(1);
            let primary = &mut l[0];
            let secondary = &mut r[0];

            let edges: DataHV<[view::EdgeShape; 2]> = Axis::hv().into_map(|axis| {
                Side::fb().map(|from| secondary.get_edge(axis, from, false).to_shape())
            });
            let adjacency_edges: DataHV<[Option<view::EdgeShape>; 2]> =
                comb::surround::get_surround_inside_edge(primary, surround)
                    .into_map(|list| list.map(|e| e.map(|e| e.to_shape())));

            let axis_edges = Axis::hv().into_map(|axis| {
                Side::fb().map(|from| {
                    if surround.hv_get_into(axis) == from.inverse().to_section() {
                        let (p1, _, p2) =
                            comb::surround::get_primary_area_edge(primary, surround, axis, from);
                        [p1, p2]
                    } else {
                        [const { None }; 2]
                    }
                    .map(|edge| edge.map(|e| e.to_shape()))
                })
            });

            // primary
            match &mut primary.cdata {
                CompData::Single {
                    assigns,
                    view,
                    proto,
                    ..
                } => {
                    let area = view.surround_area(*surround).unwrap();
                    let settings = value
                        .get("surround")
                        .and_then(|value| {
                            value.get("primary").and_then(|value| {
                                sj::from_value::<Vec<config::EdgeCheck<String>>>(value.clone()).ok()
                            })
                        })
                        .unwrap_or_default();

                    let original_val = comb::surround::get_original_assign_value(
                        view,
                        proto,
                        assigns,
                        secondary,
                        surround,
                        intervals,
                        intervals_val,
                        &primary.blanks,
                    );
                    let weights = comb::surround::get_assign_weights(proto, assigns, &original_val);

                    for axis in Axis::list() {
                        let blanks = primary.blanks.hv_get_mut(axis);
                        let area = area.hv_get(axis);

                        if let Some(idx) = blanks.iter().position(|v| v.base != 0.0) {
                            for tcheck in settings.iter() {
                                let r = tcheck.is_match_surround(
                                    axis,
                                    surround,
                                    &edges,
                                    &axis_edges,
                                    &adjacency_edges,
                                    &comb.attrs.get::<attrs::Adjacencies>().unwrap(),
                                    |k, _| Err(config::CheckError::UnknowKey(k.to_string())),
                                );

                                match r {
                                    Ok(r) => {
                                        if r {
                                            let mut area_assign = assigns.hv_get(axis)
                                                [area[0]..area[1]]
                                                .iter()
                                                .sum::<AssignVal>();
                                            let mut b_weight =
                                                [area_assign.total(), blanks[idx].total()];
                                            let total = b_weight[0] + b_weight[1];

                                            let vars = HashMap::from([
                                                ("val", *b_weight.last().unwrap() as f64),
                                                ("total", total as f64),
                                            ]);
                                            match config::Exps::try_from(tcheck.setup.as_str())
                                                .map_err(|e| e.to_string())
                                                .and_then(|exps: config::Exps<'_>| {
                                                    exps.eval(&vars).map_err(|e| e.to_string())
                                                }) {
                                                Ok(r) => b_weight[1] = r as f32,
                                                Err(e) => log::error!("{e}"),
                                            }
                                            b_weight[0] = (total - b_weight[1]).max(0.0);

                                            al::reallocate_on_weights(
                                                [&mut area_assign, &mut blanks[idx]].into_iter(),
                                                &b_weight,
                                                1.0,
                                            );

                                            let weights = &weights.hv_get(axis)[area[0]..area[1]];
                                            let area_assigns =
                                                &mut assigns.hv_get_mut(axis)[area[0]..area[1]];
                                            al::reassign(
                                                area_assigns.iter_mut(),
                                                area_assign.total(),
                                            )
                                            .unwrap();
                                            al::reallocate_on_weights(
                                                area_assigns.iter_mut(),
                                                weights,
                                                1.0,
                                            );
                                            break;
                                        }
                                    }
                                    Err(e) => eprintln!("In trend surround setting: {e}"),
                                }
                            }
                        }
                    }
                }
                _ => todo!(), // surround in
            }

            // secondary
            let settings = value
                .get("surround")
                .and_then(|value| {
                    value.get("secondary").and_then(|value| {
                        sj::from_value::<Vec<config::EdgeCheck<[Option<String>; 2]>>>(value.clone())
                            .ok()
                    })
                })
                .unwrap_or_default();
            for axis in Axis::list() {
                let intervals = intervals.hv_get(axis);

                for tcheck in settings.iter() {
                    if [0, 1].into_iter().any(|i| {
                        tcheck.setup[i].is_some()
                            && (intervals[i] == 0 && secondary.blanks.hv_get(axis)[i].base == 0.0)
                    }) {
                        continue;
                    }

                    let r = tcheck.is_match_surround(
                        axis,
                        surround,
                        &edges,
                        &axis_edges,
                        &adjacency_edges,
                        &comb.attrs.get::<attrs::Adjacencies>().unwrap(),
                        |k, _| Err(config::CheckError::UnknowKey(k.to_string())),
                    );

                    match r {
                        Ok(r) => {
                            if r {
                                let c_assign = secondary.get_assign_value(axis, false);

                                let whites = [0, 1].map(|i| {
                                    if intervals[i] == 0 {
                                        secondary.blanks.hv_get(axis)[i]
                                    } else {
                                        intervals_val.hv_get(axis)[i]
                                    }
                                });

                                let mut assign_list =
                                    [whites[0].clone(), whites[1].clone(), c_assign];
                                let mut b_weight = assign_list.map(|v| v.total());
                                let total = b_weight.iter().sum::<f32>();

                                let mut vars = HashMap::from([
                                    ("s_val", b_weight[0] as f64),
                                    ("e_val", b_weight[1] as f64),
                                    ("m_val", b_weight[2] as f64),
                                    ("total", total as f64),
                                ]);
                                for i in [0, 1] {
                                    vars.insert("val", b_weight[i] as f64);
                                    match tcheck.setup[i].as_ref().map(
                                        |str| -> anyhow::Result<f32> {
                                            let exps = config::Exps::try_from(str.as_str())?;
                                            Ok(exps.eval(&vars)? as f32)
                                        },
                                    ) {
                                        Some(Ok(r)) => b_weight[i] = r,
                                        Some(Err(e)) => log::error!("{e}"),
                                        None => {}
                                    }
                                }
                                b_weight[2] = total - b_weight[0] - b_weight[1];

                                al::reallocate_on_weights(assign_list.iter_mut(), &b_weight, 1.0);

                                for i in [0, 1] {
                                    if intervals[i] == 0 {
                                        secondary.blanks.hv_get_mut(axis)[i] = assign_list[i];
                                    } else {
                                        intervals_val.hv_get_mut(axis)[i] = assign_list[i];
                                    }
                                }
                                secondary.reassign_space(
                                    assign_list[2].total()
                                        + secondary
                                            .blanks
                                            .hv_get(axis)
                                            .iter()
                                            .sum::<AssignVal>()
                                            .total(),
                                    false,
                                    axis,
                                );
                                break;
                            }
                        }
                        Err(e) => eprintln!("In trend surround setting: {e}"),
                    }
                }
            }

            ctrl_trend(secondary, value)
        }
    }
}

pub fn ctrl_interval(comb: &mut StrucComb, value: &sj::Value) {
    match &mut comb.cdata {
        CompData::Single { .. } => {}
        CompData::Scale {
            comps,
            axis: c_axis,
            intervals,
            intervals_val,
            ..
        } => {
            let axis = *c_axis;

            match sj::from_value::<Vec<config::EdgeCheck<String>>>(value.clone()) {
                Err(e) => {
                    log::error!("Error in interval ctrl settings: {e}");
                }
                Ok(settings) => {
                    let records = comb::scale::flatten(comps, intervals, intervals_val, axis);
                    comps.iter_mut().for_each(|c| ctrl_interval(c, value));

                    let mut c_assigns: Vec<_> = comps
                        .iter()
                        .map(|c| c.get_assign_value(axis, true))
                        .collect();
                    let assign_len = c_assigns
                        .iter()
                        .chain(intervals_val.iter())
                        .map(|v| v.total())
                        .sum::<f32>();

                    let mut ok = false;
                    // for (comps, (&interval, interval_val)) in comps
                    //     .windows(2)
                    //     .zip(intervals.iter().zip(intervals_val.iter_mut()))
                    for idx in 0..intervals.len() {
                        let comps = &comps[idx..=idx + 1];
                        let interval = intervals[idx];
                        let interval_val = &mut intervals_val[idx];

                        if interval == 0 {
                            continue;
                        }

                        for check in settings.iter() {
                            let r = check.is_match_interval(
                                axis,
                                interval,
                                &comps[0].get_edge(axis, Side::Back, true).to_shape(),
                                &comps[1].get_edge(axis, Side::Front, true).to_shape(),
                                |k, v| match k {
                                    "min_base" => match v.as_str() {
                                        Some(str) => {
                                            match config::BoolExps::try_from(str).and_then(|exps| {
                                                exps.eval(
                                                    &[(
                                                        "val",
                                                        comps[0].get_bases_length(axis, true).min(
                                                            comps[1].get_bases_length(axis, true),
                                                        )
                                                            as isize,
                                                    )]
                                                    .into(),
                                                )
                                            }) {
                                                Ok(b) => Some(Ok(b)),
                                                Err(e) => {
                                                    Some(Err(config::CheckError::ValueError {
                                                        key: k.to_string(),
                                                        value: e.to_string(),
                                                    }))
                                                }
                                            }
                                        }
                                        None => Some(Err(config::CheckError::ValueError {
                                            key: k.to_string(),
                                            value: "Required 'min_base' is a string".to_string(),
                                        })),
                                    },
                                    _ => None,
                                },
                            );
                            match r {
                                Err(e) => eprintln!("In interval ctrl setting: {e}"),
                                Ok(r) => {
                                    if r {
                                        let old_val = *&interval_val.total();
                                        let vars = HashMap::from([
                                            ("val", old_val as f64),
                                            (
                                                "min_c_val",
                                                c_assigns[idx]
                                                    .total()
                                                    .min(c_assigns[idx + 1].total())
                                                    as f64,
                                            ),
                                            (
                                                "max_c_val",
                                                c_assigns[idx]
                                                    .total()
                                                    .max(c_assigns[idx + 1].total())
                                                    as f64,
                                            ),
                                        ]);
                                        match config::Exps::try_from(check.setup.as_str())
                                            .map_err(|e| e.to_string())
                                            .and_then(|exps| {
                                                exps.eval(&vars).map_err(|e| e.to_string())
                                            }) {
                                            Ok(r) => {
                                                interval_val.uncheck_set(r as f32);
                                                ok = true;
                                                break;
                                            }
                                            Err(e) => log::error!("{e}"),
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if ok {
                        let mut changes = vec![true; c_assigns.len() + intervals_val.len()];
                        changes[c_assigns.len()..].fill(false);
                        al::reassign_in(
                            c_assigns.iter_mut().chain(intervals_val.iter_mut()),
                            &changes,
                            assign_len,
                        )
                        .unwrap();
                        comps
                            .iter_mut()
                            .zip(c_assigns.into_iter())
                            .for_each(|(c, val)| c.reassign_space(val.total(), true, axis));
                    }

                    comb::scale::restore(records, axis, comps, intervals, intervals_val);
                }
            };
        }
        CompData::Surround { comps, .. } => comps.iter_mut().for_each(|c| ctrl_interval(c, value)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::algorithm as al;
    use super::*;

    #[test]
    fn test_subarea_ctrl() {
        let settings = sj::json!({"h": {"factor": 1.0}});

        // 艹
        let proto = crate::combination::StrucProto {
            paths: vec![
                KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
                KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
                KeyPath::from([key_pos(3, 0), key_pos(3, 2)]),
            ],
            attrs: Default::default(),
        };
        let mut comb = StrucComb {
            name: "test".to_string(),
            blanks: Default::default(),
            cdata: CompData::Single {
                view: crate::combination::StrucView::new(&proto),
                proto,
                level: Default::default(),
                assigns: DataHV::new(
                    vec![
                        AssignVal::new(0.1, 0.1),
                        AssignVal::new(0.2, 0.2),
                        AssignVal::new(0.1, 0.1),
                    ],
                    vec![AssignVal::new(0.1, 0.1), AssignVal::new(0.1, 0.1)],
                ),
            },
            attrs: Default::default(),
        };

        ctrl_subarea(&mut comb, &settings);
        if let CompData::Single {
            assigns: assigns_hv,
            ..
        } = &comb.cdata
        {
            let assigns = &assigns_hv.h;
            assert_eq!(assigns[0].total(), 0.2);
            assert_eq!(assigns[2].total(), 0.2);
            assert_eq!(assigns[1].total(), 0.4);

            let assigns = &assigns_hv.v;
            assert_eq!(assigns[0].total(), 0.2);
            assert_eq!(assigns[1].total(), 0.2);
        }

        // 干
        let proto = crate::combination::StrucProto {
            paths: vec![
                KeyPath::from([key_pos(1, 0), key_pos(3, 0)]),
                KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
            ],
            attrs: Default::default(),
        };
        let mut comb = StrucComb {
            name: "test".to_string(),
            blanks: Default::default(),
            cdata: CompData::Single {
                view: crate::combination::StrucView::new(&proto),
                proto,
                level: Default::default(),
                assigns: DataHV::new(
                    vec![AssignVal::new(0.1, 0.1); 4],
                    vec![AssignVal::new(0.1, 0.1); 2],
                ),
            },
            attrs: Default::default(),
        };

        ctrl_subarea(&mut comb, &settings);
        if let CompData::Single {
            assigns: assigns_hv,
            ..
        } = &comb.cdata
        {
            let assigns = &assigns_hv.h;
            assert!(
                (assigns[0].total() - 0.8 / 6.0).abs() < al::NORMAL_OFFSET,
                "{} != {}",
                assigns[0].total(),
                0.8 / 6.0
            );
            assert!(
                (assigns[1].total() - 0.8 / 6.0 * 2.0).abs() < al::NORMAL_OFFSET,
                "{} != {}",
                assigns[1].total(),
                0.8 / 6.0 * 2.0
            );
            assert!(
                (assigns[2].total() - 0.8 / 6.0 * 2.0).abs() < al::NORMAL_OFFSET,
                "{} != {}",
                assigns[2].total(),
                0.8 / 6.0 * 2.0
            );
            assert!(
                (assigns[3].total() - 0.8 / 6.0).abs() < al::NORMAL_OFFSET,
                "{} != {}",
                assigns[2].total(),
                0.8 / 6.0
            );

            let assigns = &assigns_hv.v;
            assert_eq!(assigns[0].total(), 0.2);
            assert_eq!(assigns[1].total(), 0.2);
        }
    }
}
