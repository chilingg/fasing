use super::{
    attribute::{PaddingPointAttr, PointAttribute, StrucAttributes},
    space::*,
    DataHV, Error, StrucProto,
};

use std::{collections::BTreeSet, fmt::Write};

#[allow(unused)]
fn generate_attr(pas: &BTreeSet<PointAttribute>) -> String {
    pas.iter().map(|pa| pa.to_string()).collect()
}

pub struct StrucAttrView {
    pub view: Vec<Vec<BTreeSet<PointAttribute>>>,
    pub real: DataHV<Vec<usize>>,
}

impl StrucAttrView {
    pub fn new(struc: &StrucProto) -> Result<Self, Error> {
        let maps = struc.maps_to_not_mark_pos();
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
                                    let min_pos = IndexPoint::new(x1, y1);
                                    let (mut offset_1, mut offset_2) =
                                        match min_pos == real_pos || min_pos == next {
                                            true => (1, 0),
                                            false => (0, 1),
                                        };

                                    (x1 + offset_1..=x2 - offset_2).for_each(|x| {
                                        view[y1][x].insert(padding_t);
                                    });
                                    (y1 + offset_1..=y2 - offset_2).for_each(|y| {
                                        view[y][x1].insert(padding_l);
                                    });
                                    std::mem::swap(&mut offset_1, &mut offset_2);
                                    (x1 + offset_1..=x2 - offset_2).for_each(|x| {
                                        view[y2][x].insert(padding_b);
                                    });
                                    (y1 + offset_1..=y2 - offset_2).for_each(|y| {
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

        let real_map = struc.maps_to_real_point();
        let mut real = DataHV::<Vec<usize>> {
            h: maps
                .h
                .iter()
                .filter_map(|(i, &n)| match real_map.h.contains_key(i) {
                    true => Some(n),
                    false => None,
                })
                .collect(),
            v: maps
                .v
                .iter()
                .filter_map(|(i, &n)| match real_map.v.contains_key(i) {
                    true => Some(n),
                    false => None,
                })
                .collect(),
        };
        real.h.sort();
        real.v.sort();

        Ok(Self { view, real })
    }

    pub fn get_space_attrs(&self) -> StrucAttributes {
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

        let (x_list, y_list) = (&self.real.h, &self.real.v);

        for x in 1..x_list.len() {
            for y in 0..view.len() {
                let mut ok = y + 1 == view.len() || y == y_list[0];
                let mut cur_pad1 = String::new();
                let mut cur_pad2 = String::new();

                view[y][x_list[x - 1]]
                    .iter()
                    .filter(|ps| {
                        ps.this_point()
                            != PointAttribute::symbol_of_type(Some(KeyPointType::Vertical))
                    })
                    .for_each(|p_attr| {
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
                                    cur_pad1.push(p_attr.this_point());
                                    cur_pad1.push(c);
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
                            'v' | 'l' | 'r' => {
                                attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                                pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            }
                            't' | 'b' => {
                                cur_pad1.push(p_attr.this_point());
                                cur_pad1.push(p_attr.front_connect());
                            }
                            '0' if p_attr.next_connect() == '0' => {
                                cur_pad1.push(p_attr.this_point());
                                cur_pad1.push('0')
                            }
                            _ => {}
                        }
                    });
                view[y][x_list[x]]
                    .iter()
                    .filter(|ps| {
                        ps.this_point()
                            != PointAttribute::symbol_of_type(Some(KeyPointType::Vertical))
                    })
                    .for_each(|p_attr| {
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
                                    cur_pad2.push(p_attr.this_point());
                                    cur_pad2.push(c);
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
                            'v' | 'l' | 'r' => {
                                attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                                pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            }
                            't' | 'b' => {
                                cur_pad2.push(p_attr.this_point());
                                cur_pad2.push(p_attr.front_connect());
                            }
                            '0' if p_attr.next_connect() == '0' => {
                                cur_pad2.push(p_attr.this_point());
                                cur_pad2.push('0')
                            }
                            _ => {}
                        }
                    });

                if !ok {
                    padding1.extend(attr1.drain(..).flatten().chain(cur_pad1.drain(..)));
                    padding2.extend(attr2.drain(..).flatten().chain(cur_pad2.drain(..)));
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
                            ">{}{}-{}{}<{}-{};",
                            padding1,
                            cur_pad1,
                            padding2,
                            cur_pad2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else {
                        write!(
                            output,
                            ">{}{}-{}{}<{}-{};{}-{}",
                            padding1,
                            cur_pad1,
                            padding2,
                            cur_pad2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>(),
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    }
                    padding1.clear();
                    padding2.clear();
                    padding1.extend(cur_pad1.drain(..));
                    padding2.extend(cur_pad2.drain(..));
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
                x_list.len() - x - 1,
            ));
            output.clear();
            space1.clear();
            space2.clear();
            padding1.clear();
            padding2.clear();
        }

        for y in 1..y_list.len() {
            for x in 0..width {
                let mut ok = x + 1 == width || x == 0;
                let mut cur_pad1 = String::new();
                let mut cur_pad2 = String::new();
                view[y_list[y - 1]][x]
                    .iter()
                    .filter(|ps| {
                        ps.this_point()
                            != PointAttribute::symbol_of_type(Some(KeyPointType::Horizontal))
                    })
                    .for_each(|p_attr| {
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
                                    cur_pad1.push(p_attr.this_point());
                                    cur_pad1.push(c);
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
                            't' | 'b' | 'h' => {
                                attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                                pre_attr1.push([p_attr.this_point(), p_attr.front_connect()]);
                            }
                            'l' | 'r' => {
                                cur_pad1.push(p_attr.this_point());
                                cur_pad1.push(p_attr.front_connect());
                            }
                            '0' if p_attr.next_connect() == '0' => {
                                cur_pad1.push(p_attr.this_point());
                                cur_pad1.push('0')
                            }
                            _ => {}
                        }
                    });
                view[y_list[y]][x]
                    .iter()
                    .filter(|ps| {
                        ps.this_point()
                            != PointAttribute::symbol_of_type(Some(KeyPointType::Horizontal))
                    })
                    .for_each(|p_attr| {
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
                                    cur_pad2.push(p_attr.this_point());
                                    cur_pad2.push(c);
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
                            't' | 'b' | 'h' => {
                                attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                                pre_attr2.push([p_attr.this_point(), p_attr.front_connect()]);
                            }
                            'l' | 'r' => {
                                cur_pad2.push(p_attr.this_point());
                                cur_pad2.push(p_attr.front_connect());
                            }
                            '0' if p_attr.next_connect() == '0' => {
                                cur_pad2.push(p_attr.this_point());
                                cur_pad2.push('0')
                            }
                            _ => {}
                        }
                    });

                if !ok {
                    padding1.extend(attr1.drain(..).flatten().chain(cur_pad1.drain(..)));
                    padding2.extend(attr2.drain(..).flatten().chain(cur_pad2.drain(..)));
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
                            ">{}{}-{}{}<{}-{};",
                            padding1,
                            cur_pad1,
                            padding2,
                            cur_pad2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    } else {
                        write!(
                            output,
                            ">{}{}-{}{}<{}-{};{}-{}",
                            padding1,
                            cur_pad1,
                            padding2,
                            cur_pad2,
                            pre_attr1.iter().flatten().collect::<String>(),
                            pre_attr2.iter().flatten().collect::<String>(),
                            attr1.iter().flatten().collect::<String>(),
                            attr2.iter().flatten().collect::<String>()
                        )
                        .unwrap();
                    }
                    padding1.clear();
                    padding2.clear();
                    padding1.extend(cur_pad1.drain(..));
                    padding2.extend(cur_pad2.drain(..));
                    attr1.clear();
                    attr2.clear();
                    pre_attr1.clear();
                    pre_attr2.clear();
                }
            }

