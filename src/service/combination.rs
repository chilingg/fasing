use super::{CharTree, Service, algorithm as al, char_tree, space};
use crate::{
    base::*,
    combination::{CompData, SharpnessModel, StrucComb, StrucProto, StrucView, attrs, comb, view},
    config,
    construct::{CstError, CstType},
};

use std::collections::BTreeMap;

pub fn get_comb_proto_in(
    service: &impl Service,
    target: CharTree,
    adjacency: DataHV<[bool; 2]>,
    in_surround: DataHV<[bool; 2]>,
) -> Result<StrucComb, CstError> {
    match target.tp {
        CstType::Single => {
            let mut proto = service
                .get_struc_proto(&target.name)
                .cloned()
                .ok_or_else(|| CstError::Empty(target.name.clone()))?;

            proto.set_allocs_in(adjacency, in_surround);
            Ok(StrucComb::new_single(target.name, proto))
        }
        CstType::Scale(axis) => {
            let children = target.children;
            let mut combs = Vec::with_capacity(children.len());

            let end = children.len() - 1;
            for (i, c_target) in children.into_iter().enumerate() {
                let mut c_in_place = adjacency.clone();
                let mut c_in_surr = in_surround.clone();
                if i != 0 {
                    c_in_place.hv_get_mut(axis)[0] = true;
                    c_in_surr.hv_get_mut(axis)[0] = false;
                }
                if i != end {
                    c_in_place.hv_get_mut(axis)[1] = true;
                    c_in_surr.hv_get_mut(axis)[1] = false;
                }

                combs.push(get_comb_proto_in(service, c_target, c_in_place, c_in_surr)?);
            }
            Ok(StrucComb::new_complex(target.name, target.tp, combs))
        }
        CstType::Surround(c_surr) => {
            if target.children.len() != 2 {
                return Err(CstError::Surround {
                    tp: target.tp.symbol(),
                    comp: target
                        .children
                        .iter()
                        .map(|c| c.name.clone())
                        .collect::<Vec<String>>()
                        .join("、"),
                });
            }

            let mut c_in_surr = in_surround.clone();
            for axis in Axis::list() {
                if !Section::End.eq(c_surr.hv_get(axis)) {
                    c_in_surr.hv_get_mut(axis)[0] = true;
                }
                if !Section::Start.eq(c_surr.hv_get(axis)) {
                    c_in_surr.hv_get_mut(axis)[1] = true;
                }
            }

            let mut children = target.children.into_iter();
            let combs = vec![
                get_comb_proto_in(service, children.next().unwrap(), adjacency, in_surround)?,
                get_comb_proto_in(service, children.next().unwrap(), adjacency, c_in_surr)?,
            ];

            Ok(StrucComb::new_complex(target.name, target.tp, combs))
        }
    }
    .map(|mut comb| {
        comb.attrs.set::<attrs::Adjacencies>(&adjacency);
        comb.attrs.set::<attrs::InSurround>(&in_surround);
        comb
    })
}

