use super::Service;
use super::algorithm as al;
use super::space;
use crate::{
    base::*,
    combination::{CompData, SharpnessModel, StrucComb, attrs},
    construct::{CharTree, Component, CpAttrs, CstError, CstType},
};

// start point
pub fn get_char_tree(service: &impl Service, name: String) -> CharTree {
    get_char_tree_in(
        service,
        name,
        (CstType::Single, Section::Start),
        Default::default(),
    )
}

pub fn get_comp_attrs<'a, 'b>(service: &'a impl Service, name: &'b str) -> Option<&'a CpAttrs> {
    service
        .get_config()
        .supplement
        .get(name)
        .or(service.get_table().get(name))
}

fn comb_remap(service: &impl Service, attrs: &mut CpAttrs, adjacency: DataHV<[bool; 2]>) {
    // if let CstType::Scale(axis) = attrs.tp {
    // let mut idx = 0;
    // while idx != attrs.components.len() {
    //     let c = &attrs.components[idx];
    //     let c_attrs = match c {
    //         Component::Char(p_name) => {
    //             get_char_attrs_in(
    //                 service,
    //                 p_name.clone(),
    //                 (attrs.tp, Section::Start),
    //                 adjacency,
    //             )
    //             .1
    //         }
    //         Component::Complex(p_attrs) => p_attrs.clone(),
    //     };
    //     match c_attrs.tp {
    //         CstType::Scale(c_axis) if c_axis == axis => {
    //             attrs.components.splice(idx..idx + 1, c_attrs.components);
    //             continue;
    //         }
    //         _ => {}
    //     }

    //     idx += 1;
    // }
    // }
    if let CstType::Surround(surround_place) = attrs.tp {
        let primary = &attrs.components[0];
        let mut p_attrs = match primary {
            Component::Char(p_name) => {
                get_char_attrs_in(
                    service,
                    p_name.clone(),
                    (attrs.tp, Section::Start),
                    adjacency,
                )
                .1
            }
            Component::Complex(p_attrs) => p_attrs.clone(),
        };

        if let CstType::Scale(c_axis) = p_attrs.tp {
            let index = match surround_place.hv_get(c_axis) {
                Section::Start => p_attrs.components.len() - 1,
                Section::End => 0,
                Section::Middle => {
                    eprintln!(
                        "{} {} surround component in {}",
                        primary.name(),
                        p_attrs.comps_name(),
                        CstType::Surround(surround_place).symbol()
                    );
                    return;
                }
            };
            let secondary = attrs.components.pop().unwrap();
            *attrs = p_attrs;
            attrs.components[index] = Component::Complex(CpAttrs {
                tp: CstType::Surround(surround_place),
                components: vec![attrs.components[index].clone(), secondary],
            })
        } else if let CstType::Surround(c_surround) = p_attrs.tp {
            if c_surround == surround_place {
                let sc1 = p_attrs.components.pop().unwrap();
                let pc = p_attrs.components.pop().unwrap();
                let sc = if c_surround.v == Section::End {
                    vec![attrs.components[1].clone(), sc1]
                } else {
                    vec![sc1, attrs.components[1].clone()]
                };
                *attrs = CpAttrs {
                    tp: attrs.tp,
                    components: vec![
                        pc,
                        Component::Complex(CpAttrs {
                            tp: CstType::Scale(Axis::Vertical),
                            components: sc,
                        }),
                    ],
                };
            } else {
                eprintln!(
                    "{} {} surround component in {}",
                    primary.name(),
                    p_attrs.comps_name(),
                    CstType::Surround(surround_place).symbol()
                );
                return;
            }
        }
    }
}

fn get_char_attrs_in(
    service: &impl Service,
    name: String,
    in_tp: (CstType, Section),
    adjacency: DataHV<[bool; 2]>,
) -> (String, CpAttrs) {
    let name = service
        .get_config()
        .check_name_replace(&name, in_tp, adjacency)
        .unwrap_or(name);
    let attrs = get_comp_attrs(service, &name)
        .cloned()
        .unwrap_or(CpAttrs::single());

    (name, attrs)
}

fn get_char_tree_in(
    service: &impl Service,
    name: String,
    in_tp: (CstType, Section),
    adjacency: DataHV<[bool; 2]>,
) -> CharTree {
    let (name, attrs) = get_char_attrs_in(service, name, in_tp, adjacency);
    get_tree_from_attrs(service, name, attrs, adjacency)
}