            space1.sort_by(|[_, a], [_, b]| a.cmp(b));
            space2.sort_by(|[_, a], [_, b]| a.cmp(b));
            v.push(format!(
                "v:{}-{}:{}:{}-{}",
                space1.iter().flatten().collect::<String>(),
                space2.iter().flatten().collect::<String>(),
                output,
                y - 1,
                y_list.len() - y - 1,
            ));
            output.clear();
            space1.clear();
            space2.clear();
            padding1.clear();
            padding2.clear();
        }

        StrucAttributes::new(h, v)
    }
}

#[derive(Default, Clone)]
pub struct StrucAllAttrView {
    pub view: Vec<Vec<BTreeSet<PointAttribute>>>,
}

impl StrucAllAttrView {
    pub fn width(&self) -> usize {
        self.view.get(0).map_or(0, |line| line.len())
    }

    pub fn height(&self) -> usize {
        self.view.len()
    }

    pub fn new(struc: &StrucProto) -> Result<Self, Error> {
        let size = struc.size();
        let mut view = vec![vec![BTreeSet::new(); size.width]; size.height];

        for path in struc.key_paths.iter() {
            let mut iter = path.points.iter();
            let mut previous = None;
            let mut current = iter.next();
            let mut next = iter.next();

            loop {
                match current.take() {
                    Some(&kp) => {
                        let pa = PointAttribute::from_key_point(previous, kp, next.copied());
                        let pos = kp.point;

                        if let Some(next) = next {
                            let next = next.point;
                            let min_pos = pos.min(next);
                            let max_pos = pos.max(next);
                            match pa.padding_next().or_else(|e| {
                                Err(Error {
                                    msg: format!(
                                        "in pos({}, {}) {}",
                                        kp.point.x, kp.point.y, e.msg
                                    ),
                                })
                            })? {
                                PaddingPointAttr::Line(padding) => {
                                    if min_pos.x == max_pos.x {
                                        (min_pos.y + 1..max_pos.y).for_each(|y| {
                                            view[y][min_pos.x].insert(padding);
                                        });
                                    } else if min_pos.y == max_pos.y {
                                        (min_pos.x + 1..max_pos.x).for_each(|x| {
                                            view[min_pos.y][x].insert(padding);
                                        });
                                    } else {
                                        unreachable!()
                                    }
                                }
                                PaddingPointAttr::Box(
                                    [padding_t, padding_b, padding_l, padding_r],
                                ) => {
                                    let (mut offset_1, mut offset_2) =
                                        match min_pos == pos || min_pos == next {
                                            true => (1, 0),
                                            false => (0, 1),
                                        };

                                    (min_pos.x + offset_1..=max_pos.x - offset_2).for_each(|x| {
                                        view[min_pos.y][x].insert(padding_t);
                                    });
                                    (min_pos.y + offset_1..=max_pos.y - offset_2).for_each(|y| {
                                        view[y][min_pos.x].insert(padding_l);
                                    });
                                    std::mem::swap(&mut offset_1, &mut offset_2);
                                    (min_pos.x + offset_1..=max_pos.x - offset_2).for_each(|x| {
                                        view[max_pos.y][x].insert(padding_b);
                                    });
                                    (min_pos.y + offset_1..=max_pos.y - offset_2).for_each(|y| {
                                        view[y][max_pos.x].insert(padding_r);
                                    });
                                }
                            }
                        }
                        view[pos.y][pos.x].insert(pa);

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

    pub fn read_row(&self, column: usize, range: std::ops::Range<usize>) -> String {
        let mut attr = range.into_iter().fold(String::from("v:"), |mut attr, y| {
            write!(
                attr,
                "{}-",
                self.view[y][column]
                    .iter()
                    .fold(String::new(), |mut str, pa| {
                        match pa.front_connect() {
                            '0'..='9' => str.push_str(pa.to_string().as_str()),
                            _ => {}
                        };
                        str
                    })
            )
            .unwrap();
            attr
        });
        attr.push(';');
        attr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attrs() {
        let proto = StrucProto {
            tags: Default::default(),
            key_paths: vec![
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        KeyIndexPoint::new(IndexPoint::new(1, 0), KeyPointType::Mark),
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(0, 2), KeyPointType::Horizontal),
                    ],
                },
                KeyIndexPath {
                    closed: false,
                    points: vec![
                        KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(3, 1), KeyPointType::Line),
                        KeyIndexPoint::new(IndexPoint::new(2, 3), KeyPointType::Line),
                    ],
                },
            ],
        };
        let view = StrucAttrView::new(&proto).unwrap();
        let attrs = view.get_space_attrs();
        assert_eq!(attrs.h.len(), 3);
        assert_eq!(attrs.v.len(), 1);
    }
}
