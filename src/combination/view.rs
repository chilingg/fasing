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
    pub fn contain_black(&self) -> bool {
        use Direction::*;
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
        for (i, eles) in self
            .data
            .iter()
            .enumerate()
            .skip(start)
            .take(n)
            .filter_map(|(i, eles)| eles.as_ref().map(|eles| (i, eles)))
        {
            if !eles.iter().any(|ele| match self.axis {
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
            }) {
                return false;
            }
        }
        return true;
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

    pub fn add(&mut self) {
        self.data.push(None);
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

        let mut b1 = if status.is_empty() {
            ShapeTrend::Square
        } else if status[0] == 0 {
            ShapeTrend::None
        } else {
            match self.axis {
                Axis::Horizontal
                    if self.data[status[0]].as_ref().unwrap().iter().any(|ele| {
                        matches!(
                            ele,
                            ViewElement::D(Direction::LeftAbove)
                                | ViewElement::D(Direction::RightAbove)
                        )
                    }) =>
                {
                    ShapeTrend::Triangle
                }
                Axis::Vertical
                    if self.data[status[0]].as_ref().unwrap().iter().any(|ele| {
                        matches!(
                            ele,
                            ViewElement::D(Direction::LeftAbove)
                                | ViewElement::D(Direction::LeftBelow)
                        )
                    }) =>
                {
                    ShapeTrend::Triangle
                }
                _ => ShapeTrend::Square,
            }
        };

        let last = *status.last().unwrap();
        let mut b2 = if status.is_empty() {
            ShapeTrend::Square
        } else if status.last().unwrap() + 1 == self.data.len() {
            ShapeTrend::None
        } else {
            match self.axis {
                Axis::Horizontal
                    if self.data[last].as_ref().unwrap().iter().any(|ele| {
                        matches!(
                            ele,
                            ViewElement::D(Direction::LeftBelow)
                                | ViewElement::D(Direction::RightBelow)
                        )
                    }) =>
                {
                    ShapeTrend::Triangle
                }
                Axis::Vertical
                    if self.data[last].as_ref().unwrap().iter().any(|ele| {
                        matches!(
                            ele,
                            ViewElement::D(Direction::RightAbove)
                                | ViewElement::D(Direction::RightBelow)
                        )
                    }) =>
                {
                    ShapeTrend::Triangle
                }
                _ => ShapeTrend::Square,
            }
        };

        if self.data.len() > 1 {
            let median = self.data.len() / 2;
            if status[0] >= median {
                b1 = match b1 {
                    ShapeTrend::Square => ShapeTrend::SquareLarg,
                    ShapeTrend::Triangle => ShapeTrend::TriangleLarg,
                    _ => b1,
                }
            }
            let median = self.data.len() / 2 - (self.data.len() + 1) % 2;
            if last <= median {
                b2 = match b2 {
                    ShapeTrend::Square => ShapeTrend::SquareLarg,
                    ShapeTrend::Triangle => ShapeTrend::TriangleLarg,
                    _ => b2,
                }
            }
        }

        let middle = if self.contain_black() {
            match self.all_black(status[0], status.len()) {
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
    Triangle,
    TriangleLarg,
    Square,
    SquareLarg,
    None,
}

#[derive(Clone, Debug)]
pub struct EdgeShape {
    pub blank: [ShapeTrend; 2],
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
}

#[derive(Clone, Default)]
pub struct StrucView {
    data: Vec<Vec<Vec<ViewElement>>>,
    allocs: DataHV<Vec<usize>>,
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
            .filter(|path| !path.kpoints.is_empty())
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

    pub fn get_edge(&self, axis: Axis, side: Side) -> Edge {
        let data: Box<dyn Iterator<Item = Vec<ViewElement>>> = match axis {
            Axis::Horizontal => {
                let x = match side {
                    Side::Front => 0,
                    Side::Back => self.data[0].len() - 1,
                };
                Box::new(self.data.iter().map(move |col| col[x].clone()))
            }
            Axis::Vertical => match side {
                Side::Front => Box::new(self.data[0].iter().cloned()),
                Side::Back => Box::new(self.data.last().unwrap().iter().cloned()),
            },
        };
        let mut edge_data = Vec::with_capacity(self.space_size().hv_get(axis) + 1);
        for (eles, alloc) in data.zip(
            self.allocs
                .hv_get(axis.inverse())
                .iter()
                .chain(std::iter::once(&1)),
        ) {
            edge_data.push(Some(eles));
            edge_data.extend(vec![None; alloc - 1]);
        }
        Edge {
            axis,
            data: edge_data,
        }
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
    fn test_shape() {
        //  ---
        // -|-|-
        //  | |
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 2), key_pos(1, 0), key_pos(3, 0), key_pos(3, 2)]),
            KeyPath::from([key_pos(0, 1), key_pos(4, 1)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        let shape = edge.to_shape();
        assert_eq!(edge.data.len(), 5);
        assert_eq!(shape.blank, [ShapeTrend::Square, ShapeTrend::Square]);
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
        assert_eq!(shape.blank, [ShapeTrend::None, ShapeTrend::SquareLarg]);
        assert_eq!(shape.middle, ShapeState::Empty);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [ShapeTrend::SquareLarg, ShapeTrend::None]);
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
        assert_eq!(shape.blank, [ShapeTrend::Square, ShapeTrend::SquareLarg]);
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [ShapeTrend::SquareLarg, ShapeTrend::Square]);

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
        assert_eq!(
            shape.blank,
            [ShapeTrend::SquareLarg, ShapeTrend::SquareLarg]
        );
        let shape = view.get_edge(Axis::Horizontal, Side::Back).to_shape();
        assert_eq!(shape.blank, [ShapeTrend::Square, ShapeTrend::Square]);
    }

    #[test]
    fn test_blank() {
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 0), key_pos(0, 1)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 1)]),
            KeyPath::from([key_pos(2, 0), key_pos(4, 0)]),
        ]);
        let view = StrucView::new(&struc);
        let edge = view.get_edge(Axis::Vertical, Side::Front);
        assert!(!edge.all_black(0, 5));
        assert!(edge.all_black(2, 5));
    }
}
