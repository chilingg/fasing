use super::{
    attribute::{PaddingPointAttr, PointAttribute, StrucAttributes},
    space::*,
    DataHV, StrucProto,
};
use crate::{algorithm, hv::*};

use std::{collections::BTreeSet, fmt::Write};

#[allow(unused)]
fn generate_attr(pas: &BTreeSet<PointAttribute>) -> String {
    pas.iter().map(|pa| pa.to_string()).collect()
}

#[derive(Clone)]
pub struct StrucAttrView {
    pub view: Vec<Vec<BTreeSet<PointAttribute>>>,
    pub real: DataHV<Vec<usize>>,
    pub axis_type: DataHV<Vec<(bool, bool)>>,
    pub levels: DataHV<Vec<usize>>,
}

impl StrucAttrView {
    pub fn new(struc: &StrucProto) -> Self {
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
                            match pa.padding_next() {
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
        let real = DataHV::<Vec<usize>> {
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
        // real.h.sort();
        // real.v.sort();

        let axis_info = struc.axis_info();
        let axis_type: DataHV<Vec<(bool, bool)>> = axis_info.into_map(|info| {
            // let test: Vec<(usize, bool)> = info.iter().map(|(&a, &b)| (a, b)).collect();
            let mut pre_real = true;
            let mut cur = info.iter();

            let mut axis_type: Vec<_> = Default::default();
            while let Some((_, &real)) = cur.next() {
                if real {
                    let start = pre_real;
                    let end = cur.clone().next().map(|(_, real)| *real).unwrap_or(true);
                    axis_type.push((start, end));
                }
                pre_real = real;
            }
            axis_type
        });

        Self {
            view,
            real,
            axis_type,
            levels: struc.axis_allocs(),
        }
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

    pub fn get_sub_space_attr(
        &self,
        axis: Axis,
        start: usize,
        end: usize,
        segment: usize,
        place: Place,
    ) -> String {
        let view = &self.view;
        let list = &self.real.hv_get(axis);
        let in_view = |axis: Axis, i: usize, j: usize| match axis {
            Axis::Horizontal => &view[j][i],
            Axis::Vertical => &view[i][j],
        };
        let ignore_type = match axis {
            Axis::Horizontal => KeyPointType::Vertical,
            Axis::Vertical => KeyPointType::Horizontal,
        };

        let mut space1 = String::new();
        let mut space2 = String::new();
        let (real1, real2) = self.axis_type.hv_get(axis)[segment];

        let start = self.real.hv_get(axis.inverse())[start];
        let end = self.real.hv_get(axis.inverse())[end];

        let (i1, i2) = match place {
            Place::Start if segment + 1 == list.len() => (list.last(), None),
            Place::Start => (list.get(segment), list.get(segment + 1)),
            Place::End if segment == 0 => (None, list.first()),
            Place::End => (list.get(segment - 1), list.get(segment)),
            _ => unreachable!(),
        };
        let level_info = match (i1, i2) {
            (Some(i1), Some(_)) => {
                self.levels.hv_get(axis)[list.iter().position(|n| *n == *i1).unwrap()]
            }
            _ => 0,
        };

        for j in start..=end {
            if let Some(&i1) = i1 {
                in_view(axis, i1, j)
                    .iter()
                    .filter(|ps| {
                        ps.this_point() != PointAttribute::symbol_of_type(Some(ignore_type))
                            && ps.this_point() != 'N'
                    })
                    .for_each(|p_attr| {
                        let mut zero = 0;
                        match axis {
                            Axis::Horizontal => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '3' | '2' | '6' | '8' | '9' => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '1' | '4' | '7' if place == Place::Start => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                            Axis::Vertical => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '6' | '3' | '2' | '1' | '4' => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '7' | '8' | '9' if place == Place::Start => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if zero == 2 {
                            space1.push(p_attr.this_point());
                            space1.push('0');
                        }
                    });
                space1.push(',');
            }

            if let Some(&i2) = i2 {
                in_view(axis, i2, j)
                    .iter()
                    .filter(|ps| {
                        ps.this_point() != PointAttribute::symbol_of_type(Some(ignore_type))
                            && ps.this_point() != 'N'
                    })
                    .for_each(|p_attr| {
                        let mut zero = 0;
                        match axis {
                            Axis::Horizontal => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '2' | '1' | '4' | '8' | '7' => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '9' | '6' | '3' if place == Place::End => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                            Axis::Vertical => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '6' | '9' | '8' | '7' | '4' => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '1' | '2' | '3' if place == Place::End => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if zero == 2 {
                            space2.push(p_attr.this_point());
                            space2.push('0');
                        }
                    });
                space2.push(',');
            }
        }

        let marked_symbol = |marked| match marked {
            true => 'r',
            false => 'm',
        };
        let length = match place {
            Place::Start => list.len() - segment,
            Place::End => segment + 1,
            _ => unreachable!(),
        };

        format!(
            "{}{}{}:{}:{}-{};",
            marked_symbol(real1),
            marked_symbol(real2),
            length,
            level_info,
            space1,
            space2,
        )
    }

    pub fn get_space_attrs_in(&self, axis: Axis, place: Place) -> String {
        let view = &self.view;
        let size = match view.is_empty() {
            true => IndexSize::splat(0),
            false => IndexSize::new(view[0].len(), view.len()),
        };
        let list = &self.real.hv_get(axis);

        let (i1, i2) = match place {
            Place::Start => (list.get(0), list.get(1)),
            Place::End => {
                if list.len() > 1 {
                    (list.get(list.len() - 2), list.last())
                } else {
                    (None, list.first())
                }
            }
            _ => unreachable!(),
        };
        let in_view = |axis: Axis, i: usize, j: usize| match axis {
            Axis::Horizontal => &view[j][i],
            Axis::Vertical => &view[i][j],
        };
        let ignore_type = match axis {
            Axis::Horizontal => KeyPointType::Vertical,
            Axis::Vertical => KeyPointType::Horizontal,
        };

        let mut space1 = String::new();
        let mut space2 = String::new();
        let (mut marked1, mut marked2) = (true, true);

        for j in 0..*size.hv_get(axis.inverse()) {
            if let Some(&i1) = i1 {
                marked1 = self
                    .axis_type
                    .hv_get(axis)
                    .get(list.iter().position(|n| *n == i1).unwrap())
                    .unwrap()
                    .0;
                in_view(axis, i1, j)
                    .iter()
                    .filter(|ps| {
                        ps.this_point() != PointAttribute::symbol_of_type(Some(ignore_type))
                    })
                    .for_each(|p_attr| {
                        let mut zero = 0;
                        match axis {
                            Axis::Horizontal => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '3' | '2' | '6' | '8' | '9' => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                            Axis::Vertical => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '6' | '3' | '2' | '1' | '4' => {
                                            space1.push(p_attr.this_point());
                                            space1.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if zero == 2 {
                            space1.push(p_attr.this_point());
                            space1.push('0');
                        }
                    });
                space1.push(',');
            }

            if let Some(&i2) = i2 {
                marked2 = self
                    .axis_type
                    .hv_get(axis)
                    .get(list.iter().position(|n| *n == i2).unwrap())
                    .unwrap()
                    .1;
                in_view(axis, i2, j)
                    .iter()
                    .filter(|ps| {
                        ps.this_point() != PointAttribute::symbol_of_type(Some(ignore_type))
                    })
                    .for_each(|p_attr| {
                        let mut zero = 0;
                        match axis {
                            Axis::Horizontal => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '2' | '1' | '4' | '8' | '7' => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                            Axis::Vertical => {
                                for c in [p_attr.front_connect(), p_attr.next_connect()] {
                                    match c {
                                        '6' | '9' | '8' | '7' | '4' => {
                                            space2.push(p_attr.this_point());
                                            space2.push(c);
                                        }
                                        '0' => zero += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                        if zero == 2 {
                            space2.push(p_attr.this_point());
                            space2.push('0');
                        }
                    });
                space2.push(',');
            }
        }

        let marked_symbol = |marked| match marked {
            true => 'r',
            false => 'm',
        };

        format!(
            "{}{}{}:{}-{};",
            marked_symbol(marked1),
            marked_symbol(marked2),
            list.len(),
            space1,
            space2
        )
    }

    // ⿵
    pub fn surround_three_area(&self) -> Result<DataHV<[usize; 2]>, super::Error> {
        let view = &self.view;
        let indexes = &self.real;

        if indexes.v.len() < 2 {
            return Err(super::Error {
                msg: "Surround three error!".to_string(),
            });
        }

        let mut start = 0;
        let mut pairs: Vec<(usize, usize)> = vec![];

        while start + 1 != indexes.h.len() {
            match indexes.h[start..].iter().enumerate().find(|(_, x)| {
                let mut corner = 0;
                let ok = view[*indexes.v.last().unwrap()][**x].iter().all(|pa| {
                    if pa.this_point() == 'V' || pa.this_point() == 'M' {
                        true
                    } else {
                        for c in [pa.front_connect(), pa.next_connect()] {
                            match c {
                                '9' | '6' | 'h' => return false,
                                'b' => corner += 1,
                                'r' => corner -= 1,
                                _ => {}
                            }
                        }
                        true
                    }
                });
                ok && corner == 0
            }) {
                Some((l, _)) => {
                    match indexes.h[start + 1..].iter().enumerate().find(|(_, x)| {
                        view[*indexes.v.last().unwrap()][**x]
                            .iter()
                            .find(|pa| {
                                if pa.this_point() != 'V' && pa.this_point() != 'M' {
                                    for c in [pa.front_connect(), pa.next_connect()] {
                                        match c {
                                            '9' | '6' | '8' | 'b' => return true,
                                            _ => {}
                                        }
                                    }
                                }
                                false
                            })
                            .is_some()
                    }) {
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
                    .map(|x_index| {
                        indexes
                            .v
                            .iter()
                            .rev()
                            .skip(1)
                            .take_while(|&&y| {
                                let mut corner = 0;
                                let ok = view[y][indexes.h[x_index]].iter().all(|pa| {
                                    if pa.this_point() == 'V' || pa.this_point() == 'M' {
                                        true
                                    } else {
                                        for c in [pa.front_connect(), pa.next_connect()] {
                                            match c {
                                                'h' => return false,
                                                '8' if x_index != right && x_index != left => {
                                                    return false
                                                }
                                                '9' | '6' if x_index != right => return false,
                                                '7' | '4' if x_index != left => return false,
                                                'b' => corner += 1,
                                                'r' | 'l' => corner -= 1,
                                                _ => {}
                                            }
                                        }
                                        true
                                    }
                                });
                                ok && corner == 0
                            })
                            .count()
                            + 1
                    })
                    .collect();
                let (x1, x2, height, area) = algorithm::find_reactangle_three(&height_list[..]);
                (x1 + left, x2 + left, height, area)
            })
            .max_by_key(|data| data.3);

        match max_area {
            Some((x1, x2, height, _)) => Ok(DataHV::new(
                [x1, x2],
                [indexes.v.len() - height - 1, indexes.v.len() - 1],
            )),
            None => Err(super::Error {
                msg: "Surround three error!".to_string(),
            }),
        }
    }

    pub fn surround_area(
        &self,
        surround: DataHV<Place>,
    ) -> Result<DataHV<[usize; 2]>, super::Error> {
        let view = &self.view;

        if self.real.hv_iter().find(|l| l.len() < 2).is_some() {
            return Err(super::Error {
                msg: "Surround error!".to_string(),
            });
        }

        match surround
            .hv_iter()
            .filter(|place| **place == Place::Mind)
            .count()
        {
            0 => {
                let indexes = Axis::hv_data().into_map(|axis| {
                    let mut indexes = self.real.hv_get(axis).clone();
                    if *surround.hv_get(axis) == Place::Start {
                        indexes.reverse();
                    }
                    indexes
                });

                if !view[indexes.v[0]][indexes.h[0]].is_empty() {
                    return Err(super::Error {
                        msg: "Surround error!".to_string(),
                    });
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
                            .all(|pa| pa.this_point() == 'H')
                    })
                    .map(|&y| {
                        let width = indexes
                            .h
                            .iter()
                            .take(indexes.h.len() - 1)
                            .skip(1)
                            .take_while(|&&x| {
                                // let (h, v, other) =
                                //     view[y][x].iter().filter(|pa| pa.this_point() != 'V').fold(
                                //         (0, 0, 0),
                                //         |(h, v, other), pa| match pa.front_connect() {
                                //             'l' | 'r' => (h + 1, v, other),
                                //             't' | 'b' => (h, v + 1, other),
                                //             _ => (h, v, other + 1),
                                //         },
                                //     );
                                // h == v && other == 0

                                view[y][x]
                                    .iter()
                                    .filter(|pa| pa.this_point() != 'V')
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
                        Ok(DataHV::new(w, h)
                            .into_zip(Axis::hv_data())
                            .map(|&(len, axis)| match surround.hv_get(axis) {
                                Place::Start => [
                                    indexes.hv_get(axis).len() - len - 1,
                                    indexes.hv_get(axis).len() - 1,
                                ],
                                _ => [0, len],
                            }))
                    })
                    .unwrap()
            }
            1 => {
                todo!()
            }
            2 => todo!(),
            _ => unreachable!(),
        }
    }

    // ⿸
    pub fn surround_tow_area(&self) -> Result<DataHV<usize>, super::Error> {
        let view = &self.view;
        let indexes: DataHV<Vec<_>> = self
            .real
            .map(|v| v.iter().cloned().enumerate().rev().collect());
        // let mut area: DataHV<Option<usize>> = DataHV::default();

        if indexes.h.len() < 2
            || indexes.v.len() < 2
            || !view[indexes.v.first().unwrap().1][indexes.h.first().unwrap().1].is_empty()
        {
            return Err(super::Error {
                msg: "Surround error!".to_string(),
            });
        }

        let mut max_width = None;
        let size_list: Vec<(usize, usize)> = indexes
            .v
            .iter()
            .take(indexes.v.len() - 1)
            .take_while(|(_, y)| {
                view[*y][indexes.h.first().unwrap().1]
                    .iter()
                    .all(|pa| pa.this_point() == 'H' || pa.this_point() == 'M')
            })
            .map(|&(h, y)| {
                let width = indexes
                    .h
                    .iter()
                    .take(indexes.h.len() - 1)
                    .take_while(|&&(_, x)| {
                        if view[y][x]
                            .iter()
                            .all(|pa| pa.this_point() == 'V' || pa.this_point() == 'M')
                        {
                            true
                        } else if view[y][x].len() == 2 {
                            let (l, r) = view[y][x].iter().fold((false, false), |(l, r), pa| {
                                match pa.front_connect() {
                                    'l' => (true, r),
                                    'r' => (l, true),
                                    _ => (l, r),
                                }
                            });
                            l & r
                        } else {
                            false
                        }
                    })
                    .count();
                let height = indexes.v.len() - h;
                max_width = Some(max_width.unwrap_or(width).min(width));
                (max_width.unwrap(), height)
            })
            .collect();
        size_list
            .iter()
            .rev()
            .max_by_key(|&(w, h)| w * h)
            .map(|&(w, h)| {
                Ok(DataHV::new(
                    indexes.h.len() - w - 1,
                    indexes.v.len() - h - 1,
                ))
            })
            .unwrap()
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

    pub fn new(struc: &StrucProto) -> Self {
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
                            match pa.padding_next() {
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

        Self { view }
    }

    pub fn read_row(&self, column: usize, range: std::ops::Range<usize>) -> String {
        let mut attr = range.into_iter().fold(String::new(), |mut attr, y| {
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

    pub fn read_column(&self, row: usize, range: std::ops::Range<usize>) -> String {
        let mut attr = range.into_iter().fold(String::new(), |mut attr, x| {
            write!(
                attr,
                "{}-",
                self.view[row][x].iter().fold(String::new(), |mut str, pa| {
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
        let view = StrucAttrView::new(&proto);
        let attrs = view.get_space_attrs();
        assert_eq!(attrs.h.len(), 3);
        assert_eq!(attrs.v.len(), 1);
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
        let area = StrucAttrView::new(&proto)
            .surround_area(DataHV::new(Place::Start, Place::Start))
            .unwrap();
        assert_eq!(area.h[0], 1);
        assert_eq!(area.v[0], 1);

        proto.rotate(2);
        let area = StrucAttrView::new(&proto)
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
        let area = StrucAttrView::new(&proto)
            .surround_area(DataHV::new(Place::Start, Place::Start))
            .unwrap();
        assert_eq!(area.h[0], 0);
        assert_eq!(area.v[0], 0);

        proto.rotate(2);
        let area = StrucAttrView::new(&proto)
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
                KeyIndexPoint::new(IndexPoint::new(0, 1), KeyPointType::Line),
            ],
            vec![
                KeyIndexPoint::new(IndexPoint::new(0, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 0), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(2, 1), KeyPointType::Line),
                KeyIndexPoint::new(IndexPoint::new(1, 1), KeyPointType::Mark),
            ],
        ]);
        let area = StrucAttrView::new(&proto).surround_three_area().unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 0);

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
        let area = StrucAttrView::new(&proto).surround_three_area().unwrap();
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
        let area = StrucAttrView::new(&proto).surround_three_area().unwrap();
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
        let area = StrucAttrView::new(&proto).surround_three_area().unwrap();
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
        let area = StrucAttrView::new(&proto).surround_three_area().unwrap();
        assert_eq!(area.h, [0, 1]);
        assert_eq!(area.v[0], 0);
    }
}
