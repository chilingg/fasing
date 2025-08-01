use crate::{
    algorithm,
    axis::*,
    component::attrs::{self, CompAttrs},
    config::place_match,
    construct::space::*,
};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct StrucProto {
    pub paths: Vec<KeyPath>,
    pub attrs: CompAttrs,
}

impl<T: IntoIterator<Item = KeyPath>> From<T> for StrucProto {
    fn from(paths: T) -> Self {
        StrucProto {
            paths: paths.into_iter().collect(),
            attrs: Default::default(),
        }
    }
}

impl StrucProto {
    fn values(&self) -> DataHV<Vec<usize>> {
        self.paths
            .iter()
            .fold(DataHV::<BTreeSet<usize>>::default(), |mut set, path| {
                path.points.iter().for_each(|p| {
                    for axis in Axis::list() {
                        set.hv_get_mut(axis).insert(*p.hv_get(axis));
                    }
                });
                set
            })
            .into_map(|set| set.into_iter().collect())
    }

    pub fn values_map(&self) -> DataHV<BTreeMap<usize, usize>> {
        let values = self.values();
        match self.attrs.get::<attrs::Allocs>() {
            Some(allocs) => values.zip(allocs).into_map(|(values, allocs)| {
                let mut sum = 0;
                values
                    .into_iter()
                    .zip(std::iter::once(0).chain(allocs))
                    .map(|(v, a)| {
                        sum += a;
                        (v, sum)
                    })
                    .collect()
            }),
            None => values.into_map(|values| values.iter().map(|v| (*v, *v - values[0])).collect()),
        }
    }

    pub fn value_index_in_axis(&self, vals: &[usize], axis: Axis) -> Vec<usize> {
        let mut values: Vec<usize> = self.values_map().hv_get(axis).values().copied().collect();
        values.dedup();
        vals.iter()
            .map(|v| values.iter().position(|x| x == v).unwrap())
            .collect()
    }

    pub fn allocation_values(&self) -> DataHV<Vec<usize>> {
        self.attrs.get::<attrs::Allocs>().unwrap_or_else(|| {
            self.values().into_map(|values| {
                values
                    .iter()
                    .zip(values.iter().skip(1))
                    .map(|(&n1, &n2)| n2 - n1)
                    .collect()
            })
        })
    }

    pub fn allocation_space(&self) -> DataHV<Vec<usize>> {
        self.allocation_values()
            .into_map(|l| l.into_iter().filter(|n| *n != 0).collect())
    }

    pub fn allocation_size(&self) -> DataHV<usize> {
        self.allocation_values().into_map(|l| l.into_iter().sum())
    }

