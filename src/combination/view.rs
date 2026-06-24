use crate::{base::*, combination::struc::StrucProto};

pub enum SharpnessModel {
    ZeroOne,
}

#[derive(Debug, Clone)]
pub struct Edge {
    axis: Axis,
    data: Vec<Option<Vec<ViewElement>>>,
}

impl Edge {
    pub fn new(axis: Axis) -> Self {
        Self { axis, data: vec![] }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_line(&self) -> &Vec<Option<Vec<ViewElement>>> {
        &self.data
    }

    pub fn contain_black(&self) -> bool {
        use Direction::*;
        if self.data.len() < 2 {
            return false;
        }

        for (i, eles) in self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, eles)| eles.as_ref().map(|eles| (i, eles)))
        {
            for ele in eles {
                let b = match self.axis {
                    Axis::Horizontal => match ele {
                        ViewElement::D(Above) if i != 0 => true,
                        ViewElement::D(Below) if i + 1 != self.data.len() => true,
                        ViewElement::Vertical => true,
                        _ => false,
                    },
                    Axis::Vertical => match ele {
                        ViewElement::D(Left) if i != 0 => true,
                        ViewElement::D(Right) if i + 1 != self.data.len() => true,
                        ViewElement::Horizontal => true,
                        _ => false,
                    },
                };
                if b {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn all_black(&self, start: usize, n: usize) -> bool {
        use Direction::*;

        if start + n > self.data.len() {
            return false;
        }

        let iter = self
            .data
            .iter()
            .skip(start)
            .take(n)
            .filter_map(|eles| eles.as_ref());

        iter.clone()
            .zip(iter.skip(1))
            .all(|(f, b)| match self.axis {
                Axis::Horizontal => {
                    f.iter()
                        .any(|&e| e == ViewElement::D(Below) || e == ViewElement::Vertical)
                        && b.iter()
                            .any(|&e| e == ViewElement::D(Above) || e == ViewElement::Vertical)
                }
                Axis::Vertical => {
                    f.iter()
                        .any(|&e| e == ViewElement::D(Right) || e == ViewElement::Horizontal)
                        && b.iter()
                            .any(|&e| e == ViewElement::D(Left) || e == ViewElement::Horizontal)
                }
            })
    }

    pub fn sharpness(&self, model: SharpnessModel) -> f32 {
        match model {
            SharpnessModel::ZeroOne => {
                if self.contain_black() {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    pub fn connect(&mut self, mut other: Edge, space: usize) {
        if space == 0 {
            self.data
                .last_mut()
                .unwrap()
                .get_or_insert_default()
                .append(other.data[0].take().get_or_insert_default());
            self.data.extend(other.data.into_iter().skip(1));
        } else {
            if space > 1 {
                self.data.extend(vec![None; space - 1]);
            }
            self.data.append(&mut other.data);
        }
    }

    pub fn split_off(&mut self, at: usize) -> Edge {
        let new_data = self.data.split_off(at);
        Self {
            axis: self.axis,
            data: new_data,
        }
    }

    pub fn add(&mut self) {
        self.data.push(None);
    }

    pub fn add_in(&mut self, idx: usize, n: usize) {
        self.data.splice(idx..idx, vec![None; n]);
    }

    pub fn add_head(&mut self) {
        self.data.insert(0, None);
    }

    pub fn backspace(&mut self) {
        self.data.fill(None);
    }

    pub fn to_shape(&self) -> EdgeShape {
        let status: Vec<usize> = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(i, vec)| {
                let mut r = None;
                if let Some(vec) = vec {
                    if vec.iter().any(|ele| !ele.is_diagonal_padding()) {
                        r = Some(i)
                    }
                }
                r
            })
            .collect();

        let (b1, b2) = if status.is_empty() {
            (2, 2)
        } else {
            let median1 = self.data.len() / 2;
            let mut median2 = self.data.len() / 2;
            if self.data.len() % 2 == 0 {
                median2 = median2.checked_sub(1).unwrap_or_default();
            }

            let i = *status.first().unwrap();
            let b1 = if i == 0 {
                0
            } else if i + 1 == self.data.len() || i >= median1 {
                2
            } else {
                1
            };
            let i = *status.last().unwrap();
            let b2 = if i + 1 == self.data.len() {
                0
            } else if i == 0 || i <= median2 {
                2
            } else {
                1
            };

            (b1, b2)
        };

        let (t1, t2) = if status.is_empty() {
            (ShapeTrend::None, ShapeTrend::None)
        } else {
            let (mut t1, mut t2) = (ShapeTrend::None, ShapeTrend::None);
            let f_iter = self.data[status[0]].as_ref().unwrap().iter();
            let b_iter = self.data[*status.last().unwrap()].as_ref().unwrap().iter();

            let trends = [ShapeTrend::Acute, ShapeTrend::Right, ShapeTrend::Obtuse];
            let targets = match self.axis {
                Axis::Horizontal => [
                    vec![
                        ViewElement::D(Direction::LeftAbove),
                        ViewElement::D(Direction::RightAbove),
                    ],
                    vec![
                        ViewElement::D(Direction::Left),
                        ViewElement::D(Direction::Right),
                        ViewElement::D(Direction::None),
                    ],
                    vec![
                        ViewElement::D(Direction::LeftBelow),
                        ViewElement::D(Direction::RightBelow),
                    ],
                ],
                Axis::Vertical => [
                    vec![
                        ViewElement::D(Direction::LeftAbove),
                        ViewElement::D(Direction::LeftBelow),
                    ],
                    vec![
                        ViewElement::D(Direction::Above),
                        ViewElement::D(Direction::Below),
                        ViewElement::D(Direction::None),
                    ],
                    vec![
                        ViewElement::D(Direction::RightAbove),
                        ViewElement::D(Direction::RightBelow),
                    ],
                ],
            };
            for section in Section::list() {
                if f_iter.clone().any(|ele| targets[section.n()].contains(ele)) {
                    t1 = trends[section.n()];
                    break;
                }
            }
            for section in Section::list() {
                if b_iter
                    .clone()
                    .any(|ele| targets[section.inverse().n()].contains(ele))
                {
                    t2 = trends[section.n()];
                    break;
                }
            }
            (t1, t2)
        };

        let middle = if self.contain_black() {
            match self.all_black(status[0], status.last().unwrap() - status[0]) {
                true => ShapeState::Dense,
                false => ShapeState::Breach,
            }
        } else {
            match status.len() {
                0 | 2 => ShapeState::Empty,
                1 => ShapeState::Acute,
                _ => ShapeState::Sparse,
            }
        };

        EdgeShape {
            blank: [b1, b2],
            trend: [t1, t2],
            middle,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeState {
    Empty,
    Acute,
    Sparse,
    Breach,
    Dense,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeTrend {
    Acute,
    Right,
    Obtuse,
    None,
}

#[derive(Clone, Debug)]
pub struct EdgeShape {
    pub blank: [usize; 2],
    pub trend: [ShapeTrend; 2],
    pub middle: ShapeState,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    LeftBelow,
    Below,
    RightBelow,
    Left,
    None,
    Right,
    LeftAbove,
    Above,
    RightAbove,
}

impl Direction {
    pub fn new(from: IndexPoint, to: Option<IndexPoint>) -> Self {
        use std::cmp::Ordering::*;

        match to {
            Some(to) => match (to.x.cmp(&from.x), to.y.cmp(&from.y)) {
                (Less, Greater) => Self::LeftBelow,
                (Less, Equal) => Self::Left,
                (Less, Less) => Self::LeftAbove,
                (Equal, Greater) => Self::Below,
                (Equal, Equal) => Self::None,
                (Equal, Less) => Self::Above,
                (Greater, Greater) => Self::RightBelow,
                (Greater, Equal) => Self::Right,
                (Greater, Less) => Self::RightAbove,
            },
            None => Self::None,
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            Self::LeftBelow => '1',
            Self::Below => '2',
            Self::RightBelow => '3',
            Self::Left => '4',
            Self::None => '5',
            Self::Right => '6',
            Self::LeftAbove => '7',
            Self::Above => '8',
            Self::RightAbove => '9',
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Self::Left | Self::LeftAbove | Self::LeftBelow)
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Self::Right | Self::RightAbove | Self::RightBelow)
    }

    pub fn is_above(&self) -> bool {
        matches!(self, Self::Above | Self::LeftAbove | Self::RightAbove)
    }

    pub fn is_below(&self) -> bool {
        matches!(self, Self::Below | Self::LeftBelow | Self::RightBelow)
    }

    pub fn in_side(&self, axis: Axis, side: Side) -> bool {
        if Self::None.eq(self) {
            return true;
        }
        match (axis, side) {
            (Axis::Horizontal, Side::Front) => {
                self.is_left() || matches!(self, Self::Above | Self::Below)
            }
            (Axis::Horizontal, Side::Back) => {
                self.is_right() || matches!(self, Self::Above | Self::Below)
            }
            (Axis::Vertical, Side::Front) => {
                self.is_above() || matches!(self, Self::Left | Self::Right)
            }
            (Axis::Vertical, Side::Back) => {
                self.is_below() || matches!(self, Self::Left | Self::Right)
            }
        }
    }

    pub fn is_diagonal(&self) -> bool {
        match self {
            Self::LeftAbove | Self::LeftBelow | Self::RightAbove | Self::RightBelow => true,
            _ => false,
        }
    }

    pub fn is_block(&self, axis: Axis) -> bool {
        if self.is_diagonal() {
            return true;
        }

        match self {
            Self::Above | Self::Below if axis == Axis::Horizontal => true,
            Self::Left | Self::Right if axis == Axis::Vertical => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewElement {
    D(Direction),
    Vertical,
    Horizontal,
    DiagonalPad,
    DiagonalL,
    DiagonalR,
    DiagonalT,
    DiagonalB,
    DiagonalLA,
    DiagonalLB,
    DiagonalRA,
    DiagonalRB,
}

impl ViewElement {
    pub fn is_diagonal_padding(&self) -> bool {
        match self {
            ViewElement::DiagonalPad
            | ViewElement::DiagonalL
            | ViewElement::DiagonalR
            | ViewElement::DiagonalT
            | ViewElement::DiagonalB
            | ViewElement::DiagonalLA
            | ViewElement::DiagonalLB
            | ViewElement::DiagonalRA
            | ViewElement::DiagonalRB => true,
            _ => false,
        }
    }

    pub fn in_quadrant(&self, quadrant: usize, h: bool, v: bool) -> bool {
        use Direction::*;

        if *self == Self::DiagonalPad {
            return true;
        }
        if matches!(self, Self::D(None)) && (h || v) {
            return true;
        }

        match quadrant {
            1 => match self {
                Self::DiagonalRA | Self::D(RightAbove) => true,
                Self::DiagonalT | Self::DiagonalB | Self::D(Right) | Self::Horizontal => h,
                Self::DiagonalL | Self::DiagonalR | Self::D(Above) | Self::Vertical => v,
                _ => false,
            },
            2 => match self {
                Self::DiagonalLA | Self::D(LeftAbove) => true,
                Self::DiagonalT | Self::DiagonalB | Self::D(Left) | Self::Horizontal => h,
                Self::DiagonalL | Self::DiagonalR | Self::D(Above) | Self::Vertical => v,
                _ => false,
            },
            3 => match self {
                Self::DiagonalLB | Self::D(LeftBelow) => true,
                Self::DiagonalT | Self::DiagonalB | Self::D(Left) | Self::Horizontal => h,
                Self::DiagonalL | Self::DiagonalR | Self::D(Below) | Self::Vertical => v,
                _ => false,
            },
            4 => match self {
                Self::DiagonalRB | Self::D(RightBelow) => true,
                Self::DiagonalT | Self::DiagonalB | Self::D(Right) | Self::Horizontal => h,
                Self::DiagonalL | Self::DiagonalR | Self::D(Below) | Self::Vertical => v,
                _ => false,
            },
            _ => panic!("Invalid quadrant {quadrant}!"),
        }
    }

    pub fn in_side(&self, axis: Axis, side: Side) -> bool {
        match self {
            Self::D(d) => d.in_side(axis, side),
            Self::Horizontal | Self::Vertical | Self::DiagonalPad => true,
            Self::DiagonalL => axis == Axis::Vertical || side == Side::Back,
            Self::DiagonalR => axis == Axis::Vertical || side == Side::Front,
            Self::DiagonalT => axis == Axis::Horizontal || side == Side::Back,
            Self::DiagonalB => axis == Axis::Horizontal || side == Side::Front,
            Self::DiagonalLB => matches!(
                (axis, side),
                (Axis::Horizontal, Side::Front) | (Axis::Vertical, Side::Back)
            ),
            Self::DiagonalLA => matches!(
                (axis, side),
                (Axis::Horizontal, Side::Front) | (Axis::Vertical, Side::Front)
            ),
            Self::DiagonalRB => matches!(
                (axis, side),
                (Axis::Horizontal, Side::Back) | (Axis::Vertical, Side::Back)
            ),
            Self::DiagonalRA => matches!(
                (axis, side),
                (Axis::Horizontal, Side::Back) | (Axis::Vertical, Side::Front)
            ),
        }
    }

    pub fn is_block(&self, axis: Axis) -> bool {
        match self {
            Self::D(d) => d.is_block(axis),
            Self::DiagonalLA | Self::DiagonalLB | Self::DiagonalRA | Self::DiagonalRB => true,
            Self::Vertical | Self::DiagonalL | Self::DiagonalR if axis == Axis::Horizontal => true,
            Self::Horizontal | Self::DiagonalB | Self::DiagonalT if axis == Axis::Vertical => true,
            _ => false,
        }
    }
}

#[derive(Clone, Default)]
pub struct StrucView {
    data: Vec<Vec<Vec<ViewElement>>>,
    pub allocs: DataHV<Vec<usize>>,
}

impl StrucView {
    pub fn new(struc: &StrucProto) -> Self {
        if struc.is_empty() {
            return Self::default();
        }

        let values = struc.values_map(true);

        let mut view: Vec<Vec<Vec<ViewElement>>> =
            vec![
                vec![vec![]; *values.h.last_key_value().unwrap().1 + 1];
                *values.v.last_key_value().unwrap().1 + 1
            ];

        struc
            .paths
            .iter()
            .filter(|path| !path.kpoints.is_empty() && !path.hide)
            .for_each(|path| {
                let mut iter = path
                    .kpoints
                    .iter()
                    .filter_map(|kp| if kp.is_mark() { None } else { Some(kp.pos) })
                    .map(|p| IndexPoint::new(values.h[&p.x], values.v[&p.y]));

                {
                    let mut iter = iter.clone();
                    let head = iter.next().unwrap();
                    if iter.all(|p| p == head) {
                        view[head.y][head.x].push(ViewElement::D(Direction::None));
                        return;
                    }
                }

                let mut pre: Option<IndexPoint> = None;
                let mut cur = iter.next();
                let mut next = iter.next();

                while let Some(kp) = cur {
                    [(kp, pre), (kp, next)]
                        .into_iter()
                        .enumerate()
                        .for_each(|(i, (from, to))| match Direction::new(from, to) {
                            Direction::None => {}
                            dir => {
                                let to = to.unwrap();
                                view[from.y][from.x].push(ViewElement::D(dir));

                                if i == 1 {
                                    let p1 = to.min(from);
                                    let p2 = to.max(from);

                                    if dir == Direction::Left || dir == Direction::Right {
                                        for x in p1.x + 1..p2.x {
                                            view[p1.y][x].push(ViewElement::Horizontal)
                                        }
                                    } else if dir == Direction::Above || dir == Direction::Below {
                                        for y in p1.y + 1..p2.y {
                                            view[y][p1.x].push(ViewElement::Vertical)
                                        }
                                    } else {
                                        for y in p1.y..=p2.y {
                                            for x in p1.x..=p2.x {
                                                let padding = match (
                                                    p1.x == x,
                                                    p2.x == x,
                                                    p1.y == y,
                                                    p2.y == y,
                                                ) {
                                                    (false, false, false, false) => {
                                                        ViewElement::DiagonalPad
                                                    }
                                                    (true, false, false, false) => {
                                                        ViewElement::DiagonalL
                                                    }
                                                    (false, true, false, false) => {
                                                        ViewElement::DiagonalR
                                                    }
                                                    (false, false, true, false) => {
                                                        ViewElement::DiagonalT
                                                    }
                                                    (false, false, false, true) => {
                                                        ViewElement::DiagonalB
                                                    }
                                                    (true, false, true, false) => {
                                                        ViewElement::DiagonalRB
                                                    }
                                                    (true, false, false, true) => {
                                                        ViewElement::DiagonalRA
                                                    }
                                                    (false, true, true, false) => {
                                                        ViewElement::DiagonalLB
                                                    }
                                                    (false, true, false, true) => {
                                                        ViewElement::DiagonalLA
                                                    }
                                                    _ => unreachable!(),
                                                };

                                                let p = IndexPoint::new(x, y);
                                                if p != to && p != from {
                                                    view[y][x].push(padding);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });

                    pre = Some(kp);
                    cur = next;
                    next = iter.next();
                }
            });

        Self {
            data: view,
            allocs: struc.allocation_space(),
        }
    }

    pub fn struc_size(&self) -> DataHV<usize> {
        DataHV::new(self.data[0].len(), self.data.len())
    }

    pub fn space_size(&self) -> DataHV<usize> {
        Axis::hv().into_map(|axis| self.allocs.hv_get(axis).iter().sum::<usize>())
    }

    pub fn space_length(&self, axis: Axis, range: std::ops::Range<usize>) -> usize {
        self.allocs.hv_get(axis)[range].iter().sum()
    }

    pub fn surround_area(&self, surround: DataHV<Section>) -> Option<DataHV<[usize; 2]>> {
        fn get_range(v1: usize, v2: usize) -> std::ops::Range<usize> {
            v1.min(v2)..v1.max(v2)
        }

        let view = &self.data;
        let size = self.struc_size();
        let indexes: DataHV<Vec<usize>> = Axis::hv().into_map(|axis| {
            let mut indexes: Vec<usize> = (0..*size.hv_get(axis)).collect();
            if surround.hv_get(axis).eq(&Section::Start) {
                indexes.reverse();
            }
            indexes
        });

        if size.iter().any(|&v| v < 2) {
            return None;
        }

        match surround
            .iter()
            .filter(|section| Section::Middle.eq(*section))
            .count()
        {
            0 => {
                if !view[indexes.v[0]][indexes.h[0]].is_empty() {
                    return None;
                }

                let mut max_j = usize::MAX;
                let size_list: Vec<(usize, usize)> = indexes
                    .v
                    .iter()
                    .enumerate()
                    .take(indexes.v.len() - 1)
                    .take_while(|&(_, &y)| view[y][indexes.h[0]].is_empty())
                    .map(|(i, &y)| {
                        let j = indexes
                            .h
                            .iter()
                            .enumerate()
                            .take(indexes.h.len() - 1)
                            .take_while(|&(_, &x)| view[y][x].is_empty())
                            .last()
                            .map(|(j, _)| j)
                            .unwrap()
                            + 1;
                        max_j = max_j.min(j);
                        (max_j, i + 1)
                    })
                    .collect();
                size_list
                    .iter()
                    .rev()
                    .max_by_key(|&&(x, y)| {
                        self.space_length(Axis::Horizontal, get_range(indexes.h[x], indexes.h[0]))
                            * self
                                .space_length(Axis::Vertical, get_range(indexes.v[y], indexes.v[0]))
                    })
                    .map(|&(x, y)| {
                        Some(DataHV::new(x, y).zip(Axis::hv()).into_map(|(i, axis)| {
                            match surround.hv_get(axis) {
                                Section::Start => {
                                    [indexes.hv_get(axis)[i], indexes.hv_get(axis)[0]]
                                }
                                _ => [0, indexes.hv_get(axis)[i]],
                            }
                        }))
                    })
                    .unwrap()
            }
            1 => {
                let mut start = 0;
                let mut pairs: Vec<(usize, usize)> = vec![];
                let (main_axis, main_indexes, sub_indexes) = match surround.h {
                    Section::Middle => (Axis::Horizontal, &indexes.h, &indexes.v),
                    _ => (Axis::Vertical, &indexes.v, &indexes.h),
                };
                let quadrant = match (main_axis.inverse(), surround.hv_get(main_axis.inverse())) {
                    (Axis::Vertical, Section::Start) => 1,
                    (Axis::Vertical, Section::End) => 4,
                    (Axis::Horizontal, Section::Start) => 3,
                    (Axis::Horizontal, Section::End) => 4,
                    _ => panic!(),
                };

                while start + 1 != main_indexes.len() {
                    let left = main_indexes[start..]
                        .iter()
                        .find(|&&i| {
                            self.get_in(main_axis, i, sub_indexes[0]).iter().all(|d| {
                                !d.in_quadrant(
                                    quadrant,
                                    main_axis == Axis::Horizontal,
                                    main_axis == Axis::Vertical,
                                )
                            })
                        })
                        .copied();
                    let right = left
                        .and_then(|l| {
                            main_indexes[l + 1..].iter().find(|&&i| {
                                i == *main_indexes.last().unwrap()
                                    || self
                                        .get_in(main_axis, i, sub_indexes[0])
                                        .iter()
                                        .any(|d| d.in_quadrant(quadrant, true, true))
                            })
                        })
                        .copied();

                    match left.zip(right) {
                        Some(p) => {
                            pairs.push(p);
                            start = p.1;
                        }
                        None => break,
                    }
                }

                let max_area = pairs
                    .into_iter()
                    .map(|(left, right)| {
                        let y0 = (left..right)
                            .map(|i| {
                                sub_indexes
                                    .iter()
                                    .take(sub_indexes.len().checked_sub(1).unwrap_or_default())
                                    .skip(1)
                                    .take_while(|&&j| {
                                        self.get_in(main_axis, i, j)
                                            .iter()
                                            .find(|d| {
                                                let mut check_axis = DataHV::splat(false);
                                                *check_axis.hv_get_mut(main_axis) = true;
                                                *check_axis.hv_get_mut(main_axis.inverse()) =
                                                    i != left && i != right;

                                                d.in_quadrant(
                                                    quadrant,
                                                    *check_axis.hv_get(Axis::Horizontal),
                                                    *check_axis.hv_get(Axis::Vertical),
                                                )
                                            })
                                            .is_none()
                                    })
                                    .count()
                                    + 1
                            })
                            .min()
                            .unwrap();

                        (
                            left,
                            right,
                            y0,
                            self.space_length(
                                main_axis,
                                get_range(main_indexes[right], main_indexes[left]),
                            ) * self.space_length(
                                main_axis.inverse(),
                                get_range(sub_indexes[y0], sub_indexes[0]),
                            ),
                        )
                    })
                    .max_by_key(|data| data.3);

                max_area.map(|(x1, x2, height, _)| {
                    let sub_area = if *surround.hv_get(main_axis.inverse()) == Section::Start {
                        [sub_indexes[0] - height, sub_indexes[0]]
                    } else {
                        [sub_indexes[0], sub_indexes[0] + height]
                    };
                    let mut r = DataHV::new([x1, x2], sub_area);
                    if main_axis == Axis::Vertical {
                        r.vh();
                    }
                    r
                })
            }
            2 => match indexes.iter().all(|i| i.len() > 1) {
                true => Some(indexes.map(|idxs| [0, *idxs.last().unwrap()])),
                false => None,
            },
            _ => unreachable!(),
        }
    }

    pub fn get_edge_in<R>(&self, axis: Axis, from: Side, main_axis: usize, cross_range: R) -> Edge
    where
        R: std::ops::RangeBounds<usize>,
    {
        // assert!(cross_range.start)
        let len = self.struc_size().hv_get_into(axis.inverse());
        let start = (0..len).find(|i| cross_range.contains(i)).unwrap();
        let end = (0..len).rev().find(|i| cross_range.contains(i)).unwrap();

        let allocs = &self.allocs.hv_get(axis.inverse())[start..end];
        let data: Box<dyn Iterator<Item = &Vec<ViewElement>>> = match axis {
            Axis::Horizontal => Box::new(
                self.data[start..=end]
                    .iter()
                    .map(move |col| &col[main_axis]),
            ),
            Axis::Vertical => Box::new(
                self.data[main_axis]
                    .iter()
                    .skip(start)
                    .take(end - start + 1),
            ),
        };

        let mut edge_data = Vec::with_capacity(allocs.iter().sum::<usize>() + 1);
        let (q1, q2) = match (axis, from) {
            (Axis::Horizontal, Side::Front) => (4, 1),
            (Axis::Horizontal, Side::Back) => (3, 2),
            (Axis::Vertical, Side::Front) => (4, 3),
            (Axis::Vertical, Side::Back) => (1, 2),
        };
        for (i, (eles, alloc)) in data
            .zip(allocs.iter().chain(std::iter::once(&1)))
            .enumerate()
        {
            let mut list: Vec<_> = eles
                .iter()
                .copied()
                .filter(|ele| ele.in_side(axis, from.inverse()))
                .collect();

            if i == 0 {
                list.retain(|ele| ele.in_quadrant(q1, true, true));
            }
            if i == allocs.len() {
                list.retain(|ele| ele.in_quadrant(q2, true, true));
            }

            edge_data.push(Some(list));
            edge_data.extend(vec![None; alloc - 1]);
        }
        Edge {
            axis,
            data: edge_data,
        }
    }

    pub fn get_edge_range<R>(&self, axis: Axis, from: Side, cross_range: R) -> Edge
    where
        R: std::ops::RangeBounds<usize>,
    {
        let main_axis = match axis {
            Axis::Horizontal => match from {
                Side::Front => 0,
                Side::Back => self.data[0].len() - 1,
            },
            Axis::Vertical => match from {
                Side::Front => 0,
                Side::Back => self.data.len() - 1,
            },
        };
        self.get_edge_in(axis, from, main_axis, cross_range)
    }

    pub fn get_edge(&self, axis: Axis, from: Side) -> Edge {
        let (main_axis, cross_range) = match axis {
            Axis::Horizontal => {
                let x = match from {
                    Side::Front => 0,
                    Side::Back => self.data[0].len() - 1,
                };
                (x, 0..self.data.len())
            }
            Axis::Vertical => {
                let y = match from {
                    Side::Front => 0,
                    Side::Back => self.data.len() - 1,
                };
                (y, 0..self.data[0].len())
            }
        };
        self.get_edge_in(axis, from, main_axis, cross_range)
    }

    pub fn get_complexity_in<R>(&self, axis: Axis, range: R) -> usize
    where
        R: std::ops::RangeBounds<usize>,
    {
        let size = self.struc_size();
        let main_len = *size.hv_get(axis);

        let start = match (0..main_len).find(|i| range.contains(i)) {
            Some(start) => start,
            None => return 0,
        };
        let end = (0..main_len).rev().find(|i| range.contains(i)).unwrap();
        if start == end {
            return 0;
        }

        let mut max = 1;
        for cross_idx in 0..*size.hv_get(axis.inverse()) {
            let mut count = 1;
            for main_idx in start + 1..=end {
                let list = self.get_in(axis, main_idx, cross_idx);
                if list.iter().any(|ele| ele.is_block(axis)) && main_idx != end {
                    count += 1;
                }
            }
            max = max.max(count)
        }
        max
    }

    pub fn get_complexity(&self, axis: Axis) -> usize {
        let size = self.struc_size();
        self.get_complexity_in(axis, 0..*size.hv_get(axis))
    }

    pub fn get_in(&self, axis: Axis, main: usize, cross: usize) -> &Vec<ViewElement> {
        match axis {
            Axis::Horizontal => &self.data[cross][main],
            Axis::Vertical => &self.data[main][cross],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surround_area_three() {
        use crate::base::*;

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([KeyPoint::new(1, 0), KeyPoint::new(1, 2)]),
                KeyPath::from([
                    KeyPoint::new(0, 4),
                    KeyPoint::new(0, 1),
                    KeyPoint::new(2, 1),
                    KeyPoint::new(2, 4),
                ]),
            ],
            attrs: Default::default(),
        };

        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Middle, Section::Start))
            .unwrap();
        assert_eq!(area.h, [0, 2]);
        assert_eq!(area.v, [2, 3]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([KeyPoint::new(1, 1), KeyPoint::new(4, 1)]),
                KeyPath::from([KeyPoint::new(3, 0), KeyPoint::new(3, 2)]),
                KeyPath::from([
                    KeyPoint::new(0, 4),
                    KeyPoint::new(1, 4),
                    KeyPoint::new(1, 2),
                    KeyPoint::new(4, 2),
                    KeyPoint::new(4, 4),
                    KeyPoint::new(5, 4),
                ]),
            ],
            attrs: Default::default(),
        };

        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Middle, Section::End))
            .unwrap();
        assert_eq!(area.h, [0, 2]);
        assert_eq!(area.v, [0, 1]);

        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Middle, Section::Start))
            .unwrap();
        assert_eq!(area.h, [1, 3]);
        assert_eq!(area.v, [2, 3]);

        let area = view
            .surround_area(DataHV::new(Section::Start, Section::Middle))
            .unwrap();
        assert_eq!(area.h, [3, 4]);
        assert_eq!(area.v, [0, 3]);

        let area = view
            .surround_area(DataHV::new(Section::End, Section::Middle))
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v, [0, 3]);
    }

    #[test]
    fn test_surround_area() {
        let width = 1;
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    KeyPoint::new(width + 1, 0),
                    KeyPoint::new(0, 0),
                    KeyPoint::new(0, 3),
                ]),
                KeyPath::from([KeyPoint::new(0, 1), KeyPoint::new(width, 1)]),
                KeyPath::from([KeyPoint::new(0, 2), KeyPoint::new(width, 2)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Start, Section::Start))
            .unwrap();
        assert_eq!(area.h, [1, 2]);
        assert_eq!(area.v, [0, 3]);

        let width = 2;
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    KeyPoint::new(width + 1, 0),
                    KeyPoint::new(0, 0),
                    KeyPoint::new(0, 3),
                ]),
                KeyPath::from([KeyPoint::new(0, 1), KeyPoint::new(width, 1)]),
                KeyPath::from([KeyPoint::new(0, 2), KeyPoint::new(width, 2)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Start, Section::Start))
            .unwrap();
        assert_eq!(area.h, [0, 2]);
        assert_eq!(area.v, [2, 3]);

        let width = 3;
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    KeyPoint::new(width + 1, 0),
                    KeyPoint::new(0, 0),
                    KeyPoint::new(0, 3),
                ]),
                KeyPath::from([KeyPoint::new(0, 1), KeyPoint::new(width, 1)]),
                KeyPath::from([KeyPoint::new(0, 2), KeyPoint::new(width, 2)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Start, Section::Start))
            .unwrap();
        assert_eq!(area.h, [0, 2]);
        assert_eq!(area.v, [2, 3]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([KeyPoint::new(0, 2), KeyPoint::new(1, 2)]),
                KeyPath::from([KeyPoint::new(1, 0), KeyPoint::new(1, 3)]),
                KeyPath::from([KeyPoint::new(1, 1), KeyPoint::new(3, 1)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let area = view
            .surround_area(DataHV::new(Section::Start, Section::Start))
            .unwrap();
        assert_eq!(area.h, [1, 2]);
        assert_eq!(area.v, [1, 3]);

        let area = view
            .surround_area(DataHV::new(Section::End, Section::Start))
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v, [2, 3]);
    }

    #[test]
    fn test_size() {
        let mut struc = StrucProto::from(vec![KeyPath::from([
            key_pos(1, 0),
            key_pos(5, 0),
            key_pos(5, 3),
            key_pos(0, 3),
        ])]);
        let view = StrucView::new(&struc);
        let size = view.struc_size();
        assert_eq!(size.v, 2);
        assert_eq!(size.h, 3);

        struc
            .attrs
            .set::<crate::combination::attrs::ReduceAlloc>(&DataHV::splat(vec![vec![0]]));
        assert!(struc.reduce(Axis::Horizontal, false));
        let view = StrucView::new(&struc);
        let size = view.struc_size();
        assert_eq!(size.v, 2);
        assert_eq!(size.h, 2);
    }

    #[test]
    fn test_view() {
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(2, 2)]),
            KeyPath::from([key_pos(1, 1), key_pos(1, 1)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.data[0][0], vec![ViewElement::D(Direction::RightBelow)]);
        assert_eq!(view.data[0][1], vec![ViewElement::DiagonalT]);
        assert_eq!(view.data[0][2], vec![ViewElement::DiagonalLB]);
        assert_eq!(view.data[1][0], vec![ViewElement::DiagonalL]);
        assert_eq!(
            view.data[1][1],
            vec![ViewElement::DiagonalPad, ViewElement::D(Direction::None)]
        );
        assert_eq!(view.data[1][2], vec![ViewElement::DiagonalR]);
        assert_eq!(view.data[2][0], vec![ViewElement::DiagonalRA]);
        assert_eq!(view.data[2][1], vec![ViewElement::DiagonalB]);
        assert_eq!(view.data[2][2], vec![ViewElement::D(Direction::LeftAbove)]);

        let mut struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 1), key_pos(2, 1)]),
            KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.data[0][1], vec![ViewElement::D(Direction::Below)]);
        assert_eq!(
            view.data[1][1],
            vec![ViewElement::Horizontal, ViewElement::Vertical]
        );
        assert_eq!(view.data[2][1], vec![ViewElement::D(Direction::Above)]);

        struc.paths[1].kpoints[1].labels.push("mark".to_string());

        let view = StrucView::new(&struc);
        assert_eq!(view.data[0][1], vec![ViewElement::D(Direction::None)]);
        assert_eq!(view.data[1][1], vec![ViewElement::Horizontal]);
        assert_eq!(view.data[2][1], vec![]);
        let size = view.struc_size();
        assert_eq!(size.v, 3);
        assert_eq!(size.h, 3);
    }

    #[test]
    fn test_edge() {
        let edge1 = Edge {
            axis: Axis::Horizontal,
            data: vec![
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![ViewElement::D(Direction::Above)]),
            ],
        };

        let mut edge = edge1.clone();
        edge.connect(edge1.clone(), 0);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![
                    ViewElement::D(Direction::Above),
                    ViewElement::D(Direction::Below)
                ]),
                Some(vec![ViewElement::D(Direction::Above)]),
            ]
        );

        let mut edge = edge1.clone();
        edge.connect(edge1.clone(), 1);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![ViewElement::D(Direction::Above)]),
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![ViewElement::D(Direction::Above)]),
            ]
        );

        let mut edge = edge1.clone();
        edge.connect(edge1.clone(), 2);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![ViewElement::D(Direction::Above)]),
                None,
                Some(vec![ViewElement::D(Direction::Below)]),
                Some(vec![ViewElement::D(Direction::Above)]),
            ]
        );
    }

    #[test]
    fn test_get_edge() {
        //  ----
        // -|--|-
        //  |  |
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 2), key_pos(1, 0), key_pos(3, 0), key_pos(3, 2)]),
            KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
        ]);
        let view = StrucView::new(&struc);

        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![]),
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                Some(vec![
                    ViewElement::D(Direction::Left),
                    ViewElement::D(Direction::Below)
                ]),
                Some(vec![]),
            ]
        );

        let edge = view.get_edge(Axis::Vertical, Side::Back);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![]),
                Some(vec![ViewElement::D(Direction::Above),]),
                None,
                Some(vec![ViewElement::D(Direction::Above)]),
                Some(vec![]),
            ]
        );

        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![]),
                Some(vec![ViewElement::D(Direction::Right),]),
                Some(vec![]),
            ]
        );

        let edge = view.get_edge(Axis::Horizontal, Side::Back);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![]),
                Some(vec![ViewElement::D(Direction::Left),]),
                Some(vec![]),
            ]
        );
    }

    #[test]
    fn test_complexity() {
        // |x|
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(0, 4)]),
            KeyPath::from([key_pos(1, 1), key_pos(2, 2)]),
            KeyPath::from([key_pos(2, 1), key_pos(1, 2)]),
            KeyPath::from([key_pos(4, 0), key_pos(4, 4)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.get_complexity_in(Axis::Horizontal, 0..=1), 1);
        assert_eq!(view.get_complexity_in(Axis::Horizontal, 1..), 2);
        assert_eq!(view.get_complexity(Axis::Horizontal), 3);
        assert_eq!(view.get_complexity(Axis::Vertical), 3);

        // x
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(4, 4)]),
            KeyPath::from([key_pos(0, 4), key_pos(4, 0)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.get_complexity(Axis::Horizontal), 1);
        assert_eq!(view.get_complexity(Axis::Vertical), 1);

        // 一
        let struc = StrucProto::from(vec![KeyPath::from([key_pos(0, 4), key_pos(4, 4)])]);
        let view = StrucView::new(&struc);
        assert_eq!(view.get_complexity(Axis::Horizontal), 1);
        assert_eq!(view.get_complexity(Axis::Vertical), 0);

        // 彐
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(4, 0), key_pos(4, 4)]),
            KeyPath::from([key_pos(0, 2), key_pos(4, 2)]),
            KeyPath::from([key_pos(0, 4), key_pos(4, 4)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.get_complexity(Axis::Horizontal), 1);
        assert_eq!(view.get_complexity(Axis::Vertical), 2);
    }

    #[test]
    fn test_shape() {
        // /\
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 1),
            key_pos(1, 0),
            key_pos(2, 1),
        ])]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::DiagonalRB]),
                Some(vec![
                    ViewElement::D(Direction::LeftBelow),
                    ViewElement::D(Direction::RightBelow)
                ]),
                Some(vec![ViewElement::DiagonalLB]),
            ]
        );
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [2, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Acute, ShapeTrend::Acute]);
        assert_eq!(shape.middle, ShapeState::Acute);
        let edge = view.get_edge(Axis::Vertical, Side::Back);
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [0, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Obtuse, ShapeTrend::Obtuse]);
        assert_eq!(shape.middle, ShapeState::Empty);

        // >
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 0),
            key_pos(1, 1),
            key_pos(0, 2),
        ])]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::RightBelow)]),
                Some(vec![ViewElement::DiagonalRA, ViewElement::DiagonalRB]),
                Some(vec![ViewElement::D(Direction::RightAbove)]),
            ]
        );
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [0, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Obtuse, ShapeTrend::Obtuse]);
        assert_eq!(shape.middle, ShapeState::Empty);
        let edge = view.get_edge(Axis::Horizontal, Side::Back);
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [2, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Acute, ShapeTrend::Acute]);
        assert_eq!(shape.middle, ShapeState::Acute);

        // 一
        let struc = StrucProto::from(vec![KeyPath::from([key_pos(0, 2), key_pos(4, 2)])]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(
            edge.data,
            vec![Some(vec![ViewElement::D(Direction::Right)]),]
        );
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [0, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        assert_eq!(shape.middle, ShapeState::Acute);

        // 彐
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(4, 0), key_pos(4, 4)]),
            KeyPath::from([key_pos(0, 2), key_pos(4, 2)]),
            KeyPath::from([key_pos(0, 4), key_pos(4, 4)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::Right)]),
                None,
                Some(vec![ViewElement::D(Direction::Right)]),
                None,
                Some(vec![ViewElement::D(Direction::Right)]),
            ]
        );
        let shape = edge.to_shape();
        assert_eq!(edge.data.len(), 5);
        assert_eq!(shape.blank, [0, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        assert_eq!(shape.middle, ShapeState::Sparse);

        //  ---
        // -|-|-
        //  | |
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 2), key_pos(1, 0), key_pos(3, 0), key_pos(3, 2)]),
            KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![]),
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                Some(vec![
                    ViewElement::D(Direction::Left),
                    ViewElement::D(Direction::Below)
                ]),
                Some(vec![]),
            ]
        );
        let shape = edge.to_shape();
        assert_eq!(edge.data.len(), 5);
        assert_eq!(shape.blank, [1, 1]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        assert_eq!(shape.middle, ShapeState::Dense);

        // -
        // -
        //  -
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(1, 0)]),
            KeyPath::from([key_pos(0, 2), key_pos(1, 2)]),
            KeyPath::from([key_pos(1, 4), key_pos(2, 4)]),
        ]);
        let view = StrucView::new(&struc);
        let shape = view.get_edge(Axis::Horizontal, Side::Front).to_shape();
        assert_eq!(shape.blank, [0, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        assert_eq!(shape.middle, ShapeState::Empty);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [2, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        assert_eq!(shape.middle, ShapeState::Acute);

        //  |
        // -|
        //  |-
        //  |
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 1), key_pos(1, 1)]),
            KeyPath::from([key_pos(1, 2), key_pos(2, 2)]),
            KeyPath::from([key_pos(1, 0), key_pos(1, 3)]),
        ]);
        let view = StrucView::new(&struc);
        let shape = view.get_edge(Axis::Horizontal, Side::Front).to_shape();
        assert_eq!(shape.blank, [1, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [2, 1]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);

        //  |
        //  |-
        // -|
        //  |-
        //  |
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 2), key_pos(2, 2)]),
            KeyPath::from([key_pos(2, 1), key_pos(4, 1)]),
            KeyPath::from([key_pos(2, 3), key_pos(4, 3)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 4)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(edge.data.len(), 5);
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [2, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [1, 1]);
        assert_eq!(shape.trend, [ShapeTrend::Right, ShapeTrend::Right]);

        // \ |
        //   | \
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(2, 2)]),
            KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Horizontal, Side::Front);
        assert_eq!(edge.data.len(), 3);
        let shape = edge.to_shape();
        assert_eq!(shape.blank, [0, 2]);
        assert_eq!(shape.trend, [ShapeTrend::Obtuse, ShapeTrend::Acute]);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [2, 0]);
        assert_eq!(shape.trend, [ShapeTrend::Acute, ShapeTrend::Obtuse]);
    }

    #[test]
    fn test_blank() {
        let edge = Edge {
            axis: Axis::Horizontal,
            data: vec![
                Some(vec![]),
                Some(vec![ViewElement::D(Direction::Below)]),
                None,
                Some(vec![ViewElement::D(Direction::Above)]),
            ],
        };
        assert!(edge.contain_black());
        assert!(edge.all_black(1, 3));
        assert!(!edge.all_black(0, 3));

        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(2, 2), key_pos(3, 2)]),
            KeyPath::from([key_pos(3, 0), key_pos(1, 2), key_pos(0, 2)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Back);
        assert!(!edge.all_black(0, 4));

        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(0, 1)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 1)]),
            KeyPath::from([key_pos(2, 0), key_pos(4, 0)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::Below)]),
                None,
                Some(vec![
                    ViewElement::D(Direction::Below),
                    ViewElement::D(Direction::Right)
                ]),
                None,
                Some(vec![ViewElement::D(Direction::Left)]),
            ]
        );
        assert!(!edge.all_black(0, 5));
        assert!(!edge.all_black(2, 5));
        assert!(edge.all_black(2, 3));
    }

    #[test]
    fn test_mark_area() {
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 2),
            key_pos(0, 0),
            key_pos(3, 0),
            key_pos(3, 2),
            KeyPoint {
                pos: IndexPoint::new(2, 2),
                labels: vec!["mark".to_string()],
            },
        ])]);
        let view = StrucView::new(&struc);
        assert_eq!(view.space_size().h, 3);
        assert_eq!(view.struc_size().h, 3);
        let area = view
            .surround_area(DataHV::new(Section::Middle, Section::Start))
            .unwrap();
        assert_eq!(area.v, [0, 1]);
        assert_eq!(area.h, [0, 2]);

        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(0, 0),
            KeyPoint {
                pos: IndexPoint::new(2, 2),
                labels: vec!["mark".to_string()],
            },
        ])]);
        let view = StrucView::new(&struc);
        assert_eq!(
            view.data[0],
            vec![vec![ViewElement::D(Direction::None)], vec![],]
        );
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert_eq!(
            edge.data,
            vec![
                Some(vec![ViewElement::D(Direction::None)]),
                None,
                Some(vec![]),
            ]
        )
    }
}
