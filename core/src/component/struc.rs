use crate::{
    algorithm,
    axis::*,
    component::attrs::{self, CompAttr},
    construct::space::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Struc<T: Default + Clone + Copy, U> {
    pub key_paths: Vec<KeyPath<T, U>>,
    attrs: BTreeMap<String, String>,
}

impl<T, U> Struc<T, U>
where
    T: Default + Clone + Copy,
{
    pub fn new(list: Vec<Vec<KeyPoint<T, U>>>) -> Self {
        Self {
            attrs: Default::default(),
            key_paths: list.into_iter().map(|path| KeyPath::new(path)).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.key_paths.is_empty()
    }

    pub fn cast<F, T2, U2>(&self, f: F) -> Struc<T2, U2>
    where
        F: Fn(euclid::Point2D<T, U>) -> euclid::Point2D<T2, U2>,
        T2: Default + Clone + Copy,
    {
        Struc {
            key_paths: self
                .key_paths
                .iter()
                .map(|path| {
                    KeyPath::new(
                        path.points
                            .iter()
                            .map(|kp| KeyPoint::new(f(kp.point), kp.p_type))
                            .collect(),
                    )
                })
                .collect(),
            attrs: self.attrs.clone(),
        }
    }
}

pub type StrucProto = Struc<usize, IndexSpace>;

impl StrucProto {
    pub fn get_real_value<T>(val: usize, reals: &Vec<bool>, list: &Vec<T>) -> Option<T>
    where
        T: std::iter::Sum<T> + Clone,
    {
        match reals[val] {
            true => Some(
                list[..reals[..val].iter().filter(|r| **r).count()]
                    .iter()
                    .cloned()
                    .sum(),
            ),
            false => None,
        }
    }

    fn proto_value_list(&self) -> DataHV<BTreeMap<usize, bool>> {
        self.key_paths
            .iter()
            .fold(DataHV::default(), |mut list, path| {
                path.points.iter().for_each(|kp| {
                    Axis::list().into_iter().for_each(|axis| {
                        let exist_in = !kp.p_type.is_unreal(axis);
                        list.hv_get_mut(axis)
                            .entry(*kp.point.hv_get(axis))
                            .and_modify(|v| *v |= exist_in)
                            .or_insert(exist_in);
                    })
                });
                list
            })
    }

    fn proto_allocs_and_values(&self) -> (DataHV<Vec<usize>>, DataHV<BTreeMap<usize, bool>>) {
        let value_list = self.proto_value_list();
        let allocs = self
            .get_attr::<attrs::Allocs>()
            .unwrap_or_default()
            .zip(value_list.as_ref())
            .into_map(|(a, vlist)| {
                if a.len()
                    == vlist
                        .iter()
                        .filter(|(_, r)| **r)
                        .count()
                        .checked_sub(1)
                        .unwrap_or_default()
                {
                    return a;
                } else {
                    let mut iter = vlist.iter().skip_while(|(_, real)| !**real);
                    if let Some((pre, _)) = iter.next() {
                        let mut pre = *pre;
                        let mut offset = 0;
                        return iter
                            .filter_map(|(v, real)| match real {
                                false => {
                                    offset += 1;
                                    None
                                }
                                true => {
                                    let l = *v - pre - offset;
                                    offset = 0;
                                    pre = *v;
                                    Some(l)
                                }
                            })
                            .collect();
                    } else {
                        vec![]
                    }
                }
            });
        (allocs, value_list)
    }

    pub fn allocs_and_maps_and_reals(
        &self,
    ) -> (
        DataHV<Vec<usize>>,
        DataHV<BTreeMap<usize, usize>>,
        DataHV<Vec<bool>>,
    ) {
        #[derive(Clone, Copy)]
        enum State {
            Normal,
            Merge,
            Unreal,
        }

        let (allocs, values) = self.proto_allocs_and_values();
        let map_to = values
            .as_ref()
            .zip(allocs.as_ref())
            .into_map(|(values, allocs)| {
                let mut new_values = BTreeMap::<usize, usize>::default();

                let mut alloc_iter = allocs.iter().chain(std::iter::once(&1));
                let mut state = State::Normal;
                let mut count = 0;
                values.into_iter().for_each(|(&v, &r)| {
                    let to = match state {
                        State::Merge => count - 1,
                        State::Unreal if !r => count,
                        _ => {
                            count += 1;
                            count - 1
                        }
                    };
                    new_values.insert(v, to);

                    state = if r {
                        match alloc_iter.next().unwrap() {
                            0 => State::Merge,
                            _ => State::Normal,
                        }
                    } else {
                        match state {
                            State::Normal => State::Unreal,
                            other => other,
                        }
                    };
                });
                new_values
            });
        let reals = values.zip(map_to.as_ref()).into_map(|(values, map_to)| {
            match map_to.last_key_value().map(|(_, n)| *n) {
                Some(len) => {
                    let mut reals: Vec<bool> = vec![false; len + 1];
                    values
                        .iter()
                        .filter(|(_, r)| **r)
                        .for_each(|(v, _)| reals[map_to[v]] = true);
                    reals
                }
                None => vec![],
            }
        });

        (allocs, map_to, reals)
    }

    pub fn get_allocs(&self) -> DataHV<Vec<usize>> {
        self.proto_allocs_and_values()
            .0
            .into_map(|l| l.into_iter().filter(|n| *n != 0).collect())
    }

    pub fn get_axis_allocs(&self, axis: Axis) -> Vec<usize> {
        let DataHV { h, v } = self.get_allocs();
        match axis {
            Axis::Horizontal => h,
            Axis::Vertical => v,
        }
    }

    #[allow(dead_code)]
    fn proto_visual_center(&self, min_len: f32) -> WorkPoint {
        self.cast_work().visual_center(min_len).0
    }

    pub fn get_attr<'a, T: CompAttr>(&self) -> Option<T::Data>
    where
        <T as CompAttr>::Data: serde::de::DeserializeOwned,
    {
        self.attrs
            .get(T::attr_name())
            .and_then(|attr| T::parse_str(attr))
    }

    pub fn set_attr<'a, T: CompAttr>(&mut self, data: &T::Data)
    where
        <T as CompAttr>::Data: serde::Serialize,
    {
        self.attrs
            .insert(T::attr_name().to_string(), T::attr_str(data).unwrap());
    }

    pub fn set_allocs_in_place(&mut self, in_place: &DataHV<[bool; 2]>) {
        let mut allocs_proto: DataHV<Vec<usize>> = self
            .proto_allocs_and_values()
            .0
            .into_map(|l| l.into_iter().filter(|n| *n != 0).collect());

        if let Some(map) = self.get_attr::<attrs::InPlaceAllocs>() {
            attrs::place_matchs(&map, in_place)
                .into_iter()
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
                })
        }

        self.set_attr::<attrs::Allocs>(&allocs_proto);
    }

    fn size(&self) -> IndexSize {
        let mut box2d = self.key_paths.iter().fold(
            euclid::Box2D::new(
                IndexPoint::new(usize::MAX, usize::MAX),
                IndexPoint::new(usize::MIN, usize::MIN),
            ),
            |box2d, path| {
                path.points.iter().fold(box2d, |box2d, kp| {
                    euclid::Box2D::new(box2d.min.min(kp.point), box2d.max.max(kp.point))
                })
            },
        );
        if box2d.min.x == usize::MAX {
            box2d.min.x = 0;
        }
        if box2d.min.y == usize::MAX {
            box2d.min.y = 0;
        }
        (box2d.max + euclid::Vector2D::new(1, 1) - box2d.min).to_size()
    }

    pub fn alloc_size(&self) -> IndexSize {
        let size = self.proto_allocs_and_values().0;
        IndexSize::new(
            size.h.into_iter().sum::<usize>(),
            size.v.into_iter().sum::<usize>(),
        )
    }

    pub fn reduce(&mut self, axis: Axis) -> bool {
        let mut ok = false;
        let mut allocs = self.proto_allocs_and_values().0;
        if let Some(reduce_list) = self.get_attr::<attrs::ReduceAlloc>() {
            reduce_list.hv_get(axis).iter().find(|rl| {
                for (r, l) in rl.iter().zip(allocs.hv_get_mut(axis).iter_mut()) {
                    if *r < *l {
                        *l -= 1;
                        ok = true;
                    }
                }
                ok
            });
        }

        if ok {
            self.set_attr::<attrs::Allocs>(&allocs);
        }
        ok
    }

    pub fn rotate(&mut self, quater: usize) {
        let mut size = self.size();
        let mut quater = quater % 4;
        while quater != 0 {
            self.key_paths.iter_mut().for_each(|path| {
                path.points.iter_mut().for_each(|kp| {
                    kp.point = IndexPoint::new(kp.point.y, size.width - kp.point.x - 1);
                    match kp.p_type {
                        KeyPointType::Vertical => kp.p_type = KeyPointType::Horizontal,
                        KeyPointType::Horizontal => kp.p_type = KeyPointType::Vertical,
                        _ => {}
                    }
                });
            });
            size = IndexSize::new(size.height, size.width);
            quater -= 1;
        }
    }

    pub fn to_normal(&self, outside: DataHV<f32>) -> StrucWork {
        if self.is_empty() {
            Default::default()
        }

        let (allocs, values, reals) = self.allocs_and_maps_and_reals();
        let size = allocs.map(|list| list.iter().sum::<usize>() as f32);

        let map_to = |pos: IndexPoint| -> WorkPoint {
            Axis::hv_data()
                .into_map(|axis| {
                    let pro_val = pos.hv_get(axis);
                    let reals = reals.hv_get(axis);
                    let allocs = allocs.hv_get(axis);
                    let n = values.hv_get(axis)[&pro_val];

                    match Self::get_real_value(n, reals, allocs) {
                        Some(len) => len as f32 / *size.hv_get(axis),
                        None => {
                            let pre = (0..n)
                                .rev()
                                .find_map(|i| Self::get_real_value(i, reals, allocs))
                                .map(|len| len as f32 / *size.hv_get(axis));
                            let back = (n + 1..reals.len())
                                .find_map(|i| Self::get_real_value(i, reals, allocs))
                                .map(|len| len as f32 / *size.hv_get(axis));

                            match (pre, back) {
                                (Some(pre), Some(back)) => (pre + back) * 0.5,
                                (Some(pre), None) => pre + *outside.hv_get(axis),
                                (None, Some(back)) => back - *outside.hv_get(axis),
                                (None, None) => 0.0,
                            }
                        }
                    }
                })
                .to_array()
                .into()
        };

        self.cast(map_to)
    }

    pub fn to_work_in_assign(
        &self,
        assigns: DataHV<&Vec<f32>>,
        outside: DataHV<f32>,
        move_to: WorkPoint,
    ) -> StrucWork {
        let (_, mao_to, reals) = self.allocs_and_maps_and_reals();
        let get_value = |n: usize, axis: Axis| -> f32 {
            let n_real = reals.hv_get(axis)[..n].iter().filter(|r| **r).count();
            assigns.hv_get(axis)[..n_real].iter().sum::<f32>()
        };

        let key_paths = self
            .key_paths
            .iter()
            .map(|path| {
                let mut iter = path.points.iter();
                let mut pre_pos = DataHV::<Option<usize>>::default();
                let mut points = vec![];

                while let Some(kp) = iter.next() {
                    let mut pos = WorkPoint::default();
                    Axis::list().into_iter().for_each(|axis| {
                        let reals = reals.hv_get(axis);
                        let n = mao_to.hv_get(axis)[kp.point.hv_get(axis)];
                        let v = if reals[n] {
                            *pre_pos.hv_get_mut(axis) = Some(n);
                            get_value(n, axis)
                        } else {
                            let mut pre = reals
                                .iter()
                                .enumerate()
                                .rev()
                                .find(|(i, r)| **r && *i < n)
                                .map(|(i, _)| i);
                            let mut next = reals
                                .iter()
                                .enumerate()
                                .find(|(i, r)| **r && *i > n)
                                .map(|(i, _)| i);

                            if pre.is_none() && next.is_none() {
                                pre = *pre_pos.hv_get(axis);
                                next = iter.clone().find_map(|kp| {
                                    let n = mao_to.hv_get(axis)[kp.point.hv_get(axis)];
                                    match reals[n] {
                                        true => Some(n),
                                        false => None,
                                    }
                                })
                            }

                            if pre.is_some() && next.is_some() {
                                (get_value(pre.unwrap(), axis) + get_value(next.unwrap(), axis))
                                    * 0.5
                            } else {
                                match pre.or(next) {
                                    Some(anchor) => {
                                        let outside = *outside.hv_get(axis);
                                        let anchor_v = get_value(anchor, axis);
                                        match n.cmp(&anchor) {
                                            std::cmp::Ordering::Less => anchor_v - outside,
                                            std::cmp::Ordering::Greater => anchor_v + outside,
                                            std::cmp::Ordering::Equal => anchor_v,
                                        }
                                    }
                                    None => 0.0,
                                }
                            }
                        };
                        *pos.hv_get_mut(axis) = v;
                    });
                    points.push(KeyPoint::new(pos + move_to.to_vector(), kp.p_type))
                }

                KeyPath::new(points)
            })
            .collect();

        StrucWork {
            attrs: self.attrs.clone(),
            key_paths,
        }
    }

    pub fn cast_work(&self) -> StrucWork {
        self.cast(|p| p.cast::<f32>().cast_unit())
    }
}

