use crate::{base::*, combination::struc::StrucProto};

pub enum SharpnessModel {
    ZeroOne,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
pub enum SubAreaEdge {
    #[default]
    Empty,
    OutSide,
    DiagnoalPad,
    Diagnoal,
    Wall,
}

impl SubAreaEdge {
    pub fn is_obstruct(&self) -> bool {
        *self >= Self::Diagnoal
    }

    pub fn merge(&mut self, other: Self) {
        if *self < other {
            *self = other;
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            Self::Empty => 'e',
            Self::OutSide => 'o',
            Self::DiagnoalPad => 'p',
            Self::Diagnoal => 'd',
            Self::Wall => 'w',
        }
    }
}

pub struct SubArea {
    pub min: IndexPoint,
    pub max: IndexPoint,
    pub edge: DataHV<[SubAreaEdge; 2]>,
}

impl SubArea {
    pub fn is_match(&self, symbols: &str) -> bool {
        self.edge
            .hv_iter()
            .flatten()
            .map(|e| e.symbol())
            .zip(symbols.chars())
            .all(|(e, s)| s == '*' || s == e)
    }
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
    pub fn to_area_edge(&self, axis: Axis, side: Side) -> SubAreaEdge {
        use Direction::*;

        match axis {
            Axis::Vertical => match self {
                Self::Horizontal => SubAreaEdge::Wall,
                Self::DiagonalT | Self::DiagonalB => SubAreaEdge::Diagnoal,
                Self::D(Right) if side == Side::Front => SubAreaEdge::Wall,
                Self::D(RightAbove) | Self::D(RightBelow) | Self::DiagonalRB | Self::DiagonalRA
                    if side == Side::Front =>
                {
                    SubAreaEdge::Diagnoal
                }
                Self::D(Left) if side == Side::Back => SubAreaEdge::Wall,
                Self::D(LeftAbove) | Self::D(LeftBelow) | Self::DiagonalLB | Self::DiagonalLA
                    if side == Side::Back =>
                {
                    SubAreaEdge::Diagnoal
                }
                Self::DiagonalL if side == Side::Front => SubAreaEdge::DiagnoalPad,
                Self::DiagonalR if side == Side::Back => SubAreaEdge::DiagnoalPad,
                Self::DiagonalPad => SubAreaEdge::DiagnoalPad,
                _ => SubAreaEdge::Empty,
            },
            Axis::Horizontal => match self {
                Self::Vertical => SubAreaEdge::Wall,
                Self::DiagonalL | Self::DiagonalR => SubAreaEdge::Diagnoal,
                Self::D(Below) if side == Side::Front => SubAreaEdge::Wall,
                Self::D(LeftBelow) | Self::D(RightBelow) | Self::DiagonalRB | Self::DiagonalLB
                    if side == Side::Front =>
                {
                    SubAreaEdge::Diagnoal
                }
                Self::D(Above) if side == Side::Back => SubAreaEdge::Wall,
                Self::D(LeftAbove) | Self::D(RightAbove) | Self::DiagonalRA | Self::DiagonalLA
                    if side == Side::Back =>
                {
                    SubAreaEdge::Diagnoal
                }
                Self::DiagonalT if side == Side::Front => SubAreaEdge::DiagnoalPad,
                Self::DiagonalB if side == Side::Back => SubAreaEdge::DiagnoalPad,
                Self::DiagonalPad => SubAreaEdge::DiagnoalPad,
                _ => SubAreaEdge::Empty,
            },
        }
    }

