use euclid::*;
use num_traits::cast::NumCast;
use serde::{Deserialize, Serialize};

use std::{
    collections::{BTreeSet, HashMap, HashSet},
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

pub struct DataHV<T> {
    pub h: T,
    pub v: T,
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

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PointAttribute {
    symbols: [char; 3],
}

pub struct Error {
    pub msg: String,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

enum PaddingPointAttr {
    Line(PointAttribute),
    Box([PointAttribute; 4]),
}

impl PointAttribute {
    pub const SEPARATOR_SYMBOL: &'static str = ";";

    pub fn new(symbols: [char; 3]) -> Self {
        Self { symbols }
    }

    pub fn negative(connect: char) -> char {
        match connect {
            '1' => '9',
            '2' => '8',
            '3' => '7',
            '4' => '6',
            other => other,
        }
    }

    fn padding_next(&self) -> Result<PaddingPointAttr, Error> {
        match self.next_connect() {
            '1' | '3' | '9' | '7' => Ok(PaddingPointAttr::Box([
                PointAttribute::new(['t', self.this_point(), 't']),
                PointAttribute::new(['b', self.this_point(), 'b']),
                PointAttribute::new(['l', self.this_point(), 'l']),
                PointAttribute::new(['r', self.this_point(), 'r']),
            ])),
            '2' | '8' => Ok(PaddingPointAttr::Line(PointAttribute::new([
                'v',
                self.this_point(),
                'v',
            ]))),
            '6' | '4' => Ok(PaddingPointAttr::Line(PointAttribute::new([
                'h',
                self.this_point(),
                'h',
            ]))),
            n => Err(Error {
                msg: format!("not next symbol `{}`", n),
            }),
        }
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
        next: Option<KeyPoint<T, U>>,
    ) -> Self
    where
        T: Clone + Copy + NumCast,
        U: Copy,
    {
        let mut symbols = ['\0'; 3];

        symbols[0] = Self::symbol_of_connect(Some(current), previous);
        symbols[1] = Self::symbol_of_type(Some(current.p_type));
        symbols[2] = Self::symbol_of_connect(Some(current), next);

        Self { symbols }
    }

    pub fn front_connect(&self) -> char {
        self.symbols[0]
    }

    pub fn this_point(&self) -> char {
        self.symbols[1]
    }

    pub fn next_connect(&self) -> char {
        self.symbols[2]
    }
}

impl ToString for PointAttribute {
    fn to_string(&self) -> String {
        self.symbols.iter().collect()
    }
}

pub struct StrucView {
    pub view: Vec<Vec<BTreeSet<PointAttribute>>>,
}

#[allow(unused)]
fn generate_attr(pas: &BTreeSet<PointAttribute>) -> String {
    pas.iter().map(|pa| pa.to_string()).collect()
}

impl StrucView {
    pub fn new(struc: &StrucProto) -> Result<Self, Error> {
        let maps = struc.maps_to_real_point();
        let size = IndexSize::new(maps.h.len(), maps.v.len());
        let mut view = vec![vec![BTreeSet::new(); size.width]; size.height];

        for path in struc.key_paths.iter() {
            let mut iter = path
                .points
                .iter()
                .filter(|kp| kp.p_type != KeyPointType::Mark);
            let mut previous = None;
            let mut current = iter.next();
            let mut next = iter.next();

            loop {
                match current.take() {
                    Some(&kp) => {
                        let pa = PointAttribute::from_key_point(previous, kp, next.copied());
                        let real_pos = IndexPoint::new(maps.h[&kp.point.x], maps.v[&kp.point.y]);

                        if let Some(next) = next {
                            let next =
                                IndexPoint::new(maps.h[&next.point.x], maps.v[&next.point.y]);
                            let (x1, y1) = (real_pos.x.min(next.x), real_pos.y.min(next.y));
                            let (x2, y2) = (real_pos.x.max(next.x), real_pos.y.max(next.y));
                            match pa.padding_next().or_else(|e| {
                                Err(Error {
                                    msg: format!(
                                        "in pos({}, {}) {}",
                                        kp.point.x, kp.point.y, e.msg
                                    ),
                                })
                            })? {
                                PaddingPointAttr::Line(padding) => {
                                    if x1 == x2 {
                                        (y1 + 1..y2).for_each(|y| {
                                            view[y][x1].insert(padding);
                                        });
                                    } else if y1 == y2 {
                                        (x1 + 1..x2).for_each(|x| {
                                            view[y1][x].insert(padding);
                                        });
                                    } else {
                                        unreachable!()
                                    }
                                }
                                PaddingPointAttr::Box(
                                    [padding_t, padding_b, padding_l, padding_r],
                                ) => {
                                    (x1 + 1..x2).for_each(|x| {
                                        view[y1][x].insert(padding_t);
                                    });
                                    (x1 + 1..x2).for_each(|x| {
                                        view[y2][x].insert(padding_b);
                                    });
                                    (y1 + 1..y2).for_each(|y| {
                                        view[y][x1].insert(padding_l);
                                    });
                                    (y1 + 1..y2).for_each(|y| {
                                        view[y][x2].insert(padding_r);
                                    });
                                }
                            }
                        }
                        view[real_pos.y][real_pos.x].insert(pa);

                        previous = Some(kp);
                        current = next;
                        next = iter.next();
                    }
                    None => break,
                }
            }
        }

        Ok(Self { view })
    }

    fn get_space_attrs(&self) -> StrucAttributes {
        let view = &self.view;
        let width = match view.is_empty() {
            true => 0,
            false => view[0].len(),
        };

        let (mut h, mut v) = (
            Vec::with_capacity(width.max(1) - 1),
            Vec::with_capacity(view.len().max(1) - 1),
        );
        let mut output = String::new();
        let mut attr1 = Vec::<[char; 2]>::new();
        let mut attr2 = Vec::<[char; 2]>::new();
        let mut pre_attr1 = Vec::<[char; 2]>::new();
        let mut pre_attr2 = Vec::<[char; 2]>::new();
        let mut padding1 = String::new();
        let mut padding2 = String::new();
        let mut space1 = Vec::<[char; 2]>::new();
        let mut space2 = Vec::<[char; 2]>::new();

        for x in 1..width {
            for y in 0..view.len() {
                // let test1 = generate_attr(&view[y][x - 1]);
                // let test2 = generate_attr(&view[y][x]);
                let mut ok = y + 1 == view.len() || y == 0;

                view[y][x - 1].iter().for_each(|p_attr| {
                    for c in [p_attr.front_connect(), p_attr.next_connect()] {
                        match c {
                            '3' | '2' => {
                                attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '6' => {
                                attr1.push([p_attr.this_point(), c]);
                                pre_attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '8' | '9' => {
                                pre_attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '1' | '4' | '7' => {
                                padding1.push(p_attr.this_point());
                                padding1.push('5');
                            }
                            _ => {}
                        }
                    }
                    match p_attr.front_connect() {
                        'h' => {
                            attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            ok = true;
                        }
                        'v' => {
                            attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                        }
                        't' | 'b' | 'l' | 'r' => {
                            padding1.push(p_attr.this_point());
                            padding1.push(p_attr.front_connect());
                        }
                        _ => {}
                    }
                });
                view[y][x].iter().for_each(|p_attr| {
                    for c in [p_attr.front_connect(), p_attr.next_connect()] {
                        match c {
                            '2' | '1' => {
                                attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '4' => {
                                attr2.push([p_attr.this_point(), c]);
                                pre_attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '8' | '7' => {
                                pre_attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '9' | '6' | '3' => {
                                padding2.push(p_attr.this_point());
                                padding2.push('5');
                            }
                            _ => {}
                        }
                    }
                    match p_attr.front_connect() {
                        'h' => {
                            attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            ok = true;
                        }
                        'v' => {
                            attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                        }
                        't' | 'b' | 'l' | 'r' => {
                            padding2.push(p_attr.this_point());
                            padding2.push(p_attr.front_connect());
                        }
                        _ => {}
                    }
                });

                if !ok {
                    padding1.extend(attr1.drain(..).into_iter().flatten());
                    padding2.extend(attr2.drain(..).into_iter().flatten());
                    pre_attr1.clear();
                    pre_attr2.clear();
                } else {
                    attr1.sort_by(|[_, a], [_, b]| a.cmp(b));
                    attr2.sort_by(|[_, a], [_, b]| a.cmp(b));
                    pre_attr1.sort_by(|[_, a], [_, b]| a.cmp(b));
                    pre_attr2.sort_by(|[_, a], [_, b]| a.cmp(b));

                    if y == 0 {
                        write!(
                            output,
                            "{}-{}",
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else if y + 1 == view.len() {
                        write!(
                            output,
                            ">{}-{}<{}-{};",
                            padding1,
                            padding2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else {
                        write!(
                            output,
                            ">{}-{}<{}-{};{}-{}",
                            padding1,
                            padding2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>(),
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    }
                    padding1.clear();
                    padding2.clear();
                    attr1.clear();
                    attr2.clear();
                    pre_attr1.clear();
                    pre_attr2.clear();
                }
            }

            space1.sort_by(|[_, a], [_, b]| a.cmp(b));
            space2.sort_by(|[_, a], [_, b]| a.cmp(b));
            h.push(format!(
                "h:{}-{}:{}:{}-{}",
                space1.iter().flatten().collect::<String>(),
                space2.iter().flatten().collect::<String>(),
                output,
                x - 1,
                width - x - 1,
            ));
            output.clear();
            space1.clear();
            space2.clear();
        }

        for y in 1..view.len() {
            for x in 0..width {
                let mut ok = x + 1 == width || x == 0;
                view[y - 1][x].iter().for_each(|p_attr| {
                    for c in [p_attr.front_connect(), p_attr.next_connect()] {
                        match c {
                            '6' | '3' => {
                                attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '2' => {
                                attr1.push([p_attr.this_point(), c]);
                                pre_attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '1' | '4' => {
                                pre_attr1.push([p_attr.this_point(), c]);
                                space1.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '7' | '8' | '9' => {
                                padding1.push(p_attr.this_point());
                                padding1.push('5');
                            }
                            _ => {}
                        }
                    }
                    match p_attr.front_connect() {
                        'v' => {
                            attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            ok = true;
                        }
                        'h' => {
                            attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                        }
                        't' | 'b' | 'l' | 'r' => {
                            padding1.push(p_attr.this_point());
                            padding1.push(p_attr.front_connect());
                        }
                        _ => {}
                    }
                });
                view[y][x].iter().for_each(|p_attr| {
                    for c in [p_attr.front_connect(), p_attr.next_connect()] {
                        match c {
                            '6' | '9' => {
                                attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '8' => {
                                attr2.push([p_attr.this_point(), c]);
                                pre_attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '7' | '4' => {
                                pre_attr2.push([p_attr.this_point(), c]);
                                space2.push([p_attr.this_point(), c]);
                                ok = true;
                            }
                            '1' | '2' | '3' => {
                                padding2.push(p_attr.this_point());
                                padding2.push('5');
                            }
                            _ => {}
                        }
                    }
                    match p_attr.front_connect() {
                        'v' => {
                            attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            ok = true;
                        }
                        'h' => {
                            attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                        }
                        't' | 'b' | 'l' | 'r' => {
                            padding2.push(p_attr.this_point());
                            padding2.push(p_attr.front_connect());
                        }
                        _ => {}
                    }
                });

                if !ok {
                    padding1.extend(attr1.drain(..).into_iter().flatten());
                    padding2.extend(attr2.drain(..).into_iter().flatten());
                    pre_attr1.clear();
                    pre_attr2.clear();
                } else {
                    attr1.sort_by(|[_, a], [_, b]| a.cmp(b));
                    attr2.sort_by(|[_, a], [_, b]| a.cmp(b));
                    pre_attr1.sort_by(|[_, a], [_, b]| a.cmp(b));
                    pre_attr2.sort_by(|[_, a], [_, b]| a.cmp(b));

                    if x == 0 {
                        write!(
                            output,
                            "{}-{}",
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else if x + 1 == width {
                        write!(
                            output,
                            ">{}-{}<{}-{};",
                            padding1,
                            padding2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else {
                        write!(
                            output,
                            ">{}-{}<{}-{};{}-{}",
                            padding1,
                            padding2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>(),
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    }
                    padding1.clear();
                    padding2.clear();
                    attr1.clear();
                    attr2.clear();
                    pre_attr1.clear();
                    pre_attr2.clear();
                }
            }

            space1.sort_by(|[_, a], [_, b]| a.cmp(b));
            space1.sort_by(|[_, a], [_, b]| a.cmp(b));
            v.push(format!(
                "v:{}-{}:{}:{}-{}",
                space1.iter().flatten().collect::<String>(),
                space2.iter().flatten().collect::<String>(),
                output,
                y - 1,
                view.len() - y - 1,
            ));
            output.clear();
            space1.clear();
            space2.clear();
        }

        StrucAttributes::new(h, v)
    }
}

#[derive(Default)]
pub struct StrucAttributes {
    pub h: Vec<String>,
    pub v: Vec<String>,
}

impl StrucAttributes {
    pub fn new(h_attrs: Vec<String>, v_attrs: Vec<String>) -> Self {
        Self {
            h: h_attrs,
            v: v_attrs,
        }
    }

    pub fn all_match_indexes(&self, regex: &regex::Regex) -> DataHV<Vec<usize>> {
        DataHV {
            h: self
                .h
                .iter()
                .enumerate()
                .filter_map(|(i, attr)| match regex.is_match(&attr) {
                    true => Some(i),
                    false => None,
                })
                .collect(),
            v: self
                .v
                .iter()
                .enumerate()
                .filter_map(|(i, attr)| match regex.is_match(&attr) {
                    true => Some(i),
                    false => None,
                })
                .collect(),
        }
    }
}

impl StrucProto {
    const OFFSET: f32 = 0.01;

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
            let mut pre = None;
            while let Some(ref cur_val) = iter.next() {
                if let Some(cur_val) = cur_val {
                    pre = Some(cur_val);
                    map.push(*cur_val);
                } else {
                    match iter.clone().find_map(|v| *v) {
                        Some(las_val) => {
                            if let Some(pre_val) = pre {
                                map.push((pre_val + las_val) * 0.5);
                            } else {
                                map.push(las_val - 0.5);
                            }
                        }
                        None => {
                            if let Some(pre_val) = pre {
                                map.push(pre_val + 0.5);
                            } else {
                                map.push(0.0);
                            }
                        }
                    };
                }
            }

            map
        }

        let unreliable_list = self.unreliable_in();
        let (h_map, v_map) = (
            process(unreliable_list.h, h_alloc),
            process(unreliable_list.v, v_alloc),
        );

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

    pub fn attributes(&self) -> Result<StrucAttributes, Error> {
        Ok(StrucView::new(self)?.get_space_attrs())
    }

    pub fn attributes_segment(&self) -> Result<StrucAttributes, Error> {
        Ok(StrucView::new(self)?.get_space_attrs())
    }

    pub fn to_normal(&self) -> StrucWokr {
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
        let size = self.maps_to_real_point();
        Size2D::new(size.h.len(), size.v.len())
    }

    pub fn maps_to_real_point(&self) -> DataHV<HashMap<usize, usize>> {
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
            path.points.iter().for_each(|p| {
                if p.p_type == KeyPointType::Mark {
                    h1.insert(p.point.x);
                    v1.insert(p.point.y);
                } else {
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