pub type StrucWork = Struc<f32, WorkSpace>;

impl StrucWork {
    pub fn merge(&mut self, other: Self) {
        self.key_paths.extend(other.key_paths);
    }

    pub fn marker_shrink(&mut self, sub_area: WorkBox) {
        // let orders: DataHV<Vec<f32>> = self
        //     .key_paths
        //     .iter_mut()
        //     .fold(DataHV::splat(vec![]), |mut list, path| {
        //         path.points.iter().for_each(|kp| {
        //             if !kp.p_type.is_unreal(Axis::Horizontal) {
        //                 list.h.push(kp.point.x);
        //             }
        //             if !kp.p_type.is_unreal(Axis::Vertical) {
        //                 list.v.push(kp.point.y);
        //             }
        //         });
        //         list
        //     })
        //     .into_map(|mut list| {
        //         list.sort_by(|a, b| a.partial_cmp(b).unwrap());
        //         list.dedup();
        //         list
        //     });

        self.key_paths.iter_mut().for_each(|path| {
            let mut shrink_pos = [DataHV::splat(None); 2];
            let mut iter = path.points.iter();
            let mut rev_iter = path.points.iter().rev();
            [
                (iter.next(), iter.next()),
                (rev_iter.next(), rev_iter.next()),
            ]
            .into_iter()
            .enumerate()
            .for_each(|(i, kp)| {
                if let (Some(kp), Some(fixed_kp)) = kp {
                    Axis::list().into_iter().for_each(|axis| {
                        if kp.p_type.is_unreal(axis) && !fixed_kp.p_type.is_unreal(axis)
                        // && !orders.hv_get(axis).contains(kp.point.hv_get(axis))
                        {
                            let edge = if sub_area
                                .contains_include(kp.point, algorithm::NORMAL_OFFSET)
                                && (*fixed_kp.point.hv_get(axis) > *sub_area.max.hv_get(axis)
                                    || *fixed_kp.point.hv_get(axis) < *sub_area.min.hv_get(axis))
                            {
                                [*sub_area.min.hv_get(axis), *sub_area.max.hv_get(axis)]
                                    .into_iter()
                                    .min_by(|a, b| {
                                        (*a - *fixed_kp.point.hv_get(axis))
                                            .abs()
                                            .partial_cmp(&(*b - *fixed_kp.point.hv_get(axis)).abs())
                                            .unwrap()
                                    })
                            } else {
                                None
                            };
                            if let Some(edge) = edge {
                                *shrink_pos[i].hv_get_mut(axis) =
                                    Some((edge + *fixed_kp.point.hv_get(axis)) * 0.5);
                            }
                        }
                    })
                }
            });

            [0, path.points.len().checked_sub(1).unwrap_or_default()]
                .into_iter()
                .enumerate()
                .for_each(|(i, kp_index)| {
                    if let Some(kp) = path.points.get_mut(kp_index) {
                        Axis::list().into_iter().for_each(|axis| {
                            if let Some(v) = shrink_pos[i].hv_get(axis) {
                                *kp.point.hv_get_mut(axis) = *v;
                            }
                        })
                    }
                });
        })
    }

