use super::{
    attrs::{self, CompAttrs},
    view::Direction,
};
use crate::base::*;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct StrucProto {
    pub paths: Vec<IdxKeyPath>,
    pub attrs: CompAttrs,
}

impl<T: IntoIterator<Item = IdxKeyPath>> From<T> for StrucProto {
    fn from(paths: T) -> Self {
        StrucProto {
            paths: paths.into_iter().collect(),
            attrs: Default::default(),
        }
    }
}

impl StrucProto {
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn strokes(&self) -> Vec<String> {
        self.paths
            .iter()
            .filter(|path| !path.hide)
            .map(|path| {
                path.kpoints
                    .windows(2)
                    .map(|pos| Direction::new(pos[0].pos, Some(pos[1].pos)).symbol())
                    .fold(
                        String::with_capacity(
                            path.kpoints.len().checked_sub(1).unwrap_or_default(),
                        ),
                        |mut strokes, s| {
                            strokes.push(s);
                            strokes
                        },
                    )
            })
            .collect()
    }

    fn values(&self) -> DataHV<Vec<usize>> {
        self.paths
            .iter()
            .fold(DataHV::<BTreeSet<usize>>::default(), |mut set, path| {
                path.kpoints.iter().for_each(|p| {
                    for axis in Axis::list() {
                        set.hv_get_mut(axis).insert(*p.pos.hv_get(axis));
                    }
                });
                set
            })
            .into_map(|set| set.into_iter().collect())
    }

    pub fn size(&self) -> DataHV<usize> {
        self.allocation_values().into_map(|vals| vals.iter().sum())
    }

    pub fn values_map(&self, idx: bool) -> DataHV<BTreeMap<usize, usize>> {
        let values = self.values();
        match self.attrs.get::<attrs::Allocs>() {
            Some(allocs) => values.zip(allocs).into_map(|(values, allocs)| {
                let mut n = 0;
                values
                    .into_iter()
                    .zip(std::iter::once(0).chain(allocs))
                    .map(|(v, a)| {
                        let advence = if idx { 1 } else { a };
                        if a != 0 {
                            n += advence
                        }
                        (v, n)
                    })
                    .collect()
            }),
            None => values.into_map(|values| {
                if idx {
                    values.iter().enumerate().map(|a| (*a.1, a.0)).collect()
                } else {
                    let first = values.first().copied().unwrap_or_default();
                    values.iter().map(|&a| (a, a - first)).collect()
                }
            }),
        }
    }

    pub fn allocation_values_proto(&self) -> DataHV<Vec<usize>> {
        self.values()
            .into_map(|values| values.windows(2).map(|vec| vec[1] - vec[0]).collect())
    }

    pub fn allocation_values(&self) -> DataHV<Vec<usize>> {
        self.attrs
            .get::<attrs::Allocs>()
            .unwrap_or_else(|| self.allocation_values_proto())
    }

    pub fn allocation_space(&self) -> DataHV<Vec<usize>> {
        self.allocation_values()
            .into_map(|l| l.into_iter().filter(|n| *n != 0).collect())
    }

    pub fn set_allocs_in_adjacency(&mut self, adjacency: DataHV<[bool; 2]>) {
        let mut allocs_proto = self.allocation_values();
        if let Some(ipa) = self.attrs.get::<attrs::InPlaceAllocs>() {
            ipa.into_iter()
                .filter_map(
                    |(rule, allocs)| match crate::config::place_match(&rule, adjacency) {
                        true => Some(allocs),
                        false => None,
                    },
                )
                .for_each(|allocs| {
                    Axis::list().into_iter().for_each(|axis| {
                        allocs_proto
                            .hv_get_mut(axis)
                            .iter_mut()
                            .zip(allocs.hv_get(axis))
                            .for_each(|(val, exp)| {
                                if *val > *exp {
                                    *val = *exp;
                                }
                            })
                    });
                });
        }
        self.attrs.set::<attrs::Allocs>(&allocs_proto);
    }