    pub fn set_allocs_in_adjacency(&mut self, adjacencies: DataHV<[bool; 2]>) {
        let mut allocs_proto = self.allocation_values();
        let mut b = DataHV::splat(false);
        if let Some(ipa) = self.attrs.get::<attrs::InPlaceAllocs>() {
            ipa.into_iter()
                .filter_map(|(rule, allocs)| match place_match(&rule, adjacencies) {
                    true => Some(allocs),
                    false => None,
                })
                .for_each(|allocs| {
                    Axis::list().into_iter().for_each(|axis| {
                        allocs_proto
                            .hv_get_mut(axis)
                            .iter_mut()
                            .zip(allocs.hv_get(axis))
                            .for_each(|(val, exp)| {
                                if *val > *exp {
                                    *val = *exp;
                                    *b.hv_get_mut(axis) = true;
                                }
                            })
                    });
                });
        }

        let mut r_target = self.attrs.get::<attrs::ReduceTarget>().unwrap_or_default();
        Axis::list().into_iter().for_each(|axis| {
            if *b.hv_get(axis) {
                *r_target.hv_get_mut(axis) = Some(allocs_proto.hv_get(axis).iter().sum());
            }
        });

        self.attrs.set::<attrs::ReduceTarget>(&r_target);
        self.attrs.set::<attrs::Adjacencies>(&adjacencies);
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
            let mut r_target = self.attrs.get::<attrs::ReduceTarget>().unwrap_or_default();
            *r_target.hv_get_mut(axis) = Some(allocs.hv_get(axis).iter().sum());
            self.attrs.set::<attrs::ReduceTarget>(&r_target);

            self.attrs.set::<attrs::Allocs>(&allocs);
        }
        ok
    }

    pub fn to_path_in_range(
        &self,
        start: WorkPoint,
        assigns: DataHV<Vec<f32>>,
        range: DataHV<Option<std::ops::RangeInclusive<f32>>>,
    ) -> Vec<KeyWorkPath> {
        let range = Axis::hv().into_map(|axis| {
            let r = range.hv_get(axis).clone().unwrap_or_else(|| {
                let o = *start.hv_get(axis);
                o..=(o + assigns.hv_get(axis).iter().sum::<f32>())
            });
            (*r.start(), *r.end())
        });
        let mut paths = self.to_paths(start, assigns);

        paths.push(KeyWorkPath {
            points: vec![
                WorkPoint::new(range.h.0, range.v.0),
                WorkPoint::new(range.h.1, range.v.0),
                WorkPoint::new(range.h.1, range.v.1),
                WorkPoint::new(range.h.0, range.v.1),
                WorkPoint::new(range.h.0, range.v.0),
            ],
            hide: true,
        });

        algorithm::split_intersect(&mut paths, 0.);
        paths.pop();

        paths
            .into_iter()
            .map(|path| {
                let mut new_paths = vec![];
                let mut outside = true;
                path.points.into_iter().for_each(|pos| {
                    if pos.x >= range.h.0
                        && pos.x <= range.h.1
                        && pos.y >= range.v.0
                        && pos.y <= range.v.1
                    {
                        if outside {
                            new_paths.push(vec![]);
                            outside = false
                        }
                        new_paths.last_mut().unwrap().push(pos);
                    } else {
                        outside = true;
                    }
                });
                new_paths
                    .into_iter()
                    .map(|new_path| KeyWorkPath {
                        points: new_path,
                        hide: path.hide,
                    })
                    .collect::<Vec<KeyWorkPath>>()
            })
            .flatten()
            .filter(|path| path.points.len() > 1)
            .collect()
    }

    pub fn to_path_in_index(
        &self,
        start: WorkPoint,
        assigns: DataHV<Vec<f32>>,
        range: DataHV<Option<std::ops::RangeInclusive<usize>>>,
    ) -> Vec<KeyWorkPath> {
        let alloc_to_assign: DataHV<BTreeMap<usize, f32>> = assigns
            .clone()
            .zip(self.allocation_space())
            .into_map(|(assigns, allocs)| {
                let mut origin = (0, 0.0);
                std::iter::once(origin)
                    .chain(allocs.into_iter().zip(assigns).map(|(alloc, assig)| {
                        origin.0 += alloc;
                        origin.1 += assig;
                        origin
                    }))
                    .collect()
            });

        let range = range
            .zip(alloc_to_assign)
            .into_map(|(range, map)| range.map(|range| map[range.start()]..=map[range.end()]));

        self.to_path_in_range(start, assigns, range)
    }

    pub fn to_paths(&self, start: WorkPoint, assigns: DataHV<Vec<f32>>) -> Vec<KeyWorkPath> {
        let pos_to_alloc = self.values_map();
        let alloc_to_assign: DataHV<BTreeMap<usize, f32>> = start
            .to_hv_data()
            .zip(assigns)
            .zip(self.allocation_space())
            .into_map(|((start, assigns), allocs)| {
                let mut origin = (0, start);
                std::iter::once(origin)
                    .chain(allocs.into_iter().zip(assigns).map(|(alloc, assig)| {
                        origin.0 += alloc;
                        origin.1 += assig;
                        origin
                    }))
                    .collect()
            });
        self.paths
            .iter()
            .map(|path| {
                let points = path
                    .points
                    .iter()
                    .map(|p| {
                        let pos = p
                            .to_hv_data()
                            .zip(pos_to_alloc.as_ref())
                            .zip(alloc_to_assign.as_ref())
                            .into_map(|((v, m1), m2)| m2[&m1[&v]]);
                        WorkPoint::new(pos.h, pos.v)
                    })
                    .collect();

                KeyWorkPath {
                    points,
                    hide: path.hide,
                }
            })
            .collect()
    }

    pub fn visual_weight(&self) -> f32 {
        fn func_val(p1: WorkPoint, p2: WorkPoint, x: f32) -> f32 {
            return (x - p2.x) * (p1.y - p2.y) / (p1.x - p2.x) + p2.y;
        }

        let allocs = self.allocation_space();
        let mut size = allocs.map(|list| list.iter().sum::<usize>());
        size.h += 1;
        size.v += 1;

        let mut set: std::collections::HashSet<IndexPoint> = Default::default();
        self.paths
            .iter()
            .filter(|path| !path.hide)
            .for_each(|path| {
                path.points.windows(2).for_each(|line| {
                    let min = line[0].min(line[1]);
                    let max = line[0].max(line[1]);
                    if min.x == max.x {
                        (min.y..=max.y).for_each(|y| {
                            set.insert(IndexPoint::new(min.x, y));
                        });
                    } else if min.y == max.y {
                        (min.x..=max.x).for_each(|x| {
                            set.insert(IndexPoint::new(x, min.y));
                        });
                    } else {
                        let p1: WorkPoint = line[0].cast().cast_unit();
                        let p2: WorkPoint = line[1].cast().cast_unit();
                        (min.x..=max.x).for_each(|x| {
                            set.insert(IndexPoint::new(
                                x,
                                func_val(p1, p2, x as f32).round() as usize,
                            ));
                        });
                        let p1 = p1.yx();
                        let p2 = p2.yx();
                        (min.y..=max.y).for_each(|y| {
                            set.insert(IndexPoint::new(
                                func_val(p1, p2, y as f32).round() as usize,
                                y,
                            ));
                        });
                    }
                });
            });

        set.len() as f32 / (size.h * size.v) as f32
    }

    pub fn line_length(&self, scale: DataHV<f32>) -> f32 {
        let to_alloc = self.values_map();

        let mut dot = 0.0;
        let paths: Vec<Vec<WorkPoint>> = self
            .paths
            .iter()
            .filter_map(|path| match path.hide || path.points.is_empty() {
                true => None,
                false => Some(
                    path.points
                        .iter()
                        .map(|p| WorkPoint::new(to_alloc.h[&p.x] as f32, to_alloc.v[&p.y] as f32))
                        .collect::<Vec<WorkPoint>>(),
                ),
            })
            .filter(|path| {
                if path.iter().all(|p| path[0].eq(p)) {
                    dot += 1.0;
                    false
                } else {
                    true
                }
            })
            .collect();

        let len = paths.iter().fold(0.0, |mut len, path| {
            path.windows(2).for_each(|line| {
                let mut v: WorkVec = line[1] - line[0];
                v.x *= scale.h;
                v.y *= scale.v;
                len += v.length();
            });
            len
        }) + dot;

        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_weight() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                KeyPath::from([IndexPoint::new(0, 1), IndexPoint::new(2, 1)]),
            ],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.line_length(DataHV::new(1.0, 2.0)), 6.0);
        assert_eq!(struc.line_length(DataHV::new(1.0, 1.0)), 4.0);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(0, 0), IndexPoint::new(0, 0)]),
                KeyPath::from([IndexPoint::new(0, 2), IndexPoint::new(0, 4)]),
            ],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.line_length(DataHV::new(1.0, 1.0)), 3.0);
        assert_eq!(struc.line_length(DataHV::new(2.0, 1.0)), 3.0);
        assert_eq!(struc.line_length(DataHV::new(1.0, 2.0)), 5.0);
    }

    #[test]
    fn test_visual_weight() {
        let struc = [
            StrucProto {
                paths: vec![
                    KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                    KeyPath::from([IndexPoint::new(0, 1), IndexPoint::new(2, 1)]),
                ],
                attrs: CompAttrs::default(),
            },
            StrucProto {
                paths: vec![KeyPath::from([
                    IndexPoint::new(0, 0),
                    IndexPoint::new(2, 0),
                    IndexPoint::new(2, 2),
                    IndexPoint::new(0, 2),
                    IndexPoint::new(0, 0),
                ])],
                attrs: CompAttrs::default(),
            },
        ];
        assert!(struc[0].visual_weight() < struc[1].visual_weight());

        let struc = [
            StrucProto {
                paths: vec![
                    KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                    KeyPath::from([IndexPoint::new(0, 1), IndexPoint::new(2, 1)]),
                ],
                attrs: CompAttrs::default(),
            },
            StrucProto {
                paths: vec![
                    KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                    KeyPath::from([IndexPoint::new(0, 0), IndexPoint::new(2, 0)]),
                ],
                attrs: CompAttrs::default(),
            },
        ];
        assert_eq!(struc[0].visual_weight(), struc[0].visual_weight());

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 0),
                IndexPoint::new(2, 0),
                IndexPoint::new(2, 2),
                IndexPoint::new(0, 2),
                IndexPoint::new(0, 0),
            ])],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.visual_weight(), 8.0 / 9.0);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 0),
                IndexPoint::new(2, 2),
            ])],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.visual_weight(), 1.0 / 3.0);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 1)]),
                KeyPath {
                    points: vec![IndexPoint::new(2, 0), IndexPoint::new(2, 1)],
                    hide: true,
                },
            ],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.visual_weight(), 0.5);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(1, 0),
                IndexPoint::new(1, 1),
            ])],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.visual_weight(), 1.0);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(1, 1),
                IndexPoint::new(1, 1),
            ])],
            attrs: CompAttrs::default(),
        };
        assert_eq!(struc.visual_weight(), 1.0);
    }

    #[test]
    fn test_allocs() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    IndexPoint::new(0, 0),
                    IndexPoint::new(2, 0),
                    IndexPoint::new(2, 2),
                ]),
                KeyPath::from([IndexPoint::new(1, 1), IndexPoint::new(1, 1)]),
            ],
            attrs: CompAttrs::default(),
        };
        let values = struc.values();
        assert_eq!(values.h, vec![0, 1, 2]);
        assert_eq!(values.v, vec![0, 1, 2]);

        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    IndexPoint::new(1, 1),
                    IndexPoint::new(2, 1),
                    IndexPoint::new(2, 2),
                ]),
                KeyPath::from([IndexPoint::new(4, 1), IndexPoint::new(2, 1)]),
            ],
            attrs: CompAttrs::default(),
        };

        assert_eq!(struc.allocation_values(), DataHV::new(vec![1, 2], vec![1]));
        assert_eq!(
            struc.values_map().h,
            BTreeMap::from([(1, 0), (2, 1), (4, 3)])
        );

        struc
            .attrs
            .set::<attrs::Allocs>(&DataHV::new(vec![0, 1], vec![2]));
        assert_eq!(struc.allocation_values(), DataHV::new(vec![0, 1], vec![2]));
        assert_eq!(struc.allocation_space(), DataHV::new(vec![1], vec![2]));
        assert_eq!(
            struc.values_map().h,
            BTreeMap::from([(1, 0), (2, 0), (4, 1)])
        );
    }

    #[test]
    fn test_to_path_in() {
        let assigns = DataHV::splat(vec![1.0, 1.0]);
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(2, 0)]),
                KeyPath::from([IndexPoint::new(1, 1), IndexPoint::new(3, 1)]),
                KeyPath::from([IndexPoint::new(1, 2), IndexPoint::new(3, 2)]),
            ],
            attrs: CompAttrs::default(),
        };

        let paths = struc.to_path_in_index(
            WorkPoint::zero(),
            assigns.clone(),
            DataHV::new(Some(0..=1), None),
        );
        assert_eq!(paths.len(), 3);
        assert_eq!(
            paths[0].points,
            vec![WorkPoint::new(0., 0.), WorkPoint::new(1., 0.)]
        );
        assert_eq!(
            paths[1].points,
            vec![WorkPoint::new(0., 1.), WorkPoint::new(1., 1.)]
        );
        assert_eq!(
            paths[2].points,
            vec![WorkPoint::new(0., 2.), WorkPoint::new(1., 2.)]
        );

        let paths = struc.to_path_in_index(
            WorkPoint::zero(),
            assigns.clone(),
            DataHV::new(Some(1..=2), None),
        );
        assert_eq!(paths.len(), 2);
        assert_eq!(
            paths[0].points,
            vec![WorkPoint::new(1., 1.), WorkPoint::new(2., 1.)]
        );
        assert_eq!(
            paths[1].points,
            vec![WorkPoint::new(1., 2.), WorkPoint::new(2., 2.)]
        );
    }
}