    pub fn transform(mut self, scale: WorkVec, moved: WorkVec) -> Self {
        self.key_paths.iter_mut().for_each(|path| {
            path.points.iter_mut().for_each(|p| {
                let p = &mut p.point;
                p.x = p.x * scale.x + moved.x;
                p.y = p.y * scale.y + moved.y;
            })
        });
        self
    }

    pub fn to_proto(mut self, unit: WorkSize) -> StrucProto {
        let mut offset = self.align_cells(unit).min();
        let unreal_len = unit.to_hv_data().map(|v| v * 0.5);

        let mut values: DataHV<Vec<(f32, bool)>> =
            self.key_paths
                .iter()
                .fold(Default::default(), |mut vs, path| {
                    path.points.iter().for_each(|kp| {
                        vs.h.push((kp.point.x, kp.p_type.is_unreal(Axis::Horizontal)));
                        vs.v.push((kp.point.y, kp.p_type.is_unreal(Axis::Vertical)));
                    });
                    vs
                });

        let maps: DataHV<Vec<(f32, usize)>> =
            Axis::list()
                .into_iter()
                .fold(Default::default(), |mut maps, axis| {
                    let values = values.hv_get_mut(axis);
                    values.sort_by(|a, b| match a.0.partial_cmp(&b.0).unwrap() {
                        std::cmp::Ordering::Equal => match a.1 {
                            false => std::cmp::Ordering::Less,
                            true if !b.1 => std::cmp::Ordering::Greater,
                            _ => std::cmp::Ordering::Equal,
                        },
                        ord => ord,
                    });
                    values.dedup_by_key(|v| v.0);

                    *maps.hv_get_mut(axis) = values
                        .iter()
                        .map(|&(v, is_unreaal)| {
                            let correction = if is_unreaal {
                                *offset.hv_get_mut(axis) -= unit.hv_get(axis);
                                *unreal_len.hv_get(axis)
                            } else {
                                0.0
                            };

                            (
                                v,
                                ((v - correction - offset.hv_get(axis)) / unit.hv_get(axis)).round()
                                    as usize,
                            )
                        })
                        .collect();
                    maps
                });

        Struc {
            key_paths: self
                .key_paths
                .into_iter()
                .map(|path| KeyPath {
                    points: path
                        .points
                        .into_iter()
                        .map(|kp| {
                            let mut point = Axis::list()
                                .map(|axis| {
                                    maps.hv_get(axis)
                                        .iter()
                                        .find_map(|&(from, to)| {
                                            match *kp.point.hv_get(axis) == from {
                                                true => Some(to),
                                                false => None,
                                            }
                                        })
                                        .unwrap()
                                })
                                .into_iter();
                            KeyPoint {
                                p_type: kp.p_type,
                                point: IndexPoint::new(
                                    point.next().unwrap(),
                                    point.next().unwrap(),
                                ),
                            }
                        })
                        .collect(),
                })
                .collect(),
            attrs: self.attrs,
        }
    }

