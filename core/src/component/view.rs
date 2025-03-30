use crate::{axis::*, component::struc::*, construct::space::*};

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
    Diagonal,
    DiagonalSide { from: IndexPoint, to: IndexPoint },
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
            Self::Vertical => 'v',
            Self::Horizontal => 'h',
            Self::Diagonal => 't',
            Self::DiagonalSide { .. } => 't',
        }
    }

    pub fn is_diagonal_padding(&self) -> bool {
        match self {
            Self::Diagonal | Self::DiagonalSide { .. } => true,
            _ => false,
        }
    }

    pub fn is_face(list1: &Vec<Self>, list2: &Vec<Self>, axis: Axis) -> bool {
        match axis {
            Axis::Horizontal => {
                let r1 = list1
                    .iter()
                    .find(|&&d| d == Self::Below || d == Self::Vertical)
                    .is_some();
                let r2 = list2
                    .iter()
                    .find(|&&d| d == Self::Above || d == Self::Vertical)
                    .is_some();
                r1 | r2
            }
            Axis::Vertical => {
                let r1 = list1
                    .iter()
                    .find(|&&d| d == Self::Right || d == Self::Horizontal)
                    .is_some();
                let r2 = list2
                    .iter()
                    .find(|&&d| d == Self::Left || d == Self::Horizontal)
                    .is_some();
                r1 | r2
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Edge {
    pub dots: Vec<bool>,
    pub faces: Vec<f32>,
}

impl Edge {
    pub fn gray_scale(&self, dot_val: f32) -> f32 {
        let face_val = if self.faces.is_empty() {
            0.0
        } else {
            self.faces.iter().sum::<f32>() / self.faces.len() as f32
        };
        face_val + self.dots.iter().filter(|b| **b).count() as f32 * dot_val
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
            .filter(|path| !path.hide || path.points.is_empty())
            .for_each(|path| {
                if path.points.iter().all(|p| path.points[0].eq(p)) {
                    view[values.h[&path.points[0].x]][values.v[&path.points[0].y]]
                        .push(Direction::None);
                }

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
                            dir if dir != Direction::None => {
                                let to = to.unwrap();
                                view[from.y][from.x].push(dir);

                                if i == 1 {
                                    let p1 = to.min(from);
                                    let p2 = to.max(from);

                                    if dir == Direction::Left || dir == Direction::Right {
                                        for x in p1.x + 1..p2.x {
                                            view[p1.y][x].push(Direction::Horizontal)
                                        }
                                    } else if dir == Direction::Above || dir == Direction::Below {
                                        for y in p1.y + 1..p2.y {
                                            view[y][p1.x].push(Direction::Vertical)
                                        }
                                    } else {
                                        for y in p1.y + 1..p2.y {
                                            for x in p1.x + 1..p2.x {
                                                view[y][x].push(Direction::Diagonal);
                                            }
                                        }

                                        let padding = Direction::DiagonalSide { from, to };
                                        for y in p1.y..p2.y {
                                            view[y][p1.x].push(padding);
                                            view[y][p2.x].push(padding);
                                        }
                                        for x in p1.x..p2.x {
                                            view[p1.y][x].push(padding);
                                            view[p2.y][x].push(padding);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        });

                    pre = Some(kp);
                    cur = next;
                    next = iter.next();
                }
            });

        Self(view)
    }

    pub fn size(&self) -> DataHV<usize> {
        DataHV::new(self[0].len(), self.len())
    }

    pub fn read_edge(&self, axis: Axis, place: Place) -> Edge {
        let size = self.size();
        let segment = match place {
            Place::Start => 0,
            Place::End => *size.hv_get(axis) - 1,
            Place::Middle => unreachable!(),
        };
        self.read_edge_in(axis, 0, *size.hv_get(axis.inverse()) - 1, segment, place)
    }

    pub fn read_edge_in(
        &self,
        axis: Axis,
        start: usize,
        end: usize,
        segment: usize,
        place: Place,
    ) -> Edge {
        let in_view = |i: usize, j: usize| match axis {
            Axis::Horizontal => &self[j][i],
            Axis::Vertical => &self[i][j],
        };
        let axis_end = self.size().hv_get(axis) - 1;

        let inside = match place {
            Place::Start if segment == axis_end => None,
            Place::Start => Some(segment + 1),
            Place::End if segment == 0 => None,
            Place::End => Some(segment - 1),
            _ => unreachable!(),
        };

        let mut dots = Vec::with_capacity(end + 1);
        let mut faces = vec![0.0; end];

        if start == end {
            let here = in_view(segment, start);
            dots.push(!here.is_empty() && here.iter().all(|d| !d.is_diagonal_padding()));
        } else {
            (start..=end).for_each(|i| {
                let b = in_view(segment, i)
                    .iter()
                    .find(|d| !d.is_diagonal_padding())
                    .is_some();
                dots.push(b);
            });
            for i in start..end {
                let list1 = in_view(segment, i);
                let list2 = in_view(segment, i + 1);
                if Direction::is_face(list1, list2, axis) {
                    dots[i] = false;
                    dots[i + 1] = false;
                    faces[i] = 1.0;
                } else if let Some(inside) = inside {
                    let diagonals: std::collections::HashSet<_> = list1
                        .iter()
                        .chain(list2.iter())
                        .filter_map(|&d| match d {
                            Direction::DiagonalSide { from, to } => Some((from, to)),
                            _ => None,
                        })
                        .collect();
                    diagonals.into_iter().for_each(|(p1, p2)| {
                        let x1 = p1.x as f32;
                        let x2 = p2.x as f32;
                        let y1 = p1.y as f32;
                        let y2 = p2.y as f32;
                        let min_v = segment.min(inside) as f32;
                        let max_v = segment.max(inside) as f32;

                        match axis {
                            Axis::Horizontal => {
                                if (p1.x.min(p2.x)..p1.x.max(p2.x)).contains(&inside) {
                                    let middle_x =
                                        ((i as f32 + 0.5) - y1) / (y1 - y2) * (x1 - x2) + x1;
                                    if (min_v..max_v).contains(&middle_x) {
                                        faces[i] += (middle_x - inside as f32).abs();
                                    }
                                }
                            }
                            Axis::Vertical => {
                                if (p1.y.min(p2.y)..p1.y.max(p2.y)).contains(&inside) {
                                    let middle_y =
                                        ((i as f32 + 0.5) - x1) / (x1 - x2) * (y1 - y2) + y1;
                                    if (min_v..max_v).contains(&middle_y) {
                                        faces[i] += (middle_y - inside as f32).abs();
                                    }
                                }
                            }
                        }
                    });
                    if !(dots[i] | dots[i + 1])
                        && Direction::is_face(in_view(inside, i), in_view(inside, i + 1), axis)
                    {
                        faces[i] += 0.5;
                    }
                }

                faces[i] = faces[i].min(1.0);
            }
        }

        Edge { dots, faces }
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
        assert_eq!(view.size(), DataHV::new(4, 5));

        struc
            .attrs
            .set::<crate::component::attrs::Allocs>(&DataHV::new(vec![0, 1], vec![1, 2]));
        let view = StrucView::new(&struc);
        assert_eq!(view.len(), 4);
        assert_eq!(view[0].len(), 2);
    }

    #[test]
    fn test_read_edge() {
        let dot_val = 0.05;

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 1),
                IndexPoint::new(5, 1),
            ])],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);

        let edge = view.read_edge(Axis::Horizontal, Place::Start);
        assert_eq!(edge.gray_scale(dot_val), dot_val);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    IndexPoint::new(1, 1),
                    IndexPoint::new(5, 1),
                    IndexPoint::new(5, 3),
                    IndexPoint::new(1, 3),
                ]),
                KeyPath {
                    points: vec![IndexPoint::new(3, 3), IndexPoint::new(3, 4)],
                    hide: true,
                },
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);

        let edge = view.read_edge(Axis::Vertical, Place::Start);
        assert_eq!(edge.dots, vec![false; 5]);
        assert_eq!(edge.faces, vec![1.0; 4]);
        assert_eq!(edge.gray_scale(dot_val), 1.0);

        let edge = view.read_edge(Axis::Vertical, Place::End);
        assert_eq!(edge.dots, vec![false; 5]);
        assert_eq!(edge.faces, vec![0.5; 4]);
        assert_eq!(edge.gray_scale(dot_val), 0.5);

        let edge = view.read_edge(Axis::Horizontal, Place::Start);
        assert_eq!(edge.dots, vec![true, false, true, false]);
        assert_eq!(edge.faces, vec![0.0; 3]);
        assert_eq!(edge.gray_scale(dot_val), dot_val * 2.0);

        let edge = view.read_edge(Axis::Horizontal, Place::End);
        assert_eq!(edge.dots, vec![false; 4]);
        assert_eq!(edge.faces, vec![1.0, 1.0, 0.0]);
        assert_eq!(edge.gray_scale(dot_val), 2.0 / 3.0);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([
                    IndexPoint::new(2, 1),
                    IndexPoint::new(1, 1),
                    IndexPoint::new(3, 4),
                ]),
                KeyPath::from([IndexPoint::new(2, 2), IndexPoint::new(2, 4)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);

        let edge = view.read_edge(Axis::Vertical, Place::Start);
        assert_eq!(edge.dots, vec![false; 3]);
        assert_eq!(edge.faces, vec![1.0, 0.0]);
        assert_eq!(edge.gray_scale(dot_val), 0.5);

        let edge = view.read_edge(Axis::Vertical, Place::End);
        assert_eq!(edge.dots, vec![false, true, true]);
        assert_eq!(edge.faces[0], 0.0);
        assert!(edge.faces[1] < 0.5);
        assert!(edge.gray_scale(dot_val) < dot_val * 2.0 + 0.25);

        let edge = view.read_edge(Axis::Horizontal, Place::Start);
        assert_eq!(edge.dots, vec![true, false, false, false]);
        assert!(edge.faces[0] > 0.5);
        assert!(edge.faces[1] == 0.5);
        assert!(edge.faces[2] == 0.5);
        assert!(edge.gray_scale(dot_val) > dot_val + 0.5);

        let edge = view.read_edge(Axis::Horizontal, Place::End);
        assert_eq!(edge.dots, vec![false, false, false, true]);
        assert!(edge.faces[2] > 0.5);
        assert!(edge.faces[1] == 0.5);
        assert!(edge.faces[0] == 0.0);
        assert!(edge.gray_scale(dot_val) > 1.0 / 3.0);
    }
}
