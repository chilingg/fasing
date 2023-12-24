use crate::{algorithm, axis::*, component::struc::*, construct::space::*};
use serde::{Deserialize, Serialize};

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
    DiagonalLeft,
    DiagonalRight,
    DiagonalAbove,
    DiagonalBelow,
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
            Self::DiagonalLeft => 'l',
            Self::DiagonalRight => 'r',
            Self::DiagonalAbove => 'a',
            Self::DiagonalBelow => 'b',
        }
    }

    pub fn padding_in(&self, axis: Axis, place: Place) -> Self {
        match (axis, place, self) {
            (Axis::Horizontal, _, Self::Left) | (Axis::Horizontal, _, Self::Right) => {
                Self::Horizontal
            }
            (Axis::Vertical, _, Self::Above) | (Axis::Vertical, _, Self::Below) => Self::Vertical,
            (Axis::Horizontal, Place::Start, d) if d.is_diagonal() => Self::DiagonalAbove,
            (Axis::Horizontal, Place::End, d) if d.is_diagonal() => Self::DiagonalBelow,
            (Axis::Vertical, Place::Start, d) if d.is_diagonal() => Self::DiagonalLeft,
            (Axis::Vertical, Place::End, d) if d.is_diagonal() => Self::DiagonalRight,
            _ => unreachable!(),
        }
    }

    pub fn is_padding(&self) -> bool {
        match self {
            Self::Vertical
            | Self::Horizontal
            | Self::DiagonalLeft
            | Self::DiagonalRight
            | Self::DiagonalAbove
            | Self::DiagonalBelow => true,
            _ => false,
        }
    }

    pub fn is_diagonal(&self) -> bool {
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

    pub fn is_diagonal_padding(&self) -> bool {
        match self {
            Self::DiagonalLeft
            | Self::DiagonalRight
            | Self::DiagonalAbove
            | Self::DiagonalBelow => true,
            _ => false,
        }
    }

    pub fn to_element_in(&self, axis: Axis) -> Option<Element> {
        match self {
            Self::Above | Self::Below => match axis {
                Axis::Horizontal => Some(Element::Face),
                Axis::Vertical => Some(Element::Dot),
            },
            Self::Left | Self::Right => match axis {
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

#[derive(Clone, Copy, Debug)]
pub struct GridType {
    pub point: KeyPointType,
    pub connect: Option<KeyPointType>,
    pub direction: Direction,
}

impl GridType {
    pub fn new(from: KeyIndexPoint, to: Option<KeyIndexPoint>) -> Self {
        Self {
            point: from.p_type,
            connect: to.map(|kp| kp.p_type),
            direction: Direction::new(from.point, to.map(|kp| kp.point)),
        }
    }

    pub fn new_point(tp: KeyPointType, dir: Direction) -> Self {
        Self {
            point: tp,
            connect: None,
            direction: dir,
        }
    }

    pub fn padding_in(&self, axis: Axis, place: Place) -> Self {
        Self {
            point: self.point,
            connect: None,
            direction: self.direction.padding_in(axis, place),
        }
    }

    pub fn is_real(&self, axis: Axis) -> bool {
        !self.point.is_unreal(axis) && self.connect.map_or(true, |t| !t.is_unreal(axis))
    }
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub line: Vec<(Vec<GridType>, Vec<GridType>)>,
    pub real: [bool; 2],
    pub alloc: usize,
}

impl Default for Edge {
    fn default() -> Self {
        Self {
            line: Default::default(),
            real: [true; 2],
            alloc: 9999,
        }
    }
}

impl std::fmt::Display for Edge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn real_symbol(r: bool) -> char {
            match r {
                true => 'r',
                false => 'm',
            }
        }
        let (attr1, attr2): (String, String) = self.line.iter().fold(
            (String::new(), String::new()),
            |(mut a1, mut a2), (v1, v2)| {
                a1.extend(
                    v1.iter()
                        .filter(|gt| !gt.direction.is_padding())
                        .map(|gt| [gt.point.symbol(), gt.direction.symbol()])
                        .flatten()
                        .chain(std::iter::once(',')),
                );
                a2.extend(
                    v2.iter()
                        .filter(|gt| !gt.direction.is_padding())
                        .map(|gt| [gt.point.symbol(), gt.direction.symbol()])
                        .flatten()
                        .chain(std::iter::once(',')),
                );
                (a1, a2)
            },
        );
        write!(
            f,
            "real:{}{};{}-{};len:{};",
            real_symbol(self.real[0]),
            real_symbol(self.real[1]),
            attr1,
            attr2,
            self.alloc
        )
    }
}

impl Edge {
    pub fn connect(self, other: Edge) -> Edge {
        Edge {
            line: self.line.into_iter().chain(other.line).collect(),
            real: [self.real[0] & other.real[0], self.real[1] & other.real[1]],
            alloc: self.alloc.min(other.alloc),
        }
    }

    // pub fn connect_result<E: std::fmt::Debug>(
    //     e1: Result<Edge, E>,
    //     e2: Result<Edge, E>,
    // ) -> Result<Edge, E> {
    //     if e1.is_ok() && e2.is_ok() {
    //         Ok(e1.unwrap().connect(e2.unwrap()))
    //     } else {
    //         e1.and(e2)
    //     }
    // }

    pub fn to_elements(&self, axis: Axis, place: Place) -> Vec<Element> {
        let mut face_start = false;
        self.line
            .iter()
            .map(|(f, b)| match place {
                Place::Start => f,
                Place::End => b,
                Place::Mind => unreachable!(),
            })
            .fold(vec![], |mut list, in_point| {
                let elements: Vec<Element> = in_point
                    .iter()
                    .filter(|gt| gt.is_real(axis))
                    .filter_map(|gt| gt.direction.to_element_in(axis))
                    .collect();
                if elements.contains(&Element::Face) {
                    if face_start {
                        list.push(Element::Face);
                    }
                    face_start = !face_start;
                } else if !face_start {
                    list.extend(elements)
                }
                list
            })
    }

    pub fn is_container_edge<'a>(&'a self, axis: Axis, place: Place) -> bool {
        let in_place = |line: &'a (Vec<GridType>, Vec<GridType>)| -> &'a Vec<GridType> {
            match place {
                Place::Start => &line.0,
                Place::End => &line.1,
                Place::Mind => unreachable!(),
            }
        };

        let fb = [&self.line[0], self.line.last().unwrap()];
        fb.iter().all(|line| {
            let e = in_place(line);
            match e.len() {
                0 => place == Place::End,
                1 => e[0].direction.to_element_in(axis) != Some(Element::Face),
                _ => false,
            }
        }) && match (in_place(&fb[0]).first(), in_place(&fb[1]).first()) {
            (Some(l), Some(r)) => l.direction.is_diagonal() == r.direction.is_diagonal(),
            _ => true,
        } && self
            .line
            .iter()
            .take(self.line.len().checked_sub(1).unwrap_or_default())
            .skip(1)
            .all(|line| {
                in_place(line)
                    .iter()
                    .all(|gt| gt.direction.is_diagonal_padding())
            })
        // fb.iter().all(|line| {
        //     let e = in_place(line);
        //     e.len() == 1 && e[0].direction.to_element_in(axis) != Some(Element::Face)
        // }) && in_place(&fb[0])[0].direction.is_diagonal()
        //     == in_place(&fb[1])[0].direction.is_diagonal()
        //     && self
        //         .line
        //         .iter()
        //         .take(self.line.len().checked_sub(1).unwrap_or_default())
        //         .skip(1)
        //         .all(|line| {
        //             in_place(line)
        //                 .iter()
        //                 .all(|gt| gt.direction.is_diagonal_padding())
        //         })
    }
}

#[derive(Clone)]
pub struct StrucView {
    pub view: Vec<Vec<Vec<GridType>>>,
    pub reals: DataHV<Vec<bool>>,
    pub levels: DataHV<Vec<usize>>,
}

impl StrucView {
    pub fn new(struc: &StrucProto) -> Self {
        let (_, values, reals) = struc.allocs_and_maps_and_reals();
        // let test: DataHV<Vec<_>> = values.map(|vl| vl.iter().map(|x| (*x.0, *x.1)).collect());
        let mut view: Vec<Vec<Vec<GridType>>> =
            vec![
                vec![vec![]; values.h.values().max().map(|l| l + 1).unwrap_or_default()];
                values.v.values().max().map(|l| l + 1).unwrap_or_default()
            ];

        struc.key_paths.iter().for_each(|path| {
            let mut iter = path.points.iter().map(|kp| {
                KeyIndexPoint::new(
                    IndexPoint::new(values.h[&kp.point.x], values.v[&kp.point.y]),
                    kp.p_type,
                )
            });
            let mut pre = None;
            let mut cur = iter.next();
            let mut next = iter.next();

            while let Some(kp) = cur {
                [(kp, pre), (kp, next)]
                    .into_iter()
                    .enumerate()
                    .for_each(|(i, (from, to))| match GridType::new(from, to) {
                        gtype if !gtype.direction.is_none() => {
                            let from = from.point;
                            let to = to.unwrap().point;
                            view[from.y][from.x].push(gtype);

                            if i == 1 {
                                let p1 = to.min(from);
                                let p2 = to.max(from);

                                if gtype.direction.is_horizontal() {
                                    (p1.x + 1..p2.x).for_each(|x| {
                                        view[p1.y][x]
                                            .push(gtype.padding_in(Axis::Horizontal, Place::Start))
                                    });
                                } else if gtype.direction.is_vertival() {
                                    (p1.y + 1..p2.y).for_each(|y| {
                                        view[y][p1.x]
                                            .push(gtype.padding_in(Axis::Vertical, Place::Start))
                                    });
                                } else {
                                    assert!(gtype.direction.is_diagonal());
                                    let (topx, bottomx) = if from.y < to.y {
                                        (from.x, to.x)
                                    } else {
                                        (to.x, from.x)
                                    };
                                    (p1.x..=p2.x).for_each(|x| {
                                        if x != topx {
                                            view[p1.y][x].push(
                                                gtype.padding_in(Axis::Horizontal, Place::Start),
                                            )
                                        }
                                        if x != bottomx {
                                            view[p2.y][x].push(
                                                gtype.padding_in(Axis::Horizontal, Place::End),
                                            )
                                        }
                                    });
                                    let (lefty, righty) = if from.x < to.x {
                                        (from.y, to.y)
                                    } else {
                                        (to.y, from.y)
                                    };
                                    (p1.y..=p2.y).for_each(|y| {
                                        if y != lefty {
                                            view[y][p1.x].push(
                                                gtype.padding_in(Axis::Vertical, Place::Start),
                                            )
                                        }
                                        if y != righty {
                                            view[y][p2.x]
                                                .push(gtype.padding_in(Axis::Vertical, Place::End))
                                        }
                                    });
                                }

                                // (p1.x + 1..p2.x).for_each(|x| {
                                //     view[p1.y][x]
                                //         .push(gtype.padding_in(Axis::Horizontal, Place::Start))
                                // });
                                // if p1.y != p2.y {
                                //     (p1.x + 1..p2.x).for_each(|x| {
                                //         view[p2.y][x]
                                //             .push(gtype.padding_in(Axis::Horizontal, Place::End))
                                //     });
                                // }
                                // (p1.y + 1..p2.y).for_each(|y| {
                                //     view[y][p1.x]
                                //         .push(gtype.padding_in(Axis::Vertical, Place::Start))
                                // });
                                // if p1.x != p2.x {
                                //     (p1.y + 1..p2.y).for_each(|y| {
                                //         view[y][p2.x]
                                //             .push(gtype.padding_in(Axis::Vertical, Place::End))
                                //     });
                                // }
                            }
                        }
                        _ => {}
                    });

                pre = Some(kp);
                cur = next;
                next = iter.next();
            }
        });

        Self {
            view,
            reals,
            levels: struc.get_allocs(),
        }
    }

    // pub fn width(&self) -> usize {
    //     self.view.get(0).map(|v| v.len()).unwrap_or_default()
    // }

    // pub fn heigh(&self) -> usize {
    //     self.view.len()
    // }

    // pub fn size(&self) -> IndexSize {
    //     IndexSize::new(self.width(), self.heigh())
    // }

    pub fn real_size(&self) -> DataHV<usize> {
        self.reals.map(|rs| rs.iter().filter(|r| **r).count())
    }

    pub fn is_container_in(&self, axis: Axis, place: Place) -> bool {
        self.read_edge(axis, place).is_container_edge(axis, place)
    }

    pub fn get_real_indexes(&self, axis: Axis) -> Vec<usize> {
        self.reals
            .hv_get(axis)
            .iter()
            .enumerate()
            .filter_map(|(i, r)| match r {
                true => Some(i),
                false => None,
            })
            .collect()
    }

    pub fn read_edge(&self, axis: Axis, place: Place) -> Edge {
        let size = self
            .real_size()
            .map(|v| v.checked_sub(1).unwrap_or_default());
        let segment = match place {
            Place::Start => 0,
            Place::End => *size.hv_get(axis),
            Place::Mind => unreachable!(),
        };
        self.read_edge_in(axis, 0, *size.hv_get(axis.inverse()), segment, place)
    }

    pub fn read_edge_in(
        &self,
        axis: Axis,
        start: usize,
        end: usize,
        segment: usize,
        place: Place,
    ) -> Edge {
        let in_view = |axis: Axis, i: usize, j: usize| match axis {
            Axis::Horizontal => &self.view[j][i],
            Axis::Vertical => &self.view[i][j],
        };
        let list = self.get_real_indexes(axis);
        let real = {
            match list.get(segment) {
                Some(&i) => {
                    let front = i == 0 || self.reals.hv_get(axis)[i - 1];
                    let back =
                        i + 1 == self.reals.hv_get(axis).len() || self.reals.hv_get(axis)[i + 1];
                    [front, back]
                }
                None => [true, true],
            }
        };
        let range = {
            let is = self.get_real_indexes(axis.inverse());
            is[start]..=is[end]
            // is[start]..=is.get(end).or(is.last()).copied().unwrap_or_default()
        };

        let (i1, i2) = match place {
            Place::Start if segment + 1 == list.len() => (list.last(), None),
            Place::Start => (list.get(segment), list.get(segment + 1)),
            Place::End if segment == 0 => (None, list.first()),
            Place::End => (list.get(segment - 1), list.get(segment)),
            _ => unreachable!(),
        };

        let mut line: Vec<(Vec<GridType>, Vec<GridType>)> = vec![];
        for j in range {
            line.push((vec![], vec![]));

            if let Some(&i1) = i1 {
                line.last_mut().unwrap().0.extend(
                    in_view(axis, i1, j)
                        .iter()
                        .filter(|ps| ps.is_real(axis) && ps.point != KeyPointType::Hide),
                )
            }
            if let Some(&i2) = i2 {
                line.last_mut().unwrap().1.extend(
                    in_view(axis, i2, j)
                        .iter()
                        .filter(|ps| ps.is_real(axis) && ps.point != KeyPointType::Hide),
                )
            }
        }

        let alloc = match (i1, i2) {
            (Some(i1), Some(_)) => {
                self.levels.hv_get(axis)[list.iter().position(|n| *n == *i1).unwrap()]
            }
            _ => 0,
        };

        Edge { line, real, alloc }
    }

    pub fn read_surround_edge(
        &self,
        surround: DataHV<Place>,
        axis: Axis,
    ) -> Option<[Option<Edge>; 2]> {
        let area = self.surround_area(surround)?;

        let start = area.hv_get(axis.inverse())[0];
        let end = area.hv_get(axis.inverse())[1];
        let surround_place = *surround.hv_get(axis);

        let attr1 = if surround_place != Place::End {
            Some(self.read_edge_in(axis, start, end, area.hv_get(axis)[0], Place::End))
        } else {
            None
        };
        let attr2 = if surround_place != Place::Start {
            Some(self.read_edge_in(axis, start, end, area.hv_get(axis)[1], Place::Start))
        } else {
            None
        };
        Some([attr1, attr2])
    }

    pub fn read_edge_in_surround(
        &self,
        surround: DataHV<Place>,
        axis: Axis,
        place: Place,
    ) -> Option<[Option<Edge>; 2]> {
        let area = self.surround_area(surround)?;

        let left = area.hv_get(axis.inverse())[0];
        let right = area.hv_get(axis.inverse())[1];

        let surround_place = *surround.hv_get(axis);
        assert_ne!(surround_place, Place::Mind);
        assert_ne!(surround_place, place);
        let surround_place = *surround.hv_get(axis.inverse());

        let size = self
            .real_size()
            .map(|v| v.checked_sub(1).unwrap_or_default());
        let segment = match place {
            Place::Start => 0,
            Place::End => *size.hv_get(axis),
            Place::Mind => unreachable!(),
        };
        let length = *size.hv_get(axis.inverse());

        let attr1 = if surround_place != Place::End {
            Some(self.read_edge_in(axis, 0, left, segment, place))
        } else {
            None
        };
        let attr2 = if surround_place != Place::Start {
            Some(self.read_edge_in(axis, right, length, segment, place))
        } else {
            None
        };
        Some([attr1, attr2])
    }

    pub fn surround_area(&self, surround: DataHV<Place>) -> Option<DataHV<[usize; 2]>> {
        let view = &self.view;
        let indexes = Axis::hv_data().into_map(|axis| {
            let mut indexes = self.get_real_indexes(axis);
            if *surround.hv_get(axis) == Place::Start {
                indexes.reverse();
            }
            indexes
        });

        let in_view = |axis: Axis, i: usize, j: usize| match axis {
            Axis::Horizontal => &view[j][i],
            Axis::Vertical => &view[i][j],
        };

        if self
            .reals
            .hv_iter()
            .all(|l| l.iter().filter(|r| **r).count() < 1)
        {
            eprintln!("The size of the surrounding component is less than 2!");
            return None;
        }

        match surround
            .hv_iter()
            .filter(|place| **place == Place::Mind)
            .count()
        {
            0 => {
                if !view[indexes.v[0]][indexes.h[0]].is_empty() {
                    eprintln!("Surround error!");
                    return None;
                }

                let mut max_width = usize::MAX;
                let mut y_count = 1;
                let size_list: Vec<(usize, usize)> = indexes
                    .v
                    .iter()
                    .take(indexes.v.len() - 1)
                    .take_while(|&&y| {
                        view[y][indexes.h[0]]
                            .iter()
                            .all(|pa| !pa.is_real(Axis::Vertical))
                    })
                    .map(|&y| {
                        let width = indexes
                            .h
                            .iter()
                            .take(indexes.h.len() - 1)
                            .skip(1)
                            .take_while(|&&x| {
                                view[y][x]
                                    .iter()
                                    .filter(|pa| pa.is_real(Axis::Horizontal))
                                    .count()
                                    == 0
                            })
                            .count()
                            + 1;
                        let height = y_count;
                        y_count += 1;
                        max_width = max_width.min(width);
                        (max_width, height)
                    })
                    .collect();
                size_list
                    .iter()
                    .rev()
                    .max_by_key(|(w, h)| w * h)
                    .map(|&(w, h)| {
                        Some(DataHV::new(w, h).zip(Axis::hv_data()).map(|&(len, axis)| {
                            match surround.hv_get(axis) {
                                Place::Start => [
                                    indexes.hv_get(axis).len() - len - 1,
                                    indexes.hv_get(axis).len() - 1,
                                ],
                                _ => [0, len],
                            }
                        }))
                    })
                    .unwrap()
            }
            1 => {
                let mut start = 0;
                let mut pairs: Vec<(usize, usize)> = vec![];
                let (main_axis, main_indexes, sub_indexes) = match surround.h {
                    Place::Mind => (Axis::Horizontal, &indexes.h, &indexes.v),
                    _ => (Axis::Vertical, &indexes.v, &indexes.h),
                };
                while start + 1 != main_indexes.len() {
                    match main_indexes[start..].iter().enumerate().find(|(_, &i)| {
                        let ok = in_view(main_axis, i, sub_indexes[0])
                            .iter()
                            .filter(|gt| gt.is_real(main_axis))
                            .all(|pa| match pa.direction {
                                Direction::RightAbove
                                | Direction::Right
                                | Direction::RightBelow
                                | Direction::Horizontal
                                    if main_axis == Axis::Horizontal =>
                                {
                                    false
                                }
                                Direction::LeftBelow
                                | Direction::Below
                                | Direction::RightBelow
                                | Direction::Vertical
                                    if main_axis == Axis::Vertical =>
                                {
                                    false
                                }
                                _ => true,
                            });
                        ok
                    }) {
                        Some((l, _)) => {
                            match main_indexes[start + l + 1..].iter().enumerate().find(
                                |(_, &i)| {
                                    !in_view(main_axis, i, sub_indexes[0])
                                        .iter()
                                        .filter(|gt| gt.is_real(main_axis))
                                        .all(|pa| match pa.direction {
                                            Direction::RightAbove
                                            | Direction::Right
                                            | Direction::RightBelow
                                            | Direction::Above
                                            | Direction::Below
                                            | Direction::DiagonalAbove
                                            | Direction::DiagonalBelow
                                                if main_axis == Axis::Horizontal =>
                                            {
                                                false
                                            }
                                            Direction::Right
                                            | Direction::Left
                                            | Direction::RightBelow
                                            | Direction::Below
                                            | Direction::LeftBelow
                                            | Direction::DiagonalLeft
                                            | Direction::DiagonalRight
                                                if main_axis == Axis::Vertical =>
                                            {
                                                false
                                            }
                                            _ => true,
                                        })
                                        || i == *main_indexes.last().unwrap()
                                },
                            ) {
                                Some((r, _)) => {
                                    pairs.push((l + start, r + start + 1));
                                    start = r + start + 1;
                                }
                                None => break,
                            }
                        }
                        None => break,
                    }
                }

                let max_area = pairs
                    .into_iter()
                    .map(|(left, right)| {
                        let height_list: Vec<usize> = (left..=right)
                            .map(|i_index| {
                                sub_indexes
                                    .iter()
                                    .skip(1)
                                    .take_while(|&&j| {
                                        in_view(main_axis, main_indexes[i_index], j)
                                            .iter()
                                            .filter(|gt| gt.is_real(main_axis.inverse()))
                                            .all(|pa| match main_axis {
                                                Axis::Horizontal => match pa.direction {
                                                    Direction::Horizontal => false,
                                                    Direction::Above | Direction::Below
                                                        if i_index != right && i_index != left =>
                                                    {
                                                        false
                                                    }
                                                    Direction::RightAbove
                                                    | Direction::Right
                                                    | Direction::RightBelow
                                                        if i_index != right =>
                                                    {
                                                        false
                                                    }
                                                    Direction::LeftAbove
                                                    | Direction::Left
                                                    | Direction::LeftBelow
                                                        if i_index != left =>
                                                    {
                                                        false
                                                    }
                                                    _ => true,
                                                },
                                                Axis::Vertical => match pa.direction {
                                                    Direction::Vertical => false,
                                                    Direction::Left | Direction::Right
                                                        if i_index != right && i_index != left =>
                                                    {
                                                        false
                                                    }
                                                    Direction::RightBelow
                                                    | Direction::Below
                                                    | Direction::LeftBelow
                                                        if i_index != right =>
                                                    {
                                                        false
                                                    }
                                                    Direction::RightAbove
                                                    | Direction::Above
                                                    | Direction::LeftAbove
                                                        if i_index != left =>
                                                    {
                                                        false
                                                    }
                                                    _ => true,
                                                },
                                            })
                                    })
                                    .count()
                                    + 1
                            })
                            .collect();
                        let (x1, x2, height, area) =
                            algorithm::find_reactangle_three(&height_list[..]);
                        (x1 + left, x2 + left, height, area)
                    })
                    .max_by_key(|data| data.3);

                max_area.map(|(x1, x2, height, _)| {
                    let mut r = DataHV::new(
                        [x1, x2],
                        [sub_indexes.len() - height - 1, sub_indexes.len() - 1],
                    );
                    if main_axis == Axis::Vertical {
                        r = r.vh();
                    }
                    r
                })
            }
            2 => match indexes.hv_iter().all(|i| i.len() == 2) {
                true => Some(DataHV::splat([0, 1])),
                false => None,
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view() {
        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
        ]);
        let view = StrucView::new(&proto);
        let in_pos: Vec<Direction> = view.view[1][1].iter().map(|gt| gt.direction).collect();
        assert_eq!(
            in_pos,
            vec![
                Direction::Horizontal,
                Direction::Above,
                Direction::LeftBelow
            ]
        );
        let in_pos: Vec<Direction> = view.view[2][1].iter().map(|gt| gt.direction).collect();
        assert_eq!(
            in_pos,
            vec![Direction::DiagonalBelow, Direction::DiagonalRight]
        );
    }

    #[test]
    fn test_surround_area() {
        // surround tow
        let mut proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 3), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 3), KeyPointType::Mark),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(DataHV::new(Place::Start, Place::Start))
            .unwrap();
        assert_eq!(area.h[0], 1);
        assert_eq!(area.v[0], 1);

        proto.rotate(2);
        let area = StrucView::new(&proto)
            .surround_area(DataHV::new(Place::End, Place::End))
            .unwrap();
        assert_eq!(area.h[1], 2);
        assert_eq!(area.v[1], 2);

        let mut proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Vertical),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(DataHV::new(Place::Start, Place::Start))
            .unwrap();
        assert_eq!(area.h[0], 0);
        assert_eq!(area.v[0], 0);

        proto.rotate(2);
        let area = StrucView::new(&proto)
            .surround_area(DataHV::new(Place::End, Place::End))
            .unwrap();
        assert_eq!(area.h[1], 1);
        assert_eq!(area.v[1], 2);
    }

    #[test]
    fn test_surround_three_area() {
        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(DataHV::splat(Place::Mind))
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v, [0, 1]);

        let surround_place = DataHV::new(Place::Mind, Place::Start);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 4), KeyPointType::Vertical),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 4), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(4, 3), KeyPointType::Mark),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(3, 2), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 4), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 1);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 2]);
        assert_eq!(area.v[0], 1);

        let mut proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Mark),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 0);

        proto.rotate(1);
        let area = StrucView::new(&proto)
            .surround_area(surround_place.vh())
            .unwrap();
        assert_eq!(area.v, [0, 1]);
        assert_eq!(area.h[0], 0);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 0);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [1, 2]);
        assert_eq!(area.v[0], 0);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 2), KeyPointType::Line),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 3]);
        assert_eq!(area.v[0], 1);

        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Vertical),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(3, 2), KeyPointType::Vertical),
            ],
        ]);
        let area = StrucView::new(&proto)
            .surround_area(surround_place)
            .unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 0);
    }

    #[test]
    fn test_to_elements() {
        let proto = StrucProto::new(vec![
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 2), KeyPointType::Line),
            ],
        ]);
        let ele = StrucView::new(&proto)
            .read_edge(Axis::Horizontal, Place::Start)
            .to_elements(Axis::Horizontal, Place::Start);
        assert_eq!(ele, vec![Element::Face])
    }
}
