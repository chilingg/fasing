use crate::{
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

    pub fn set_allocs_in_adjacency(&mut self, adjacencies: DataHV<[bool; 2]>) {
        let mut allocs_proto = self.allocation_values();
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
                                    *val = *exp
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
