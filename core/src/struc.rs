use euclid::*;
use num_traits::cast::NumCast;
use serde::{Deserialize, Serialize};

use std::{
    collections::{BTreeSet, HashSet},
    fmt::Write,
};

#[derive(Default, Serialize, Deserialize, Clone, Copy)]
pub struct IndexSpace;
pub type IndexPoint = Point2D<usize, IndexSpace>;
pub type IndexSize = Size2D<usize, IndexSpace>;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub struct WorkSpace;
pub type WorkPoint = Point2D<f32, WorkSpace>;
pub type WorkSize = Size2D<f32, WorkSpace>;
pub type WorkVec = Vector2D<f32, WorkSpace>;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyPointType {
    Line,
    Mark,
    Hide,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyPoint<T: Clone + Copy, U> {
    pub p_type: KeyPointType,
    pub point: Point2D<T, U>,
}

impl<T: Clone + Copy, U> KeyPoint<T, U> {
    pub fn new(point: Point2D<T, U>, p_type: KeyPointType) -> Self {
        Self { point, p_type }
    }

    pub fn new_line_point(point: Point2D<T, U>) -> Self {
        Self {
            point,
            p_type: KeyPointType::Line,
        }
    }
}

impl<T: Clone + Copy + NumCast, U> KeyPoint<T, U> {
    pub fn cast<NewT, NewU>(&self) -> KeyPoint<NewT, NewU>
    where
        NewT: Clone + Copy + NumCast,
    {
        KeyPoint {
            p_type: self.p_type,
            point: self.point.cast().cast_unit(),
        }
    }
}

pub type KeyIndexPoint = KeyPoint<usize, IndexSpace>;
pub type KeyFloatPoint<U> = KeyPoint<f32, U>;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct KeyPath<T: Clone + Copy, U> {
    pub closed: bool,
    pub points: Vec<KeyPoint<T, U>>,
}

impl<T: Clone + Copy + NumCast, U> KeyPath<T, U> {
    pub fn new(points: Vec<KeyPoint<T, U>>, closed: bool) -> Self {
        Self { closed, points }
    }

    pub fn cast<NewT, NewU>(&self) -> KeyPath<NewT, NewU>
    where
        NewT: Clone + Copy + NumCast,
    {
        KeyPath {
            closed: self.closed,
            points: self.points.iter().map(|p| p.cast()).collect(),
        }
    }

    pub fn hide(&mut self) {
        self.points
            .iter_mut()
            .for_each(|p| p.p_type = KeyPointType::Hide);
    }
}

pub type KeyIndexPath = KeyPath<usize, IndexSpace>;
pub type KeyFloatPath<U> = KeyPath<f32, U>;

impl KeyFloatPath<WorkSpace> {
    pub fn from_lines<I>(path: I, closed: bool) -> Self
    where
        I: IntoIterator<Item = WorkPoint>,
    {
        Self {
            closed,
            points: path
                .into_iter()
                .map(|p| KeyFloatPoint::new(p.cast(), KeyPointType::Line))
                .collect(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Struc<T: Default + Clone + Copy, U> {
    pub key_paths: Vec<KeyPath<T, U>>,
    pub tags: BTreeSet<String>,
}

pub type StrucProto = Struc<usize, IndexSpace>;
pub type StrucWokr = Struc<f32, WorkSpace>;

impl StrucWokr {
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

#[derive(Clone)]
pub struct PointAttribute {
    symbols: [char; 5],
}

impl PointAttribute {
    pub const SEPARATOR_SYMBOL: char = ';';

    pub fn is_match(&self, rule: [char; 5]) -> bool {
        (0..5).into_iter().fold(true, |mut ok, i| {
            ok &= match rule[i] {
                '*' => true,
                c => c == self.symbols[i],
            };
            ok
        })
    }
    pub fn symbol_of_type(p_type: Option<KeyPointType>) -> char {
        match p_type {
            Some(p_type) => match p_type {
                KeyPointType::Hide => 'H',
                KeyPointType::Line => 'L',
                KeyPointType::Mark => 'M',
            },
            None => 'X',
        }
    }

    pub fn symbol_of_connect<T, U>(p1: Option<KeyPoint<T, U>>, p2: Option<KeyPoint<T, U>>) -> char
    where
        T: Clone + Copy + NumCast,
    {
        match p1 {
            Some(p1) => match p2 {
                Some(p2) => {
                    let p1: Point2D<f32, _> = p1.point.cast();
                    let p2: Point2D<f32, _> = p2.point.cast();

                    if p1 == p2 {
                        '0'
                    } else {
                        let angle = (p2 - p1).angle_from_x_axis();

                        if angle == Angle::zero() {
                            '6'
                        } else if angle > Angle::zero() && angle < Angle::frac_pi_2() {
                            '3'
                        } else if angle == Angle::frac_pi_2() {
                            '2'
                        } else if angle > Angle::frac_pi_2() && angle < Angle::pi() {
                            '1'
                        } else if angle == Angle::pi() || angle == -Angle::pi() {
                            '4'
                        } else if angle > -Angle::pi() && angle < -Angle::frac_pi_2() {
                            '7'
                        } else if angle == -Angle::frac_pi_2() {
                            '8'
                        } else {
                            '9'
                        }
                    }
                }
                None => '0',
            },
            None => '0',
        }
    }

    pub fn from_key_point<T, U>(
        previous: Option<KeyPoint<T, U>>,
        current: KeyPoint<T, U>,
        later: Option<KeyPoint<T, U>>,
    ) -> Self
    where
        T: Clone + Copy + NumCast,
        U: Copy,
    {
        let mut symbols = ['\0'; 5];

        symbols[0] = Self::symbol_of_type(previous.map(|kp| kp.p_type));
        symbols[1] = Self::symbol_of_connect(previous, Some(current));
        symbols[2] = Self::symbol_of_type(Some(current.p_type));
        symbols[3] = Self::symbol_of_connect(Some(current), later);
        symbols[4] = Self::symbol_of_type(later.map(|kp| kp.p_type));

        Self { symbols }
    }
}

impl ToString for PointAttribute {
    fn to_string(&self) -> String {
        let mut str = String::with_capacity(6);
        str.extend(self.symbols.iter());
        str.push(';');
        str
    }
}

#[derive(Default)]
pub struct StrucAttributes<const PREFIX_H: char = 'h', const PREFIX_V: char = 'v'> {
    pub h_attrs: Vec<String>,
    pub v_attrs: Vec<String>,
}

impl<const PREFIX_H: char, const PREFIX_V: char> StrucAttributes<PREFIX_H, PREFIX_V> {
    fn get_attr_info<const PREFIX: char>(
        index: usize,
        attrs: &Vec<String>,
        buffer: &mut String,
    ) -> Result<(), std::fmt::Error> {
        buffer.clear();
        if index < attrs.len() {
            let real_points: Vec<usize> = attrs
                .iter()
                .enumerate()
                .filter_map(|(i, attr)| match StrucProto::mark_regex().is_match(attr) {
                    false => Some(i),
                    true => None,
                })
                .collect();
            if let Some(index) = real_points.iter().position(|&n| n == index) {
                buffer.push(PREFIX);
                if index != 0 {
                    write!(buffer, "{}-", attrs[real_points[index - 1]].as_str())?;
                }
                write!(
                    buffer,
                    "{}>{}<{}",
                    index,
                    attrs[real_points[index]],
                    real_points.len() - index - 1
                )?;
                if index + 1 != real_points.len() {
                    write!(buffer, "-{}", attrs[real_points[index + 1]].as_str())?;
                }
            } else {
                buffer.push_str(attrs[index].as_str());
            }
        }

        Ok(())
    }

    fn match_indexes<const PREFIX: char>(
        regex: &regex::Regex,
        attrs: &Vec<String>,
        buffer: &mut String,
    ) -> Vec<usize> {
        (0..attrs.len())
            .into_iter()
            .filter_map(|i| {
                Self::get_attr_info::<PREFIX>(i, attrs, buffer).unwrap();
                if regex.is_match(buffer) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn new(h_attrs: Vec<String>, v_attrs: Vec<String>) -> Self {
        Self { h_attrs, v_attrs }
    }

    pub fn from_point_attr(
        h_attrs: Vec<Vec<PointAttribute>>,
        v_attrs: Vec<Vec<PointAttribute>>,
    ) -> Self {
        Self {
            h_attrs: h_attrs
                .into_iter()
                .map(|pas| pas.into_iter().map(|pa| pa.to_string()).collect())
                .collect(),
            v_attrs: v_attrs
                .into_iter()
                .map(|pas| pas.into_iter().map(|pa| pa.to_string()).collect())
                .collect(),
        }
    }

    pub fn match_indexes_all(
        &self,
        regex: &regex::Regex,
        buffer: &mut String,
    ) -> (Vec<usize>, Vec<usize>) {
        (
            Self::match_indexes::<PREFIX_H>(&regex, &self.h_attrs, buffer),
            Self::match_indexes::<PREFIX_V>(&regex, &self.v_attrs, buffer),
        )
    }

    pub fn get_horizontal_attr<'attr>(
        &'attr self,
        index: usize,
        buffer: &'attr mut String,
    ) -> Option<&'attr str> {
        if index >= self.horizontal_len() {
            None
        } else {
            Self::get_attr_info::<PREFIX_H>(index, &self.h_attrs, buffer).unwrap();
            Some(buffer.as_str())
        }
    }

    pub fn get_vertical_attr<'attr>(
        &'attr self,
        index: usize,
        buffer: &'attr mut String,
    ) -> Option<&'attr str> {
        if index >= self.vertical_len() {
            None
        } else {
            Self::get_attr_info::<PREFIX_V>(index, &self.v_attrs, buffer).unwrap();
            Some(buffer.as_str())
        }
    }

    pub fn horizontal_len(&self) -> usize {
        self.h_attrs.len()
    }

    pub fn vertical_len(&self) -> usize {
        self.v_attrs.len()
    }
}

impl StrucProto {
    const OFFSET: f32 = 0.01;

    pub fn mark_regex() -> &'static regex::Regex {
        static MARK_REGEX: once_cell::sync::OnceCell<regex::Regex> =
            once_cell::sync::OnceCell::new();
        MARK_REGEX.get_or_init(|| regex::Regex::new("(..M..;)+").unwrap())
    }

    pub fn from_work(struc: &StrucWokr) -> Self {
        Self::from_work_offset(struc, Self::OFFSET)
    }

    pub fn from_work_offset(struc: &StrucWokr, offset: f32) -> Self {
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

    pub fn to_work(&self) -> StrucWokr {
        StrucWokr::from_prototype(self)
    }

    pub fn to_work_in_weight(&self, h_alloc: Vec<usize>, v_alloc: Vec<usize>) -> StrucWokr {
        let mut advance = -1.0;
        let temp: Vec<Option<f32>> = h_alloc
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
        let mut h_map = vec![];
        let mut pre = None;
        while let Some(ref cur_val) = iter.next() {
            if let Some(cur_val) = cur_val {
                pre = Some(cur_val);
                h_map.push(*cur_val);
            } else {
                match iter.clone().find_map(|v| *v) {
                    Some(las_val) => {
                        if let Some(pre_val) = pre {
                            h_map.push((pre_val + las_val) * 0.5);
                        } else {
                            h_map.push(las_val - 0.5);
                        }
                    }
                    None => {
                        if let Some(pre_val) = pre {
                            h_map.push(pre_val + 0.5);
                        } else {
                            h_map.push(0.0);
                        }
                    }
                };
            }
        }

        advance = -1.0;
        let temp: Vec<Option<f32>> = v_alloc
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
        let mut v_map = vec![];
        let mut pre = None;
        while let Some(ref cur_val) = iter.next() {
            if let Some(cur_val) = cur_val {
                pre = Some(cur_val);
                v_map.push(*cur_val);
            } else {
                match iter.clone().find_map(|v| *v) {
                    Some(las_val) => {
                        if let Some(pre_val) = pre {
                            v_map.push((pre_val + las_val) * 0.5);
                        } else {
                            v_map.push(las_val - 0.5);
                        }
                    }
                    None => {
                        if let Some(pre_val) = pre {
                            v_map.push(pre_val + 0.5);
                        } else {
                            v_map.push(0.0);
                        }
                    }
                };
            }
        }

        StrucWokr {
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
                            newp.point.x = h_map[p.point.x];
                            newp.point.y = v_map[p.point.y];
                            newp
                        })
                        .collect(),
                })
                .collect(),
        }
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

    pub fn attributes<const PREFIX_H: char, const PREFIX_V: char>(
        &self,
    ) -> StrucAttributes<PREFIX_H, PREFIX_V> {
        let (h, v) = self.point_attributes();

        StrucAttributes::from_point_attr(h, v)
    }

    pub fn to_normal(&self) -> StrucWokr {
        fn get_weight_horizontal(attr: &Vec<PointAttribute>) -> usize {
            let weight = if attr.iter().all(|attr| attr.symbols[2] == 'M') {
                0
            } else {
                1
            };
            weight
        }

        fn get_weight_vertical(attr: &Vec<PointAttribute>) -> usize {
            let weight = if attr.iter().all(|attr| attr.symbols[2] == 'M') {
                0
            } else {
                1
            };
            weight
        }

        if self.is_empty() {
            Default::default()
        }

        let (h_attrs, v_attrs) = self.point_attributes();

        let mut pre_attr = None;
        let v_weight: Vec<_> = v_attrs
            .into_iter()
            .map(|attr| {
                let wight = get_weight_vertical(&attr);
                pre_attr = Some(attr);
                wight
            })
            .collect();
        pre_attr = None;
        let h_weight: Vec<_> = h_attrs
            .into_iter()
            .map(|attr| {
                let wight = get_weight_horizontal(&attr);
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

        StrucWokr {
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
}

impl StrucProto {
    pub fn size(&self) -> IndexSize {
        self.key_paths.iter().fold(Size2D::default(), |size, path| {
            path.points.iter().fold(size, |size, kp| {
                Size2D::new(
                    size.width.max(kp.point.x + 1),
                    size.height.max(kp.point.y + 1),
                )
            })
        })
    }

    pub fn real_size(&self) -> IndexSize {
        let (mut v, mut h) = (HashSet::new(), HashSet::new());

        self.key_paths.iter().for_each(|path| {
            path.points.iter().for_each(|p| {
                if p.p_type != KeyPointType::Mark {
                    h.insert(p.point.x);
                    v.insert(p.point.y);
                }
            })
        });

        Size2D::new(h.len(), v.len())
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
    fn test_size() {
        let mut key_points = StrucWokr::default();
        key_points.add_lines([WorkPoint::new(0.0, 2.0), WorkPoint::new(2.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(1.0, 0.0), WorkPoint::new(1.0, 3.0)], false);
        let key_points = key_points.to_prototype();

        assert_eq!(key_points.size(), Size2D::new(3, 3));
    }

    #[test]
    fn test_normal() {
        let mut key_points = StrucWokr::default();
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

        let mut key_points = StrucWokr::default();
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

        let mut key_points = StrucWokr::default();
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