    pub fn align_cells(&mut self, unit: WorkSize) -> WorkRect {
        let mut min_pos = WorkPoint::splat(f32::MAX);
        let mut max_pos = WorkPoint::splat(f32::MIN);

        self.key_paths.iter_mut().for_each(|path| {
            path.points.iter_mut().for_each(|kp| {
                Axis::list().into_iter().for_each(|axis| {
                    let v = kp.point.hv_get_mut(axis);
                    let mut unit_size = *unit.hv_get(axis);
                    if kp.p_type.is_unreal(axis) {
                        unit_size *= 0.5;
                        *v = (*v / unit_size).round() * unit_size;
                    } else {
                        *v = (*v / unit_size).round() * unit_size;
                        *min_pos.hv_get_mut(axis) = min_pos.hv_get(axis).min(*v);
                        *max_pos.hv_get_mut(axis) = max_pos.hv_get(axis).max(*v);
                    }
                })
            })
        });

        euclid::Box2D::new(min_pos, max_pos).to_rect()
    }

    pub fn split_intersect(&self, min_lens: f32) -> Vec<Vec<KeyFloatPoint<WorkSpace>>> {
        let min_len_square = min_lens.powi(2);
        let mut paths: Vec<Vec<KeyFloatPoint<WorkSpace>>> = self
            .key_paths
            .iter()
            .map(|path| {
                let mut new_path: Vec<KeyFloatPoint<WorkSpace>> = path
                    .points
                    .iter()
                    .filter(|kp| kp.p_type != KeyPointType::Mark)
                    .cloned()
                    .collect();
                new_path.dedup_by_key(|kp| kp.point);
                new_path
            })
            .collect();

        for i in (1..paths.len()).rev() {
            for j in 0..i {
                let mut p1_i = 1;
                while p1_i < paths[i].len() {
                    let p11 = paths[i][p1_i - 1].point.to_hv_data();
                    let mut p12 = paths[i][p1_i].point.to_hv_data();

                    let mut p2_i = 1;
                    while p2_i < paths[j].len() {
                        let p21 = paths[j][p2_i - 1].point.to_hv_data();
                        let p22 = paths[j][p2_i].point.to_hv_data();

                        match algorithm::intersection(p11, p12, p21, p22) {
                            Some((new_point, t)) => {
                                let p = new_point;

                                // if t.into_iter().all(|t| 0.0 < t && t < 1.0) {
                                //     p12 = p;
                                //     paths[i].insert(
                                //         p1_i,
                                //         KeyPoint::new(p.to_array().into(), KeyPointType::Line),
                                //     );
                                //     paths[j].insert(
                                //         p2_i,
                                //         KeyPoint::new(p.to_array().into(), KeyPointType::Line),
                                //     );
                                //     p2_i += 1;
                                // }

                                if 0.0 < t[0]
                                    && t[0] < 1.0
                                    && (p11.h - p.h).powi(2)
                                        + (p11.v - p.v).powi(2)
                                        + algorithm::NORMAL_OFFSET
                                        >= min_len_square
                                    && (p.h - p12.h).powi(2)
                                        + (p.v - p12.v).powi(2)
                                        + algorithm::NORMAL_OFFSET
                                        >= min_len_square
                                {
                                    p12 = p;
                                    paths[i].insert(
                                        p1_i,
                                        KeyPoint::new(p.to_array().into(), KeyPointType::Line),
                                    );
                                }
                                if 0.0 < t[1]
                                    && t[1] < 1.0
                                    && (p21.h - p.h).powi(2)
                                        + (p21.v - p.v).powi(2)
                                        + algorithm::NORMAL_OFFSET
                                        >= min_len_square
                                    && (p.h - p22.h).powi(2)
                                        + (p.v - p22.v).powi(2)
                                        + algorithm::NORMAL_OFFSET
                                        >= min_len_square
                                {
                                    paths[j].insert(
                                        p2_i,
                                        KeyPoint::new(p.to_array().into(), KeyPointType::Line),
                                    );
                                    p2_i += 1;
                                }
                            }
                            _ => {}
                        }
                        p2_i += 1;
                    }
                    p1_i += 1;
                }
            }
        }

        paths
    }

