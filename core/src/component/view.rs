use crate::{axis::*, component::struc::*, construct::space::*};

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

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
    Vertical,
    Horizontal,
    DiagonalTop,
    DiagonalBottom,
    DiagonalLeft,
    DiagonalRight,
    DiagonalLT,
    DiagonalLB,
    DiagonalRT,
    DiagonalRB,
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

    // (to.x, from.y), (from.x, to.y)
    pub fn new_diagonal_padding(from: IndexPoint, to: IndexPoint) -> (Self, Self) {
        match (from.x.cmp(&to.x), from.y.cmp(&to.y)) {
            (std::cmp::Ordering::Less, std::cmp::Ordering::Less) => {
                (Direction::DiagonalRT, Direction::DiagonalLB)
            }
            (std::cmp::Ordering::Greater, std::cmp::Ordering::Greater) => {
                (Direction::DiagonalLB, Direction::DiagonalRT)
            }
            (std::cmp::Ordering::Less, std::cmp::Ordering::Greater) => {
                (Direction::DiagonalRB, Direction::DiagonalLT)
            }
            (std::cmp::Ordering::Greater, std::cmp::Ordering::Less) => {
                (Direction::DiagonalLT, Direction::DiagonalRB)
            }
            _ => unreachable!(),
        }
    }

    pub fn in_here(self, axis: Axis, place: Place) -> bool {
        match axis {
            Axis::Horizontal => match place {
                Place::Start => match self {
                    Self::LeftAbove | Self::Left | Self::LeftBelow => true,
                    _ => false,
                },
                Place::End => match self {
                    Self::RightAbove | Self::Right | Self::RightBelow => true,
                    _ => false,
                },
                Place::Middle => unreachable!(),
            },
            Axis::Vertical => match place {
                Place::Start => match self {
                    Self::LeftAbove | Self::Above | Self::RightAbove => true,
                    _ => false,
                },
                Place::End => match self {
                    Self::LeftBelow | Self::Below | Self::RightBelow => true,
                    _ => false,
                },
                Place::Middle => unreachable!(),
            },
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
            Self::Vertical => 'v',
            Self::Horizontal => 'h',
            Self::DiagonalTop => 't',
            Self::DiagonalBottom => 'b',
            Self::DiagonalLeft => 'l',
            Self::DiagonalRight => 'r',
            Self::DiagonalLT => 'd',
            Self::DiagonalLB => 'd',
            Self::DiagonalRT => 'd',
            Self::DiagonalRB => 'd',
        }
    }

    pub fn is_padding(&self) -> bool {
        match self {
            Self::Vertical
            | Self::Horizontal
            | Self::DiagonalTop
            | Self::DiagonalBottom
            | Self::DiagonalLeft
            | Self::DiagonalRight
            | Self::DiagonalLT
            | Self::DiagonalLB
            | Self::DiagonalRT
            | Self::DiagonalRB => true,
            _ => false,
        }
    }

    pub fn is_diagonal_padding(&self) -> bool {
        match self {
            Self::DiagonalTop
            | Self::DiagonalBottom
            | Self::DiagonalLeft
            | Self::DiagonalRight
            | Self::DiagonalLT
            | Self::DiagonalLB
            | Self::DiagonalRT
            | Self::DiagonalRB => true,
            _ => false,
        }
    }

    pub fn is_diagonal_line(&self) -> bool {
        match self {
            Self::LeftAbove | Self::LeftBelow | Self::RightAbove | Self::RightBelow => true,
            _ => false,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        match self {
            Self::Left | Self::Right => true,
            _ => false,
        }
    }

    pub fn is_vertival(&self) -> bool {
        match self {
            Self::Above | Self::Below => true,
            _ => false,
        }
    }

    pub fn to_element_in(&self, axis: Axis, start: bool, end: bool) -> Option<Element> {
        if start {
            if *self == Self::Above || *self == Self::Right {
                return Some(Element::Dot);
            }
        } else if end {
            if *self == Self::Below || *self == Self::Left {
                return Some(Element::Dot);
            }
        }

        match self {
            Self::Above | Self::Below | Self::Vertical => match axis {
                Axis::Horizontal => Some(Element::Face),
                Axis::Vertical => Some(Element::Dot),
            },
            Self::Left | Self::Right | Self::Horizontal => match axis {
                Axis::Horizontal => Some(Element::Dot),
                Axis::Vertical => Some(Element::Face),
            },
            Self::LeftAbove | Self::LeftBelow | Self::RightAbove | Self::RightBelow => {
                Some(Element::Diagonal)
            }
            _ => None,
        }
    }

    pub fn is_none(&self) -> bool {
        Self::None.eq(self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum Element {
    Dot,
    Diagonal,
    Face,
}

#[derive(Debug, Clone, Default)]
pub struct Edge {
    pub line: Vec<Vec<Direction>>,
}

impl Edge {
    pub fn connect(self, other: Edge) -> Edge {
        Edge {
            line: self.line.into_iter().chain(other.line).collect(),
        }
    }

    pub fn to_elements(&self, axis: Axis) -> Vec<Vec<Element>> {
        self.line
            .iter()
            .enumerate()
            .map(|(i, space)| {
                space
                    .iter()
                    .filter_map(|gt| gt.to_element_in(axis, i == 0, i == self.line.len() - 1))
                    .collect()
            })
            .collect()
    }
}

#[derive(Clone)]
pub struct StrucView(Vec<Vec<Vec<Direction>>>);

impl Deref for StrucView {
    type Target = Vec<Vec<Vec<Direction>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for StrucView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl StrucView {
    pub fn new(struc: &StrucProto) -> Self {
        let values = struc.values_map();
        let mut view: Vec<Vec<Vec<Direction>>> =
            vec![
                vec![vec![]; values.h.values().max().map(|l| l + 1).unwrap_or_default()];
                values.v.values().max().map(|l| l + 1).unwrap_or_default()
            ];

        struc
            .paths
            .iter()
            .filter(|path| !path.hide)
            .for_each(|path| {
                let mut iter = path
                    .points
                    .iter()
                    .map(|kp| IndexPoint::new(values.h[&kp.x], values.v[&kp.y]));
                let mut pre = None;
                let mut cur = iter.next();
                let mut next = iter.next();

                while let Some(kp) = cur {
                    [(kp, pre), (kp, next)]
                        .into_iter()
                        .enumerate()
                        .for_each(|(i, (from, to))| match Direction::new(from, to) {
                            Direction::None => {
                                if i == 0 && to.is_none() {
                                    view[from.y][from.x].push(Direction::None);
                                }
                            }
                            dir => {
                                let from = from;
                                let to = to.unwrap();
                                view[from.y][from.x].push(dir);

                                if i == 1 {
                                    let p1 = to.min(from);
                                    let p2 = to.max(from);

                                    if dir.is_horizontal() {
                                        (p1.x + 1..p2.x).for_each(|x| {
                                            view[p1.y][x].push(Direction::Horizontal)
                                        });
                                    } else if dir.is_vertival() {
                                        (p1.y + 1..p2.y)
                                            .for_each(|y| view[y][p1.x].push(Direction::Vertical));
                                    } else {
                                        assert!(dir.is_diagonal_line());

                                        (p1.x + 1..p2.x).for_each(|x| {
                                            view[p1.y][x].push(Direction::DiagonalTop);
                                            view[p2.y][x].push(Direction::DiagonalBottom);
                                        });
                                        (p1.y + 1..p2.y).for_each(|y| {
                                            view[y][p1.x].push(Direction::DiagonalLeft);
                                            view[y][p2.x].push(Direction::DiagonalRight);
                                        });

                                        let (dir1, dir2) =
                                            Direction::new_diagonal_padding(from, to);
                                        view[from.y][to.x].push(dir1);
                                        view[to.y][from.x].push(dir2);
                                    }
                                }
                            }
                        });

                    pre = Some(kp);
                    cur = next;
                    next = iter.next();
                }
            });

        Self(view)
    }

    pub fn end_index(&self) -> DataHV<usize> {
        DataHV::new(self[0].len() - 1, self.len() - 1)
    }

    pub fn read_edge(&self, axis: Axis, place: Place) -> Edge {
        let ends = self.end_index();
        let segment = match place {
            Place::Start => 0,
            Place::End => *ends.hv_get(axis),
            _ => panic!("Incorrect reading edge in {:?}", place),
        };
        self.read_edge_in(axis, 0, *ends.hv_get(axis.inverse()), segment)
    }

    pub fn read_edge_in(&self, axis: Axis, start: usize, end: usize, segment: usize) -> Edge {
        let in_view = |i: usize, j: usize| match axis {
            Axis::Horizontal => &self[j][i],
            Axis::Vertical => &self[i][j],
        };
        let range = start..=end;

        let line = range
            .into_iter()
            .fold(Vec::with_capacity(end - start + 1), |mut line, n| {
                line.push(in_view(segment, n).clone());
                line
            });

        Edge { line }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view() {
        let mut struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(2, 1), IndexPoint::new(2, 5)]),
                KeyPath::from([IndexPoint::new(1, 2), IndexPoint::new(4, 2)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        assert_eq!(view.len(), 5);
        assert_eq!(view[0].len(), 4);
        assert_eq!(view.end_index(), DataHV::new(3, 4));

        struc
            .attrs
            .set::<crate::component::attrs::Allocs>(&DataHV::new(vec![0, 1], vec![1, 2]));
        let view = StrucView::new(&struc);
        assert_eq!(view.len(), 4);
        assert_eq!(view[0].len(), 2);
    }

    #[test]
    fn test_read_edge() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    IndexPoint::new(2, 1),
                    IndexPoint::new(2, 4),
                    IndexPoint::new(3, 5),
                    IndexPoint::new(4, 4),
                ]),
                KeyPath::from([IndexPoint::new(4, 1), IndexPoint::new(4, 4)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view.read_edge(Axis::Horizontal, Place::End).line;
        assert_eq!(
            edge,
            vec![
                vec![Direction::Below],
                vec![Direction::Vertical],
                vec![Direction::Vertical],
                vec![Direction::LeftBelow, Direction::Above],
                vec![Direction::DiagonalRB],
            ]
        );

        let edge = view.read_edge(Axis::Vertical, Place::End).line;
        assert_eq!(
            edge,
            vec![
                vec![Direction::DiagonalLB],
                vec![Direction::LeftAbove, Direction::RightAbove],
                vec![Direction::DiagonalRB],
            ]
        );
    }

    #[test]
    fn test_read_edge_in() {
        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 1), IndexPoint::new(3, 1)]),
                KeyPath::from([IndexPoint::new(1, 2), IndexPoint::new(1, 4)]),
                KeyPath::from([IndexPoint::new(1, 3), IndexPoint::new(3, 3)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view.read_edge_in(Axis::Horizontal, 0, 1, 0);
        assert_eq!(
            edge.line,
            vec![vec![Direction::Right], vec![Direction::Below],]
        );
        assert_eq!(
            edge.to_elements(Axis::Horizontal),
            vec![vec![Element::Dot], vec![Element::Dot],]
        );

        let edge = view.read_edge_in(Axis::Horizontal, 0, 2, 0);
        assert_eq!(
            edge.line,
            vec![
                vec![Direction::Right],
                vec![Direction::Below],
                vec![Direction::Vertical, Direction::Right],
            ]
        );
        assert_eq!(
            edge.to_elements(Axis::Horizontal),
            vec![
                vec![Element::Dot],
                vec![Element::Face],
                vec![Element::Face, Element::Dot],
            ]
        );
    }
}
