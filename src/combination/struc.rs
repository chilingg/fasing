use super::attrs::{self, CompAttrs};
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

    pub fn get_paths(&self, start: WorkPoint, assigns: DataHV<Vec<f32>>) -> Vec<WorkKeyPath> {
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
                let kpoints = path
                    .kpoints
                    .iter()
                    .map(|p| {
                        let pos = p
                            .pos
                            .to_hv_data()
                            .zip(pos_to_alloc.as_ref())
                            .zip(alloc_to_assign.as_ref())
                            .into_map(|((v, m1), m2)| m2[&m1[&v]]);
                        KeyPoint::new(WorkPoint::new(pos.h, pos.v))
                    })
                    .collect();

                WorkKeyPath {
                    kpoints,
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
    fn test_values() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
            ],
            attrs: Default::default(),
        };
        let values = struc.values();
        assert_eq!(values.h, vec![1, 2, 4]);
        assert_eq!(values.v, vec![0, 1, 2]);
    }
}