    pub fn split_paths(
        paths: &Vec<Vec<KeyFloatPoint<WorkSpace>>>,
        area: WorkBox,
    ) -> Vec<Vec<KeyFloatPoint<WorkSpace>>> {
        paths
            .iter()
            .map(|path| {
                let mut next_p = path.iter().skip(1);
                path.iter().fold(vec![], |mut list, kp| {
                    let inside1 = area.contains_include(kp.point, algorithm::NORMAL_OFFSET);
                    if inside1 {
                        list.push(*kp);
                    }
                    if let Some(next_kp) = next_p.next() {
                        let inside2 =
                            area.contains_include(next_kp.point, algorithm::NORMAL_OFFSET);
                        if inside1 ^ inside2 {
                            let area_box = [
                                area.min.to_hv_data(),
                                DataHV::new(area.min.x, area.max.y),
                                area.max.to_hv_data(),
                                DataHV::new(area.max.x, area.min.y),
                            ];

                            'query: for i in 0..4 {
                                for j in i + 1..4 {
                                    if let Some((new_p, t)) = algorithm::intersection(
                                        kp.point.to_hv_data(),
                                        next_kp.point.to_hv_data(),
                                        area_box[i],
                                        area_box[j],
                                    ) {
                                        if t[0] < algorithm::NORMAL_OFFSET && !inside2 {
                                            list.pop();
                                        } else if 1.0 - t[0] > algorithm::NORMAL_OFFSET {
                                            list.push(KeyPoint::new(
                                                new_p.to_array().into(),
                                                KeyPointType::Line,
                                            ));
                                            break 'query;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    list
                })
            })
            .filter(|path| !path.is_empty())
            .collect()
    }

    pub fn visual_center_in_paths(
        paths: &Vec<Vec<KeyFloatPoint<WorkSpace>>>,
    ) -> (WorkPoint, WorkSize) {
        let mut size = DataHV::splat([f32::MAX, f32::MIN]);
        let (pos, count) = paths.iter().fold(
            (DataHV::splat(0.0), DataHV::splat(0)),
            |(mut pos, mut count), path| {
                let visible = path.iter().all(|kp| kp.p_type != KeyPointType::Hide);

                path.iter().zip(path.iter().skip(1)).for_each(|(kp1, kp2)| {
                    Axis::list().into_iter().for_each(|axis| {
                        let (r1, r2) = (!kp1.p_type.is_unreal(axis), !kp2.p_type.is_unreal(axis));
                        if r1 && r2 {
                            let val1 = *kp1.point.hv_get(axis);
                            let val2 = *kp2.point.hv_get(axis);

                            let len = size.hv_get_mut(axis);
                            len[0] = len[0].min(val1).min(val2);
                            len[1] = len[1].max(val1).max(val2);

                            if visible {
                                *count.hv_get_mut(axis) += 1;
                                *pos.hv_get_mut(axis) += (val1 + val2) * 0.5;
                            }
                        } else {
                            if let Some(val) = if r1 {
                                Some(*kp1.point.hv_get(axis))
                            } else if r2 {
                                Some(*kp2.point.hv_get(axis))
                            } else {
                                None
                            } {
                                let len = size.hv_get_mut(axis);
                                len[0] = len[0].min(val);
                                len[1] = len[1].max(val);
                            }
                        }
                    })
                });

                (pos, count)
            },
        );

        let center = pos
            .zip(count)
            .zip(size.as_ref())
            .into_map(|((v, n), len)| {
                let l = len[1] - len[0];
                if n == 0 || l <= 0.0 {
                    0.0
                } else {
                    let center = (v / n as f32 - len[0]) / l;
                    if (center - 0.5).abs() < algorithm::NORMAL_OFFSET {
                        0.5
                    } else {
                        center
                    }
                }
            })
            .to_array()
            .into();

        (center, size.into_map(|[f, b]| b - f).to_array().into())
    }

    pub fn visual_center(&self, min_len: f32) -> (WorkPoint, WorkSize) {
        let paths = self.split_intersect(min_len);
        Self::visual_center_in_paths(&paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_center() {
        let min_len = 1.0;

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(5, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(5, 1), KeyPointType::Line),
            ],
        ]);
        assert_eq!(proto.proto_visual_center(min_len), WorkPoint::new(0.5, 0.5));

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(5, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(5, 1), KeyPointType::Line),
            ],
        ]);
        assert_eq!(
            proto.proto_visual_center(min_len),
            WorkPoint::new(9.0 / 5.0 / 5.0, 4.5 / 5.0 / 2.0)
        );

        // 丁
        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 0), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Horizontal),
            ],
        ]);
        assert_eq!(
            proto.proto_visual_center(min_len),
            WorkPoint::new(7.0 / 4.0 / 3.0, 1.0 / 3.0 / 2.0)
        );

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(5, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(5, 1), KeyPointType::Line),
            ],
        ]);
        assert_eq!(
            proto.proto_visual_center(min_len),
            WorkPoint::new(11.0 / 7.0 / 5.0, 6.5 / 7.0 / 2.0)
        );
    }

    #[test]
    fn test_proto() {
        // x-L- -L
        //   |   |
        //   | V |
        //   L   L
        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Mark),
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 3), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 3), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Vertical),
            ],
        ]);
        let (allocs, map_to, reals) = proto.allocs_and_maps_and_reals();
        assert_eq!(
            map_to,
            DataHV::new(
                BTreeMap::from([(0, 0), (1, 1), (2, 2), (3, 3)]),
                BTreeMap::from([(0, 0), (2, 1), (3, 2)]),
            )
        );
        assert_eq!(allocs, DataHV::new(vec![1], vec![2, 1]));
        assert_eq!(
            reals,
            DataHV::new(vec![false, true, false, true], vec![true, true, true])
        );

        let struc = StrucWork::new(vec![
            vec![
                KeyFloatPoint::new(WorkPoint::new(10.1, 10.2), KeyPointType::Line),
                KeyFloatPoint::new(WorkPoint::new(10.1, 30.0), KeyPointType::Line),
            ],
            vec![
                KeyFloatPoint::new(WorkPoint::new(30.1, 0.2), KeyPointType::Line),
                KeyFloatPoint::new(WorkPoint::new(30.2, 40.0), KeyPointType::Line),
            ],
        ])
        .to_proto(WorkSize::new(10., 10.));
        assert_eq!(
            struc.key_paths[0].points,
            vec![
                KeyPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyPoint::new(IndexPoint::new(0, 3), KeyPointType::Line)
            ]
        );
        assert_eq!(
            struc.key_paths[1].points,
            vec![
                KeyPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyPoint::new(IndexPoint::new(2, 4), KeyPointType::Line)
            ]
        );

        let struc_work = struc.to_normal(Default::default());
        let unit = 1.0 / 4.0;
        assert_eq!(
            struc_work.key_paths[0].points,
            vec![
                KeyPoint::new(WorkPoint::new(0.0, 1.0 * unit), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(0.0, 3.0 * unit), KeyPointType::Line)
            ]
        );
        assert_eq!(
            struc_work.key_paths[1].points,
            vec![
                KeyPoint::new(WorkPoint::new(1.0, 0.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(1.0, 1.0), KeyPointType::Line)
            ]
        );
    }

    #[test]
    fn test_to_work() {
        // ⺄
        let struc = StrucProto::new(vec![vec![
            KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
            KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
            KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            KeyIndexPoint::new(IndexPoint::new(2, 3), KeyPointType::Line),
            KeyIndexPoint::new(IndexPoint::new(3, 2), KeyPointType::Mark),
        ]])
        .to_normal(DataHV::splat(0.1));
        assert_eq!(
            struc.key_paths[0].points,
            vec![
                KeyPoint::new(WorkPoint::new(0.0, 0.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(0.5, 0.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(0.5, 0.5), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(1.0, 1.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(1.1, 0.75), KeyPointType::Mark),
            ]
        );
    }

    #[test]
    fn test_split_paths() {
        let paths = vec![
            vec![
                KeyPoint::new(WorkPoint::new(0.0, 0.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(2.0, 2.0), KeyPointType::Line),
            ],
            vec![
                KeyPoint::new(WorkPoint::new(2.0, 1.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(3.0, 1.0), KeyPointType::Line),
            ],
            vec![
                KeyPoint::new(WorkPoint::new(2.0, 2.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(4.0, 2.0), KeyPointType::Line),
            ],
        ];
        assert_eq!(
            StrucWork::split_paths(
                &paths,
                WorkBox::new(WorkPoint::new(1.0, 0.0), WorkPoint::new(3.0, 2.0))
            ),
            vec![
                vec![
                    KeyPoint::new(WorkPoint::new(1.0, 1.0), KeyPointType::Line),
                    KeyPoint::new(WorkPoint::new(2.0, 2.0), KeyPointType::Line),
                ],
                vec![
                    KeyPoint::new(WorkPoint::new(2.0, 1.0), KeyPointType::Line),
                    KeyPoint::new(WorkPoint::new(3.0, 1.0), KeyPointType::Line),
                ],
                vec![
                    KeyPoint::new(WorkPoint::new(2.0, 2.0), KeyPointType::Line),
                    KeyPoint::new(WorkPoint::new(3.0, 2.0), KeyPointType::Line),
                ],
            ]
        );
        assert_eq!(
            StrucWork::split_paths(
                &paths,
                WorkBox::new(WorkPoint::new(0.0, 0.0), WorkPoint::new(2.0, 2.0))
            ),
            vec![vec![
                KeyPoint::new(WorkPoint::new(0.0, 0.0), KeyPointType::Line),
                KeyPoint::new(WorkPoint::new(2.0, 2.0), KeyPointType::Line),
            ]]
        );
    }
}