fn get_tree_from_attrs(
    service: &impl Service,
    name: String,
    mut attrs: CpAttrs,
    adjacency: DataHV<[bool; 2]>,
) -> CharTree {
    comb_remap(service, &mut attrs, adjacency);
    fn get_tree_from_comp(
        service: &impl Service,
        comp: Component,
        in_tp: (CstType, Section),
        adjacency: DataHV<[bool; 2]>,
    ) -> CharTree {
        match comp {
            Component::Char(c_name) => get_char_tree_in(service, c_name, in_tp, adjacency),
            Component::Complex(c_attrs) => {
                get_tree_from_attrs(service, c_attrs.comps_name(), c_attrs, adjacency)
            }
        }
    }

    match attrs.tp {
        CstType::Single => CharTree::new_single(name),
        CstType::Scale(axis) => {
            let end = attrs.components.len();
            let children = attrs
                .components
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut c_adjacency = adjacency.clone();
                    if i != 0 {
                        c_adjacency.hv_get_mut(axis)[0] = true;
                    }
                    if i + 1 != end {
                        c_adjacency.hv_get_mut(axis)[1] = true;
                    }
                    let in_tp = match i {
                        0 => Section::Start,
                        n if n + 1 == end => Section::End,
                        _ => Section::Middle,
                    };
                    get_tree_from_comp(service, c, (attrs.tp, in_tp), c_adjacency)
                })
                .collect();
            CharTree {
                name,
                tp: attrs.tp,
                children,
            }
        }
        CstType::Surround(surround_place) => {
            let mut adjacency = [adjacency.clone(); 2];
            Axis::list().into_iter().for_each(|axis| {
                let surround_place = *surround_place.hv_get(axis);
                if surround_place != Section::End {
                    // in_place[0].hv_get_mut(axis)[1] = true;
                    adjacency[1].hv_get_mut(axis)[0] = true;
                }
                if surround_place != Section::Start {
                    // in_place[0].hv_get_mut(axis)[0] = true;
                    adjacency[1].hv_get_mut(axis)[1] = true;
                }
            });

            let sc = get_tree_from_comp(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Section::End),
                adjacency[1],
            );
            let pc = get_tree_from_comp(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Section::Start),
                adjacency[0],
            );

            CharTree {
                name,
                tp: attrs.tp,
                children: vec![pc, sc],
            }
        }
    }
}