    pub fn reduce(&mut self, axis: Axis, check: bool) -> bool {
        let mut ok = false;
        let mut allocs = self.allocation_values();
        if let Some(reduce_list) = self.attrs.get::<attrs::ReduceAlloc>() {
            let fiexd_alloc = self.attrs.get::<attrs::FixedAlloc>().unwrap_or_default();

            reduce_list.hv_get(axis).iter().find(|rl| {
                for (i, (r, l)) in rl
                    .iter()
                    .zip(allocs.hv_get_mut(axis).iter_mut())
                    .enumerate()
                {
                    if !fiexd_alloc.hv_get(axis).contains(&i) && *r < *l {
                        if !check {
                            *l -= 1;
                        }
                        ok = true;
                    }
                }
                ok
            });
        }

        if ok {
            self.attrs.set::<attrs::Allocs>(&allocs);
        }
        ok
    }

    pub fn get_paths(
        &self,
        start: WorkPoint,
        assigns: &DataHV<Vec<f32>>,
    ) -> Vec<Vec<WorkKeyPoint>> {
        let pos_to_alloc = self.values_map(true);
        let alloc_to_assign: DataHV<BTreeMap<usize, f32>> = start
            .to_hv_data()
            .zip(assigns.as_ref())
            .into_map(|(start, assigns)| {
                let mut origin = start;
                std::iter::once(origin)
                    .chain(assigns.iter().map(|assig| {
                        origin += assig;
                        origin
                    }))
                    .enumerate()
                    .collect()
            });
        let mut paths: Vec<_> = self
            .paths
            .iter()
            .filter(|p| !p.hide)
            .map(|path| {
                let kpoints: Vec<_> = path
                    .kpoints
                    .iter()
                    .map(|p| {
                        let pos = p
                            .pos
                            .to_hv_data()
                            .zip(pos_to_alloc.as_ref())
                            .zip(alloc_to_assign.as_ref())
                            .into_map(|((v, m1), m2)| m2[&m1[&v]]);
                        KeyPoint {
                            pos: WorkPoint::new(pos.h, pos.v),
                            labels: p.labels.clone(),
                        }
                    })
                    .collect();

                kpoints
            })
            .collect();

        paths.iter_mut().for_each(|path| {
            if path.len() > 2 {
                path.dedup_by_key(|p| p.pos);
            }
        });

        paths
    }

    pub fn subarea_line_weight(
        &self,
        assigns: &DataHV<Vec<f32>>,
        zero: DataHV<f32>,
    ) -> DataHV<Vec<f32>> {
        let values_map = self.values_map(true);
        let mut weights: DataHV<Vec<Option<f32>>> =
            Axis::hv().into_map(|axis| vec![None; assigns.hv_get(axis).len()]);

        self.paths.iter().for_each(|path| {
            let iter = path
                .kpoints
                .iter()
                .filter(|pos| !pos.is_mark())
                .map(|kp| IndexPoint::new(values_map.h[&kp.pos.x], values_map.v[&kp.pos.y]));
            iter.clone().zip(iter.skip(1)).for_each(|(p1, p2)| {
                let min = p1.min(p2);
                let max = p1.max(p2);

                let mut set_weights = |axis| {
                    for i in *min.hv_get(axis)..*max.hv_get(axis) {
                        *weights.hv_get_mut(axis)[i].get_or_insert_default() +=
                            assigns.hv_get(axis)[i]
                    }
                };

                match Direction::new(p1, Some(p2)) {
                    Direction::Above | Direction::Below => set_weights(Axis::Vertical),
                    Direction::Left | Direction::Right => set_weights(Axis::Horizontal),
                    Direction::None => {}
                    _ => Axis::list().into_iter().for_each(|axis| set_weights(axis)),
                }
            });
        });

        weights
            .zip(assigns.as_ref())
            .zip(zero)
            .into_map(|((weights, allocs), zero)| {
                weights
                    .into_iter()
                    .zip(allocs)
                    .map(|(w, a)| w.unwrap_or(zero * a))
                    .collect()
            })
    }

