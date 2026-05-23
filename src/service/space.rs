use super::algorithm as al;
use crate::{
    base::*,
    combination::{CompData, StrucComb, attrs, view},
    config,
};

use serde_json as sj;

pub fn ctrl_subarea(comb: &mut StrucComb, value: &sj::Value) {
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
            let alloc_weights =
                proto.subarea_weight(assigns.map(|list| list.iter().map(|v| v.total()).sum()));
            let weights = proto.subarea_line_weight(&alloc_weights, zero);

            Axis::list().into_iter().for_each(|axis| {
                if let Some(factor) = *factor.hv_get(axis) {
                    al::reallocate_on_weights(
                        assigns.hv_get_mut(axis),
                        weights.hv_get(axis),
                        factor,
                    );
                }
            });
        }
        CompData::Scale { comps, .. } => comps.iter_mut().for_each(|c| ctrl_subarea(c, value)),
        CompData::Surround { .. } => todo!(), // surround
    }
}

pub fn ctrl_subcomp(comb: &mut StrucComb, value: &sj::Value) {
    static DEFAULT_SETTING: (f32, bool, [f32; 3]) = (1.0, false, [1.0; 3]);
    match &mut comb.cdata {
        CompData::Single { .. } => {}
        CompData::Scale { comps, axis, .. } => {
            let settings = config::get_axis_val(value).into_map(|val| match val {
                None => DEFAULT_SETTING,
                Some(val) => (
                    val.get("factor")
                        .and_then(|val| val.as_f64().map(|val| val as f32))
                        .unwrap_or(DEFAULT_SETTING.0),
                    val.get("same")
                        .and_then(|val| val.as_bool())
                        .unwrap_or(DEFAULT_SETTING.1),
                    val.get("section")
                        .and_then(|val| sj::from_value::<[f32; 3]>(val.clone()).ok())
                        .unwrap_or(DEFAULT_SETTING.2),
                ),
            });

            let axis = *axis;
            let (factor, same, section_weight) = settings.hv_get(axis);
            let line_weights: Vec<f32> =
                if *same && comps.iter().skip(1).all(|c| c.name == comps[0].name) {
                    vec![1.0; comps.len()]
                } else {
                    comps
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let base = c.get_bases_length(axis, true);
                            if base == 0 {
                                0.0
                            } else {
                                let mut weight = c.get_line_weight().reduce(|h, v| *h + *v)
                                    + section_weight[Section::from_idx(i, comps.len()).n()];
                                if let CompData::Single { proto, .. } = &c.cdata {
                                    let factors = proto
                                        .attrs
                                        .get::<attrs::LineWeight>()
                                        .unwrap_or(DataHV::splat(None));

                                    weight *= factors.hv_get(axis).unwrap_or(1.0);
                                }

                                weight
                            }
                        })
                        .collect()
                };
            let mut comp_assigns: Vec<AssignVal> = comps
                .iter()
                .map(|c| c.get_assign_value(axis, true))
                .collect();

            al::reallocate_on_weights(&mut comp_assigns, &line_weights, *factor);
            comps.iter_mut().zip(comp_assigns).for_each(|(c, v)| {
                c.reassign_space(v.total(), false, axis);
                ctrl_subcomp(c, value);
            });
        }
        CompData::Surround { .. } => todo!(), // surround
    }
}

pub fn ctrl_trend(comb: &mut StrucComb, value: &sj::Value) {
    match &mut comb.cdata {
        CompData::Single { .. } => {}
        CompData::Scale { comps, axis, .. } => {
            let axis = axis.inverse();
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

            match value
                .get("scale")
                .and_then(|value| value.get(axis.symbol()))
                .map(|val| sj::from_value::<Vec<config::EdgeCheck<f32>>>(val.clone()))
            {
                Some(Ok(settings)) => {
                    for i in 0..comps.len() {
                        let blanks = comps[i].get_blank_base(axis);
                        for tcheck in settings.iter() {
                            if blanks
                                .iter()
                                .zip(tcheck.setup.iter())
                                .any(|(b, s)| s.is_some() == (*b == 0))
                            {
                                continue;
                            }

                            let r = tcheck.is_match(
                                axis,
                                i,
                                comps.len(),
                                |i, axis, side| get_edge(i, axis, side, &mut edge_datas, comps),
                                |k, _| Err(config::CheckError::UnknowKey(k.to_string())),
                            );

                            match r {
                                Ok(r) => {
                                    if r {
                                        let c_assign = comps[i].get_assign_value(axis, false);
                                        let blanks = comps[i].blanks.hv_get_mut(axis);
                                        let mut assign_list: Vec<_> = blanks
                                            .iter()
                                            .copied()
                                            .chain(std::iter::once(c_assign))
                                            .collect();
                                        let b_weight: Vec<f32> = [0, 1]
                                            .into_iter()
                                            .map(|j| {
                                                tcheck.setup[j].unwrap_or(1.0) * blanks[j].total()
                                            })
                                            .chain(std::iter::once(c_assign.total()))
                                            .collect();

                                        al::reallocate_on_weights(&mut assign_list, &b_weight, 1.0);
                                        blanks
                                            .iter_mut()
                                            .zip(assign_list.iter())
                                            .for_each(|(b, v)| *b = *v);
                                        comps[i].reassign_space(
                                            assign_list[2].total(),
                                            false,
                                            axis,
                                        );
                                        break;
                                    }
                                }
                                Err(e) => eprintln!("In subcomp edge setting: {e}"),
                            }
                        }
                    }
                }
                Some(Err(e)) => eprintln!("Error in Trend settings: {e}"),
                None => {}
            }

            comps.iter_mut().for_each(|c| ctrl_trend(c, value));
        }
        CompData::Surround { .. } => todo!(), // surround
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