pub fn get_comb_proto_in(
    service: &impl Service,
    target: CharTree,
    adjacency: DataHV<[bool; 2]>,
) -> Result<StrucComb, CstError> {
    match target.tp {
        CstType::Single => {
            let mut proto = service
                .get_struc_proto(&target.name)
                .cloned()
                .ok_or_else(|| CstError::Empty(target.name.clone()))?;

            proto.set_allocs_in_adjacency(adjacency);
            Ok(StrucComb::new_single(target.name, proto))
        }
        CstType::Scale(axis) => {
            let children = target.children;
            let mut combs = Vec::with_capacity(children.len());

            let end = children.len() - 1;
            for (i, c_target) in children.into_iter().enumerate() {
                let mut c_in_place = adjacency.clone();
                if i != 0 {
                    c_in_place.hv_get_mut(axis)[0] = true;
                }
                if i != end {
                    c_in_place.hv_get_mut(axis)[1] = true;
                }

                combs.push(get_comb_proto_in(service, c_target, c_in_place)?);
            }
            Ok(StrucComb::new_complex(target.name, target.tp, combs))
        }
        CstType::Surround(..) => Err(CstError::Empty(target.tp.symbol().to_string())), // todo!() complex
    }
    .map(|mut comb| {
        comb.attrs.set::<attrs::Adjacencies>(&adjacency);
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
                let new_tree = get_char_tree(service, new_name.to_string());
                let mut new_comb = get_comb_proto_in(
                    service,
                    new_tree,
                    comb.attrs.get::<attrs::Adjacencies>().unwrap(),
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
        CompData::Surround { .. } => todo!(), // surround
    }
}

fn init_edge_at_scale(
    service: &impl Service,
    comps: &mut Vec<StrucComb>,
    intervals: &mut Vec<usize>,
    axis: Axis,
) -> Result<DataHV<usize>, CstError> {
    enum Record {
        None,
        Name {
            name: String,
            sub_recard: Vec<Record>,
        },
    }

    fn flatten(comp: StrucComb, axis: Axis, records: &mut Vec<Record>) -> Vec<StrucComb> {
        match comp.cdata {
            CompData::Scale {
                axis: c_axis,
                comps,
                ..
            } if c_axis == axis => {
                let mut new_records = Vec::with_capacity(comps.len());
                let list = comps
                    .into_iter()
                    .flat_map(|c| flatten(c, axis, &mut new_records))
                    .collect();
                records.push(Record::Name {
                    name: comp.name,
                    sub_recard: new_records,
                });
                list
            }
            _ => {
                records.push(Record::None);
                vec![comp]
            }
        }
    }

    fn restore(
        records: Vec<Record>,
        comps: &mut Vec<StrucComb>,
        intervals: &mut Vec<usize>,
        axis: Axis,
    ) -> (Vec<StrucComb>, Vec<usize>) {
        let mut new_comps = Vec::with_capacity(records.len());
        let mut new_intervals = vec![0; records.len() - 1];
        for (i, record) in records.into_iter().enumerate() {
            if i != new_intervals.len() {
                new_intervals[i] = intervals.pop().unwrap();
            }
            match record {
                Record::None => {
                    new_comps.push(comps.pop().unwrap());
                }
                Record::Name { name, sub_recard } => {
                    let (sub_comps, sub_intervals) = restore(sub_recard, comps, intervals, axis);
                    let mut sub_comp =
                        StrucComb::new_complex(name, CstType::Scale(axis), sub_comps);
                    if let CompData::Scale { intervals, .. } = &mut sub_comp.cdata {
                        *intervals = sub_intervals;
                    }
                    new_comps.push(sub_comp);
                }
            }
        }
        (new_comps, new_intervals)
    }

    let mut records = Vec::with_capacity(comps.len());
    let mut new_comps: Vec<StrucComb> = comps
        .drain(..)
        .flat_map(|c| flatten(c, axis, &mut records))
        .collect();

    let mut len_list: Vec<DataHV<usize>> = Vec::with_capacity(new_comps.len());
    let cfg = service.get_config();

    let mut new_intervals = loop {
        len_list.clear();
        for c in new_comps.iter_mut() {
            len_list.push(init_edges(service, c)?);
        }

        cfg.set_main_comp_axis(
            &mut new_comps,
            axis.inverse(),
            &len_list
                .iter()
                .map(|list| *list.hv_get(axis.inverse()))
                .collect(),
        );
        if let Some(new_intervals) = cfg.set_intervals_axis(&mut new_comps, axis) {
            break new_intervals;
        }
    };

    let size = Axis::hv().into_map(|i_axis| {
        if i_axis == axis {
            len_list.iter().map(|cl| *cl.hv_get(i_axis)).sum::<usize>()
                + new_intervals.iter().sum::<usize>()
        } else {
            new_comps
                .iter()
                .zip(len_list.iter())
                .map(|(c, l)| c.get_blank_base(i_axis).iter().sum::<usize>() + *l.hv_get(i_axis))
                .max()
                .unwrap()
        }
    });

    new_comps.reverse();
    new_intervals.reverse();
    let restore = restore(records, &mut new_comps, &mut new_intervals, axis);
    *comps = restore.0;
    *intervals = restore.1;

    Ok(size)
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
        CompData::Surround { .. } => todo!(), // surround
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

pub fn assign_space(service: &impl Service, comb: &mut StrucComb, mut assigns: DataHV<f32>) {
    let cfg_units = &service.get_config().units;
    let mut levels = DataHV::splat(0);
    let mut units = DataHV::splat(0.0);

    let space_setting = service.get_config().get_space_assign_settings();
    for axis in Axis::list() {
        let mut base_len = comb.get_bases_length(axis, true) as f32;
        if base_len != 0.0 {
            let white_cfg = space_setting.0.hv_get(axis);
            let visual_corr = space_setting.1.hv_get(axis);

            let assign = assigns.hv_get_mut(axis);

            let white_base = comb.get_blank_base(axis).map(|v| v as f32);
            let scale = *assign / base_len;
            base_len -= white_base[0] + white_base[1];

            let (level, unit) = cfg_units
                .hv_get(axis)
                .iter()
                .copied()
                .enumerate()
                .find(|(_, u)| scale >= *u)
                .unwrap();
            let white_unit = space_setting.2.hv_get(axis).unwrap_or(unit).min(unit);
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
            al::reallocate_on_weights(&mut assigns_val, &weights, 1.0);

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

    match &mut comb.cdata {
        CompData::Single {
            proto,
            assigns: asgs,
            level: c_levels,
            ..
        } => {
            *c_levels = levels;
            let allocs = proto.allocation_space();
            let weights = proto.subarea_weight(assigns);
            for axis in Axis::list() {
                let allocs = allocs.hv_get(axis);

                let alloc_total = allocs.iter().sum::<usize>() as f32;
                *asgs.hv_get_mut(axis) = if alloc_total == 0.0 {
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

                    al::reallocate_on_weights(&mut asgs, weights, 1.0);
                    asgs
                }
            }
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
            let scale = assign / base_len as f32;

            let limit = service
                .get_config()
                .get_interval_limit(*c_axis)
                .unwrap_or(1.0);
            let unit = *units.hv_get(*c_axis);
            for &l in intervals.iter() {
                let b_len = l as f32;
                let base = unit * b_len;
                let excess = (scale * b_len * limit - base).max(0.0);
                assign -= base + excess;

                intervals_val.push(AssignVal::new(base, excess));
            }

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
        CompData::Surround { .. } => todo!(), // surround
    }
}

pub fn process_space(service: &impl Service, comb: &mut StrucComb) {
    let cfg = service.get_config();
    if let (order, Some(setting)) = cfg.get_space_ctrls() {
        for ctrl in order {
            match (ctrl, setting.get(ctrl)) {
                ("subarea", Some(v)) => space::ctrl_subarea(comb, v),
                ("trend", Some(v)) => space::ctrl_trend(comb, v),
                ("subcomp", Some(v)) => space::ctrl_subcomp(comb, v),
                _ => log::warn!("Incorrect space control label: {ctrl}"),
            }
        }
    }
}
