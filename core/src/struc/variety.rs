use crate::{
    construct::{self, Component, Format},
    fas_file::{AllocateRule, AllocateTable, ComponetConfig, Error, StrokeMatch, StrokeReplace},
    hv::*,
    struc::{
        attribute::PointAttribute, space::*, view::StrucAttrView, StrokePath, StrucAllocates,
        StrucAttributes, StrucProto, StrucWork,
    },
};

use once_cell::sync::Lazy;
use serde::Serialize;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
};

#[derive(Clone)]
pub struct StrucDataCache {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub view: StrucAttrView,
}

impl StrucDataCache {
    pub fn new(proto: StrucProto) -> Self {
        let allocs = proto.axis_info().into_map(|info| {
            let mut advance = 0;
            info.into_iter()
                .filter_map(|(n, is_real)| {
                    if is_real {
                        let map_to = n - advance;
                        advance = n;
                        Some(map_to)
                    } else {
                        advance += 1;
                        None
                    }
                })
                .skip(1)
                .collect()
        });
        Self {
            view: StrucAttrView::new(&proto),
            attrs: proto.attributes(),
            proto,
            allocs,
        }
    }

    pub fn from_alloc_table(proto: StrucProto, table: &AllocateTable) -> Self {
        static DEFAULT_TAG: Lazy<BTreeSet<String>> =
            Lazy::new(|| BTreeSet::from(["default".to_string()]));

        let tags = match proto.tags.is_empty() {
            true => &DEFAULT_TAG,
            false => &proto.tags,
        };
        let rules: Vec<&AllocateRule> = table
            .iter()
            .filter(|rule| rule.filter.is_empty() || !rule.filter.is_disjoint(tags))
            .collect();

        let attrs = proto.attributes();
        let allocs: DataHV<Vec<usize>> = attrs.map(|attrs| {
            attrs
                .iter()
                .map(|attr| {
                    rules
                        .iter()
                        .find_map(|rule| match rule.regex.is_match(attr) {
                            true => Some(rule.weight),
                            false => None,
                        })
                        .unwrap_or(1)
                })
                .collect()
        });

        Self {
            view: StrucAttrView::new(&proto),
            proto,
            attrs,
            allocs,
        }
    }

    fn reduce(&mut self, axis: Axis, regex: &regex::Regex) -> bool {
        let range: Vec<_> = (0..self.attrs.hv_get(axis).len()).collect();
        let (front, back) = range.split_at(range.len() / 2);
        let front_reduce = front
            .iter()
            .find(|n| regex.is_match(self.attrs.hv_get(axis)[**n].as_str()))
            .copied();
        let back_reduce = back
            .iter()
            .rev()
            .find(|n| regex.is_match(self.attrs.hv_get(axis)[**n].as_str()))
            .copied();
        if front_reduce.is_none() && back_reduce.is_none() {
            false
        } else {
            if let Some(re) = back_reduce {
                let mut temp = Default::default();
                std::mem::swap(&mut temp, &mut self.proto);
                self.proto = temp.reduce(axis, re);
                self.allocs.hv_get_mut(axis).remove(re);
            }
            if let Some(re) = front_reduce {
                let mut temp = Default::default();
                std::mem::swap(&mut temp, &mut self.proto);
                self.proto = temp.reduce(axis, re);
                self.allocs.hv_get_mut(axis).remove(re);
            }
            self.view = StrucAttrView::new(&self.proto);
            self.attrs = self.proto.attributes();
            true
        }
    }
}

#[derive(Clone, Default, Serialize)]
pub struct TransformValue {
    pub length: f32,
    pub level: usize,
    // pub level_min: f32,
    pub allocs: Vec<usize>,
    pub assign: Vec<f32>,
}

impl TransformValue {
    pub const DEFAULT_MIN_VALUE: f32 = 0.1;

    pub fn from_allocs(
        allocs: Vec<usize>,
        length: f32,
        assign_values: &Vec<f32>,
        min_values: &Vec<f32>,
        base_values: &Vec<f32>,
        level: Option<usize>,
    ) -> Result<Self, Error> {
        Self::from_allocs_and_intervals_assign(
            allocs,
            length,
            assign_values,
            min_values,
            base_values,
            level,
            0.0,
        )
    }

    pub fn from_allocs_and_intervals_assign(
        allocs: Vec<usize>,
        length: f32,
        assign_values: &Vec<f32>,
        min_values: &Vec<f32>,
        base_values: &Vec<f32>,
        level: Option<usize>,
        intervals: f32,
    ) -> Result<Self, Error> {
        if allocs.iter().cloned().max().unwrap_or_default() == 0 {
            return Ok(Self {
                length: 0.0,
                level: 0,
                assign: vec![0.0; allocs.len()],
                allocs,
            });
        }

        let base_list: Vec<f32> = allocs
            .iter()
            .map(|v| match v {
                0 => 0.0,
                v => *base_values
                    .get(v - 1)
                    .or(base_values.last())
                    .unwrap_or(&1.0),
            })
            .collect();
        let base_count = base_list.iter().sum::<f32>();
        let assign_list: Vec<f32> = allocs
            .iter()
            .map(|v| match v {
                0 => 0.0,
                v => *assign_values
                    .get(v - 1)
                    .or(assign_values.last())
                    .unwrap_or(&1.0),
            })
            .collect();
        let assign_count = assign_list.iter().sum::<f32>();

        // let test: Vec<f32> = min_values
        //     .iter()
        //     .map(|v| length - v * (num + intervals))
        //     .collect();
        let level = {
            let val = match min_values
                .iter()
                .position(|v| length - v * (base_count + intervals) > -0.001)
            {
                Some(level) => level,
                None => {
                    return Err(Error::Transform {
                        alloc_len: allocs.iter().sum(),
                        length,
                        min: *min_values.last().unwrap_or(&Self::DEFAULT_MIN_VALUE),
                    });
                }
            };

            if let Some(level) = level {
                level.max(val)
            } else {
                val
            }
        };

        let min = *min_values
            .get(level)
            .or(min_values.last())
            .unwrap_or(&Self::DEFAULT_MIN_VALUE);
        let assign_total = length - (base_count + intervals) * min;
        let assign: Vec<f32> = if assign_count == 0.0 {
            base_list.iter().map(|&n| n * min).collect()
        } else {
            let one_assign = assign_total / assign_count;
            base_list
                .iter()
                .zip(assign_list.iter())
                .map(|(&n, &a)| min * n + one_assign * a)
                .collect()
        };

        Ok(Self {
            level,
            length: assign.iter().sum::<f32>(),
            allocs,
            assign,
        })
    }
}

#[derive(Clone)]
pub enum StrucComb {
    Single {
        rotate: usize,
        name: String,
        limit: Option<WorkSize>,
        cache: StrucDataCache,
        trans: Option<DataHV<TransformValue>>,
    },
    Complex {
        name: String,
        limit: Option<WorkSize>,
        format: Format,
        comps: Vec<StrucComb>,
        intervals: Vec<i32>,
        assign_intervals: Vec<f32>,
    },
}

impl StrucComb {
    pub fn name(&self) -> &str {
        match self {
            Self::Single { name, .. } => name,
            Self::Complex { name, .. } => name,
        }
    }

    pub fn get_limit_mut(&mut self) -> &mut Option<WorkSize> {
        match self {
            Self::Single { limit, .. } => limit,
            Self::Complex { limit, .. } => limit,
        }
    }

    pub fn new(
        mut name: String,
        const_table: &construct::Table,
        // alloc_table: &AllocateTable,
        components: &BTreeMap<String, StrucProto>,
        config: &ComponetConfig,
    ) -> Result<Self, Error> {
        while let Some(map_name) = config
            .replace_list
            .get(&Format::Single)
            .and_then(|fs| fs.get(&0).and_then(|is| is.get(&name)))
        {
            if name == *map_name {
                break;
            }
            name = map_name.to_owned();
        }

        let limit = config.format_limit.get(&Format::Single).and_then(|fs| {
            fs.get(&0).and_then(|group| {
                group.iter().find_map(|(group, size)| {
                    if group.contains(&name) {
                        Some(size.min(WorkSize::new(1.0, 1.0)))
                    } else {
                        None
                    }
                })
            })
        });
        let const_attr = {
            let mut chars = name.chars();
            let char_name = chars.next().unwrap();
            if chars.next().is_none() {
                match const_table.get(&char_name) {
                    Some(attrs) => attrs,
                    None => construct::Attrs::single(),
                }
            } else {
                construct::Attrs::single()
            }
        };

        Self::from_format(
            name,
            limit,
            const_attr,
            const_table,
            // alloc_table,
            components,
            config,
        )
    }

