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

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StandardEdge {
    pub dots: [bool; 5],
    pub faces: [f32; 4],
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

pub struct ViewLines {
    pub l: Vec<[Vec<Direction>; 2]>,
    pub place: Place,
    pub axis: Axis,
    pub segment: usize,
}

impl ViewLines {
    const BACKSPACE_VAL: f32 = 0.333;

    pub fn add_gap(&mut self, place: Place) {
        match place {
            Place::Start => self.l.insert(0, Default::default()),
            Place::End => self.l.push(Default::default()),
            _ => panic!("Parameter cannot be {:?}", place),
        }
    }

    pub fn to_standard_edge(&self, dot_val: f32) -> StandardEdge {
        let ViewLines {
            l: lines,
            place,
            axis,
            segment,
        } = self;

        let i_end = lines.len() - 1;
        let (i_main, i_sub) = self.place_index();
        let mut faces = [0.0; 4];

        let inside = match place {
            Place::Start if *segment == i_end => None,
            Place::Start => Some(segment + 1),
            Place::End if *segment == 0 => None,
            Place::End => Some(segment - 1),
            _ => unreachable!(),
        };

        let mut dots_real = lines
            .iter()
            .fold(Vec::with_capacity(lines.len()), |mut list, line| {
                let b = line[i_main]
                    .iter()
                    .find(|d| !d.is_diagonal_padding())
                    .is_some();
                list.push(b);
                list
            });
        let mut dots = match dots_real.len() {
            0 => unreachable!(),
            1 => [false, false, dots_real[0], false, false],
            2 => [dots_real[0], false, false, false, dots_real[1]],
            3 => [dots_real[0], false, dots_real[1], false, dots_real[2]],
            4 => [
                dots_real[0],
                dots_real[1],
                false,
                dots_real[2],
                dots_real[3],
            ],
            5 => dots_real.clone().try_into().unwrap(),
            n => {
                if n & 1 == 0 {
                    [
                        dots_real[0],
                        dots_real[1],
                        false,
                        dots_real[n - 2],
                        dots_real[n - 1],
                    ]
                } else {
                    let median = n / 2;
                    [
                        dots_real[0],
                        dots_real[1],
                        dots_real[median],
                        dots_real[n - 2],
                        dots_real[n - 1],
                    ]
                }
            }
        };

        if i_end == 0 {
        } else if i_end < 4 {
            let iter = match i_end {
                1 => [(0, 1. / 8.), (0, 3. / 8.), (0, 5. / 8.), (0, 7. / 8.)],
                2 => [(0, 1. / 4.), (0, 3. / 4.), (1, 5. / 4.), (1, 7. / 4.)],
                3 => [(0, 0.5), (1, 1.25), (1, 1.75), (2, 2.5)],
                _ => unreachable!(),
            };
            for (i, (i_real, pos)) in iter.into_iter().enumerate() {
                let list1 = &lines[i_real][i_main];
                let list2 = &lines[i_real + 1][i_main];
                if Direction::is_face(&list1, &list2, *axis) {
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
                        let min_v = (*segment.min(&inside)) as f32;
                        let max_v = (*segment.max(&inside)) as f32;

                        match axis {
                            Axis::Horizontal => {
                                if (p1.x.min(p2.x)..=p1.x.max(p2.x)).contains(&inside) {
                                    let middle_x = (pos - y1) / (y1 - y2) * (x1 - x2) + x1;
                                    if (min_v..max_v).contains(&middle_x) {
                                        faces[i] += (middle_x - inside as f32).abs();
                                    }
                                }
                            }
                            Axis::Vertical => {
                                if (p1.y.min(p2.y)..=p1.y.max(p2.y)).contains(&inside) {
                                    let middle_y = (pos - x1) / (x1 - x2) * (y1 - y2) + y1;
                                    if (min_v..max_v).contains(&middle_y) {
                                        faces[i] += (middle_y - inside as f32).abs();
                                    }
                                }
                            }
                        }
                    });
                    if Direction::is_face(&lines[i_real][i_sub], &lines[i_real + 1][i_sub], *axis) {
                        faces[i] += Self::BACKSPACE_VAL;
                    }
                }
                faces[i] = faces[i].min(1.0);
            }
        } else {
            let mut dot_lines = std::collections::HashMap::from([
                (0, vec![0, 1]),
                (1, vec![1]),
                (i_end - 2, vec![3]),
                (i_end - 1, vec![3, 4]),
            ]);
            if i_end & 1 == 0 {
                let median = i_end / 2;
                dot_lines.insert(median - 1, vec![2]);
                dot_lines.insert(median, vec![2]);
            }

            for i in 0..i_end {
                let list1 = &lines[i][i_main];
                let list2 = &lines[i + 1][i_main];
                let to = if i == 0 {
                    vec![0]
                } else if i == i_end - 1 {
                    vec![3]
                } else {
                    let median = (i_end - 1) as f32 / 2.0;
                    match median.partial_cmp(&(i as f32)).unwrap() {
                        std::cmp::Ordering::Less => vec![2],
                        std::cmp::Ordering::Greater => vec![1],
                        std::cmp::Ordering::Equal => vec![1, 2],
                    }
                };

                if Direction::is_face(&list1, &list2, *axis) {
                    dots_real[i] = false;
                    dots_real[i + 1] = false;
                    if let Some(l) = dot_lines.get(&i) {
                        l.iter().for_each(|j| dots[*j] = false);
                    };
                    to.iter().for_each(|&j| faces[j] = 1.0 / to.len() as f32);
                } else {
                    if let Some(inside) = inside {
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
                            let min_v = (*segment.min(&inside)) as f32;
                            let max_v = (*segment.max(&inside)) as f32;

                            match axis {
                                Axis::Horizontal => {
                                    if (p1.x.min(p2.x)..=p1.x.max(p2.x)).contains(&inside) {
                                        let middle_x =
                                            ((i as f32 + 0.5) - y1) / (y1 - y2) * (x1 - x2) + x1;
                                        if (min_v..max_v).contains(&middle_x) {
                                            to.iter().for_each(|&j| {
                                                faces[j] += (middle_x - inside as f32).abs()
                                                    / to.len() as f32
                                            });
                                        }
                                    }
                                }
                                Axis::Vertical => {
                                    if (p1.y.min(p2.y)..=p1.y.max(p2.y)).contains(&inside) {
                                        let middle_y =
                                            ((i as f32 + 0.5) - x1) / (x1 - x2) * (y1 - y2) + y1;
                                        if (min_v..max_v).contains(&middle_y) {
                                            to.iter().for_each(|&j| {
                                                faces[j] += (middle_y - inside as f32).abs()
                                                    / to.len() as f32
                                            });
                                        }
                                    }
                                }
                            }
                        });
                        if Direction::is_face(&lines[i][i_sub], &lines[i + 1][i_sub], *axis) {
                            to.iter()
                                .for_each(|&j| faces[j] += Self::BACKSPACE_VAL / to.len() as f32);
                        }
                    }
                }
            }

            let weight = (i_end as f32 - 2.0) / 2.0;
            faces[1] /= weight;
            faces[2] /= weight;

            {
                let n = dots_real.len();
                if n & 1 == 0 {
                    faces[1] += dots_real[2..n / 2].iter().filter(|d| **d).count() as f32 * dot_val;
                    faces[2] +=
                        dots_real[n / 2..n - 2].iter().filter(|d| **d).count() as f32 * dot_val;
                } else {
                    let median = n / 2;
                    faces[1] +=
                        dots_real[2..median].iter().filter(|d| **d).count() as f32 * dot_val;
                    faces[2] += dots_real[median + 1..n - 2].iter().filter(|d| **d).count() as f32
                        * dot_val;
                }
            }

            faces.iter_mut().for_each(|val| *val = val.min(1.0));
        }

        StandardEdge { dots, faces }
    }

    pub fn to_edge(&self) -> Edge {
        let ViewLines {
            l: lines,
            place,
            axis,
            segment,
        } = self;

        let i_end = lines.len() - 1;
        let (i_main, i_sub) = self.place_index();

        let mut dots = Vec::with_capacity(lines.len());
        let mut faces = vec![0.0; i_end];

        let inside = match place {
            Place::Start if *segment == i_end => None,
            Place::Start => Some(segment + 1),
            Place::End if *segment == 0 => None,
            Place::End => Some(segment - 1),
            _ => unreachable!(),
        };

        lines.iter().for_each(|line| {
            let b = line[i_main]
                .iter()
                .find(|d| !d.is_diagonal_padding())
                .is_some();
            dots.push(b);
        });
        for i in 0..i_end {
            let list1 = &lines[i][i_main];
            let list2 = &lines[i + 1][i_main];
            if Direction::is_face(&list1, &list2, *axis) {
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
                    let min_v = (*segment.min(&inside)) as f32;
                    let max_v = (*segment.max(&inside)) as f32;

                    match axis {
                        Axis::Horizontal => {
                            if (p1.x.min(p2.x)..=p1.x.max(p2.x)).contains(&inside) {
                                let middle_x = ((i as f32 + 0.5) - y1) / (y1 - y2) * (x1 - x2) + x1;
                                if (min_v..max_v).contains(&middle_x) {
                                    faces[i] += (middle_x - inside as f32).abs();
                                }
                            }
                        }
                        Axis::Vertical => {
                            if (p1.y.min(p2.y)..=p1.y.max(p2.y)).contains(&inside) {
                                let middle_y = ((i as f32 + 0.5) - x1) / (x1 - x2) * (y1 - y2) + y1;
                                if (min_v..max_v).contains(&middle_y) {
                                    faces[i] += (middle_y - inside as f32).abs();
                                }
                            }
                        }
                    }
                });
                if Direction::is_face(&lines[i][i_sub], &lines[i + 1][i_sub], *axis) {
                    faces[i] += Self::BACKSPACE_VAL;
                }
            }

            faces[i] = faces[i].min(1.0);
        }

        Edge { dots, faces }
    }

    pub fn place_index(&self) -> (usize, usize) {
        match self.place {
            Place::Start => (0, 1),
            Place::End => (1, 0),
            _ => unreachable!(),
        }
    }

    pub fn backspace(&mut self) {
        let (i_main, _) = self.place_index();
        self.l.iter_mut().for_each(|lines| {
            lines.swap(0, 1);
            lines[i_main].clear();
        });
    }

    pub fn connect(&mut self, other: Self) {
        self.l.extend(other.l);
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
                    view[values.v[&path.points[0].y]][values.h[&path.points[0].x]]
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

    pub fn read_lines(&self, axis: Axis, place: Place) -> ViewLines {
        let size = self.size();
        let segment = match place {
            Place::Start => 0,
            Place::End => *size.hv_get(axis) - 1,
            Place::Middle => unreachable!(),
        };
        self.read_lines_in(axis, 0, *size.hv_get(axis.inverse()) - 1, segment, place)
    }

    pub fn read_lines_in(
        &self,
        axis: Axis,
        start: usize,
        end: usize,
        segment: usize,
        place: Place,
    ) -> ViewLines {
        let in_view = |i: usize, j: usize| match axis {
            Axis::Horizontal => &self[j][i],
            Axis::Vertical => &self[i][j],
        };

        let (i1, i2) = match place {
            Place::Start if segment + 1 == *self.size().hv_get(axis) => (Some(segment), None),
            Place::Start => (Some(segment), Some(segment + 1)),
            Place::End if segment == 0 => (None, Some(segment)),
            Place::End => (Some(segment - 1), Some(segment)),
            _ => unreachable!(),
        };

        let mut line: Vec<[Vec<_>; 2]> = Vec::with_capacity(end + 1 - start);
        for j in start..=end {
            line.push(Default::default());

            if let Some(i1) = i1 {
                line.last_mut().unwrap()[0].extend(in_view(i1, j).iter())
            }
            if let Some(i2) = i2 {
                line.last_mut().unwrap()[1].extend(in_view(i2, j).iter())
            }
        }

        ViewLines {
            l: line,
            place,
            axis,
            segment,
        }
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
                IndexPoint::new(0, 0),
                IndexPoint::new(1, 1),
            ])],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view.read_lines(Axis::Horizontal, Place::Start).to_edge();
        assert_eq!(edge.dots, [true, false]);
        assert_eq!(edge.faces, [0.5]);
        assert_eq!(edge.gray_scale(dot_val), dot_val + 0.5);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 1),
                IndexPoint::new(5, 1),
            ])],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view.read_lines(Axis::Horizontal, Place::Start).to_edge();
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

        let edge = view.read_lines(Axis::Vertical, Place::Start).to_edge();
        assert_eq!(edge.dots, vec![false; 5]);
        assert_eq!(edge.faces, vec![1.0; 4]);
        assert_eq!(edge.gray_scale(dot_val), 1.0);

        let edge = view.read_lines(Axis::Vertical, Place::End).to_edge();
        assert_eq!(edge.dots, vec![false; 5]);
        assert_eq!(edge.faces, vec![0.5; 4]);
        assert_eq!(edge.gray_scale(dot_val), 0.5);

        let edge = view.read_lines(Axis::Horizontal, Place::Start).to_edge();
        assert_eq!(edge.dots, vec![true, false, true, false]);
        assert_eq!(edge.faces, vec![0.0; 3]);
        assert_eq!(edge.gray_scale(dot_val), dot_val * 2.0);

        let edge = view.read_lines(Axis::Horizontal, Place::End).to_edge();
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

        let edge = view.read_lines(Axis::Vertical, Place::Start).to_edge();
        assert_eq!(edge.dots, vec![false; 3]);
        assert_eq!(edge.faces, vec![1.0, 0.0]);
        assert_eq!(edge.gray_scale(dot_val), 0.5);

        let mut lines = view.read_lines(Axis::Vertical, Place::Start);
        lines.backspace();
        let edge = lines.to_edge();
        assert_eq!(edge.dots, vec![false; 3]);
        assert_eq!(edge.faces, vec![0.5, 0.0]);
        assert_eq!(edge.gray_scale(dot_val), 0.25);

        let edge = view.read_lines(Axis::Vertical, Place::End).to_edge();
        assert_eq!(edge.dots, vec![false, true, true]);
        assert_eq!(edge.faces[0], 0.0);
        assert!(edge.faces[1] < 0.5);
        assert!(edge.gray_scale(dot_val) < dot_val * 2.0 + 0.25);

        let edge = view.read_lines(Axis::Horizontal, Place::Start).to_edge();
        assert_eq!(edge.dots, vec![true, false, false, false]);
        assert!(edge.faces[0] > 0.5);
        assert!(edge.faces[1] == 0.5);
        assert!(edge.faces[2] == 0.5);
        assert!(edge.gray_scale(dot_val) > dot_val + 0.5);

        let edge = view.read_lines(Axis::Horizontal, Place::End).to_edge();
        assert_eq!(edge.dots, vec![false, false, false, true]);
        assert!(edge.faces[2] > 0.5);
        assert!(edge.faces[1] == 0.5);
        assert!(edge.faces[0] == 0.0);
        assert!(edge.gray_scale(dot_val) > 1.0 / 3.0);
    }

    #[test]
    fn test_standard_edge() {
        let dot_val = 0.05;

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(0, 2), IndexPoint::new(7, 2)]),
                KeyPath::from([IndexPoint::new(0, 0), IndexPoint::new(0, 2)]),
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                KeyPath::from([IndexPoint::new(5, 0), IndexPoint::new(5, 2)]),
                KeyPath::from([
                    IndexPoint::new(2, 0),
                    IndexPoint::new(3, 0),
                    IndexPoint::new(3, 2),
                ]),
                KeyPath::from([IndexPoint::new(4, 0), IndexPoint::new(5, 1)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Vertical, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [true, true, false, false, false]);
        assert_eq!(edge.faces, [0.0, 0.4, 0.2 + dot_val * 2., 0.0]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(0, 2), IndexPoint::new(6, 2)]),
                KeyPath::from([IndexPoint::new(0, 0), IndexPoint::new(0, 2)]),
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                KeyPath::from([
                    IndexPoint::new(2, 0),
                    IndexPoint::new(3, 0),
                    IndexPoint::new(3, 2),
                ]),
                KeyPath::from([IndexPoint::new(4, 0), IndexPoint::new(3, 1)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Vertical, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [true, true, false, false, false]);
        assert_eq!(edge.faces, [0.0, 0.5, 0.25 + dot_val, 0.0]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(2, 2)]),
                KeyPath::from([IndexPoint::new(0, 2), IndexPoint::new(3, 2)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Vertical, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [false, true, false, false, false]);
        assert_eq!(edge.faces, [0.0, 0.5, 0.0, 0.0]);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 0),
                IndexPoint::new(2, 0),
            ])],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Horizontal, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [false, false, true, false, false]);
        assert_eq!(edge.faces, [0.0, 0.0, 0.0, 0.0]);

        let struc = StrucProto {
            paths: vec![
                KeyPath::from([IndexPoint::new(1, 0), IndexPoint::new(1, 2)]),
                KeyPath::from([IndexPoint::new(0, 1), IndexPoint::new(2, 1)]),
            ],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Vertical, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [false, false, true, false, false]);
        assert_eq!(edge.faces, [0.5, 0.5, 0.5, 0.5]);

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 0),
                IndexPoint::new(1, 1),
            ])],
            attrs: Default::default(),
        };
        let view = StrucView::new(&struc);
        let edge = view
            .read_lines(Axis::Vertical, Place::Start)
            .to_standard_edge(dot_val);
        assert_eq!(edge.dots, [true, false, false, false, false]);
        assert_eq!(edge.faces, [7. / 8., 5. / 8., 3. / 8., 1. / 8.]);
    }
}