    pub fn subarea_weight(&self, size: DataHV<f32>) -> DataHV<Vec<f32>> {
        let allocs = self.allocation_values();
        let mut weights: DataHV<Vec<f32>> = allocs.map(|list| {
            list.iter()
                .filter(|&&v| v != 0)
                .map(|&v| v as f32)
                .collect()
        });

        if let Some(settings) = self.attrs.get::<attrs::AreaWeights>() {
            let mut size_weights = settings.get_weights(size);
            for axis in Axis::hv() {
                let setting = size_weights.hv_get_mut(axis);
                let allocs = allocs.hv_get(axis);
                if setting.len() == allocs.len() {
                    let mut iter = allocs.iter();
                    setting.retain(|_| *iter.next().unwrap() != 0);
                    weights
                        .hv_get_mut(axis)
                        .iter_mut()
                        .zip(setting)
                        .for_each(|(w, s)| {
                            *w *= *s;
                        });
                } else {
                    if !setting.is_empty() {
                        log::error!("Number of incorrect in weight settings!");
                    }
                }
            }
        }
        weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_values() {
        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
            ],
            attrs: Default::default(),
        };
        let values = struc.values();
        assert_eq!(values.h, vec![1, 2, 4]);
        assert_eq!(values.v, vec![0, 1, 2]);
        let values = struc.values_map(true);
        assert_eq!(values.h, BTreeMap::from([(1, 0), (2, 1), (4, 2)]));
        assert_eq!(values.v, BTreeMap::from([(0, 0), (1, 1), (2, 2)]));
        let values = struc.values_map(false);
        assert_eq!(values.h, BTreeMap::from([(1, 0), (2, 1), (4, 3)]));
        assert_eq!(values.v, BTreeMap::from([(0, 0), (1, 1), (2, 2)]));

        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![0, 1], vec![1, 1]));
        let values = struc.values_map(true);
        assert_eq!(values.h, BTreeMap::from([(1, 0), (2, 0), (4, 1)]));
        let values = struc.values_map(false);
        assert_eq!(values.h, BTreeMap::from([(1, 0), (2, 0), (4, 1)]));
    }

    #[test]
    fn test_equal_point() {
        let assigns = DataHV::new(vec![1.0, 1.0], vec![]);
        let mut struc = StrucProto {
            paths: vec![KeyPath::from([key_pos(1, 1), key_pos(2, 1), key_pos(4, 1)])],
            attrs: Default::default(),
        };
        let paths = struc.get_paths(Default::default(), &assigns);
        assert_eq!(paths[0].len(), 3);

        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![0, 1], vec![]));
        let paths = struc.get_paths(Default::default(), &assigns);
        assert_eq!(paths[0].len(), 2);
    }

    #[test]
    fn test_size() {
        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
            ],
            attrs: Default::default(),
        };
        struc
            .attrs
            .set::<attrs::ReduceAlloc>(&DataHV::new(vec![vec![0, 1]], vec![]));
        let size = struc.size();
        assert_eq!(size.h, 3);
        assert_eq!(size.v, 2);

        assert!(struc.reduce(Axis::Horizontal, false));
        let size = struc.size();
        assert_eq!(size.h, 1);
        assert_eq!(size.v, 2);

        assert!(!struc.reduce(Axis::Horizontal, false));
        let size = struc.size();
        assert_eq!(size.h, 1);
        assert_eq!(size.v, 2);
    }

    #[test]
    fn test_strokes() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(1, 0), key_pos(4, 1)]),
                KeyPath::from([key_pos(1, 0), key_pos(1, 1), key_pos(0, 2)]),
            ],
            attrs: Default::default(),
        };
        let strokes = struc.strokes();
        assert_eq!(
            strokes,
            vec!["2".to_string(), "3".to_string(), "21".to_string()]
        );
    }

    #[test]
    fn test_subarea_wline_eight() {
        let zero = DataHV::splat(0.0);
        // 艹
        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
                KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
                KeyPath::from([key_pos(3, 0), key_pos(3, 2)]),
            ],
            attrs: Default::default(),
        };
        let assign = struc
            .allocation_space()
            .into_map(|v| v.into_iter().map(|v| v as f32).collect());
        let weights = struc.subarea_line_weight(&assign, zero);
        assert_eq!(weights.h, vec![1.0, 2.0, 1.0]);
        assert_eq!(weights.v, vec![2.0, 2.0]);
        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![1, 1, 1], vec![2, 1]));
        let weights = struc.subarea_line_weight(&assign, zero);
        assert_eq!(weights.h, vec![1.0, 2.0, 1.0]);
        assert_eq!(weights.v, vec![2.0, 2.0]);

        // 小
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(1, 1), key_pos(0, 2)]),
                KeyPath::from([key_pos(2, 0), key_pos(2, 3)]),
                KeyPath::from([key_pos(3, 1), key_pos(4, 2)]),
            ],
            attrs: Default::default(),
        };
        let assign = struc
            .allocation_space()
            .into_map(|v| v.into_iter().map(|v| v as f32).collect());
        let weights = struc.subarea_line_weight(&assign, zero);
        assert_eq!(weights.h, vec![1.0, 0.0, 0.0, 1.0]);
        assert_eq!(weights.v[0], 1.0);
        assert_eq!(weights.v[2], 1.0);
        assert!(
            (weights.v[1] - 3.0).abs() < 0.001,
            "{} != {}",
            weights.v[1],
            3.0
        );
        let weights = struc.subarea_line_weight(&assign, DataHV::splat(1.0));
        assert_eq!(weights.h, vec![1.0, 1.0, 1.0, 1.0]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(0, 0), key_pos(3, 3)]),
                KeyPath::from([key_pos(2, 0), key_pos(1, 1)]),
            ],
            attrs: Default::default(),
        };
        let assign = struc
            .allocation_space()
            .into_map(|v| v.into_iter().map(|v| v as f32).collect());
        let weights = struc.subarea_line_weight(&assign, zero);
        assert_eq!(weights.h, vec![1.0, 2.0, 1.0]);
        assert_eq!(weights.v, vec![2.0, 2.0]);
    }

    #[test]
    fn test_subarea_weight() {
        // 田
        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(0, 0), key_pos(2, 0), key_pos(2, 2), key_pos(0, 2)]),
                KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
                KeyPath::from([key_pos(0, 1), key_pos(2, 1)]),
            ],
            attrs: Default::default(),
        };
        struc
            .attrs
            .set::<attrs::AreaWeights>(&attrs::AreaWeights::new(
                DataHV::new(vec![1.0, 1.0], vec![1.0, 1.0]),
                DataHV::new(Some(vec![0.0, 1.0]), Some(vec![1.0, 0.0])),
            ));
        let weights = struc.subarea_weight(DataHV::new(1.0, 0.5));
        assert_eq!(weights.h, vec![0.5, 1.0]);
        assert_eq!(weights.v, vec![1.0, 1.0]);
        let weights = struc.subarea_weight(DataHV::new(0.5, 1.0));
        assert_eq!(weights.h, vec![1.0, 1.0]);
        assert_eq!(weights.v, vec![1.0, 0.5]);
        let weights = struc.subarea_weight(DataHV::splat(1.0));
        assert_eq!(weights.h, vec![1.0, 1.0]);
        assert_eq!(weights.v, vec![1.0, 1.0]);

        let size = DataHV::splat(1.0);
        let mut struc = StrucProto {
            paths: vec![KeyPath::from([key_pos(0, 0), key_pos(0, 2), key_pos(0, 4)])],
            attrs: Default::default(),
        };
        let weights = struc.subarea_weight(size);
        assert_eq!(weights.v, vec![2.0, 2.0]);
        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![], vec![0, 1]));
        let weights = struc.subarea_weight(size);
        assert_eq!(weights.v, vec![1.0]);

        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![], vec![2, 1]));
        let weights = struc.subarea_weight(size);
        assert_eq!(weights.v, vec![2.0, 1.0]);
        struc
            .attrs
            .set::<attrs::AreaWeights>(&attrs::AreaWeights::new(
                DataHV::new(vec![], vec![1.0, 0.5]),
                Default::default(),
            ));
        let weights = struc.subarea_weight(size);
        assert_eq!(weights.v, vec![2.0, 0.5]);
    }
}