    pub fn from_format(
        name: String,
        size_limit: Option<WorkSize>,
        const_attrs: &construct::Attrs,
        const_table: &construct::Table,
        // alloc_table: &AllocateTable,
        components: &BTreeMap<String, StrucProto>,
        config: &ComponetConfig,
    ) -> Result<Self, Error> {
        use Format::*;

        let get_real_name = |name: &str, fmt: Format, in_fmt: usize| -> Option<&str> {
            let mut new_name = None;
            while let Some(map_name) = config
                .replace_list
                .get(&fmt)
                .and_then(|fs| {
                    fs.get(&in_fmt)
                        .and_then(|is| is.get(new_name.unwrap_or(name)))
                })
                .map(|s| s.as_str())
            {
                new_name = Some(map_name);
            }
            new_name
        };

        let get_size_limit = |name: &str, fmt: Format, in_fmt: usize| {
            config.format_limit.get(&fmt).and_then(|fs| {
                fs.get(&in_fmt).and_then(|group| {
                    group.iter().find_map(|(group, size)| {
                        if group.contains(name) {
                            Some(size.min(WorkSize::new(1.0, 1.0)))
                        } else {
                            None
                        }
                    })
                })
            })
        };

        let get_const_attr = |name: &str| {
            let mut chars = name.chars();
            let char_name = chars.next().unwrap();
            if chars.next().is_none() {
                match const_table.get(&char_name) {
                    Some(attrs) => attrs,
                    None => construct::Attrs::single(),
                }
            } else {
                construct::Attrs::single()
            }
        };

        let get_cache = |name: &str| -> Result<StrucDataCache, Error> {
            let proto = components.get(name).ok_or(Error::Empty(name.to_owned()))?;
            Ok(StrucDataCache::new(proto.clone()))
        };

        // Define end ----------------

        match const_attrs.format {
            Single => Ok(Self::from_single(get_cache(&name)?, size_limit, name)),
            LeftToRight
            | LeftToMiddleAndRight
            | AboveToBelow
            | AboveToMiddleAndBelow
            | SurroundFromLowerLeft
            | SurroundFromUpperRight
            | SurroundFromUpperLeft => {
                let mut combs_info: Vec<(String, construct::Attrs, Option<WorkSize>, Component)> =
                    Vec::with_capacity(const_attrs.format.number_of());

                let mut real_attrs = const_attrs.components.clone();
                loop {
                    for (in_fmt, comp) in real_attrs.iter().enumerate() {
                        let (name, attr) =
                            match get_real_name(comp.name().as_str(), const_attrs.format, in_fmt) {
                                Some(map_name) => (map_name.to_owned(), get_const_attr(map_name)),
                                None => match comp {
                                    Component::Char(comp_name) => {
                                        (comp_name.to_owned(), get_const_attr(comp_name))
                                    }
                                    Component::Complex(ref complex_attrs) => {
                                        (format!("{}", complex_attrs), complex_attrs)
                                    }
                                },
                            };
                        let limit = get_size_limit(&name, const_attrs.format, in_fmt);
                        combs_info.push((name, attr.clone(), limit, comp.clone()));
                    }

                    match const_attrs.format {
                        SurroundFromLowerLeft | SurroundFromUpperRight | SurroundFromUpperLeft => {
                            match combs_info[0].1.format {
                                SurroundFromLowerLeft
                                | SurroundFromUpperRight
                                | SurroundFromUpperLeft => {
                                    let (secondery1, secondery2) =
                                        if combs_info[0].1.format == SurroundFromLowerLeft {
                                            (
                                                combs_info[1].3.clone(),
                                                combs_info[0].1.components[1].clone(),
                                            )
                                        } else {
                                            (
                                                combs_info[0].1.components[1].clone(),
                                                combs_info[1].3.clone(),
                                            )
                                        };

                                    let secondery = Component::Complex(construct::Attrs {
                                        format: Format::AboveToBelow,
                                        components: vec![secondery1, secondery2],
                                    });
                                    real_attrs =
                                        vec![combs_info[0].1.components[0].clone(), secondery];
                                    combs_info.clear();
                                }
                                _ => break,
                            }
                        }
                        _ => break,
                    }
                }

                let mut comps = Vec::with_capacity(const_attrs.format.number_of());
                for (name, attrs, limit, _) in combs_info {
                    comps.push(StrucComb::from_format(
                        name,
                        limit,
                        &attrs,
                        const_table,
                        // alloc_table,
                        components,
                        config,
                    )?);
                }

                Ok(StrucComb::from_complex(
                    const_attrs.format,
                    comps,
                    size_limit,
                    name,
                ))
            }
            _ => Err(Error::Empty(
                const_attrs.format.to_symbol().unwrap().to_string(),
            )),
        }
    }

    pub fn from_complex(
        format: Format,
        comps: Vec<StrucComb>,
        limit: Option<WorkSize>,
        name: String,
    ) -> Self {
        Self::Complex {
            name,
            format,
            comps,
            limit,
            intervals: vec![],
            assign_intervals: vec![],
        }
    }

    pub fn from_single(cache: StrucDataCache, limit: Option<WorkSize>, name: String) -> Self {
        Self::Single {
            name,
            limit,
            cache,
            trans: Default::default(),
            rotate: 0,
        }
    }

    pub fn to_skeleton(
        &self,
        level: DataHV<usize>,
        stroke_match: &Vec<StrokeReplace>,
        offset: WorkPoint,
        rect: WorkRect,
        min_values: &DataHV<Vec<f32>>,
    ) -> Vec<StrokePath> {
        let struc = self.to_work(offset, rect, level, min_values);
        let collisions_counter = struc.key_paths.iter().fold(
            vec![],
            |mut collisions: Vec<(WorkPoint, BTreeMap<char, usize>)>, path| {
                let mut pre = None;
                let mut next = path.points.iter().cloned();
                next.next();

                path.points.iter().for_each(|kp| {
                    let connect = [
                        PointAttribute::symbol_of_connect(Some(*kp), pre),
                        PointAttribute::symbol_of_connect(Some(*kp), next.next()),
                    ]
                    .into_iter()
                    .filter(|c| *c == '0');

                    if let Some((_, counter)) = collisions.iter_mut().find(|(p, _)| *p == kp.point)
                    {
                        connect.for_each(|c| {
                            counter.entry(c).and_modify(|n| *n += 1).or_insert(1);
                        });
                    } else {
                        collisions.push((kp.point, connect.map(|c| (c, 1)).collect()));
                    }

                    pre = Some(*kp);
                });
                collisions
            },
        );

        struc
            .key_paths
            .iter()
            .filter(|p| p.points.iter().all(|kp| kp.p_type != KeyPointType::Hide))
            .fold(vec![], |mut strokes, path| {
                let stroke_type = path.stroke_type();
                let boxed = path.boxed();
                let size = boxed.size();
                let p_types: Vec<KeyPointType> = path.points.iter().map(|kp| kp.p_type).collect();

                let mut pre = None;
                let mut next = path.points.iter().cloned();
                next.next();
                let p_collisions: Vec<Vec<char>> = path
                    .points
                    .iter()
                    .map(|kp| {
                        let connect = [
                            PointAttribute::symbol_of_connect(Some(*kp), pre),
                            PointAttribute::symbol_of_connect(Some(*kp), next.next()),
                        ];
                        pre = Some(*kp);

                        collisions_counter
                            .iter()
                            .find_map(|(p, counter)| match *p == kp.point {
                                true => {
                                    let symbols: Vec<char> = counter
                                        .iter()
                                        .filter_map(|(&c, &n)| match connect.contains(&c) {
                                            true if n > 1 => Some(c),
                                            false => Some(c),
                                            _ => None,
                                        })
                                        .collect();

                                    Some(symbols)
                                }
                                false => None,
                            })
                            .unwrap()
                    })
                    .collect();

                let match_map = stroke_match.iter().find_map(|replace| {
                    let StrokeMatch {
                        stroke,
                        min_size,
                        min_level,
                        collision,
                        pos_types,
                    } = &replace.matchs;

                    if stroke_type == *stroke {
                        if (min_size.h.is_none() || min_size.h.unwrap() <= size.width)
                            && (min_size.v.is_none() || min_size.v.unwrap() <= size.height)
                        {
                            if (min_level.h.is_none() || min_level.h.unwrap() <= level.h)
                                && (min_level.v.is_none() || min_level.v.unwrap() <= level.v)
                            {
                                if pos_types.len() <= p_types.len()
                                    && pos_types
                                        .iter()
                                        .enumerate()
                                        .all(|(i, t)| t.is_none() || t.unwrap() == p_types[i])
                                {
                                    if collision.len() <= p_collisions.len()
                                        && collision.iter().enumerate().all(|(i, c)| {
                                            c.is_none()
                                                || c.as_ref()
                                                    .unwrap()
                                                    .iter()
                                                    .all(|c| p_collisions[i].contains(c))
                                        })
                                    {
                                        let mut replace_path = replace.replace.clone();
                                        replace_path
                                            .transform(size.to_vector(), boxed.min.to_vector());
                                        return Some(replace_path);
                                    }
                                }
                            }
                        }
                    }

                    return None;
                });

                strokes.push(match match_map {
                    Some(stroke) => stroke,
                    None => StrokePath::from_key_path(path),
                });
                strokes
            })
    }

    pub fn to_work(
        &self,
        offset: WorkPoint,
        rect: WorkRect,
        levels: DataHV<usize>,
        min_values: &DataHV<Vec<f32>>,
    ) -> StrucWork {
        let mut struc = Default::default();
        self.merge(&mut struc, offset, rect, min_values);
        // struc.center_marker_pos(min_values.zip(&levels).map(|(list, _level)| {
        //     list.last()
        //         .cloned()
        //         .unwrap_or(TransformValue::DEFAULT_MIN_VALUE)
        // }));
        struc
    }

