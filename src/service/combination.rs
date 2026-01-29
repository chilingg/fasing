use super::Service;
use super::algorithm as al;
use crate::{
    base::*,
    combination::{CompData, SharpnessModel, StrucComb, attrs},
    construct::{CharTree, Component, CpAttrs, CstError, CstType},
};

pub fn get_comp_attrs<'a, 'b>(service: &'a impl Service, name: &'b str) -> Option<&'a CpAttrs> {
    service
        .get_config()
        .supplement
        .get(name)
        .or(service.get_table().get(name))
}

pub fn get_char_tree(service: &impl Service, name: String) -> CharTree {
    get_char_tree_in(
        service,
        name,
        (CstType::Single, Place::Start),
        Default::default(),
    )
}

fn surround_comb_remap(service: &impl Service, attrs: &mut CpAttrs, adjacency: DataHV<[bool; 2]>) {
    if let CstType::Surround(surround_place) = attrs.tp {
        let primary = &attrs.components[0];
        let mut p_attrs = match primary {
            Component::Char(p_name) => {
                get_char_attrs_in(service, p_name.clone(), (attrs.tp, Place::Start), adjacency).1
            }
            Component::Complex(p_attrs) => p_attrs.clone(),
        };

        if let CstType::Scale(c_axis) = p_attrs.tp {
            let index = match surround_place.hv_get(c_axis) {
                Place::Start => p_attrs.components.len() - 1,
                Place::End => 0,
                Place::Middle => {
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
                let sc = if c_surround.v == Place::End {
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
    in_tp: (CstType, Place),
    adjacency: DataHV<[bool; 2]>,
) -> (String, CpAttrs) {
    let name = service
        .get_config()
        .check_name_replace(&name, in_tp, adjacency)
        .unwrap_or(name);
    let mut attrs = get_comp_attrs(service, &name)
        .cloned()
        .unwrap_or(CpAttrs::single());
    surround_comb_remap(service, &mut attrs, adjacency);

    (name, attrs)
}

fn get_char_tree_in(
    service: &impl Service,
    name: String,
    in_tp: (CstType, Place),
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
    fn get_tree_from_comp(
        service: &impl Service,
        comp: Component,
        in_tp: (CstType, Place),
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
                        0 => Place::Start,
                        n if n + 1 == end => Place::End,
                        _ => Place::Middle,
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
                if surround_place != Place::End {
                    // in_place[0].hv_get_mut(axis)[1] = true;
                    adjacency[1].hv_get_mut(axis)[0] = true;
                }
                if surround_place != Place::Start {
                    // in_place[0].hv_get_mut(axis)[0] = true;
                    adjacency[1].hv_get_mut(axis)[1] = true;
                }
            });

            let sc = get_tree_from_comp(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Place::End),
                adjacency[1],
            );
            let pc = get_tree_from_comp(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Place::Start),
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
        _ => todo!(), // complex
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

                let old_len = comb.get_bases_length(axis);
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
                        let new_len = new_comb.get_bases_length(axis.inverse());
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

    match &comb.cdata {
        CompData::Single { .. } => replace(service, comb, axis),
        CompData::Complex { .. } => todo!(), // complex
    }
}

fn init_edges(_service: &impl Service, comb: &mut StrucComb) -> Result<DataHV<usize>, CstError> {
    match &mut comb.cdata {
        CompData::Single { proto, .. } => {
            if proto.is_empty() {
                return Err(CstError::Empty(comb.name.clone()));
            }
            proto.attrs.set::<attrs::FixedAlloc>(&Default::default());
            Ok(proto.allocation_values().map(|allocs| allocs.iter().sum()))
        }
        CompData::Complex { .. } => todo!(), // complex
    }
}

pub fn check_space(
    service: &impl Service,
    comb: &mut StrucComb,
) -> Result<(DataHV<f32>, DataHV<[AssignVal; 2]>, DataHV<usize>), CstError> {
    let cfg = service.get_config();
    let zishen = cfg
        .size
        .zip(comb.get_char_box().size().to_hv_data())
        .into_map(|(a, b)| a * b);

    let mut assign = DataHV::default();
    let mut offsets: DataHV<[AssignVal; 2]> = DataHV::default();
    let mut levels: DataHV<usize> = DataHV::default();

    let mut base_lens = init_edges(service, comb)?;
    let mut check_state: Vec<Axis> = Axis::list().into();
    let mut first = true;

    while !check_state.is_empty() {
        let axis = check_state[0];
        let zishen = *zishen.hv_get(axis);
        let zimian_gener = cfg.zimian.hv_get(axis);
        let white = (*cfg.size.hv_get(axis) - zimian_gener.max_val()) / 2.0;
        let reduce_trigger = cfg.get_reduce_trigger(axis);

        loop {
            let base_len = *base_lens.hv_get(axis);
            let mut length = zimian_gener.val_in(base_len).min(zishen - 2.0 * white);

            let r = cfg
                .min_val
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

                    if base_len == 0 {
                        length = 0.0;
                    }
                    *assign.hv_get_mut(axis) = length;
                    *offsets.hv_get_mut(axis) =
                        [AssignVal::new(white, ((zishen - length) / 2.0 - white).max(0.0)); 2];
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

    Ok((assign, offsets, levels))
}

pub fn assign_space(
    service: &impl Service,
    comb: &mut StrucComb,
    assigns: DataHV<f32>,
    offsets: DataHV<[AssignVal; 2]>,
    levels: DataHV<usize>,
) {
    let min_val =
        Axis::hv().into_map(|axis| service.get_config().min_val.hv_get(axis)[*levels.hv_get(axis)]);
    comb.offsets = offsets;

    match &mut comb.cdata {
        CompData::Single {
            proto,
            view,
            assigns: asgs,
            ..
        } => {
            let allocs = proto.allocation_space();
            for axis in Axis::list() {
                let mut assign = *assigns.hv_get(axis);
                let offsets = comb.offsets.hv_get_mut(axis);

                let vc_val = service.get_config().get_visual_corr(axis);
                Place::se().into_iter().for_each(|place| {
                    let idx = place.index(0, 1);
                    let vcorr = ((1.0 - view.edge_sharpness(axis, place, SharpnessModel::ZeroOne))
                        * vc_val)
                        .min(offsets[place.index(0, 1)].excess);
                    assign += vcorr;
                    offsets[idx].excess -= vcorr;
                });

                let allocs = allocs.hv_get(axis);
                let min_val = *min_val.hv_get(axis);

                let alloc_total = allocs.iter().sum::<usize>() as f32;
                *asgs.hv_get_mut(axis) = if alloc_total == 0.0 {
                    vec![Default::default(); allocs.len()]
                } else {
                    let scale = assign / alloc_total;
                    allocs
                        .iter()
                        .map(|&n| {
                            let n = n as f32;
                            let space = n * scale;
                            let base = n * min_val;
                            AssignVal {
                                base,
                                excess: space - base,
                            }
                        })
                        .collect()
                }
            }
        }
        CompData::Complex { .. } => todo!(), // complex
    }
}

pub fn process_space(service: &impl Service, comb: &mut StrucComb) {
    // todo!() // process
}
