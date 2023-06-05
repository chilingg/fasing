use euclid::Point2D;
use serde::{Deserialize, Serialize};

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{fas_file::TransformValue, hv::*};
pub mod space;
use space::*;
pub mod attribute;
use attribute::*;
pub mod view;
use view::StrucAttrView;
pub mod variety;
pub use variety::StrucVarietys;
pub use variety::VarietysComb;

pub struct Error {
    pub msg: String,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Struc<T: Default + Clone + Copy, U> {
    pub key_paths: Vec<KeyPath<T, U>>,
    pub tags: BTreeSet<String>,
}

impl<T, U> Struc<T, U>
where
    T: Default + Clone + Copy,
{
    pub fn meger(&mut self, mut other: Self) {
        self.key_paths.append(&mut other.key_paths);
        self.tags.append(&mut other.tags);
    }
}

pub type StrucProto = Struc<usize, IndexSpace>;
pub type StrucWork = Struc<f32, WorkSpace>;

impl StrucWork {
    pub fn from_prototype(proto: &StrucProto) -> Self {
        Self {
            key_paths: proto.key_paths.iter().map(|path| path.cast()).collect(),
            tags: proto.tags.clone(),
        }
    }

    pub fn add_lines<I: IntoIterator<Item = WorkPoint>>(&mut self, lines: I, closed: bool) {
        self.key_paths.push(KeyFloatPath::from_lines(lines, closed));
    }

    pub fn to_prototype(&self) -> StrucProto {
        StrucProto::from_work(self)
    }

    pub fn to_prototype_offset(&self, offset: f32) -> StrucProto {
        StrucProto::from_work_offset(self, offset)
    }

    pub fn transform(&mut self, scale: WorkVec, moved: WorkVec) {
        self.key_paths.iter_mut().for_each(|path| {
            path.points.iter_mut().for_each(|p| {
                let p = &mut p.point;
                p.x = p.x * scale.x + moved.x;
                p.y = p.y * scale.y + moved.y;
            })
        })
    }
}

impl StrucProto {
    const OFFSET: f32 = 0.01;

    pub fn from_work(struc: &StrucWork) -> Self {
        Self::from_work_offset(struc, Self::OFFSET)
    }

    pub fn from_work_offset(struc: &StrucWork, offset: f32) -> Self {
        let mut x_sort = vec![];
        let mut y_sort = vec![];

        struc.key_paths.iter().for_each(|path| {
            path.points.iter().for_each(|p| {
                x_sort.push(p.point.x);
                y_sort.push(p.point.y);
            })
        });

        x_sort.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_sort.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let x_map = x_sort.iter().fold(vec![], |mut map: Vec<Vec<f32>>, &n| {
            if !map.is_empty() && (n - map.last().unwrap().last().unwrap()).abs() < offset {
                map.last_mut().unwrap().push(n);
            } else {
                map.push(vec![n]);
            }
            map
        });
        let y_map = y_sort.iter().fold(vec![], |mut map: Vec<Vec<f32>>, &n| {
            if !map.is_empty() && (n - map.last().unwrap().last().unwrap()).abs() < offset {
                map.last_mut().unwrap().push(n);
            } else {
                map.push(vec![n]);
            }
            map
        });

        let key_paths: Vec<KeyIndexPath> =
            struc
                .key_paths
                .iter()
                .fold(vec![], |mut key_paths, f_path| {
                    let path = f_path.points.iter().fold(vec![], |mut path, p| {
                        let pos = p.point;
                        let x = x_map
                            .iter()
                            .enumerate()
                            .find_map(|(i, map)| map.iter().find(|&&n| n == pos.x).and(Some(i)))
                            .unwrap();
                        let y = y_map
                            .iter()
                            .enumerate()
                            .find_map(|(i, map)| map.iter().find(|&&n| n == pos.y).and(Some(i)))
                            .unwrap();
                        path.push(KeyPoint::new(IndexPoint::new(x, y), p.p_type));
                        path
                    });
                    key_paths.push(KeyIndexPath::new(path, f_path.closed));
                    key_paths
                });

        StrucProto {
            key_paths,
            tags: struc.tags.clone(),
        }
    }

    pub fn to_work(&self) -> StrucWork {
        StrucWork::from_prototype(self)
    }