    pub fn merge(
        &self,
        struc: &mut StrucWork,
        offset: WorkPoint,
        rect: WorkRect,
        min_values: &DataHV<Vec<f32>>,
    ) -> WorkSize {
        fn merge_in_axis(
            comps: &Vec<StrucComb>,
            struc: &mut StrucWork,
            offset: WorkPoint,
            rect: WorkRect,
            intervals: &Vec<f32>,
            axis: Axis,
            min_values: &DataHV<Vec<f32>>,
        ) -> WorkSize {
            let max_length = comps
                .iter()
                .map(|vc| vc.axis_length(axis.inverse()))
                .reduce(f32::max)
                .unwrap_or_default();
            let mut advence = WorkSize::zero();
            let mut interval = intervals.iter().cloned();

            comps
                .iter()
                .fold(offset, |mut offset, vc| {
                    let interval_val = interval.next().unwrap_or_default();
                    let mut sub_offset = offset;

                    let length = vc.axis_length(axis.inverse());
                    *sub_offset.hv_get_mut(axis.inverse()) += (max_length - length) * 0.5;

                    let sub_advence = vc.merge(struc, sub_offset, rect, min_values);
                    *offset.hv_get_mut(axis) += sub_advence.hv_get(axis) + interval_val;

                    *advence.hv_get_mut(axis.inverse()) = sub_advence
                        .hv_get(axis.inverse())
                        .max(*advence.hv_get(axis.inverse()));
                    *advence.hv_get_mut(axis) += sub_advence.hv_get(axis) + interval_val;

                    offset
                })
                .hv_get(axis);

            advence
        }

        // ⿸
        fn merge_in_surround_tow(
            comps: &Vec<StrucComb>,
            struc: &mut StrucWork,
            offset: WorkPoint,
            rect: WorkRect,
            intervals: &Vec<f32>,
            fmt: Format,
            min_values: &DataHV<Vec<f32>>,
        ) -> WorkSize {
            let mut primery_struc = StrucWork::default();
            let advance = comps[0].merge(&mut primery_struc, offset, rect, min_values);

            let sub_length = Axis::hv_data().map(|&axis| comps[1].axis_length(axis));
            let (intervals, alignment, sub_area) = {
                match fmt {
                    Format::SurroundFromLowerLeft => (
                        vec![intervals[1], 0.0],
                        vec![
                            (advance.width - intervals[1] - sub_length.h) * 0.5,
                            (advance.height - intervals[0] - sub_length.v) * 0.5,
                        ],
                        WorkBox::new(
                            WorkPoint::new(offset.x + intervals[1], offset.y),
                            WorkPoint::new(
                                offset.x + advance.width,
                                offset.y + advance.height - intervals[0],
                            ),
                        ),
                    ),
                    Format::SurroundFromUpperRight => (
                        vec![0.0, intervals[0]],
                        vec![
                            (advance.width - intervals[1] - sub_length.h) * 0.5,
                            (advance.height - intervals[0] - sub_length.v) * 0.5,
                        ],
                        WorkBox::new(
                            WorkPoint::new(offset.x, offset.y + intervals[0]),
                            WorkPoint::new(
                                offset.x + advance.width - intervals[1],
                                offset.y + advance.height,
                            ),
                        ),
                    ),
                    Format::SurroundFromUpperLeft => (
                        intervals.clone(),
                        vec![
                            (advance.width - intervals[0] - sub_length.h) * 0.5,
                            (advance.height - intervals[1] - sub_length.v) * 0.5,
                        ],
                        WorkBox::new(
                            WorkPoint::new(offset.x + intervals[0], offset.y + intervals[1]),
                            WorkPoint::new(offset.x + advance.width, offset.y + advance.height),
                        ),
                    ),
                    _ => unreachable!(),
                }
            };

            primery_struc.marker_shrink(sub_area, WorkBox::from_origin_and_size(offset, advance));
            struc.merge(primery_struc);

            comps[1].merge(
                struc,
                WorkPoint::new(
                    offset.x + intervals[0] + alignment[0],
                    offset.y + intervals[1] + alignment[1],
                ),
                rect,
                min_values,
            );

            advance
        }

        match self {
            Self::Single { cache, trans, .. } => {
                let trans = trans.as_ref().unwrap();
                let struc_work = cache
                    .proto
                    .to_work_in_transform(trans, min_values)
                    .transform(
                        rect.size.to_vector(),
                        WorkVec::new(
                            rect.origin.x + (offset.x) * rect.width(),
                            rect.origin.y + (offset.y) * rect.height(),
                        ),
                    );
                let advence = WorkSize::new(trans.h.length, trans.v.length);
                struc.merge(struc_work);
                advence
            }
            Self::Complex {
                format,
                comps,
                assign_intervals,
                ..
            } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => merge_in_axis(
                    comps,
                    struc,
                    offset,
                    rect,
                    assign_intervals,
                    Axis::Vertical,
                    min_values,
                ),
                Format::LeftToMiddleAndRight | Format::LeftToRight => merge_in_axis(
                    comps,
                    struc,
                    offset,
                    rect,
                    assign_intervals,
                    Axis::Horizontal,
                    min_values,
                ),
                Format::SurroundFromLowerLeft
                | Format::SurroundFromUpperLeft
                | Format::SurroundFromUpperRight => merge_in_surround_tow(
                    comps,
                    struc,
                    offset,
                    rect,
                    assign_intervals,
                    *format,
                    min_values,
                ),
                _ => unreachable!(),
            },
        }
    }

    pub fn detection(&self) -> Result<(), Error> {
        match self {
            Self::Complex { format, comps, .. } => Self::detection_comps(*format, comps),
            _ => Ok(()),
        }
    }

    fn detection_comps(fmt: Format, comps: &Vec<StrucComb>) -> Result<(), Error> {
        match fmt {
            Format::SurroundFromLowerLeft
            | Format::SurroundFromLowerRight
            | Format::SurroundFromUpperLeft
            | Format::SurroundFromUpperRight => {
                let quarter = fmt.rotate_to_surround_tow();
                let mut primery_comp = comps[0].clone();
                primery_comp.rotate(quarter);
                match primery_comp.last_comp() {
                    Self::Single { cache, .. } => match cache.view.surround_area() {
                        Ok(_) => Ok(()),
                        Err(_) => Err(Error::Surround(
                            fmt,
                            comps[0].name().to_string(),
                            comps[1].name().to_string(),
                        )),
                    },
                    _ => unreachable!(),
                }
            }
            _ => comps
                .iter()
                .find_map(|c| match c.detection() {
                    Err(e) => Some(Err(e)),
                    _ => None,
                })
                .unwrap_or(Ok(())),
        }
    }

    pub fn allocation(
        &mut self,
        size_limit: WorkSize,
        config: &ComponetConfig,
        level: DataHV<Option<usize>>,
    ) -> Result<DataHV<TransformValue>, Error> {
        match self {
            Self::Single {
                limit,
                cache,
                trans,
                ..
            } => {
                let mut other_options = DataHV::default();
                let size = match limit {
                    Some(limit) => {
                        let other = WorkSize::new(
                            //limit.width * size_limit.width,
                            limit.width,
                            //limit.height * size_limit.height,
                            limit.height,
                        );
                        let min_size = size_limit.min(other);
                        let max_size = size_limit.max(other);
                        other_options = DataHV::new(Some(max_size.width), Some(max_size.height));
                        min_size
                    }
                    None => size_limit,
                };

                let mut results = Vec::with_capacity(2);
                for ((((allocs, length), other), level), axis) in cache
                    .allocs
                    .hv_iter()
                    .zip(size.hv_iter())
                    .zip(other_options.hv_iter())
                    .zip(level.hv_iter())
                    .zip(Axis::list())
                {
                    match TransformValue::from_allocs(
                        allocs.clone(),
                        *length,
                        &config.assign_values.hv_get(axis),
                        &config.min_values.hv_get(axis),
                        config.base_values.hv_get(axis),
                        *level,
                    ) {
                        Ok(tv) => results.push(tv),
                        Err(_) if other.is_some() => results.push(TransformValue::from_allocs(
                            allocs.clone(),
                            other.unwrap(),
                            &config.assign_values.hv_get(axis),
                            config.min_values.hv_get(axis),
                            config.base_values.hv_get(axis),
                            *level,
                        )?),
                        Err(e) => return Err(e),
                    };
                }

                results.swap(0, 1);
                let trans_result = DataHV::new(results.pop().unwrap(), results.pop().unwrap());
                *trans = Some(trans_result.clone());
                Ok(trans_result)
            }
            Self::Complex {
                format,
                comps,
                limit,
                intervals,
                assign_intervals,
                name,
            } => {
                let size_limit = if let Some(limit) = limit {
                    size_limit.min(*limit)
                } else {
                    size_limit
                };

                let r = match format {
                    Format::LeftToMiddleAndRight | Format::LeftToRight => {
                        let axis = Axis::Horizontal;
                        Self::allocation_axis(comps, size_limit, config, axis, level).and_then(
                            |(tfv, n_intervals, a_intervals)| {
                                *intervals = n_intervals;
                                *assign_intervals = a_intervals;
                                Ok(tfv)
                            },
                        )
                    }
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                        let axis = Axis::Vertical;
                        Self::allocation_axis(comps, size_limit, config, axis, level).and_then(
                            |(tfv, n_intervals, a_intervals)| {
                                *intervals = n_intervals;
                                *assign_intervals = a_intervals;
                                Ok(tfv)
                            },
                        )
                    }
                    Format::SurroundFromUpperRight
                    | Format::SurroundFromUpperLeft
                    | Format::SurroundFromLowerLeft
                    | Format::SurroundFromLowerRight => {
                        Self::allocation_surround_tow(comps, size_limit, config, level, *format)
                            .and_then(|(tfv, n_intervals, a_intervals)| {
                                *intervals = n_intervals;
                                *assign_intervals = a_intervals;
                                Ok(tfv)
                            })
                    }
                    _ => Err(Error::Empty(format.to_symbol().unwrap().to_string())),
                };

                match r {
                    Err(Error::Transform { .. }) => {
                        eprintln!(
                            "`{}`{:?}",
                            name,
                            comps.iter().map(|c| c.name()).collect::<Vec<&str>>()
                        );
                    }
                    _ => {}
                }

                r
            }
        }
    }

    // ⿸
    fn allocation_surround_tow(
        comps: &mut Vec<StrucComb>,
        size_limit: WorkSize,
        config: &ComponetConfig,
        mut level: DataHV<Option<usize>>,
        fmt: Format,
    ) -> Result<(DataHV<TransformValue>, Vec<i32>, Vec<f32>), Error> {
        if let Some((primary_cache, mut sub_comp1, rotate)) = match &mut comps[0] {
            Self::Complex { format, comps, .. } => match format {
                Format::SurroundFromUpperLeft
                | Format::SurroundFromUpperRight
                | Format::SurroundFromLowerLeft
                | Format::SurroundFromLowerRight => {
                    let sub_comp1 = comps.pop().unwrap();
                    let (primary_cache, rotate) = match &comps[0] {
                        Self::Single { cache, rotate, .. } => (cache.clone(), *rotate),
                        _ => unreachable!(),
                    };
                    Some((primary_cache, vec![sub_comp1], rotate))
                }
                // Format::LeftToMiddleAndRight | Format::LeftToRight => {

                // }
                _ => None,
            },
            _ => None,
        } {
            let (sub_comp1, sub_comp2) = match rotate {
                0 | 1 => (sub_comp1.pop().unwrap(), comps.pop().unwrap()),
                2 | 3 => (comps.pop().unwrap(), sub_comp1.pop().unwrap()),
                _ => unreachable!(),
            };

            let sub_name = format!(
                "{}{}{}",
                Format::AboveToBelow.to_symbol().unwrap(),
                sub_comp1.name(),
                sub_comp2.name()
            );
            let primary_name = comps[0].name().to_string();
            *comps = vec![
                Self::from_single(primary_cache, None, primary_name),
                Self::from_complex(
                    Format::AboveToBelow,
                    vec![sub_comp1, sub_comp2],
                    None,
                    sub_name,
                ),
            ];
        }

        let quater = fmt.rotate_to_surround_tow();
        comps.iter_mut().for_each(|c| c.rotate(quater));
        let (config, size_limit) = if quater % 2 == 1 {
            level = level.vh();
            (
                config.vh(),
                WorkSize::new(size_limit.height, size_limit.width),
            )
        } else {
            (config.clone(), size_limit)
        };

        let mut intervals: DataHV<i32> = Default::default();
        let mut intervals_assign: DataHV<f32> = Default::default();
        let mut sub_areas_assign: DataHV<Vec<f32>> = Default::default();
        let mut use_other = DataHV::splat(false);
        let mut real_size = [WorkSize::splat(1.0); 2];
        let mut primary_area_length: DataHV<f32> = Default::default();

        for axis in Axis::list() {
            let mut reduce_checks: HashMap<usize, Vec<&regex::Regex>> = Default::default();
            config.reduce_checks.iter().for_each(|wr| {
                reduce_checks
                    .entry(wr.weight)
                    .and_modify(|list| list.push(&wr.regex))
                    .or_insert(vec![&wr.regex]);
            });
            let level_info: Vec<(f32, Vec<&regex::Regex>)> = config
                .min_values
                .hv_get(axis)
                .iter()
                .enumerate()
                .map(|(i, v)| (*v, reduce_checks.get(&i).cloned().unwrap_or_default()))
                .collect();

            let length = *size_limit.hv_get(axis);
            let mut ok = false;
            let mut comp_intervals: [Vec<i32>; 2] = Default::default();
            let mut segments: usize = Default::default();
            let mut sub_allocs = vec![];
            let mut allocs = vec![];

            'query_level: for (min, regexs) in level_info.into_iter() {
                loop {
                    *intervals.hv_get_mut(axis) = Self::surround_interval(
                        &comps[0], &comps[1], axis, &config,
                    )
                    .map_err(|_| {
                        Error::Surround(
                            fmt,
                            comps[0].name().to_string(),
                            comps[1].name().to_string(),
                        )
                    })?;

                    let interval_total = {
                        let ((tmp_allocs1, tmp_intervals1), mut tmp_sub_allocs) =
                            match comps[0].surround_allocs(axis, &config) {
                                Ok(r) => r,
                                Err(_) => {
                                    return Err(Error::Surround(
                                        fmt,
                                        comps[0].name().to_string(),
                                        comps[1].name().to_string(),
                                    ))
                                }
                            };
                        let (mut tmp_allocs2, tmp_intervals2) =
                            comps[1].axis_allocs(axis, &config)?;

                        let self_value = config.get_base_total(axis, &tmp_sub_allocs);
                        let other_value = config.get_base_total(axis, &tmp_allocs2)
                            + config.get_interval_value(axis, *intervals.hv_get(axis))
                            + config.get_interval_base_total(axis, &tmp_intervals2);

                        segments = tmp_allocs1.len();
                        comp_intervals[0] = tmp_intervals1;
                        comp_intervals[1] = tmp_intervals2;
                        allocs = tmp_allocs1;
                        if self_value < other_value {
                            *use_other.hv_get_mut(axis) = true;
                            allocs.append(&mut tmp_allocs2);
                            sub_allocs = tmp_sub_allocs;
                            comp_intervals
                                .iter()
                                .map(|v| config.get_interval_base_total(axis, v))
                                .sum::<f32>()
                                + config.get_interval_value(axis, *intervals.hv_get(axis))
                        } else {
                            sub_allocs = tmp_sub_allocs.clone();
                            allocs.append(&mut tmp_sub_allocs);
                            config.get_interval_base_total(axis, &comp_intervals[0])
                                + config.get_interval_value(axis, *intervals.hv_get(axis))
                        }
                    };

                    let allocs_count = config.get_base_total(axis, &allocs) + interval_total;
                    if allocs_count == 0.0 {
                        ok = true;
                        break 'query_level;
                    } else {
                        if allocs_count * min <= length {
                            ok = true;
                            break 'query_level;
                        } else if regexs.iter().fold(false, |ok, regex| {
                            Self::axis_reduce_comps(comps, axis, regex, &config) | ok
                        }) {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }
            if !ok {
                return Err(Error::Transform {
                    alloc_len: allocs.iter().sum(),
                    length,
                    min: *config
                        .min_values
                        .hv_get(axis)
                        .last()
                        .unwrap_or(&TransformValue::DEFAULT_MIN_VALUE),
                });
            }

            let tmp_intervals: Vec<i32> = if *use_other.hv_get(axis) {
                comp_intervals[0]
                    .iter()
                    .chain(comp_intervals[1].iter())
                    .chain(std::iter::once(intervals.hv_get(axis)))
                    .cloned()
                    .collect()
            } else {
                comp_intervals[0].clone()
            };

            sub_allocs.push(intervals.hv_get(axis).abs() as usize);
            let (mut all_tfv, tmp_intervals_assign_list, mut primary_sub_area) = config
                .get_trans_and_interval(
                    axis,
                    length,
                    allocs,
                    &tmp_intervals,
                    *level.hv_get(axis),
                    Some(&sub_allocs),
                )
                .unwrap();
            sub_allocs.pop();

            *intervals_assign.hv_get_mut(axis) = primary_sub_area.as_mut().unwrap().pop().unwrap();
            if intervals.hv_get(axis).is_negative() {
                *intervals_assign.hv_get_mut(axis) = -*intervals_assign.hv_get(axis);
            }
            *level.hv_get_mut(axis) = level
                .hv_get(axis)
                .map(|l| l.max(all_tfv.level))
                .or(Some(all_tfv.level));
            let assigns: Vec<f32> = all_tfv.assign.drain(0..segments).collect();

            let tmp_primary_length = assigns.iter().sum::<f32>()
                + tmp_intervals_assign_list[0..comp_intervals[0].len()]
                    .iter()
                    .sum::<f32>();

            *real_size[0].hv_get_mut(axis) = tmp_primary_length
                + primary_sub_area
                    .as_ref()
                    .map(|l| l.iter().sum::<f32>())
                    .unwrap_or_default();
            match comps[0] {
                Self::Complex { format, .. } => match format {
                    Format::LeftToMiddleAndRight | Format::LeftToRight => {
                        real_size[0].height = size_limit.height
                    }
                    Format::AboveToMiddleAndBelow | Format::AboveToBelow => {
                        real_size[0].width = size_limit.width
                    }
                    _ => {}
                },
                _ => {}
            }

            *real_size[1].hv_get_mut(axis) =
                size_limit.hv_get(axis) - tmp_primary_length - *intervals_assign.hv_get(axis);
            *sub_areas_assign.hv_get_mut(axis) = {
                let mut count_valid = 0;
                if sub_allocs.iter().all(|&v| {
                    if v == 1 {
                        count_valid += 1;
                        true
                    } else if v == 0 {
                        true
                    } else {
                        false
                    }
                }) {
                    if count_valid == 0 {
                        vec![0.0; sub_allocs.len()]
                    } else {
                        let one = (length - tmp_primary_length) / count_valid as f32;
                        sub_allocs.iter().map(|&v| v as f32 * one).collect()
                    }
                } else {
                    config
                        .get_trans_and_interval(
                            axis,
                            length - tmp_primary_length,
                            sub_allocs,
                            &vec![],
                            Some(all_tfv.level),
                            None,
                        )
                        .unwrap()
                        .0
                        .assign
                }
            };
            *primary_area_length.hv_get_mut(axis) = tmp_primary_length;
        }

        let primary_tvs = comps[0].allocation(real_size[0], &config, level).unwrap();
        if let Self::Single {
            trans: Some(trans), ..
        } = &mut comps[0].last_comp_mut()
        {
            Axis::list().for_each(|axis| {
                let trans = trans.hv_get_mut(axis);
                trans
                    .assign
                    .iter_mut()
                    .rev()
                    .zip(sub_areas_assign.hv_get(axis).iter().rev())
                    .for_each(|(a, b)| {
                        *a = *b;
                    });
                trans.length = trans.assign.iter().sum();
            });
        }
        let secondary_tvs = comps[1].allocation(real_size[1], &config, level).unwrap();
        comps.iter_mut().for_each(|c| c.rotate(4 - quater));

        let tvs = Axis::hv_data().map(|&axis| {
            if *use_other.hv_get(axis) {
                let primary_tvs = primary_tvs.hv_get(axis);
                let secondary_tvs = secondary_tvs.hv_get(axis);

                let split = sub_areas_assign.hv_get(axis).len();
                let assign: Vec<_> = primary_tvs.assign[0..primary_tvs.assign.len() - split]
                    .iter()
                    .chain(secondary_tvs.assign.iter())
                    .cloned()
                    .collect();
                let allocs = primary_tvs.allocs[0..primary_tvs.allocs.len() - split]
                    .iter()
                    .chain(secondary_tvs.allocs.iter())
                    .cloned()
                    .collect();

                TransformValue {
                    level: primary_tvs.level.max(secondary_tvs.level),
                    length: assign.iter().sum::<f32>() + *intervals_assign.hv_get(axis),
                    assign,
                    allocs,
                }
            } else {
                primary_tvs.hv_get(axis).clone()
            }
        });

        Ok((
            tvs,
            intervals.into_iter().collect(),
            intervals_assign
                .into_iter()
                .zip(primary_area_length.into_iter())
                .map(|(a, b)| a + b)
                .collect(),
        ))
    }

    fn last_comp_mut(&mut self) -> &mut Self {
        match self {
            Self::Single { .. } => self,
            Self::Complex { comps, .. } => comps.last_mut().unwrap().last_comp_mut(),
        }
    }

    fn last_comp(&self) -> &Self {
        match self {
            Self::Single { .. } => self,
            Self::Complex { comps, .. } => comps.last().unwrap().last_comp(),
        }
    }

    // ⿸
    fn surround_allocs(
        &self,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Result<((Vec<usize>, Vec<i32>), Vec<usize>), Error> {
        match self {
            Self::Single { cache, .. } => {
                let area = *cache
                    .view
                    .surround_area()
                    .map_err(|e| Error::Message(format!("{:?}", e)))?
                    .hv_get(axis);
                let (mut allocs, intervals) = self.axis_allocs(axis, config)?;
                let sub_alloc = allocs.split_off(area);
                Ok(((allocs, intervals), sub_alloc))
            }
            Self::Complex { comps, format, .. } => {
                let mut other = match format {
                    Format::LeftToMiddleAndRight | Format::LeftToRight => {
                        if axis == Axis::Horizontal {
                            let mut list = vec![];
                            for c in comps[0..comps.len() - 1].iter() {
                                list.push(c.axis_allocs(axis, config)?);
                            }

                            let mut axis_intervals =
                                Self::axis_comps_intervals(comps, axis, &config);
                            let mut other = list
                                .into_iter()
                                .reduce(|mut a, mut b| {
                                    a.0.append(&mut b.0);
                                    a.1.append(&mut b.1);
                                    a
                                })
                                .unwrap_or_default();
                            other.1.append(&mut axis_intervals);
                            other
                        } else {
                            (vec![], vec![])
                        }
                    }
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                        if axis == Axis::Vertical {
                            let mut list = vec![];
                            for c in comps[0..comps.len() - 1].iter() {
                                list.push(c.axis_allocs(axis, config)?);
                            }

                            let mut axis_intervals =
                                Self::axis_comps_intervals(comps, axis, &config);
                            let mut other = list
                                .into_iter()
                                .reduce(|mut a, mut b| {
                                    a.0.append(&mut b.0);
                                    a.1.append(&mut b.1);
                                    a
                                })
                                .unwrap_or_default();
                            other.1.append(&mut axis_intervals);
                            other
                        } else {
                            (vec![], vec![])
                        }
                    }
                    _ => unreachable!(),
                };
                let ((mut alloc, mut intervals_new), sub_alloc) =
                    comps.last().unwrap().surround_allocs(axis, config)?;
                alloc.append(&mut other.0);
                intervals_new.append(&mut other.1);

                Ok(((alloc, intervals_new), sub_alloc))
            }
        }
    }

    // ⿸
    fn surround_interval(
        primary_comp: &StrucComb,
        secondary_comp: &StrucComb,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Result<i32, super::Error> {
        Self::surround_read_connect(primary_comp, secondary_comp, axis, config).map(|connect| {
            for wr in &config.interval_rule {
                if wr.regex.is_match(&connect) {
                    return wr.weight;
                }
            }
            0
        })
    }

    // ⿸
    fn surround_read_connect(
        primary_comp: &StrucComb,
        secondary_comp: &StrucComb,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Result<String, super::Error> {
        let axis_symbol = match axis {
            Axis::Horizontal => 'h',
            Axis::Vertical => 'v',
        };
        let connect = format!(
            "{axis_symbol}:{}{axis_symbol}:{}",
            primary_comp.surround_read_edge(axis)?,
            secondary_comp.axis_read_edge(
                axis,
                Place::Start,
                secondary_comp.is_zero_length(axis),
                0,
                0,
                0,
                config
            )
        );
        Ok(connect)
    }

    // ⿸
    fn surround_read_edge(&self, axis: Axis) -> Result<String, super::Error> {
        match self {
            Self::Single { cache, .. } => {
                let area = cache.view.surround_area()?;
                let start = *cache
                    .view
                    .real
                    .hv_get(axis.inverse())
                    .get(*area.hv_get(axis.inverse()))
                    .unwrap();
                let end = *cache.view.real.hv_get(axis.inverse()).last().unwrap();
                Ok(cache.view.get_sub_space_attr(
                    axis,
                    start,
                    end,
                    *cache
                        .view
                        .real
                        .hv_get(axis)
                        .get(*area.hv_get(axis))
                        .unwrap(),
                    Place::End,
                    0,
                ))
            }
            Self::Complex { comps, .. } => comps.last().unwrap().surround_read_edge(axis),
        }
    }

    fn rotate(&mut self, quarter: usize) {
        use euclid::*;

        match self {
            Self::Single {
                limit,
                cache,
                trans,
                rotate,
                ..
            } => {
                cache.proto.rotate(quarter);

                let mut quater = quarter % 4;
                *rotate = (*rotate + quarter) % 4;
                while quater != 0 {
                    *limit = limit.map(|limit| Size2D::new(limit.height, limit.width));
                    std::mem::swap(&mut cache.allocs.v, &mut cache.allocs.h);
                    cache.allocs.v.reverse();
                    trans.as_mut().map(|trans| {
                        std::mem::swap(&mut trans.v, &mut trans.h);
                        trans.v.allocs.reverse();
                        trans.v.assign.reverse();
                        trans
                    });

                    quater -= 1;
                }
                cache.attrs = cache.proto.attributes();
                cache.view = StrucAttrView::new(&cache.proto);
            }
            Self::Complex {
                format,
                comps,
                limit,
                intervals,
                assign_intervals,
                ..
            } => {
                let mut quarter = quarter % 4;
                match format {
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow if quarter > 1 => {
                        comps.reverse();
                        intervals.reverse();
                        assign_intervals.reverse();
                    }
                    Format::LeftToRight | Format::LeftToMiddleAndRight
                        if 0 < quarter && quarter < 3 =>
                    {
                        comps.reverse();
                        intervals.reverse();
                        assign_intervals.reverse();
                    }
                    _ => {}
                }
                *format = format.rotate(quarter);
                comps.iter_mut().for_each(|c| c.rotate(quarter));
                while quarter != 0 {
                    *limit = limit.map(|limit| Size2D::new(limit.height, limit.width));
                    quarter -= 1;
                }
            }
        }
    }

    fn allocation_axis(
        comps: &mut Vec<StrucComb>,
        size_limit: WorkSize,
        config: &ComponetConfig,
        axis: Axis,
        level: DataHV<Option<usize>>,
    ) -> Result<(DataHV<TransformValue>, Vec<i32>, Vec<f32>), Error> {
        // if let Axis::Horizontal = axis {
        //     Self::axis_reduce_comps(comps, axis, &config.reduce_check);
        // }

        let mut allocs: Vec<usize> = Default::default();
        let mut segments: Vec<usize> = Default::default();
        let mut intervals: Vec<i32> = Default::default();
        let mut comp_intervals: Vec<Vec<i32>> = Default::default();

        let mut reduce_checks: HashMap<usize, Vec<&regex::Regex>> = Default::default();
        config.reduce_checks.iter().for_each(|wr| {
            reduce_checks
                .entry(wr.weight)
                .and_modify(|list| list.push(&wr.regex))
                .or_insert(vec![&wr.regex]);
        });
        let level_info: Vec<(f32, Vec<&regex::Regex>)> = config
            .min_values
            .hv_get(axis)
            .iter()
            .enumerate()
            .map(|(i, v)| (*v, reduce_checks.get(&i).cloned().unwrap_or_default()))
            .collect();

        let length = *size_limit.hv_get(axis);
        let mut cur_level = 0;
        while cur_level != level_info.len() {
            let (min, regexs) = level_info.get(cur_level).unwrap();

            if let Some(e) = comps.iter().find_map(|c| match c.detection() {
                Err(e) => Some(Err(e)),
                Ok(_) => None,
            }) {
                return e;
            }

            intervals = Self::axis_comps_intervals(comps, axis, &config);

            segments.clear();
            allocs.clear();
            comp_intervals.clear();
            for c in comps.iter() {
                let mut c_info = c.axis_allocs(axis, config)?;
                segments.push(c_info.0.len());
                allocs.append(&mut c_info.0);
                comp_intervals.push(c_info.1);
            }

            let allocs_count = config.get_base_total(axis, &allocs)
                + config.get_interval_base_total(
                    axis,
                    &comp_intervals.iter().flatten().copied().collect(),
                )
                + config.get_interval_base_total(axis, &intervals);
            if allocs_count == 0.0 {
                break;
            } else {
                if allocs_count * min <= length {
                    break;
                } else if regexs.iter().fold(false, |ok, regex| {
                    Self::axis_reduce_comps(comps, axis, regex, config) | ok
                }) {
                    continue;
                } else {
                    if cur_level + 1 == level_info.len() {
                        return Err(Error::Transform {
                            alloc_len: allocs.iter().sum(),
                            length,
                            min: *config
                                .min_values
                                .hv_get(axis)
                                .last()
                                .unwrap_or(&TransformValue::DEFAULT_MIN_VALUE),
                        });
                    } else {
                        cur_level += 1;
                    }
                }
            }
        }

        let (mut primary_tfv, mut intervals_assign, _) = config
            .get_trans_and_interval(
                axis,
                length,
                allocs,
                &intervals
                    .iter()
                    .chain(comp_intervals.iter().flatten())
                    .copied()
                    .collect(),
                *level.hv_get(axis),
                None,
            )
            .unwrap();
        let comp_intervals: Vec<f32> = comp_intervals
            .iter()
            .map(|ci| {
                intervals_assign
                    .split_off(intervals_assign.len() - ci.len())
                    .iter()
                    .sum::<f32>()
            })
            .collect();

        let mut secondary_tfv = TransformValue::default();
        let mut primary_assign = primary_tfv.assign;
        let mut level_limit = level;
        if primary_tfv.level != 0 {
            *level_limit.hv_get_mut(axis) = Some(primary_tfv.level);
        }
        primary_tfv = TransformValue::default();
        for ((comp, n), interval) in comps
            .iter_mut()
            .zip(segments)
            .zip(comp_intervals.iter().chain(std::iter::once(&0.0)))
        {
            let assigns: Vec<f32> = primary_assign.drain(0..n).collect();

            let mut size_limit = size_limit;
            *size_limit.hv_get_mut(axis) = assigns.iter().sum::<f32>() + interval;

            let tfv = comp.allocation(size_limit, config, level_limit).unwrap();

            let sub_primary = tfv.hv_get(axis);
            primary_tfv.allocs.extend(&sub_primary.allocs);
            primary_tfv.assign.extend(&sub_primary.assign);
            primary_tfv.length += sub_primary.length;
            primary_tfv.level = primary_tfv.level.max(sub_primary.level);

            let sub_secondary_tfv = tfv.hv_get(axis.inverse());
            if secondary_tfv.length < sub_secondary_tfv.length {
                secondary_tfv = sub_secondary_tfv.clone();
            } else {
                match (secondary_tfv.allocs.len(), sub_secondary_tfv.allocs.len()) {
                    (a, b) if a < b => {
                        secondary_tfv = sub_secondary_tfv.clone();
                    }
                    (a, b) if a == b => {
                        if secondary_tfv.allocs.iter().sum::<usize>()
                            < sub_secondary_tfv.allocs.iter().sum::<usize>()
                        {
                            secondary_tfv = sub_secondary_tfv.clone();
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut tfv = DataHV::<TransformValue>::default();
        *tfv.hv_get_mut(axis) = primary_tfv;
        *tfv.hv_get_mut(axis.inverse()) = secondary_tfv;

        Ok((tfv, intervals, intervals_assign))
    }

    pub fn axis_length(&self, axis: Axis) -> f32 {
        fn all(comps: &Vec<StrucComb>, axis: Axis) -> f32 {
            comps.iter().map(|c| c.axis_length(axis)).sum()
        }

        fn one(comps: &Vec<StrucComb>, axis: Axis) -> f32 {
            comps
                .iter()
                .map(|c| c.axis_length(axis))
                .reduce(f32::max)
                .unwrap_or_default()
        }

        match self {
            Self::Single { trans, .. } => {
                trans
                    .as_ref()
                    .expect("Unallocate transform value!")
                    .hv_get(axis)
                    .length
            }
            Self::Complex {
                comps,
                format,
                assign_intervals,
                ..
            } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis) + assign_intervals.iter().sum::<f32>(),
                    Axis::Vertical => one(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis) + assign_intervals.iter().sum::<f32>(),
                    Axis::Horizontal => one(comps, axis),
                },
                _ => comps[0].axis_length(axis),
            },
        }
    }

    fn axis_reduce(&mut self, axis: Axis, regex: &regex::Regex, config: &ComponetConfig) -> bool {
        fn one(
            comps: &mut Vec<StrucComb>,
            axis: Axis,
            regex: &regex::Regex,
            config: &ComponetConfig,
        ) -> bool {
            let list: Vec<(f32, usize)> = comps
                .iter_mut()
                .enumerate()
                .map(|(i, c)| (c.axis_base_total(axis, config), i))
                .collect();
            let max = list
                .iter()
                .max_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap())
                .map(|(n, _)| *n);
            max.and_then(|max| {
                list.into_iter().fold(None, |ok, (n, i)| {
                    if n == max && comps[i].axis_reduce(axis, regex, config) {
                        Some(1)
                    } else {
                        ok
                    }
                })
            })
            .is_some()
        }

        match self {
            Self::Single { cache, .. } => cache.reduce(axis, regex),
            Self::Complex {
                comps,
                format,
                intervals,
                assign_intervals,
                ..
            } => {
                let ok = match format {
                    Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                        Axis::Horizontal => Self::axis_reduce_comps(comps, axis, regex, config),
                        Axis::Vertical => one(comps, axis, regex, config),
                    },
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                        Axis::Vertical => Self::axis_reduce_comps(comps, axis, regex, config),
                        Axis::Horizontal => one(comps, axis, regex, config),
                    },
                    _ => (0..comps.len())
                        .find(|i| comps[*i].axis_reduce(axis, regex, config))
                        .is_some(),
                };

                if ok {
                    intervals.clear();
                    assign_intervals.clear();
                }
                ok
            }
        }
    }

    fn surround_reduce_comps(
        _comps: &mut Vec<StrucComb>,
        _axis: Axis,
        _regex: &regex::Regex,
        _config: &ComponetConfig,
    ) -> bool {
        todo!()
    }

    fn axis_reduce_comps(
        comps: &mut Vec<StrucComb>,
        axis: Axis,
        regex: &regex::Regex,
        config: &ComponetConfig,
    ) -> bool {
        let mut list: Vec<(f32, usize)> = comps
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (c.axis_base_total(axis, config), i))
            .collect();
        list.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
        list.into_iter()
            .rev()
            .fold(None, |mut r, (n, i)| {
                if r.is_some() {
                    if r.unwrap() == n {
                        comps[i].axis_reduce(axis, regex, config);
                    }
                } else {
                    match comps[i].axis_reduce(axis, regex, config) {
                        true => r = Some(n),
                        false => {}
                    }
                }
                r
            })
            .is_some()
    }

    // todo： 对intervals缓存优化
    fn axis_allocs(
        &self,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Result<(Vec<usize>, Vec<i32>), Error> {
        fn all(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
        ) -> Result<(Vec<usize>, Vec<i32>), Error> {
            let mut intervals = StrucComb::axis_comps_intervals(comps, axis, &config);
            let mut allocs = vec![];
            for c in comps.iter() {
                let (mut c_allocs, mut c_intervals) = c.axis_allocs(axis, config)?;
                allocs.append(&mut c_allocs);
                intervals.append(&mut c_intervals);
            }
            Ok((allocs, intervals))
        }

        fn one(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
        ) -> Result<(Vec<usize>, Vec<i32>), Error> {
            let mut rs = vec![];
            for c in comps.iter() {
                let alloc = c.axis_allocs(axis, config)?;
                let length = config.get_base_total(axis, &alloc.0)
                    + config.get_interval_base_total(axis, &alloc.1);
                rs.push((alloc, length));
            }

            Ok(rs
                .into_iter()
                .reduce(
                    |item1, item2| match item1.1.partial_cmp(&item2.1).unwrap() {
                        Ordering::Less => item2,
                        Ordering::Greater => item1,
                        Ordering::Equal => match item1.0 .0.len().cmp(&item2.0 .0.len()) {
                            Ordering::Less => item2,
                            _ => item1,
                        },
                    },
                )
                .unwrap()
                .0)
        }

        match self {
            Self::Single { cache, .. } => Ok((cache.allocs.hv_get(axis).clone(), vec![])),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis, config),
                    Axis::Vertical => one(comps, axis, config),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis, config),
                    Axis::Horizontal => one(comps, axis, config),
                },
                Format::SurroundFromUpperLeft
                | Format::SurroundFromUpperRight
                | Format::SurroundFromLowerLeft
                | Format::SurroundFromLowerRight => {
                    let quarter = format.rotate_to_surround_tow();
                    let (axis, config) = if quarter % 2 == 1 {
                        (axis.inverse(), config.vh())
                    } else {
                        (axis, config.clone())
                    };
                    let mut comps = comps.clone();
                    comps.iter_mut().for_each(|c| c.rotate(quarter));

                    let ((alloc1, interval1), alloc1_sub) =
                        match comps[0].surround_allocs(axis, &config) {
                            Ok(r) => r,
                            Err(_) => {
                                return Err(Error::Surround(
                                    *format,
                                    comps[0].name().to_string(),
                                    comps[1].name().to_string(),
                                ))
                            }
                        };
                    let (alloc2, interval2) = comps[1].axis_allocs(axis, &config)?;
                    let interval =
                        Self::surround_interval(&comps[0], &comps[1], axis, &config).unwrap();

                    let val1 = config.get_base_total(axis, &alloc1_sub);
                    let val2 = config.get_base_total(axis, &alloc2)
                        + config.get_interval_base_total(axis, &interval2)
                        + config.get_interval_value(axis, interval);
                    if val1 > val2 {
                        Ok((alloc1.into_iter().chain(alloc1_sub).collect(), interval1))
                    } else {
                        Ok((
                            alloc1.into_iter().chain(alloc2).collect(),
                            interval1
                                .into_iter()
                                .chain(std::iter::once(interval))
                                .chain(interval2)
                                .collect(),
                        ))
                    }
                }
                _ => unreachable!(),
            },
        }
    }

    pub fn axis_base_total(&self, axis: Axis, config: &ComponetConfig) -> f32 {
        fn all(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
            intervals: &Vec<i32>,
        ) -> f32 {
            let interval_val = if intervals.is_empty() {
                config.get_interval_base_total(
                    axis,
                    &StrucComb::axis_comps_intervals(comps, axis, &config),
                )
            } else {
                config.get_interval_base_total(axis, intervals)
            };
            comps
                .iter()
                .map(|c| c.axis_base_total(axis, config))
                .sum::<f32>()
                + interval_val
        }

        fn one(comps: &Vec<StrucComb>, axis: Axis, config: &ComponetConfig) -> f32 {
            comps
                .iter()
                .map(|c| c.axis_base_total(axis, config))
                .reduce(f32::max)
                .unwrap_or_default()
        }

        match self {
            Self::Single { cache, .. } => config.get_base_total(axis, cache.allocs.hv_get(axis)),
            Self::Complex {
                comps,
                format,
                intervals,
                ..
            } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis, config, intervals),
                    Axis::Vertical => one(comps, axis, config),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis, config, intervals),
                    Axis::Horizontal => one(comps, axis, config),
                },
                Format::SurroundFromUpperLeft
                | Format::SurroundFromUpperRight
                | Format::SurroundFromLowerLeft
                | Format::SurroundFromLowerRight => {
                    let quarter = format.rotate_to_surround_tow();
                    let axis = if quarter % 2 == 1 {
                        axis.inverse()
                    } else {
                        axis
                    };
                    let comps: Vec<_> = comps
                        .iter()
                        .cloned()
                        .map(|mut c| {
                            c.rotate(quarter);
                            c
                        })
                        .collect();
                    let interval = if intervals.is_empty() {
                        Self::surround_interval(&comps[0], &comps[1], axis, &config).unwrap()
                    } else {
                        match axis {
                            Axis::Horizontal => intervals[0],
                            Axis::Vertical => intervals[1],
                        }
                    };

                    let ((mut allocs, tmp_intervals1), mut tmp_sub_allocs) =
                        comps[0].surround_allocs(axis, &config).unwrap();
                    let (mut tmp_allocs2, tmp_intervals2) =
                        comps[1].axis_allocs(axis, &config).unwrap();

                    let self_value = config.get_base_total(axis, &tmp_sub_allocs);
                    let other_value = config.get_base_total(axis, &tmp_allocs2)
                        + config.get_interval_value(axis, interval)
                        + config.get_interval_base_total(axis, &tmp_intervals2);

                    let interval_total = if self_value < other_value {
                        allocs.append(&mut tmp_allocs2);
                        [tmp_intervals1, tmp_intervals2]
                            .iter()
                            .map(|v| config.get_interval_base_total(axis, v))
                            .sum::<f32>()
                            + config.get_interval_value(axis, interval)
                    } else {
                        allocs.append(&mut tmp_sub_allocs);
                        config.get_interval_base_total(axis, &tmp_intervals1)
                            + config.get_interval_value(axis, interval)
                    };
                    config.get_base_total(axis, &allocs) + interval_total
                }
                _ => unreachable!(),
            },
        }
    }

    fn axis_comps_intervals(
        comps: &Vec<StrucComb>,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Vec<i32> {
        Self::axis_read_connect(comps, axis, config)
            .iter()
            .map(|connect| {
                for wr in &config.interval_rule {
                    if wr.regex.is_match(connect) {
                        return wr.weight;
                    }
                }
                0
            })
            .collect()
    }

    pub fn read_connect(
        comps: &Vec<StrucComb>,
        format: Format,
        config: &ComponetConfig,
    ) -> Vec<String> {
        match format {
            Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                Self::axis_read_connect(comps, Axis::Vertical, config)
            }
            Format::LeftToMiddleAndRight | Format::LeftToRight => {
                Self::axis_read_connect(comps, Axis::Horizontal, config)
            }
            Format::SurroundFromUpperLeft => vec![
                Self::surround_read_connect(&comps[0], &comps[1], Axis::Horizontal, config)
                    .unwrap_or_default(),
                Self::surround_read_connect(&comps[0], &comps[1], Axis::Vertical, config)
                    .unwrap_or_default(),
            ],
            Format::SurroundFromUpperRight | Format::SurroundFromLowerLeft => {
                let quarter = format.rotate_to_surround_tow();
                let comps: Vec<_> = comps
                    .iter()
                    .cloned()
                    .map(|mut c| {
                        c.rotate(quarter);
                        c
                    })
                    .collect();
                vec![
                    Self::surround_read_connect(&comps[0], &comps[1], Axis::Horizontal, config)
                        .unwrap_or_default(),
                    Self::surround_read_connect(&comps[0], &comps[1], Axis::Vertical, config)
                        .unwrap_or_default(),
                ]
            }
            _ => unreachable!(),
        }
    }

    fn axis_read_connect(
        comps: &Vec<StrucComb>,
        axis: Axis,
        config: &ComponetConfig,
    ) -> Vec<String> {
        comps
            .iter()
            .zip(comps.iter().skip(1))
            .map(|(comp1, comp2)| {
                let axis_symbol = match axis {
                    Axis::Horizontal => 'h',
                    Axis::Vertical => 'v',
                };
                format!(
                    "{axis_symbol}:{}{axis_symbol}:{}",
                    comp1.axis_read_edge(
                        axis,
                        Place::End,
                        comp1.is_zero_length(axis),
                        0,
                        0,
                        0,
                        config
                    ),
                    comp2.axis_read_edge(
                        axis,
                        Place::Start,
                        comp2.is_zero_length(axis),
                        0,
                        0,
                        0,
                        config
                    )
                )
            })
            .collect()
    }

    fn axis_subspaces_total(&self, axis: Axis, config: &ComponetConfig) -> usize {
        let (al, il) = self.axis_allocs(axis, config).unwrap();
        (al.len() as i32 + il.iter().sum::<i32>()) as usize
    }

    fn axis_read_edge(
        &self,
        axis: Axis,
        place: Place,
        zero_length: bool,
        start: usize,
        discard: usize,
        other_length: usize,
        config: &ComponetConfig,
    ) -> String {
        fn all(
            comps: &Vec<StrucComb>,
            axis: Axis,
            place: Place,
            zero_length: bool,
            start: usize,
            discard: usize,
            other_length: usize,
            config: &ComponetConfig,
        ) -> String {
            let mut limits = vec![(0, 0); comps.len()];
            *limits.first_mut().unwrap() = (start, 0);
            *limits.last_mut().unwrap() = (limits.last().unwrap().0, discard);

            comps
                .iter()
                .zip(limits)
                .filter_map(|(c, (s, d))| {
                    if c.is_zero_length(axis) && !zero_length {
                        None
                    } else {
                        Some(c.axis_read_edge(axis, place, zero_length, s, d, other_length, config))
                    }
                })
                .collect()
        }

        fn one(
            comps: &Vec<StrucComb>,
            axis: Axis,
            place: Place,
            start: usize,
            discard: usize,
            other_length: usize,
            config: &ComponetConfig,
        ) -> String {
            let (vc, other) = match place {
                Place::Start => (comps.first().unwrap(), comps[0..].iter()),
                Place::End => (comps.last().unwrap(), comps[..comps.len() - 1].iter()),
            };
            let other_l = other
                .map(|c| c.axis_subspaces_total(axis, config))
                .sum::<usize>();

            vc.axis_read_edge(
                axis,
                place,
                vc.is_zero_length(axis),
                start,
                discard,
                other_length + other_l,
                config,
            )
        }

        match self {
            Self::Single { cache, .. } => {
                let segment = match place {
                    Place::Start => cache.view.real.hv_get(axis).first().unwrap(),
                    Place::End => cache.view.real.hv_get(axis).last().unwrap(),
                };
                let real_list = cache.view.real.hv_get(axis.inverse());
                cache.view.get_sub_space_attr(
                    axis,
                    *real_list.get(start).unwrap(),
                    *real_list.get(real_list.len() - discard - 1).unwrap(),
                    *segment,
                    place,
                    other_length,
                )
            }
            Self::Complex { format, comps, .. } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => one(comps, axis, place, start, discard, other_length, config),
                    Axis::Horizontal => all(
                        comps,
                        axis,
                        place,
                        zero_length,
                        start,
                        discard,
                        other_length,
                        config,
                    ),
                },
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => {
                        one(comps, axis, place, start, discard, other_length, config)
                    }
                    Axis::Vertical => all(
                        comps,
                        axis,
                        place,
                        zero_length,
                        start,
                        discard,
                        other_length,
                        config,
                    ),
                },
                Format::SurroundFromUpperLeft
                | Format::SurroundFromUpperRight
                | Format::SurroundFromLowerLeft
                | Format::SurroundFromLowerRight => {
                    if match (format, axis, place) {
                        (Format::SurroundFromUpperLeft, _, Place::Start)
                        | (Format::SurroundFromUpperRight, Axis::Vertical, Place::Start)
                        | (Format::SurroundFromUpperRight, Axis::Horizontal, Place::End)
                        | (Format::SurroundFromLowerRight, _, Place::End)
                        | (Format::SurroundFromLowerLeft, Axis::Horizontal, Place::Start)
                        | (Format::SurroundFromLowerLeft, Axis::Vertical, Place::End) => true,
                        _ => false,
                    } {
                        return comps[0].axis_read_edge(
                            axis,
                            place,
                            false,
                            start,
                            discard,
                            other_length + comps[1].axis_subspaces_total(axis, config),
                            config,
                        );
                    } else {
                        let quarter = format.rotate_to_surround_tow();
                        let mut rotate_comp = comps[0].clone();
                        rotate_comp.rotate(quarter);

                        match rotate_comp.last_comp() {
                            Self::Single { cache, .. } => {
                                let mut area = cache.view.surround_area().unwrap();
                                let mut real_list = cache.view.real.hv_get(axis.inverse());

                                if quarter % 2 == 1 {
                                    area = area.vh();
                                    real_list = cache.view.real.hv_get(axis)
                                }

                                match (format, axis, place) {
                                    (Format::SurroundFromUpperLeft, _, Place::End)
                                    | (
                                        Format::SurroundFromUpperRight,
                                        Axis::Horizontal,
                                        Place::Start,
                                    )
                                    | (
                                        Format::SurroundFromLowerLeft,
                                        Axis::Vertical,
                                        Place::Start,
                                    ) => {
                                        let split =
                                            real_list.len() - area.hv_get(axis.inverse()) - 1;
                                        comps[0].axis_read_edge(
                                            axis,
                                            place,
                                            zero_length,
                                            start,
                                            split,
                                            other_length,
                                            config,
                                        ) + &comps[1].axis_read_edge(
                                            axis,
                                            place,
                                            zero_length,
                                            0,
                                            discard,
                                            other_length,
                                            config,
                                        )
                                    }
                                    (Format::SurroundFromLowerRight, _, Place::Start)
                                    | (
                                        Format::SurroundFromUpperRight,
                                        Axis::Vertical,
                                        Place::End,
                                    )
                                    | (
                                        Format::SurroundFromLowerLeft,
                                        Axis::Horizontal,
                                        Place::End,
                                    ) => {
                                        let split =
                                            real_list.len() - area.hv_get(axis.inverse()) - 1;
                                        comps[1].axis_read_edge(
                                            axis,
                                            place,
                                            zero_length,
                                            start,
                                            0,
                                            other_length,
                                            config,
                                        ) + &comps[0].axis_read_edge(
                                            axis,
                                            place,
                                            zero_length,
                                            split,
                                            discard,
                                            other_length,
                                            config,
                                        )
                                    }
                                    _ => unreachable!(),
                                }
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                _ => unreachable!(),
            },
        }
    }

    fn is_zero_length(&self, axis: Axis) -> bool {
        fn find(comps: &Vec<StrucComb>, axis: Axis) -> bool {
            comps.iter().all(|c| c.is_zero_length(axis))
        }

        match self {
            Self::Single { cache, .. } => cache.allocs.hv_get(axis).is_empty(),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => false,
                    Axis::Vertical => find(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => false,
                    Axis::Horizontal => find(comps, axis),
                },
                _ => false,
            },
        }
    }

    // fn subarea_count(&self, axis: Axis) -> usize {
    //     fn all(comps: &Vec<StrucComb>, axis: Axis) -> usize {
    //         comps.iter().map(|c| c.subarea_count(axis)).sum::<usize>()
    //     }

    //     fn one(comps: &Vec<StrucComb>, axis: Axis) -> usize {
    //         comps
    //             .iter()
    //             .map(|c| c.subarea_count(axis))
    //             .max()
    //             .unwrap_or_default()
    //     }

    //     match self {
    //         Self::Single { cache, .. } => cache.allocs.hv_get(axis).len(),
    //         Self::Complex { comps, format, .. } => match format {
    //             Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
    //                 Axis::Horizontal => all(comps, axis),
    //                 Axis::Vertical => one(comps, axis),
    //             },
    //             Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
    //                 Axis::Vertical => all(comps, axis),
    //                 Axis::Horizontal => one(comps, axis),
    //             },
    //             Format::SurroundFromLowerLeft
    //             | Format::SurroundFromLowerRight
    //             | Format::SurroundFromUpperLeft
    //             | Format::SurroundFromUpperRight => {
    //                 let quarter = format.rotate_to_surround_tow();
    //                 let new_comps: Vec<_> = comps
    //                     .iter()
    //                     .cloned()
    //                     .map(|mut c| {
    //                         c.rotate(quarter);
    //                         c
    //                     })
    //                     .collect();
    //                 let axis = if quarter % 2 == 1 {
    //                     axis.inverse()
    //                 } else {
    //                     axis
    //                 };

    //                 match new_comps[0] {
    //                     Self::Single { cache, .. } => cache.view.surround_area().unwrap().hv_get(axis),
    //                     Self::Complex { comps, .. } => {
    //                         comps[0..comps.len()].iter().map(|c| c.subarea_count(axis)).sum::<usize>()
    //                         + comps
    //                     }
    //                 };

    //                 todo!()
    //             }
    //             _ => unreachable!(),
    //         },
    //     }
    // }

    pub fn stroke_types(&self, list: BTreeSet<String>) -> BTreeSet<String> {
        match self {
            Self::Single { cache, .. } => {
                cache.proto.key_paths.iter().fold(list, |mut list, path| {
                    let stroke = path.stroke_type();
                    if stroke.len() != 0 {
                        list.insert(stroke);
                    }
                    list
                })
            }
            Self::Complex { comps, .. } => {
                comps.iter().fold(list, |list, cp| cp.stroke_types(list))
            }
        }
    }
}
