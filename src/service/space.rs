use crate::{
    base::*,
    combination::{CompData, StrucComb},
    config::Config,
};

use super::algorithm as al;

pub fn subarea_ctrl(config: &Config, comb: &mut StrucComb) {
    match &mut comb.cdata {
        CompData::Single { proto, assigns, .. } => {
            let assigns_val = assigns.map(|assigns| assigns.iter().map(|av| av.total()).collect());
            let weights = proto.subarea_weight(&assigns_val);
            let settings = config.get_subareas_settings().unwrap_or_default();

            Axis::list().into_iter().for_each(|axis| {
                if let Some(settings) = settings.get(axis.symbol()) {
                    let assigns = assigns.hv_get_mut(axis);
                    let zero_weight =
                        settings.get("zero").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let weights: Vec<f32> = weights
                        .hv_get(axis)
                        .iter()
                        .enumerate()
                        .map(|(i, &w)| match w {
                            0.0 => assigns[i].total() * zero_weight,
                            w => w,
                        })
                        .collect();

                    al::scale_in_weights(
                        assigns,
                        &weights,
                        settings
                            .get("factor")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0) as f32,
                    );
                }
            });
        }
        CompData::Complex { comps, .. } => comps.iter_mut().for_each(|c| subarea_ctrl(config, c)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::algorithm as al;
    use super::*;

    #[test]
    fn test_subarea_ctrl() {
        let config: Config = serde_json::from_value(serde_json::json!({
            "subareas": {"h": {}}
        }))
        .unwrap();

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
            offsets: Default::default(),
            blank: DataHV::default(),
            cdata: CompData::Single {
                view: crate::combination::StrucView::new(&proto),
                proto,
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

        subarea_ctrl(&config, &mut comb);
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
            offsets: Default::default(),
            blank: DataHV::default(),
            cdata: CompData::Single {
                view: crate::combination::StrucView::new(&proto),
                proto,
                assigns: DataHV::new(
                    vec![AssignVal::new(0.1, 0.1); 4],
                    vec![AssignVal::new(0.1, 0.1); 2],
                ),
            },
            attrs: Default::default(),
        };

        subarea_ctrl(&config, &mut comb);
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