    pub fn to_work_in_transform(&self, trans: &DataHV<TransformValue>) -> StrucWork {
        fn process(mut unreliable_list: Vec<usize>, trans: &TransformValue) -> Vec<(f32, bool)> {
            let TransformValue {
                allocs,
                length: _,
                min_step,
                step,
            } = trans;
            let min_step = match allocs.iter().all(|&n| n == 0 || n == 1) {
                true => step,
                false => min_step,
            };

            let mut map = Vec::with_capacity(allocs.len() + unreliable_list.len() + 1);
            let mut offset = 1;
            match unreliable_list.get(0) {
                Some(0) => {
                    map.extend_from_slice(&[(-trans.min_step, false), (0.0, true)]);
                    unreliable_list.remove(0);
                    offset += 1;
                }
                _ => map.push((0.0, true)),
            }

            let mut allocs: Vec<_> = allocs.into_iter().map(|n| Some(*n)).collect();
            unreliable_list
                .into_iter()
                .for_each(|n| allocs.insert(n - offset, None));

            let mut iter = allocs.iter();
            let mut advance = 0.0;
            let mut pre_val = 0.0;
            while let Some(ref cur_val) = iter.next() {
                if let Some(cur_val) = cur_val {
                    if *cur_val == 1 {
                        advance += min_step;
                    } else {
                        advance += *cur_val as f32 * step;
                    }
                    pre_val = advance;
                    map.push((advance, true));
                } else {
                    match iter.clone().find_map(|v| *v) {
                        Some(las_val) => {
                            let las_val = if las_val == 1 {
                                advance + las_val as f32 * min_step
                            } else {
                                advance + las_val as f32 * step
                            };
                            map.push(((pre_val + las_val) * 0.5, false));
                        }
                        None => {
                            map.push((advance + trans.min_step, false));
                        }
                    };
                }
            }

            map
        }

        let unreliable_list = self.unreliable_in();
        let (h_map, v_map) = (
            process(unreliable_list.h, &trans.h),
            process(unreliable_list.v, &trans.v),
        );

        StrucWork {
            tags: self.tags.clone(),
            key_paths: self
                .key_paths
                .iter()
                .map(|path| {
                    let mut iter = path.points.iter();
                    let mut pre_x: Option<&KeyPoint<usize, IndexSpace>> = None;
                    let mut pre_y: Option<&KeyPoint<usize, IndexSpace>> = None;
                    let mut points = vec![];

                    while let Some(pos) = iter.next() {
                        let newp = WorkPoint::new(
                            match h_map[pos.point.x] {
                                (x, true) => x,
                                (x, false) => {
                                    if let Some(pre_p) =
                                        pre_x.or(iter.clone().find(|kp| h_map[kp.point.x].1))
                                    {
                                        if pre_p.point.x > pos.point.x {
                                            h_map[pre_p.point.x].0 - trans.h.min_step
                                        } else {
                                            h_map[pre_p.point.x].0 + trans.h.min_step
                                        }
                                    } else {
                                        x
                                    }
                                }
                            },
                            match v_map[pos.point.y] {
                                (y, true) => y,
                                (y, false) => {
                                    if let Some(pre_p) =
                                        pre_y.or(iter.clone().find(|kp| v_map[kp.point.y].1))
                                    {
                                        if pre_p.point.y > pos.point.y {
                                            v_map[pre_p.point.y].0 - trans.v.min_step
                                        } else {
                                            v_map[pre_p.point.y].0 + trans.v.min_step
                                        }
                                    } else {
                                        y
                                    }
                                }
                            },
                        );

                        if h_map[pos.point.x].1 {
                            pre_x = Some(pos);
                        }
                        if v_map[pos.point.y].1 {
                            pre_y = Some(pos);
                        }
                        points.push(KeyFloatPoint::new(newp, pos.p_type));
                    }

                    KeyPath {
                        closed: path.closed,
                        points,
                    }
                })
                .collect(),
        }
    }