pub fn reduce_replace(
    service: &impl Service,
    comb: &mut StrucComb,
    axis: Axis,
) -> Result<bool, CstError> {
    fn replace(service: &impl Service, comb: &mut StrucComb, axis: Axis) -> Result<bool, CstError> {
        let cfg = service.get_config();
        match cfg.reduce_replace_name(axis, &comb.name) {
            Some(new_name) => {
                let new_tree = char_tree::get_char_tree(service, new_name.to_string());
                let mut new_comb = get_comb_proto_in(
                    service,
                    new_tree,
                    comb.attrs.get::<attrs::Adjacencies>().unwrap(),
                    comb.attrs.get::<attrs::InSurround>().unwrap(),
                )?;

                let old_len = comb.get_bases_length(axis, false);
                loop {
                    let new_len = *init_edges(service, &mut new_comb)?.hv_get(axis);
                    if new_len <= old_len || !new_comb.reduce_space(axis, false) {
                        break;
                    }
                }

                if let Some(target) = comb
                    .attrs
                    .get::<attrs::ReduceTarget>()
                    .and_then(|data| *data.hv_get(axis.inverse()))
                {
                    loop {
                        let new_len = new_comb.get_bases_length(axis.inverse(), false);
                        if new_len <= target || !new_comb.reduce_space(axis.inverse(), false) {
                            break;
                        }
                    }
                }

                *comb = new_comb;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    match &mut comb.cdata {
        CompData::Single { .. } => replace(service, comb, axis),
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

            let mut ok = false;
            if axis == *c_axis {
                let mut new_lengths = lenghts.clone();
                // new_lengths.last_mut().unwrap().1 = usize::MAX;
                new_lengths.sort_by_key(|(_, c)| *c);

                for (i, _) in new_lengths {
                    if reduce_replace(service, &mut comps[i], axis)? {
                        ok = true;
                        break;
                    }
                }
            } else {
                let max_len = lenghts.iter().max_by_key(|(_, l)| *l).unwrap().1;
                let targets: Vec<_> = lenghts
                    .iter()
                    .filter_map(|&(i, l)| {
                        if max_len == l && !comps[i].reduce_space(axis, true) {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect();

                for i in targets {
                    ok |= reduce_replace(service, &mut comps[i], axis)?;
                }
            }
            Ok(ok)
        }
        CompData::Surround {
            surround,
            comps,
            intervals,
            ..
        } => {
            let mut ok = Ok(false);
            let (primary, secondary) = comps.split_at_mut(1);
            comb::surround::axis_main_comp(
                &mut primary[0],
                &mut secondary[0],
                surround,
                intervals,
                axis,
                |mut list, all| {
                    if list.len() == 1 {
                        ok = reduce_replace(service, list[0], axis);
                    } else if all {
                        let mut b = true;
                        for c in list {
                            if !c.reduce_space(axis, true) {
                                match reduce_replace(service, c, axis) {
                                    Ok(r) => b &= r,
                                    Err(e) => {
                                        ok = Err(e);
                                        break;
                                    }
                                }
                            }
                        }
                        if ok.is_ok() {
                            ok = Ok(b);
                        }
                    } else {
                        ok = list
                            .into_iter()
                            .find_map(|c| {
                                if !c.reduce_space(axis, true) {
                                    match reduce_replace(service, c, axis) {
                                        Ok(r) => {
                                            if r {
                                                Some(Ok(true))
                                            } else {
                                                None
                                            }
                                        }
                                        Err(e) => Some(Err(e)),
                                    }
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(Ok(false));
                    }
                },
            );
            ok
        }
    }
}

fn init_edge_at_scale(
    service: &impl Service,
    comps: &mut Vec<StrucComb>,
    intervals: &mut Vec<usize>,
    axis: Axis,
) -> Result<DataHV<usize>, CstError> {
    let records = comb::scale::flatten(comps, intervals, &mut vec![], axis);
    let mut len_list: Vec<DataHV<usize>> = Vec::with_capacity(comps.len());
    let cfg = service.get_config();

    *intervals = loop {
        len_list.clear();
        for c in comps.iter_mut() {
            len_list.push(init_edges(service, c)?);
        }

        cfg.set_main_comp_axis(
            comps,
            axis.inverse(),
            &len_list
                .iter()
                .map(|list| *list.hv_get(axis.inverse()))
                .collect(),
        );
        match set_intervals_axis(service, comps, axis) {
            Err(Some(e)) => return Err(e),
            Ok(new_intervals) => break new_intervals,
            Err(None) => {}
        }
    };

    let size = Axis::hv().into_map(|i_axis| {
        if i_axis == axis {
            len_list.iter().map(|cl| *cl.hv_get(i_axis)).sum::<usize>()
                + intervals.iter().sum::<usize>()
        } else {
            comps
                .iter()
                .zip(len_list.iter())
                .map(|(c, l)| c.get_blank_base(i_axis).iter().sum::<usize>() + *l.hv_get(i_axis))
                .max()
                .unwrap()
        }
    });
    comb::scale::restore(records, axis, comps, intervals, &mut vec![]);

    Ok(size)
}

fn init_edge_at_surround(
    service: &impl Service,
    primary: &mut StrucComb,
    secondary: &mut StrucComb,
    intervals: &mut DataHV<[usize; 2]>,
    surround: &DataHV<Section>,
) -> Result<DataHV<usize>, CstError> {
    let cfg = service.get_config();
    let mut p_len;
    let mut s_len;

    *intervals = loop {
        comb::surround::check_surround(primary, surround)?;

        p_len = init_edges(service, primary)?;
        s_len = init_edges(service, secondary)?;

        match set_intervals_surround(service, primary, secondary, surround) {
            Err(Some(e)) => return Err(e),
            Ok(new_intervals) => break new_intervals,
            Err(None) => {}
        }
    };

    let s_len_total = Axis::hv().into_map(|axis| {
        let [area_f, _, area_b] = comb::surround::get_area_base_len(primary, surround, axis);
        s_len.hv_get_into(axis) + area_f + area_b + intervals.hv_get(axis).iter().sum::<usize>()
    });
    cfg.set_main_comp_surround(primary, secondary, surround, [p_len, s_len_total], s_len);

    Ok(Axis::hv().into_map(|axis| *p_len.hv_get(axis).max(s_len_total.hv_get(axis))))
}

fn init_edges(service: &impl Service, comb: &mut StrucComb) -> Result<DataHV<usize>, CstError> {
    comb.blanks = Default::default();

    let l = match &mut comb.cdata {
        CompData::Single { proto, .. } => {
            if proto.is_empty() {
                return Err(CstError::Empty(comb.name.clone()));
            }
            proto.attrs.set::<attrs::FixedAlloc>(&Default::default());
            Ok(proto.size())
        }
        CompData::Scale {
            axis: c_axis,
            comps,
            intervals,
            ..
        } => Ok(init_edge_at_scale(service, comps, intervals, *c_axis)?),
        CompData::Surround {
            surround,
            comps,
            intervals,
            ..
        } => {
            let (l, r) = comps.split_at_mut(1);
            let primary = &mut l[0];
            let secondary = &mut r[0];
            Ok(init_edge_at_surround(
                service, primary, secondary, intervals, surround,
            )?)
        }
    };

    l.map(|size| {
        size.zip(comb.blanks).map(|(s, b)| {
            *s + b
                .iter()
                .map(|v| if v.base == 0.0 { 0 } else { 1 })
                .sum::<usize>()
        })
    })
}

fn set_edge_alloc_surr(
    service: &impl Service,
    primary: &mut StrucComb,
    setup: &BTreeMap<String, serde_json::Value>,
    surround: &DataHV<Section>,
    axis: Axis,
    side: Side,
) -> Result<(), Option<CstError>> {
    if let Some(replace) = setup.get("replace").and_then(|v| v.as_str()) {
        *primary = service
            .get_struc_comb(service.get_char_tree(replace.to_string()))
            .map_err(|e| Some(e))?;
        return Err(None);
    }

    if let Some(target) = setup
        .get("allocs")
        .and_then(|v| v.as_u64().map(|v| v as usize))
    {
        match &mut primary.cdata {
            CompData::Single { proto, view, .. } => {
                let mut allocs_proto = proto.allocation_values();
                let allocs = allocs_proto.hv_get_mut(axis);
                let area = view.surround_area(*surround).unwrap().hv_get_into(axis);
                let idx = match side {
                    Side::Front if area[0] != 0 => area[0] - 1,
                    Side::Back if area[1] != allocs.len() => area[1],
                    _ => {
                        log::error!("Error setting allocs in Surround interval!");
                        return Ok(());
                    }
                };

                if allocs[idx] != target {
                    allocs[idx] = target;
                    proto.attrs.set::<attrs::Allocs>(&allocs_proto);
                    *view = StrucView::new(&proto);
                    return Err(None);
                } else {
                    let mut fiexd = proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();
                    fiexd.hv_get_mut(axis).insert(idx);
                    proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn set_single_edge_alloc(
    proto: &mut StrucProto,
    view: &mut StrucView,
    target: Option<Vec<usize>>,
    side: Side,
    axis: Axis,
) -> bool {
    let target = if let Some(target) = &target {
        target
    } else {
        return true;
    };

    let mut allocs_proto = proto.allocation_values();
    let allocs = allocs_proto.hv_get_mut(axis);
    let mut fiexd = proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();

    let mut indexes: Vec<usize> = (0..allocs.len()).collect();
    if side == Side::Back {
        indexes.reverse();
    }

    let mut ok = true;
    for (i, &v) in target.iter().enumerate() {
        if allocs[indexes[i]] != v {
            allocs[indexes[i]] = v;
            ok = false
        } else {
            fiexd.hv_get_mut(axis).insert(indexes[i]);
        }
    }

    if ok {
        proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
    } else {
        proto.attrs.set::<attrs::Allocs>(&allocs_proto);
        *view = StrucView::new(&proto);
    }
    ok
}

fn check_edge_alloc(
    comb: &mut StrucComb,
    edge: &view::EdgeShape,
    axis: Axis,
    side: Side,
    in_surround: Option<&str>,
) -> Option<BTreeMap<String, serde_json::Value>> {
    comb.get_edge_main_comb_mut(axis, side).and_then(|c| {
        let blanks = c.get_blank_base(axis.inverse()).map(|v| v != 0);
        match &mut c.cdata {
            CompData::Single { proto, .. } => {
                proto.get_match_interval(blanks, edge, axis, side, in_surround)
            }
            _ => unreachable!(),
        }
    })
}

fn set_edge_alloc(
    service: &impl Service,
    comb: &mut StrucComb,
    axis: Axis,
    side: Side,
    setup: &BTreeMap<String, serde_json::Value>,
) -> Result<(), Option<CstError>> {
    fn set_cross_allocs(
        proto: &mut StrucProto,
        view: &mut StrucView,
        targets: [Option<usize>; 2],
        axis: Axis,
    ) -> bool {
        let mut allocs_proto = proto.allocation_values();
        let allocs = allocs_proto.hv_get_mut(axis);
        let mut fiexd = proto.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();

        let mut ok = true;
        for side in Side::fb() {
            let idx = [0, allocs.len() - 1][side.n()];
            if let Some(target) = targets[side.n()] {
                if allocs[idx] != target {
                    allocs[idx] = target;
                    ok = false;
                } else {
                    fiexd.hv_get_mut(axis).insert(idx);
                }
            }
        }

        if ok {
            proto.attrs.set::<attrs::FixedAlloc>(&fiexd);
        } else {
            proto.attrs.set::<attrs::Allocs>(&allocs_proto);
            *view = StrucView::new(&proto);
        }
        ok
    }

    match comb.get_edge_main_comb_mut(axis, side) {
        Some(comb) => match &mut comb.cdata {
            CompData::Single { proto, view, .. } => {
                if let Some(replace) = setup.get("replace").and_then(|v| v.as_str()) {
                    let target = service.get_char_tree(replace.to_string());
                    let adjacency = comb.attrs.get::<attrs::Adjacencies>().unwrap_or_default();
                    let in_surround = comb.attrs.get::<attrs::InSurround>().unwrap_or_default();
                    *comb = get_comb_proto_in(service, target, adjacency, in_surround)?;
                    return Err(None);
                }

                let target = setup.get("allocs").and_then(|v| {
                    if v.is_u64() {
                        v.as_u64().map(|v| vec![v as usize])
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                });

                let mut r = set_single_edge_alloc(proto, view, target, side, axis);

                if let Some(targets) = setup
                    .get("cross_allocs")
                    .and_then(|v| serde_json::from_value::<[Option<usize>; 2]>(v.clone()).ok())
                {
                    r &= set_cross_allocs(proto, view, targets, axis.inverse());
                }

                match r {
                    true => Ok(()),
                    false => Err(None),
                }
            }
            _ => unreachable!(),
        },
        None => {
            debug_assert!(setup.is_empty());
            Ok(())
        }
    }
}

fn set_intervals_axis(
    service: &impl Service,
    comps: &mut Vec<StrucComb>,
    axis: Axis,
) -> Result<Vec<usize>, Option<CstError>> {
    let mut intervals = Vec::with_capacity(comps.len() - 1);
    for i1 in 0..comps.len() - 1 {
        let i2 = i1 + 1;
        let mut val = 0;
        let edge1 = comps[i1].get_edge(axis, Side::Back, true).to_shape();
        let edge2 = comps[i2].get_edge(axis, Side::Front, true).to_shape();

        let (left, right) = comps.split_at_mut(i2);

        let r1 = check_edge_alloc(&mut left[i1], &edge2, axis, Side::Back, None);
        let r2 = check_edge_alloc(&mut right[0], &edge1, axis, Side::Front, None);
        let get_interval = |r: &BTreeMap<String, serde_json::Value>| -> Option<usize> {
            r.get("interval")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        };
        let r = match (
            r1.as_ref().and_then(get_interval),
            r2.as_ref().and_then(get_interval),
        ) {
            (Some(r1), Some(r2)) => {
                if r1 == r2 {
                    Some(r1)
                } else {
                    None
                }
            }
            (None, None) => {
                set_edge_alloc(
                    service,
                    &mut left[i1],
                    axis,
                    Side::Back,
                    &r1.unwrap_or_default(),
                )?;
                set_edge_alloc(
                    service,
                    &mut right[0],
                    axis,
                    Side::Front,
                    &r2.unwrap_or_default(),
                )?;
                None
            }
            (ri1, ri2) => {
                if r1.is_some() || r2.is_some() {
                    set_edge_alloc(
                        service,
                        &mut left[i1],
                        axis,
                        Side::Back,
                        &r1.unwrap_or_default(),
                    )?;
                    set_edge_alloc(
                        service,
                        &mut right[0],
                        axis,
                        Side::Front,
                        &r2.unwrap_or_default(),
                    )?;
                }
                ri1.or(ri2)
            }
        };

        if let Some(i_val) = r {
            val = i_val;
        } else {
            let rules = service.get_config().get_interval_rules();
            if let Some(i_val) = rules
                .iter()
                .find_map(|rule| rule.is_match(&edge1, &edge2, axis))
            {
                val = i_val;
            }
        };

        intervals.push(val);
    }

    Ok(intervals)
}

pub fn set_intervals_surround(
    service: &impl Service,
    primary: &mut StrucComb,
    secondary: &mut StrucComb,
    surround: &DataHV<Section>,
) -> Result<DataHV<[usize; 2]>, Option<CstError>> {
    let mut intervals = DataHV::splat([Option::<usize>::None; 2]);
    let p_edges = comb::surround::get_surround_inside_edge(primary, surround)
        .into_map(|list| list.map(|e| e.map(|e| e.to_shape())));
    let s_edges = Axis::hv()
        .into_map(|axis| Side::fb().map(|from| secondary.get_edge(axis, from, true).to_shape()));
    let symbol = CstType::Surround(*surround).symbol().to_string();

    for axis in Axis::list() {
        for side in Side::fb() {
            if surround.hv_get_into(axis) != side.inverse().to_section() {
                let mut r1 = None;

                if let Some(mut p_setting) = primary.get_surr_interval_setting().remove(&symbol) {
                    let shape = &s_edges.hv_get(axis)[side.n()];
                    if let Some(setup) = p_setting
                        .remove(&axis)
                        .and_then(|mut map| map.remove(&side))
                        .and_then(|list| {
                            list.into_iter()
                                .find(|setting| {
                                    setting.rules.iter().any(|rule| rule.is_match(shape))
                                })
                                .map(|setting| setting.setup)
                        })
                    {
                        r1 = Some(setup)
                        // if let Some(interval) =
                        //     set_edge_alloc_surr(service, primary, setup, surround, axis, side)?
                        // {
                        //     target_interval = Some(interval);
                        // }
                    }
                }

                let r2 = check_edge_alloc(
                    secondary,
                    p_edges.hv_get(axis)[side.n()].as_ref().unwrap(),
                    axis,
                    side,
                    Some(&symbol),
                );

                let get_interval = |r: &BTreeMap<String, serde_json::Value>| -> Option<usize> {
                    r.get("interval")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                };
                if let Some(interval) = match (
                    r1.as_ref().and_then(get_interval),
                    r2.as_ref().and_then(get_interval),
                ) {
                    (Some(r1), Some(r2)) => {
                        if r1 == r2 {
                            Some(r1)
                        } else {
                            None
                        }
                    }
                    (r1, r2) => r1.or(r2),
                } {
                    set_edge_alloc_surr(
                        service,
                        primary,
                        &r1.unwrap_or_default(),
                        surround,
                        axis,
                        side,
                    )?;
                    set_edge_alloc(service, secondary, axis, side, &r2.unwrap_or_default())?;
                    intervals.hv_get_mut(axis)[side.n()] = Some(interval);
                };

                // target_interval = match check_edge_alloc(
                //     service,
                //     secondary,
                //     p_edges.hv_get(axis)[side.n()].as_ref().unwrap(),
                //     axis,
                //     side,
                //     target_interval,
                //     Some(&symbol),
                // ) {
                //     Err(Some(e)) => return Err(Some(e)),
                //     r => r?,
                // }
                // .or(target_interval);
            }
        }
    }

    let rules = service.get_config().get_interval_rules_surr();
    let list: Vec<_> = rules
        .iter()
        .map(|rule| rule.is_match(surround, &p_edges, &s_edges))
        .collect();

    for axis in Axis::list() {
        for side in Side::fb() {
            if surround.hv_get_into(axis) != side.inverse().to_section()
                && intervals.hv_get_mut(axis)[side.n()].is_none()
            {
                intervals.hv_get_mut(axis)[side.n()] =
                    list.iter().find_map(|setup| setup.hv_get(axis)[side.n()]);
            }
        }
    }

    Ok(intervals.into_map(|list| list.map(|v| v.unwrap_or(0))))
}

pub fn check_space(
    service: &impl Service,
    comb: &mut StrucComb,
) -> Result<(DataHV<f32>, DataHV<usize>), CstError> {
    let cfg = service.get_config();
    let zishen = cfg
        .size
        .zip(comb.get_char_box().size().to_hv_data())
        .into_map(|(a, b)| a * b);

    let mut assign = DataHV::default();
    let mut white_area: DataHV<[f32; 2]> = DataHV::default();
    let mut levels: DataHV<usize> = DataHV::default();

    let mut base_lens = init_edges(service, comb)?;
    let mut check_state: Vec<Axis> = Axis::list().into();
    let mut first = true;

    while !check_state.is_empty() {
        let axis = check_state[0];
        let zishen = *zishen.hv_get(axis);
        let zimian_gener = cfg.zimian.hv_get(axis);
        let min_white = (*cfg.size.hv_get(axis) - zimian_gener.max_val()) / 2.0;
        let reduce_trigger = cfg.get_reduce_trigger(axis);
        let replace_trigger = cfg.get_replace_trigger(axis);
        let vc_val = cfg.get_visual_corr(axis);

        loop {
            let base_len = *base_lens.hv_get(axis);
            let mut length = zimian_gener.val_in(base_len);

            let white = (zishen - length) / 2.0;
            let vcorr = Side::fb().map(|side| {
                let vcorr = ((1.0
                    - comb
                        .get_edge(axis, side, false)
                        .sharpness(SharpnessModel::ZeroOne))
                    * vc_val)
                    .min(white - min_white);
                length += vcorr;
                vcorr
            });

            let r = cfg
                .units
                .hv_get(axis)
                .iter()
                .position(|&min| (min * base_len as f32) < length + al::NORMAL_OFFSET);
            match r {
                Some(level) => {
                    let scale = length / base_len as f32;
                    if scale < reduce_trigger && comb.reduce_space(axis, false) {
                        if check_state.len() == 1 {
                            check_state.push(axis.inverse());
                        }
                        base_lens = init_edges(service, comb)?;
                        continue;
                    }
                    if scale < replace_trigger && reduce_replace(service, comb, axis)? {
                        if check_state.len() == 1 {
                            check_state.push(axis.inverse());
                        }
                        base_lens = init_edges(service, comb)?;
                        continue;
                    }

                    if base_len == 0 {
                        length = 0.0;
                        *white_area.hv_get_mut(axis) = [zishen / 2.0; 2];
                    } else {
                        *white_area.hv_get_mut(axis) = vcorr.map(|v| white - v);
                    }

                    *assign.hv_get_mut(axis) = length;
                    *levels.hv_get_mut(axis) = level;
                    break;
                }
                None => {
                    if comb.reduce_space(axis, false) || reduce_replace(service, comb, axis)? {
                        if check_state.len() == 1 {
                            check_state.push(axis.inverse());
                        }
                        base_lens = init_edges(service, comb)?;
                        continue;
                    } else if first {
                        check_state.swap(0, 1);
                        first = false;
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
        first = false;
        check_state.remove(0);
    }

    comb.set_white_area(&white_area);
    Ok((assign, levels))
}

fn assign_space_in_surround(
    service: &impl Service,
    primary: &mut StrucComb,
    surround: &DataHV<Section>,
    assigns: DataHV<f32>,
    base_len: DataHV<usize>,
) {
    match &mut primary.cdata {
        CompData::Single { .. } => {
            let (p_len, mut p_assign) = Axis::hv()
                .into_map(|axis| {
                    let p_len = primary.get_bases_length(axis, true);
                    let p_assign = (p_len as f32 / base_len.hv_get_into(axis) as f32)
                        * assigns.hv_get_into(axis);
                    (p_len, p_assign)
                })
                .unzip();
            let (levels, units) = assign_blank_space(
                service,
                &mut p_assign,
                primary,
                service.get_config().get_white_assign_settings_axis(),
            );

            match &mut primary.cdata {
                CompData::Single {
                    view,
                    level: c_level,
                    assigns: asgs,
                    proto,
                    ..
                } => {
                    *c_level = levels;
                    let weights = Axis::hv().into_map(|axis| {
                        let mut size = p_assign.clone();
                        *size.hv_get_mut(axis) = assigns.hv_get_into(axis);
                        proto.subarea_weight(size).hv_get_into(axis)
                    });
                    let area = view.surround_area(*surround).unwrap();
                    let mut c_assigns =
                        assign_single_space(proto, p_assign, units, &weights, Some(area));

                    for axis in Axis::list() {
                        let p_assign = p_assign.hv_get_into(axis);
                        let assign = assigns.hv_get_into(axis);
                        let p_len = *p_len.hv_get(axis);
                        let base_len = *base_len.hv_get(axis);

                        if p_len < base_len {
                            let surr = surround.hv_get_into(axis);
                            if let Some(side) = Side::fb().into_iter().find(|side| {
                                side.inverse().to_section() == surr
                                    && primary.blanks.hv_get(axis)[side.n()].base != 0.0
                            }) {
                                primary.blanks.hv_get_mut(axis)[side.n()]
                                    .set(assign - p_assign)
                                    .unwrap();
                            } else {
                                let area = area.hv_get(axis);
                                let c_assigns = &mut c_assigns.hv_get_mut(axis)[area[0]..area[1]];
                                let new_assign = assign - p_assign
                                    + c_assigns
                                        .iter()
                                        .chain(primary.blanks.hv_get(axis).iter())
                                        .sum::<AssignVal>()
                                        .total();
                                al::reassign(c_assigns.iter_mut(), new_assign).unwrap();
                                al::reallocate_on_weights(
                                    c_assigns.iter_mut(),
                                    &weights.hv_get(axis)[area[0]..area[1]],
                                    1.0,
                                );
                            }
                        }
                    }
                    *asgs = c_assigns;
                }
                _ => unreachable!(),
            }
        }
        _ => todo!(), // surround in
    }
}

fn assign_interval(
    unit: f32,
    intervals: &[usize],
    intervals_val: &mut [AssignVal],
    assign: &mut f32,
    base_len: usize,
    limit: Option<&config::Exps<'_>>,
) {
    let mut base_len = base_len as f32;
    let scale = *assign / base_len as f32;
    for (i, &l) in intervals.iter().enumerate() {
        let b_len = l as f32;
        base_len -= b_len;
        let mut val = scale * b_len;

        if l != 0
            && let Some(limit) = limit
        {
            match limit.eval(&[("val", val as f64), ("base", l as f64)].into()) {
                Ok(r) => val = r as f32,
                Err(e) => log::error!("Assign interval: {e}"),
            }
        }

        let mut interval = AssignVal::new(unit * b_len, 0.0);
        interval.uncheck_set(val);

        intervals_val[i] = interval;
    }

    let mut c_assign = AssignVal::from_base(
        (*assign - intervals_val.iter().sum::<AssignVal>().total()).max(unit * base_len),
        unit * base_len,
    );
    let changes: Vec<_> = intervals_val
        .iter()
        .map(|_| true)
        .chain(std::iter::once(false))
        .collect();
    al::reassign_in(
        intervals_val
            .iter_mut()
            .chain(std::iter::once(&mut c_assign)),
        &changes,
        *assign,
    )
    .unwrap();
    *assign -= intervals_val.iter().sum::<AssignVal>().total();
}

fn assign_blank_space(
    service: &impl Service,
    assigns: &mut DataHV<f32>,
    comb: &mut StrucComb,
    setting: config::WhiteAssignSetting,
) -> (DataHV<usize>, DataHV<f32>) {
    let cfg_units = &service.get_config().units;
    let mut levels = DataHV::splat(0);
    let mut units = DataHV::splat(0.0);

    for axis in Axis::list() {
        let mut base_len = comb.get_bases_length(axis, true) as f32;
        if base_len != 0.0 {
            let white_cfg = setting.white.hv_get(axis);
            let visual_corr = setting.corr.hv_get(axis);

            let assign = assigns.hv_get_mut(axis);

            let white_base = comb.get_blank_base(axis).map(|v| v as f32);
            let mut scale = *assign / base_len;
            base_len -= white_base[0] + white_base[1];

            let (level, unit) = cfg_units
                .hv_get(axis)
                .iter()
                .copied()
                .enumerate()
                .find(|(_, u)| scale + al::NORMAL_OFFSET >= *u)
                .unwrap();
            scale = al::correction_val(unit, scale, al::NORMAL_OFFSET);
            let white_unit = setting
                .units
                .hv_get(axis)
                .as_ref()
                .and_then(|units| units.get(level).or(units.last()))
                .copied()
                .unwrap_or(unit)
                .min(unit);
            *levels.hv_get_mut(axis) = level;
            *units.hv_get_mut(axis) = unit;

            let mut assigns_val = vec![
                AssignVal::from_base(white_base[0] * scale, white_unit * white_base[0]),
                AssignVal::from_base(base_len * scale, unit * base_len),
                AssignVal::from_base(white_base[1] * scale, white_unit * white_base[1]),
            ];
            let weights = vec![
                white_base[0] * white_cfg[0],
                base_len,
                white_base[1] * white_cfg[1],
            ];
            al::reallocate_on_weights(assigns_val.iter_mut(), &weights, 1.0);

            if base_len.abs() > 0.5 {
                // No zero
                Side::fb().into_iter().for_each(|side| {
                    let vcorr = (1.0
                        - comb
                            .get_edge(axis, side, false)
                            .sharpness(SharpnessModel::ZeroOne))
                        * visual_corr[side.n()];
                    if vcorr < assigns_val[[0, 2][side.n()]].excess {
                        assigns_val[1].excess += vcorr;
                        assigns_val[[0, 2][side.n()]].excess -= vcorr;
                    } else {
                        assigns_val[1].excess += assigns_val[[0, 2][side.n()]].excess;
                        assigns_val[[0, 2][side.n()]].excess = 0.0;
                    }
                });
            }

            *comb.blanks.hv_get_mut(axis) = [assigns_val[0], assigns_val[2]];
            *assign = assigns_val[1].total();
        }
    }

    (levels, units)
}

fn assign_single_space(
    proto: &StrucProto,
    assigns: DataHV<f32>,
    units: DataHV<f32>,
    weights: &DataHV<Vec<f32>>,
    _surround_area: Option<DataHV<[usize; 2]>>,
) -> DataHV<Vec<AssignVal>> {
    let allocs = proto.allocation_space();
    Axis::hv().into_map(|axis| {
        let allocs = allocs.hv_get(axis);

        let alloc_total = allocs.iter().sum::<usize>() as f32;
        if alloc_total == 0.0 {
            vec![Default::default(); allocs.len()]
        } else {
            let assign = *assigns.hv_get(axis);
            let weights = weights.hv_get(axis);
            let unit = *units.hv_get(axis);

            let scale = assign / alloc_total;
            let mut asgs: Vec<_> = allocs
                .iter()
                .map(|&n| {
                    let n = n as f32;
                    let space = n * scale;
                    let base = n * unit;
                    AssignVal::new(base, space - base)
                })
                .collect();

            // if let Some(area) = surround_area {
            //     let area = area.hv_get(axis);
            //     al::reallocate_on_weights(&mut asgs[..area[0]], &weights[..area[0]], 1.0);
            //     al::reallocate_on_weights(
            //         &mut asgs[area[0]..area[1]],
            //         &weights[area[0]..area[1]],
            //         1.0,
            //     );
            //     al::reallocate_on_weights(&mut asgs[area[1]..], &weights[area[1]..], 1.0);
            // } else {
            //     al::reallocate_on_weights(&mut asgs, weights, 1.0);
            // }
            al::reallocate_on_weights(asgs.iter_mut(), weights, 1.0);

            asgs
        }
    })
}

fn assign(
    service: &impl Service,
    comb: &mut StrucComb,
    mut assigns: DataHV<f32>,
    setting: config::WhiteAssignSetting,
) {
    let (levels, units) = assign_blank_space(service, &mut assigns, comb, setting);

    match &mut comb.cdata {
        CompData::Single {
            proto,
            assigns: asgs,
            level: c_levels,
            ..
        } => {
            *c_levels = levels;
            let weights = proto.subarea_weight(assigns);
            *asgs = assign_single_space(proto, assigns, units, &weights, None);
        }
        CompData::Scale {
            axis: c_axis,
            comps,
            intervals,
            intervals_val,
            ..
        } => {
            let mut assign = *assigns.hv_get(*c_axis);
            let lengths: Vec<_> = comps
                .iter()
                .map(|c| c.get_bases_length(*c_axis, true))
                .collect();
            let base_len = lengths.iter().chain(intervals.iter()).sum::<usize>();

            *intervals_val = vec![Default::default(); intervals.len()];
            assign_interval(
                units.hv_get_into(*c_axis),
                intervals,
                intervals_val,
                &mut assign,
                base_len,
                service
                    .get_config()
                    .get_interval_limit(*c_axis, false)
                    .as_ref(),
            );

            let base_len = lengths.iter().sum::<usize>();
            let scale = if base_len != 0 {
                assign / base_len as f32
            } else {
                0.0
            };
            for (c, l) in comps.iter_mut().zip(lengths) {
                let mut new_assigns = assigns.clone();
                *new_assigns.hv_get_mut(*c_axis) = scale * l as f32;
                assign_space(service, c, new_assigns);
            }
        }
        CompData::Surround {
            surround,
            comps,
            intervals,
            intervals_val,
            ..
        } => {
            let (l, r) = comps.split_at_mut(1);
            let primary = &mut l[0];
            let secondary = &mut r[0];

            let base_len = Axis::hv().into_map(|axis| {
                comb::surround::get_bases_length(primary, secondary, surround, intervals, axis)
            });
            assign_space_in_surround(service, primary, surround, assigns, base_len);

            let area_assigns = comb::surround::get_area_assign_fb(primary, surround);
            let s_assign = Axis::hv().into_map(|axis| {
                let mut area_assign = area_assigns.hv_get(axis)[1].total();
                let s_len = secondary.get_bases_length(axis, true);
                let intervals = intervals.hv_get_mut(axis);
                let sub_area_len = s_len + intervals.iter().sum::<usize>();

                let intervals_val = intervals_val.hv_get_mut(axis);
                assign_interval(
                    units.hv_get_into(axis),
                    intervals,
                    intervals_val,
                    &mut area_assign,
                    sub_area_len,
                    service.get_config().get_interval_limit(axis, true).as_ref(),
                );
                area_assign
            });
            assign(
                service,
                secondary,
                s_assign,
                service.get_config().get_white_assign_settings_surround(),
            );
        }
    }
}

pub fn assign_space(service: &impl Service, comb: &mut StrucComb, assigns: DataHV<f32>) {
    assign(
        service,
        comb,
        assigns,
        service.get_config().get_white_assign_settings_axis(),
    );
}

pub fn process_space(service: &impl Service, comb: &mut StrucComb) {
    let cfg = service.get_config();
    if let (order, Some(setting)) = cfg.get_space_ctrls() {
        for ctrl in order {
            match (ctrl, setting.get(ctrl)) {
                ("subarea", Some(v)) => space::ctrl_subarea(comb, v),
                ("trend", Some(v)) => space::ctrl_trend(comb, v),
                ("subcomp", Some(v)) => space::ctrl_subcomp(comb, v),
                ("interval", Some(v)) => space::ctrl_interval(comb, v),
                ("edge_area", Some(v)) => match serde_json::from_value(v.clone()) {
                    Ok(settings) => space::ctrl_edge_area(
                        comb,
                        &settings,
                        &Default::default(),
                        &Default::default(),
                    ),
                    Err(e) => log::error!("Error space setting in edge area: {e}"),
                },
                _ => log::warn!("Incorrect space control label: {ctrl}"),
            }
        }
    }
}