    pub fn is_black(&self, axis: Axis) -> bool {
        use Direction::*;
        match self {
            Self::D(Above) | Self::D(Below) | Self::Vertical => axis == Axis::Horizontal,
            Self::D(Left) | Self::D(Right) | Self::Horizontal => axis == Axis::Vertical,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct StrucView(Vec<Vec<Vec<ViewElement>>>);

impl StrucView {
    pub fn new(struc: &StrucProto) -> Self {
        if struc.is_empty() {
            return Self(Default::default());
        }

        let values = struc.values_map();

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

        Self(view)
    }

    pub fn struc_size(&self) -> DataHV<usize> {
        DataHV::new(self.0[0].len(), self.0.len())
    }

    pub fn edge_sharpness(&self, axis: Axis, side: Side, model: SharpnessModel) -> f32 {
        let [x_iter, y_iter]: [Box<dyn Iterator<Item = usize>>; 2] = match axis {
            Axis::Horizontal => match side {
                Side::Front => [
                    Box::new(std::iter::repeat(0)),
                    Box::new((0..self.0.len()).into_iter()),
                ],
                Side::Back => [
                    Box::new(std::iter::repeat(self.0[0].len() - 1)),
                    Box::new((0..self.0.len()).into_iter()),
                ],
            },
            Axis::Vertical => match side {
                Side::Front => [
                    Box::new((0..self.0[0].len()).into_iter()),
                    Box::new(std::iter::repeat(0)),
                ],
                Side::Back => [
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

    fn get_in(&self, axis: Axis, main: usize, cross: usize) -> &Vec<ViewElement> {
        match axis {
            Axis::Horizontal => &self.0[cross][main],
            Axis::Vertical => &self.0[main][cross],
        }
    }

    pub fn get_subareas(&self, axis: Axis) -> Vec<Vec<SubArea>> {
        let size = self.struc_size();
        let mut subareas = Vec::with_capacity(*size.hv_get(axis));
        let mut min = IndexPoint::default();
        let mut max = IndexPoint::default();
        for i in 0..*size.hv_get(axis) - 1 {
            let mut subarea = vec![];
            *min.hv_get_mut(axis) = i;
            *max.hv_get_mut(axis) = i + 1;

            while min.hv_get(axis.inverse()) + 1 < *size.hv_get(axis.inverse()) {
                let mut area_edge: DataHV<[SubAreaEdge; 2]> = Default::default();
                if *min.hv_get(axis.inverse()) == 0 {
                    area_edge.hv_get_mut(axis.inverse())[0] = SubAreaEdge::OutSide;
                }
                if i == 0 {
                    area_edge.hv_get_mut(axis)[0] = SubAreaEdge::OutSide;
                } else if i + 1 == *size.hv_get(axis) - 1 {
                    area_edge.hv_get_mut(axis)[1] = SubAreaEdge::OutSide;
                }

                Side::fb().into_iter().for_each(|side| {
                    let list = self.get_in(axis, i + side.n(), *min.hv_get(axis.inverse()));
                    if !list.is_empty() {
                        let edge = list
                            .iter()
                            .map(|d| d.to_area_edge(axis.inverse(), side))
                            .max()
                            .unwrap();
                        area_edge.hv_get_mut(axis.inverse())[0].merge(edge);
                        let edge = list
                            .iter()
                            .map(|d| d.to_area_edge(axis, Side::Front))
                            .max()
                            .unwrap();
                        area_edge.hv_get_mut(axis)[side.n()].merge(edge);
                    }
                });

                for j in *min.hv_get(axis.inverse()) + 1..*size.hv_get(axis.inverse()) {
                    let b = Side::fb().map(|side| {
                        let list = self.get_in(axis, i + side.n(), j);
                        if list.is_empty() {
                            false
                        } else {
                            let edge = list
                                .iter()
                                .map(|d| d.to_area_edge(axis, Side::Back))
                                .max()
                                .unwrap();
                            area_edge.hv_get_mut(axis)[side.n()].merge(edge);

                            let edge = list
                                .iter()
                                .map(|d| d.to_area_edge(axis.inverse(), side))
                                .max()
                                .unwrap();
                            area_edge.hv_get_mut(axis.inverse())[1].merge(edge);

                            edge.is_obstruct()
                        }
                    });

                    if j + 1 == *size.hv_get(axis.inverse()) {
                        area_edge.hv_get_mut(axis.inverse())[1].merge(SubAreaEdge::OutSide);
                    }
                    *max.hv_get_mut(axis.inverse()) = j;

                    if b[0] || b[1] {
                        break;
                    }
                }
                subarea.push(SubArea {
                    min,
                    max,
                    edge: area_edge,
                });
                *min.hv_get_mut(axis.inverse()) = *max.hv_get(axis.inverse());
            }

            subareas.push(subarea);
            *min.hv_get_mut(axis.inverse()) = 0;
            *max.hv_get_mut(axis.inverse()) = 0;
        }
        subareas
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
        assert_eq!(view.0[0][0], vec![ViewElement::D(Direction::RightBelow)]);
        assert_eq!(view.0[0][1], vec![ViewElement::DiagonalT]);
        assert_eq!(view.0[0][2], vec![ViewElement::DiagonalLB]);
        assert_eq!(view.0[1][0], vec![ViewElement::DiagonalL]);
        assert_eq!(
            view.0[1][1],
            vec![ViewElement::DiagonalPad, ViewElement::D(Direction::None)]
        );
        assert_eq!(view.0[1][2], vec![ViewElement::DiagonalR]);
        assert_eq!(view.0[2][0], vec![ViewElement::DiagonalRA]);
        assert_eq!(view.0[2][1], vec![ViewElement::DiagonalB]);
        assert_eq!(view.0[2][2], vec![ViewElement::D(Direction::LeftAbove)]);

        let mut struc = StrucProto::from(vec![
            KeyPath::from([key_pos(0, 1), key_pos(2, 1)]),
            KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
        ]);
        let view = StrucView::new(&struc);
        assert_eq!(view.0[0][1], vec![ViewElement::D(Direction::Below)]);
        assert_eq!(
            view.0[1][1],
            vec![ViewElement::Horizontal, ViewElement::Vertical]
        );
        assert_eq!(view.0[2][1], vec![ViewElement::D(Direction::Above)]);

        struc.paths[1].kpoints[1].labels.push("mark".to_string());

        let view = StrucView::new(&struc);
        assert_eq!(view.0[0][1], vec![ViewElement::D(Direction::None)]);
        assert_eq!(view.0[1][1], vec![ViewElement::Horizontal]);
        assert_eq!(view.0[2][1], vec![]);
        let size = view.struc_size();
        assert_eq!(size.v, 3);
        assert_eq!(size.h, 3);
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
            Side::fb().map(|side| view.edge_sharpness(axis, side, SharpnessModel::ZeroOne))
        });
        assert_eq!(sharpness.h[0], 1.0);
        assert_eq!(sharpness.h[1], 0.0);
        assert_eq!(sharpness.v[0], 0.0);
        assert_eq!(sharpness.v[1], 0.0);
    }

    #[test]
    fn test_subarea_edge() {
        assert!(SubAreaEdge::OutSide < SubAreaEdge::Wall);
    }

    #[test]
    fn test_subarea() {
        let struc: StrucProto = serde_json::from_value(serde_json::json!({"paths":[{"kpoints":[{"pos":[3,0],"labels":[]},{"pos":[0,2],"labels":[]}],"hide":false},{"kpoints":[{"pos":[3,0],"labels":[]},{"pos":[6,2],"labels":[]}],"hide":false},{"kpoints":[{"pos":[2,2],"labels":[]},{"pos":[4,2],"labels":[]}],"hide":false},{"kpoints":[{"pos":[0,4],"labels":[]},{"pos":[6,4],"labels":[]}],"hide":false},{"kpoints":[{"pos":[3,2],"labels":[]},{"pos":[3,7],"labels":[]},{"pos":[2,7],"labels":["mark"]}],"hide":false},{"kpoints":[{"pos":[1,5],"labels":[]},{"pos":[0,7],"labels":[]}],"hide":false},{"kpoints":[{"pos":[5,5],"labels":[]},{"pos":[6,7],"labels":[]}],"hide":false}],"attrs":{}})).unwrap();
        let view = StrucView::new(&struc);
        let subareas = view.get_subareas(Axis::Horizontal);
        assert_eq!(subareas.len(), 6);
        assert_eq!(subareas[0][0].min, IndexPoint::new(0, 0));
        assert_eq!(subareas[0][0].max, IndexPoint::new(1, 1));
        assert_eq!(
            subareas[0][0].edge,
            DataHV::new(
                [SubAreaEdge::Diagnoal, SubAreaEdge::DiagnoalPad],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Diagnoal]
            )
        );

        assert_eq!(subareas[0][1].min, IndexPoint::new(0, 1));
        assert_eq!(subareas[0][1].max, IndexPoint::new(1, 2));
        assert_eq!(
            subareas[0][1].edge,
            DataHV::new(
                [SubAreaEdge::OutSide, SubAreaEdge::Empty],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Wall]
            )
        );

        let y = 2;
        let n = 0;
        assert_eq!(subareas[n][y].min, IndexPoint::new(0, 2));
        assert_eq!(subareas[n][y].max, IndexPoint::new(1, 3));
        assert_eq!(
            subareas[n][y].edge,
            DataHV::new(
                [SubAreaEdge::OutSide, SubAreaEdge::Empty],
                [SubAreaEdge::Wall, SubAreaEdge::Diagnoal]
            )
        );

        let y = 1;
        let n = 2;
        assert_eq!(subareas[n][y].min, IndexPoint::new(2, 1));
        assert_eq!(subareas[n][y].max, IndexPoint::new(3, 2));
        assert_eq!(
            subareas[n][y].edge,
            DataHV::new(
                [SubAreaEdge::Empty, SubAreaEdge::Wall],
                [SubAreaEdge::Wall, SubAreaEdge::Wall]
            )
        );

        assert_eq!(
            subareas[5][3].edge,
            DataHV::new(
                [SubAreaEdge::Diagnoal, SubAreaEdge::Diagnoal],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Diagnoal]
            )
        );
        assert_eq!(
            subareas[4][0].edge,
            DataHV::new(
                [SubAreaEdge::DiagnoalPad, SubAreaEdge::DiagnoalPad],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Diagnoal]
            )
        );
        assert_eq!(
            subareas[4][1].edge,
            DataHV::new(
                [SubAreaEdge::Empty, SubAreaEdge::Empty],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Wall]
            )
        );
        assert_eq!(
            subareas[5][1].edge,
            DataHV::new(
                [SubAreaEdge::Empty, SubAreaEdge::OutSide],
                [SubAreaEdge::Diagnoal, SubAreaEdge::Wall]
            )
        );
    }
}