    pub fn to_work_in_alloc(
        &self,
        alloc: DataHV<Vec<usize>>,
        min: f32,
        max: f32,
    ) -> Result<StrucWork, Error> {
        fn process(
            mut unreliable_list: Vec<usize>,
            allocs: Vec<usize>,
            min: f32,
            max: f32,
            advence_step: &mut f32,
        ) -> Vec<(f32, bool)> {
            let length = allocs.iter().sum::<usize>() as f32;
            let (min_step, step) = if length == 0.0 {
                (0.0, 0.0)
            } else if allocs.iter().all(|&n| n == 0 || n == 1) {
                (1.0 / length, 0.0)
            } else {
                let mut one_num = 0.0;
                let other_size = allocs
                    .iter()
                    .filter(|&&n| {
                        if n == 1 {
                            one_num += n as f32;
                            false
                        } else {
                            true
                        }
                    })
                    .sum::<usize>() as f32;

                if length * max <= 1.0 {
                    (max, (1.0 - one_num * max) / other_size)
                } else if length * min >= 1.0 {
                    (min, (1.0 - one_num * min) / other_size)
                } else {
                    let val = 1.0 / length;
                    (val, val)
                }
            };

            *advence_step = min_step;
            let mut map = Vec::with_capacity(allocs.len() + unreliable_list.len() + 1);
            let mut offset = 1;
            match unreliable_list.get(0) {
                Some(0) => {
                    map.extend_from_slice(&[(-min_step, false), (0.0, true)]);
                    unreliable_list.remove(0);
                    offset += 1;
                }
                _ => map.push((0.0, true)),
            }

            let mut allocs: Vec<_> = allocs.into_iter().map(|n| Some(n)).collect();
            unreliable_list
                .into_iter()
                .for_each(|n| allocs.insert(n - offset, None));

            let mut iter = allocs.iter();
            let mut advance = 0.0;
            let mut pre_val = 0.0;
            while let Some(ref cur_val) = iter.next() {
                if let Some(cur_val) = cur_val {
                    if *cur_val == 1 {
                        advance += min_step;
                    } else {
                        advance += *cur_val as f32 * step;
                    }
                    pre_val = advance;
                    map.push((advance, true));
                } else {
                    match iter.clone().find_map(|v| *v) {
                        Some(las_val) => {
                            let las_val = if las_val == 1 {
                                advance + las_val as f32 * min_step
                            } else {
                                advance + las_val as f32 * step
                            };
                            map.push(((pre_val + las_val) * 0.5, false));
                        }
                        None => {
                            map.push((advance + min_step, false));
                        }
                    };
                }
            }

            map
        }

        if alloc.h.iter().filter(|v| **v != 0).count() as f32 * min > 1.0
            || alloc.v.iter().filter(|v| **v != 0).count() as f32 * min > 1.0
        {
            let max_size = (1.0 / min).floor();
            return Err(Error {
                msg: format!("Maximum size is {} * {} in {}", max_size, max_size, min),
            });
        }

        let unreliable_list = self.unreliable_in();
        let (mut step_x, mut step_y) = (0.0, 0.0);
        let (h_map, v_map) = (
            process(unreliable_list.h, alloc.h, min, max, &mut step_x),
            process(unreliable_list.v, alloc.v, min, max, &mut step_y),
        );

        Ok(StrucWork {
            tags: self.tags.clone(),
            key_paths: self
                .key_paths
                .iter()
                .map(|path| {
                    let mut iter = path.points.iter();
                    let mut pre_x: Option<&KeyPoint<usize, IndexSpace>> = None;
                    let mut pre_y: Option<&KeyPoint<usize, IndexSpace>> = None;
                    let mut points = vec![];

                    while let Some(pos) = iter.next() {
                        let newp = WorkPoint::new(
                            match h_map[pos.point.x] {
                                (x, true) => x,
                                (x, false) => {
                                    if let Some(pre_p) =
                                        pre_x.or(iter.clone().find(|kp| h_map[kp.point.x].1))
                                    {
                                        if pre_p.point.x > pos.point.x {
                                            h_map[pre_p.point.x].0 - step_x
                                        } else {
                                            h_map[pre_p.point.x].0 + step_x
                                        }
                                    } else {
                                        x
                                    }
                                }
                            },
                            match v_map[pos.point.y] {
                                (y, true) => y,
                                (y, false) => {
                                    if let Some(pre_p) =
                                        pre_y.or(iter.clone().find(|kp| v_map[kp.point.y].1))
                                    {
                                        if pre_p.point.y > pos.point.y {
                                            v_map[pre_p.point.y].0 - step_y
                                        } else {
                                            v_map[pre_p.point.y].0 + step_y
                                        }
                                    } else {
                                        y
                                    }
                                }
                            },
                        );

                        if h_map[pos.point.x].1 {
                            pre_x = Some(pos);
                        }
                        if v_map[pos.point.y].1 {
                            pre_y = Some(pos);
                        }
                        points.push(KeyFloatPoint::new(newp, pos.p_type));
                    }

                    KeyPath {
                        closed: path.closed,
                        points,
                    }
                })
                .collect(),
        })
    }

