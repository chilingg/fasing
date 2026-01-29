use crate::{base::*, combination::StrucProto};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Direction {
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
    DiagonalSide {
        from: IndexPoint,
        to: IndexPoint,
        this: IndexPoint,
    },
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

    pub fn is_black(&self, axis: Axis) -> bool {
        match self {
            Self::Above | Self::Below | Self::Vertical => axis == Axis::Horizontal,
            Self::Left | Self::Right | Self::Horizontal => axis == Axis::Vertical,
            _ => false,
        }
    }
}

pub enum SharpnessModel {
    ZeroOne,
}

#[derive(Clone)]
pub struct StrucView(Vec<Vec<Vec<Direction>>>);

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
            .filter(|path| !path.kpoints.is_empty())
            .for_each(|path| {
                let mut iter = path.kpoints.iter();
                {
                    let mut iter = iter.clone();
                    let head = iter.next().unwrap().pos;
                    if iter.all(|p| p.pos == head) {
                        view[head.y][head.x].push(Direction::None);
                        return;
                    }
                }

                let mut pre: Option<&KeyPoint<usize, IndexSpace>> = None;
                let mut cur = iter.next();
                let mut next = iter.next();

                while let Some(kp) = cur {
                    [
                        (
                            (kp.pos, kp.weight.from),
                            pre.map(|kp| (kp.pos, kp.weight.to)),
                        ),
                        (
                            (kp.pos, kp.weight.to),
                            next.map(|kp| (kp.pos, kp.weight.from)),
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    .for_each(|(i, ((from, w1), to))| {
                        if to.is_some() && w1 + to.unwrap().1 == 0 {
                            return;
                        }
                        let to = to.map(|kp| kp.0);

                        match Direction::new(from, to) {
                            Direction::None => {}
                            dir => {
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

                                        for y in p1.y..p2.y {
                                            view[y][p1.x].push(Direction::DiagonalSide {
                                                from,
                                                to,
                                                this: IndexPoint::new(p1.x, y),
                                            });
                                            view[y][p2.x].push(Direction::DiagonalSide {
                                                from,
                                                to,
                                                this: IndexPoint::new(p2.x, y),
                                            });
                                        }
                                        for x in p1.x..p2.x {
                                            view[p1.y][x].push(Direction::DiagonalSide {
                                                from,
                                                to,
                                                this: IndexPoint::new(x, p1.y),
                                            });
                                            view[p2.y][x].push(Direction::DiagonalSide {
                                                from,
                                                to,
                                                this: IndexPoint::new(x, p2.y),
                                            });
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

        Self(view)
    }

    pub fn space_size(&self) -> DataHV<usize> {
        DataHV::new(self.0[0].len() - 1, self.0.len() - 1)
    }

    pub fn edge_sharpness(&self, axis: Axis, place: Place, model: SharpnessModel) -> f32 {
        let [x_iter, y_iter]: [Box<dyn Iterator<Item = usize>>; 2] = match axis {
            Axis::Horizontal => match place {
                Place::Start => [
                    Box::new(std::iter::repeat(0)),
                    Box::new((0..self.0.len()).into_iter()),
                ],
                _ => [
                    Box::new(std::iter::repeat(self.0[0].len() - 1)),
                    Box::new((0..self.0.len()).into_iter()),
                ],
            },
            Axis::Vertical => match place {
                Place::Start => [
                    Box::new((0..self.0[0].len()).into_iter()),
                    Box::new(std::iter::repeat(0)),
                ],
                _ => [
                    Box::new((0..self.0[0].len()).into_iter()),
                    Box::new(std::iter::repeat(self.0.len() - 1)),
                ],
            },
        };

        match model {
            SharpnessModel::ZeroOne => {
                let ok = y_iter
                    .zip(x_iter)
                    .find(|&(y, x)| self.0[y][x].iter().find(|d| d.is_black(axis)).is_some())
                    .is_some();
                match ok {
                    true => 1.0,
                    false => 0.0,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size() {
        let struc = StrucProto::from(vec![KeyPath::from([
            key_pos(1, 0),
            key_pos(3, 0),
            key_pos(3, 2),
            key_pos(1, 2),
        ])]);
        let view = StrucView::new(&struc);
        let size = view.space_size();
        assert_eq!(size.v, 2);
        assert_eq!(size.h, 2);
    }

    #[test]
    fn test_zero_line() {
        let mut struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 1), key_pos(2, 1)]),
            KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.0[0][1], vec![Direction::Below]);
        assert_eq!(view.0[2][1], vec![Direction::Above]);

        struc.paths[1].kpoints[0].weight.to = 0;
        struc.paths[1].kpoints[1].weight.from = 0;

        let view = StrucView::new(&struc);
        assert_eq!(view.0[0][1], vec![]);
        assert_eq!(view.0[2][1], vec![]);
        let size = view.space_size();
        assert_eq!(size.v, 2);
        assert_eq!(size.h, 2);
    }

    #[test]
    fn test_edge_sharpness() {
        let struc = StrucProto::from(vec![
            KeyPath::from([key_pos(1, 1), key_pos(1, 2)]),
            KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
            KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
        ]);
        let view = StrucView::new(&struc);
        let sharpness = Axis::hv().into_map(|axis| {
            Place::se().map(|place| view.edge_sharpness(axis, place, SharpnessModel::ZeroOne))
        });
        assert_eq!(sharpness.h[0], 1.0);
        assert_eq!(sharpness.h[1], 0.0);
        assert_eq!(sharpness.v[0], 0.0);
        assert_eq!(sharpness.v[1], 0.0);
    }
}