    pub fn to_work_in_weight(&self, weight_alloc: DataHV<Vec<usize>>) -> StrucWork {
        fn process(mut unreliable_list: Vec<usize>, mut allocs: Vec<usize>) -> Vec<f32> {
            let mut map = Vec::with_capacity(allocs.len() + unreliable_list.len() + 1);
            let mut offset = 1;
            match unreliable_list.get(0) {
                Some(0) => {
                    map.extend_from_slice(&[-0.5, 0.0]);
                    unreliable_list.swap_remove(0);
                    offset += 1;
                }
                _ => map.push(0.0),
            }
            unreliable_list
                .into_iter()
                .for_each(|n| allocs.insert(n - offset, 0));

            let mut advance = 0.0;
            let temp: Vec<Option<f32>> = allocs
                .into_iter()
                .map(|weight| {
                    if weight == 0 {
                        None
                    } else {
                        advance += weight as f32;
                        Some(advance)
                    }
                })
                .collect();
            let mut iter = temp.iter();
            let mut pre_val = 0.0;
            while let Some(ref cur_val) = iter.next() {
                if let Some(cur_val) = cur_val {
                    pre_val = *cur_val;
                    map.push(*cur_val);
                } else {
                    match iter.clone().find_map(|v| *v) {
                        Some(las_val) => {
                            map.push((pre_val + las_val) * 0.5);
                        }
                        None => {
                            map.push(pre_val + 0.5);
                        }
                    };
                }
            }

            map
        }

        let size = weight_alloc.map(|weights| match weights.iter().sum::<usize>() {
            0 => 1,
            n => n,
        });
        let unreliable_list = self.unreliable_in();
        let (h_map, v_map) = (
            process(unreliable_list.h, weight_alloc.h),
            process(unreliable_list.v, weight_alloc.v),
        );

        StrucWork {
            tags: self.tags.clone(),
            key_paths: self
                .key_paths
                .iter()
                .map(|path| KeyPath {
                    closed: path.closed,
                    points: path
                        .points
                        .iter()
                        .map(|p| {
                            let mut newp = p.cast();
                            newp.point.x = h_map[p.point.x] / size.h as f32;
                            newp.point.y = v_map[p.point.y] / size.v as f32;
                            newp
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn reduce(mut self, axis: Axis, index: usize) -> Self {
        let maps: HashMap<usize, usize> = self
            .maps_to_real_point()
            .hv_get(axis)
            .iter()
            .map(|(k, v)| (*v, *k))
            .collect();
        let start = maps[&index];
        let end = maps[&(index + 1)];
        let length = end - start;
        let range = start..=end;

        match axis {
            Axis::Horizontal => self.key_paths.iter_mut().for_each(|path| {
                path.points.iter_mut().for_each(|p| {
                    if range.contains(&p.point.x) {
                        p.point.x = *range.start();
                    } else if p.point.x > end {
                        p.point.x -= length
                    }
                })
            }),
            Axis::Vertical => self.key_paths.iter_mut().for_each(|path| {
                path.points.iter_mut().for_each(|p| {
                    if range.contains(&p.point.y) {
                        p.point.y = *range.start();
                    } else if p.point.y > end {
                        p.point.y -= length
                    }
                })
            }),
        };

        self
    }

    pub fn point_attributes(&self) -> (Vec<Vec<PointAttribute>>, Vec<Vec<PointAttribute>>) {
        let size = self.size();
        let (mut h, mut v) = (vec![vec![]; size.width], vec![vec![]; size.height]);

        self.key_paths.iter().for_each(|path| {
            let mut iter = path.points.iter();
            let mut previous = None;
            let mut current = iter.next();
            let mut later = iter.next();

            loop {
                if let Some(&p) = current.take() {
                    let attr = PointAttribute::from_key_point(previous, p, later.cloned());
                    v[p.point.y].push(attr.clone());
                    h[p.point.x].push(attr);

                    previous = Some(p);
                    current = later;
                    later = iter.next();
                } else {
                    break;
                }
            }
        });

        (h, v)
    }

    pub fn attributes(&self) -> StrucAttributes {
        StrucAttrView::new(self).get_space_attrs()
    }

    pub fn to_normal(&self) -> StrucWork {
        fn get_weight(attr: &Vec<PointAttribute>) -> usize {
            match attr.iter().all(|attr| attr.this_point() == 'M') {
                true => 0,
                false => 1,
            }
        }

        if self.is_empty() {
            Default::default()
        }

        let (h_attrs, v_attrs) = self.point_attributes();

        let mut pre_attr = None;
        let v_weight: Vec<_> = v_attrs
            .into_iter()
            .map(|attr| {
                let wight = get_weight(&attr);
                pre_attr = Some(attr);
                wight
            })
            .collect();
        pre_attr = None;
        let h_weight: Vec<_> = h_attrs
            .into_iter()
            .map(|attr| {
                let wight = get_weight(&attr);
                pre_attr = Some(attr);
                wight
            })
            .collect();

        let unit_x = match h_weight.iter().sum::<usize>() {
            0 | 1 => 1.0,
            n => 1.0 / (n - 1) as f32,
        };
        let unit_y = match v_weight.iter().sum::<usize>() {
            0 | 1 => 1.0,
            n => 1.0 / (n - 1) as f32,
        };

        let mut h_map = Vec::<f32>::with_capacity(h_weight.len());
        h_weight.into_iter().fold(-unit_x, |pre, wight| {
            if wight == 0 {
                h_map.push(pre + 0.5 * unit_x);
                pre
            } else {
                h_map.push(pre + wight as f32 * unit_x);
                *h_map.last().unwrap()
            }
        });

        let mut v_map = Vec::<f32>::with_capacity(v_weight.len());
        v_weight.into_iter().fold(-unit_y, |pre, wight| {
            if wight == 0 {
                v_map.push(pre + 0.5 * unit_y);
                pre
            } else {
                v_map.push(pre + wight as f32 * unit_y);
                *v_map.last().unwrap()
            }
        });

        StrucWork {
            tags: self.tags.clone(),
            key_paths: self
                .key_paths
                .iter()
                .map(|path| KeyPath {
                    closed: path.closed,
                    points: path
                        .points
                        .iter()
                        .map(|p| KeyPoint {
                            p_type: p.p_type,
                            point: Point2D::new(h_map[p.point.x], v_map[p.point.y]),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn size(&self) -> IndexSize {
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

    pub fn real_size(&self) -> IndexSize {
        let size = self.maps_to_real_point();
        IndexSize::new(size.h.len(), size.v.len())
    }

    pub fn maps_to_real_point(&self) -> DataHV<HashMap<usize, usize>> {
        let (mut v, mut h) = (BTreeSet::new(), BTreeSet::new());

        self.key_paths.iter().for_each(|path| {
            path.points.iter().for_each(|p| match p.p_type {
                KeyPointType::Mark => {}
                KeyPointType::Horizontal => {
                    h.insert(p.point.x);
                }
                KeyPointType::Vertical => {
                    v.insert(p.point.y);
                }
                _ => {
                    h.insert(p.point.x);
                    v.insert(p.point.y);
                }
            })
        });

        DataHV {
            h: h.into_iter().enumerate().map(|(i, n)| (n, i)).collect(),
            v: v.into_iter().enumerate().map(|(i, n)| (n, i)).collect(),
        }
    }

    pub fn maps_to_not_mark_pos(&self) -> DataHV<HashMap<usize, usize>> {
        let (mut v, mut h) = (BTreeSet::new(), BTreeSet::new());

        self.key_paths.iter().for_each(|path| {
            path.points.iter().for_each(|p| {
                if p.p_type != KeyPointType::Mark {
                    h.insert(p.point.x);
                    v.insert(p.point.y);
                }
            })
        });

        DataHV {
            h: h.into_iter().enumerate().map(|(i, n)| (n, i)).collect(),
            v: v.into_iter().enumerate().map(|(i, n)| (n, i)).collect(),
        }
    }

    pub fn unreliable_in(&self) -> DataHV<Vec<usize>> {
        let (mut v1, mut h1) = (HashSet::new(), HashSet::new());
        let (mut v2, mut h2) = (HashSet::new(), HashSet::new());

        self.key_paths.iter().for_each(|path| {
            path.points.iter().for_each(|p| match p.p_type {
                KeyPointType::Mark => {
                    h1.insert(p.point.x);
                    v1.insert(p.point.y);
                }
                KeyPointType::Vertical => {
                    h1.insert(p.point.x);
                    v2.insert(p.point.y);
                }
                KeyPointType::Horizontal => {
                    v1.insert(p.point.y);
                    h2.insert(p.point.x);
                }
                _ => {
                    h2.insert(p.point.x);
                    v2.insert(p.point.y);
                }
            })
        });

        let mut list = DataHV {
            h: h1
                .into_iter()
                .filter(|v| !h2.contains(v))
                .collect::<Vec<usize>>(),
            v: v1
                .into_iter()
                .filter(|v| !v2.contains(v))
                .collect::<Vec<usize>>(),
        };

        list.h.sort();
        list.v.sort();

        list
    }
}

impl<T: Default + Clone + Copy + Ord, U> Struc<T, U> {
    pub fn is_empty(&self) -> bool {
        self.key_paths.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduce() {
        let proto = StrucProto {
            tags: Default::default(),
            key_paths: vec![
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        // KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Mark),
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Horizontal),
                    ],
                },
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(2, 3), KeyPointType::Line),
                    ],
                },
            ],
        }
        .reduce(Axis::Vertical, 0);

        assert!(proto
            .key_paths
            .iter()
            .find(|path| path.points.iter().find(|p| p.point.y != 1).is_some())
            .is_none());
    }

    #[test]
    fn test_size() {
        let mut key_points = StrucWork::default();
        key_points.add_lines([WorkPoint::new(1.0, 2.0), WorkPoint::new(2.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(1.0, 0.0), WorkPoint::new(1.0, 3.0)], false);
        let key_points = key_points.to_prototype();

        assert_eq!(key_points.size(), IndexSize::new(2, 3));

        let proto = StrucProto {
            tags: Default::default(),
            key_paths: vec![
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Mark),
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Horizontal),
                    ],
                },
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(2, 3), KeyPointType::Line),
                    ],
                },
            ],
        };
        assert_eq!(proto.real_size(), IndexSize::new(4, 2));
    }

    #[test]
    fn test_normal() {
        let mut key_points = StrucWork::default();
        key_points.add_lines([WorkPoint::new(0.0, 0.0), WorkPoint::new(1.0, 0.0)], false);

        let normal = key_points.to_prototype().to_normal();
        assert_eq!(
            normal.key_paths[0].points[0].point,
            WorkPoint::new(0.0, 0.0)
        );
        assert_eq!(
            normal.key_paths[0].points[1].point,
            WorkPoint::new(1.0, 0.0)
        );

        let mut key_points = StrucWork::default();
        key_points.add_lines([WorkPoint::new(0.0, 0.0), WorkPoint::new(1.0, 1.0)], false);

        let normal = key_points.to_prototype().to_normal();
        assert_eq!(
            normal.key_paths[0].points[0].point,
            WorkPoint::new(0.0, 0.0)
        );
        assert_eq!(
            normal.key_paths[0].points[1].point,
            WorkPoint::new(1.0, 1.0)
        );

        let mut key_points = StrucWork::default();
        key_points.add_lines([WorkPoint::new(0.0, 1.0), WorkPoint::new(0.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(1.0, 0.0), WorkPoint::new(1.0, 3.0)], false);

        let normal = key_points.to_prototype().to_normal();
        assert_eq!(
            normal.key_paths[0].points[0].point,
            WorkPoint::new(0.0, 1.0 / 3.0)
        );
        assert_eq!(
            normal.key_paths[0].points[1].point,
            WorkPoint::new(0.0, 2.0 / 3.0)
        );
        assert_eq!(
            normal.key_paths[1].points[0].point,
            WorkPoint::new(1.0, 0.0)
        );
        assert_eq!(
            normal.key_paths[1].points[1].point,
            WorkPoint::new(1.0, 1.0)
        );
    }

    #[test]
    fn test_symbol() {
        assert_eq!(
            '0',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(None, None)
        );
        assert_eq!(
            '0',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(0, 0))),
                None
            )
        );
        assert_eq!(
            '0',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                None,
                Some(KeyPoint::new_line_point(Point2D::new(0, 0)))
            )
        );
        assert_eq!(
            '0',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(0, 0))),
                Some(KeyPoint::new_line_point(Point2D::new(0, 0)))
            )
        );
        assert_eq!(
            '6',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(0, 0))),
                Some(KeyPoint::new_line_point(Point2D::new(2, 0)))
            )
        );
        assert_eq!(
            '3',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(3, 2)))
            )
        );
        assert_eq!(
            '2',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(1, 2)))
            )
        );
        assert_eq!(
            '1',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(0, 2)))
            )
        );
        assert_eq!(
            '4',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(0, 1)))
            )
        );
        assert_eq!(
            '7',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(0, 0)))
            )
        );
        assert_eq!(
            '8',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(1, 0)))
            )
        );
        assert_eq!(
            '9',
            PointAttribute::symbol_of_connect::<usize, WorkSpace>(
                Some(KeyPoint::new_line_point(Point2D::new(1, 1))),
                Some(KeyPoint::new_line_point(Point2D::new(2, 0)))
            )
        );
    }
}
