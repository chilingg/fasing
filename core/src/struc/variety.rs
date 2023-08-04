use crate::{
    construct::{self, Component, Format},
    fas_file::{
        AllocateRule, AllocateTable, ComponetConfig, Error, StrokeMatch, StrokeReplace, WeightRegex,
    },
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

pub enum StrucComb {
    Single {
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
        if let Some(map_name) = config
            .replace_list
            .get(&Format::Single)
            .and_then(|fs| fs.get(&0).and_then(|is| is.get(&name)))
        {
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
            LeftToRight | LeftToMiddleAndRight | AboveToBelow | AboveToMiddleAndBelow => {
                let mut combs: Vec<StrucComb> = Vec::with_capacity(const_attrs.format.number_of());

                for (in_fmt, comp) in const_attrs.components.iter().enumerate() {
                    let (comp_name, comp_attrs) =
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

                    let limit = get_size_limit(&comp_name, const_attrs.format, in_fmt);
                    combs.push(StrucComb::from_format(
                        comp_name,
                        limit,
                        comp_attrs,
                        const_table,
                        // alloc_table,
                        components,
                        config,
                    )?);
                }

                let mut count: std::collections::HashMap<String, usize> = Default::default();
                combs.iter().for_each(|c| {
                    count
                        .entry(c.name().to_string())
                        .and_modify(|n| *n += 1)
                        .or_insert(1);
                });
                combs.iter_mut().for_each(|c| {
                    if count[c.name()] > 1 {
                        *c.get_limit_mut() = None;
                    }
                });

                Ok(StrucComb::from_complex(
                    const_attrs.format,
                    combs,
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
        let struc = self.to_work(offset, rect, min_values);
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
        min_values: &DataHV<Vec<f32>>,
    ) -> StrucWork {
        let mut struc = Default::default();
        self.merge(&mut struc, offset, rect, min_values);
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
                _ => Default::default(),
            },
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
                ..
            } => {
                let size_limit = if let Some(limit) = limit {
                    size_limit.min(*limit)
                } else {
                    size_limit
                };

                match format {
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
                    // Format::SurroundFromUpperRight
                    // | Format::SurroundFromUpperLeft
                    // | Format::SurroundFromLowerLeft => {
                    //     Self::allocation_surround_tow(comps, size_limit, config, level, *format)
                    //         .and_then(|(tfv, n_intervals, a_intervals)| {
                    //             *intervals = n_intervals;
                    //             *assign_intervals = a_intervals;
                    //             Ok(tfv)
                    //         })
                    // }
                    _ => Err(Error::Empty(format.to_symbol().unwrap().to_string())),
                }
            }
        }
    }

    // ⿸
    fn allocation_surround_tow(
        comps: &mut Vec<StrucComb>,
        size_limit: WorkSize,
        config: &ComponetConfig,
        level: DataHV<Option<usize>>,
        fmt: Format,
    ) -> Result<(DataHV<TransformValue>, Vec<f32>, Vec<f32>), Error> {
        // let quater = match fmt {
        //     Format::SurroundFromUpperRight => 3,
        //     Format::SurroundFromLowerLeft => 1,
        //     _ => 0,
        // };
        // comps.iter_mut().for_each(|c| c.rotate(quater));

        // let mut intervals: DataHV<f32> = Default::default();
        // let mut comp_intervals: DataHV<f32> = Default::default();
        // let mut allocs: DataHV<Vec<usize>> = Default::default();

        // for axis in Axis::list() {
        //     let mut reduce_checks: HashMap<usize, Vec<&regex::Regex>> = Default::default();
        //     config.reduce_checks.iter().for_each(|wr| {
        //         reduce_checks
        //             .entry(wr.weight)
        //             .and_modify(|list| list.push(&wr.regex))
        //             .or_insert(vec![&wr.regex]);
        //     });
        //     let level_info: Vec<(f32, Vec<&regex::Regex>)> = config
        //         .min_values
        //         .hv_get(axis)
        //         .iter()
        //         .enumerate()
        //         .map(|(i, v)| (*v, reduce_checks.get(&i).cloned().unwrap_or_default()))
        //         .collect();

        //     let length = *size_limit.hv_get(axis);
        //     let mut ok = false;
        //     for (min, regex) in level_info.into_iter() {
        //         loop {
        //             *intervals.hv_get_mut(axis) =
        //                 Self::surround_interval(&comps[0], &comps[1], axis, &config.interval_rule)
        //                     .map_err(|_| {
        //                         Error::Surround(
        //                             fmt,
        //                             comps[0].name().to_string(),
        //                             comps[1].name().to_string(),
        //                         )
        //                     })?;
        //             *comp_intervals.hv_get_mut(axis) = comps
        //                 .iter_mut()
        //                 .map(|c| c.axis_interval(axis, &config.interval_rule))
        //                 .sum();

        //             let comps_length =
        //                 length - (intervals.hv_get(axis) + comp_intervals.hv_get(axis)) * min;
        //             let primary_allocs = comps[0].surround_allocs(axis);
        //             let secondary_allocs = comps[1].axis_allocs(axis);

        //             todo!()
        //         }
        //     }
        // }

        // comps.iter_mut().for_each(|c| c.rotate(4 - quater));
        todo!()
    }

    // ⿸
    // fn surround_allocs(&self, axis: Axis) -> (Vec<usize>, Vec<usize>) {
    //     match self {
    //         Self::Single { cache, .. } => {
    //             let area = *cache.view.surround_area().unwrap().hv_get(axis);
    //             let mut alloc = self.axis_allocs(axis);
    //             let sub_alloc = alloc.split_off(area);
    //             (alloc, sub_alloc)
    //         }
    //         Self::Complex { comps, format, .. } => {
    //             let mut other: Vec<_> = match format {
    //                 Format::LeftToMiddleAndRight | Format::LeftToRight
    //                     if axis == Axis::Horizontal =>
    //                 {
    //                     comps[0..comps.len() - 1]
    //                         .iter()
    //                         .flat_map(|c| c.axis_allocs(axis))
    //                         .collect()
    //                 }
    //                 Format::AboveToBelow | Format::AboveToMiddleAndBelow
    //                     if axis == Axis::Vertical =>
    //                 {
    //                     comps[0..comps.len() - 1]
    //                         .iter()
    //                         .flat_map(|c| c.axis_allocs(axis))
    //                         .collect()
    //                 }
    //                 _ => vec![],
    //             };
    //             let (mut alloc, sub_alloc) = comps.last().unwrap().surround_allocs(axis);
    //             alloc.append(&mut other);

    //             (alloc, sub_alloc)
    //         }
    //     }
    // }

    // ⿸
    fn surround_interval(
        primary_comp: &StrucComb,
        secondary_comp: &StrucComb,
        axis: Axis,
        rules: &Vec<WeightRegex<i32>>,
    ) -> Result<i32, super::Error> {
        Self::surround_read_connect(primary_comp, secondary_comp, axis).map(|connect| {
            for wr in rules {
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
    ) -> Result<String, super::Error> {
        let axis_symbol = match axis {
            Axis::Horizontal => 'h',
            Axis::Vertical => 'v',
        };
        Ok(format!(
            "{axis_symbol}:{}{axis_symbol}:{}",
            primary_comp.surround_read_edge(axis)?,
            secondary_comp.axis_read_edge(axis, Place::Start, secondary_comp.is_zero_length(axis))
        ))
    }

    // ⿸
    fn surround_read_edge(&self, axis: Axis) -> Result<String, super::Error> {
        match self {
            Self::Single { cache, .. } => {
                let area = cache.view.surround_area()?;
                let start = *area.hv_get(axis);
                let end = *cache.view.real.hv_get(axis).last().unwrap();
                Ok(cache.view.get_sub_space_attr(
                    axis,
                    start,
                    end,
                    *area.hv_get(axis.inverse()),
                    Place::Start,
                ))
            }
            Self::Complex { comps, .. } => comps.last().unwrap().surround_read_edge(axis),
        }
    }

    fn rotate(&mut self, quater: usize) {
        use euclid::*;

        match self {
            Self::Single {
                limit,
                cache,
                trans,
                ..
            } => {
                cache.proto.rotate(quater);

                let mut quater = quater % 4;
                while quater != 0 {
                    *limit = limit.map(|limit| Size2D::new(limit.height, limit.width));
                    std::mem::swap(&mut cache.allocs.v, &mut cache.allocs.h);
                    cache.allocs.h.reverse();

                    quater -= 1;
                }
                cache.attrs = cache.proto.attributes();
                cache.view = StrucAttrView::new(&cache.proto);
                *trans = None;
            }
            Self::Complex {
                format,
                comps,
                limit,
                intervals,
                assign_intervals,
                ..
            } => {
                match format {
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow if quater % 4 == 1 => {
                        comps.reverse();
                        intervals.reverse();
                        assign_intervals.reverse();
                    }
                    Format::LeftToRight | Format::LeftToMiddleAndRight if quater % 4 == 2 => {
                        comps.reverse();
                        intervals.reverse();
                        assign_intervals.reverse();
                    }
                    _ => {}
                }
                let mut quater = quater % 4;
                while quater != 0 {
                    *limit = limit.map(|limit| Size2D::new(limit.height, limit.width));
                    quater -= 1;
                }
                *format = format.rotate(quater);
                todo!()
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
        if level_info
            .into_iter()
            .find(|(min, regexs)| loop {
                intervals = Self::axis_comps_intervals(comps, axis, &config.interval_rule);

                segments.clear();
                (allocs, comp_intervals) = comps
                    .iter()
                    .map(|c| {
                        let allocs = c.axis_allocs(axis, config);
                        segments.push(allocs.0.len());
                        allocs
                    })
                    .fold((vec![], vec![]), |(mut allocs, mut intervals), mut item| {
                        allocs.append(&mut item.0);
                        intervals.push(item.1);
                        (allocs, intervals)
                    });

                let allocs_count = config.base_total(axis, &allocs)
                    + config.interval_base_total(
                        axis,
                        &comp_intervals.iter().flatten().copied().collect(),
                    )
                    + config.interval_base_total(axis, &intervals);
                if allocs_count == 0.0 {
                    break true;
                } else {
                    if allocs_count * min <= length {
                        break true;
                    } else if regexs.iter().fold(false, |ok, regex| {
                        Self::axis_reduce_comps(comps, axis, regex) | ok
                    }) {
                        continue;
                    } else {
                        break false;
                    }
                }
            })
            .is_none()
        {
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

        let (mut primary_tfv, mut intervals_assign) = config
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
            )
            .unwrap();
        let comp_intervals = intervals_assign.split_off(intervals.len());

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
            .zip(comp_intervals.iter().chain(std::iter::repeat(&0.0)))
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
                _ => 0.0,
            },
        }
    }

    fn axis_reduce(&mut self, axis: Axis, regex: &regex::Regex) -> bool {
        fn one(comps: &mut Vec<StrucComb>, axis: Axis, regex: &regex::Regex) -> bool {
            let list: Vec<(usize, usize)> = comps
                .iter_mut()
                .enumerate()
                .map(|(i, c)| (c.subarea_count(axis), i))
                .collect();
            let max = list.iter().max_by_key(|(n, _)| *n).map(|(n, _)| *n);
            max.and_then(|max| {
                list.into_iter().fold(None, |ok, (n, i)| {
                    if n == max && comps[i].axis_reduce(axis, regex) {
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
                        Axis::Horizontal => Self::axis_reduce_comps(comps, axis, regex),
                        Axis::Vertical => one(comps, axis, regex),
                    },
                    Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                        Axis::Vertical => Self::axis_reduce_comps(comps, axis, regex),
                        Axis::Horizontal => one(comps, axis, regex),
                    },
                    _ => (0..comps.len())
                        .find(|i| comps[*i].axis_reduce(axis, regex))
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

    fn axis_reduce_comps(comps: &mut Vec<StrucComb>, axis: Axis, regex: &regex::Regex) -> bool {
        let mut list: Vec<(usize, usize)> = comps
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (c.subarea_count(axis), i))
            .collect();
        list.sort_by_key(|(n, _)| *n);
        list.into_iter()
            .rev()
            .fold(None, |mut r, (n, i)| {
                if r.is_some() {
                    if r.unwrap() == n {
                        comps[i].axis_reduce(axis, regex);
                    }
                } else {
                    match comps[i].axis_reduce(axis, regex) {
                        true => r = Some(n),
                        false => {}
                    }
                }
                r
            })
            .is_some()
    }

    fn axis_allocs(&self, axis: Axis, config: &ComponetConfig) -> (Vec<usize>, Vec<i32>) {
        fn all(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
        ) -> (Vec<usize>, Vec<i32>) {
            comps.iter().fold((vec![], vec![]), |(mut a, mut i), c| {
                let (mut allocs, mut intervals) = c.axis_allocs(axis, config);
                a.append(&mut allocs);
                i.append(&mut intervals);
                (a, i)
            })
        }

        fn one(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
        ) -> (Vec<usize>, Vec<i32>) {
            comps
                .iter()
                .map(|c| {
                    let alloc = c.axis_allocs(axis, config);
                    let length = config.base_total(axis, &alloc.0)
                        + config.interval_base_total(axis, &alloc.1);
                    (alloc, length)
                })
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
                .0
        }

        match self {
            Self::Single { cache, .. } => (cache.allocs.hv_get(axis).clone(), vec![]),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis, config),
                    Axis::Vertical => one(comps, axis, config),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis, config),
                    Axis::Horizontal => one(comps, axis, config),
                },
                _ => unreachable!(),
            },
        }
    }

    fn axis_base_total(&self, axis: Axis, config: &ComponetConfig) -> f32 {
        fn all(
            comps: &Vec<StrucComb>,
            axis: Axis,
            config: &ComponetConfig,
            intervals: &Vec<i32>,
        ) -> f32 {
            let interval_val = if intervals.is_empty() {
                config.interval_base_total(
                    axis,
                    &StrucComb::axis_comps_intervals(comps, axis, &config.interval_rule),
                )
            } else {
                config.interval_base_total(axis, intervals)
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
            Self::Single { cache, .. } => config.base_total(axis, cache.allocs.hv_get(axis)),
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
                _ => unreachable!(),
            },
        }
    }

    fn axis_comps_intervals(
        comps: &Vec<StrucComb>,
        axis: Axis,
        rules: &Vec<WeightRegex<i32>>,
    ) -> Vec<i32> {
        Self::axis_read_connect(comps, axis)
            .iter()
            .map(|connect| {
                for wr in rules {
                    if wr.regex.is_match(connect) {
                        return wr.weight;
                    }
                }
                0
            })
            .collect()
    }

    pub fn read_connect(comps: &Vec<StrucComb>, format: Format) -> Vec<String> {
        match format {
            Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                Self::axis_read_connect(comps, Axis::Vertical)
            }
            Format::LeftToMiddleAndRight | Format::LeftToRight => {
                Self::axis_read_connect(comps, Axis::Horizontal)
            }
            _ => unreachable!(),
        }
    }

    fn axis_read_connect(comps: &Vec<StrucComb>, axis: Axis) -> Vec<String> {
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
                    comp1.axis_read_edge(axis, Place::End, comp1.is_zero_length(axis)),
                    comp2.axis_read_edge(axis, Place::Start, comp2.is_zero_length(axis))
                )
            })
            .collect()
    }

    fn axis_read_edge(&self, axis: Axis, place: Place, zero_length: bool) -> String {
        fn all(comps: &Vec<StrucComb>, axis: Axis, place: Place, zero_length: bool) -> String {
            comps
                .iter()
                .filter_map(|c| {
                    if c.is_zero_length(axis) && !zero_length {
                        None
                    } else {
                        Some(c.axis_read_edge(axis, place, zero_length))
                    }
                })
                .collect()
        }

        fn one(comps: &Vec<StrucComb>, axis: Axis, place: Place) -> String {
            let vc = match place {
                Place::Start => comps.first(),
                Place::End => comps.last(),
            }
            .unwrap();

            format!(
                "{}{}",
                vc.axis_read_edge(axis, place, vc.is_zero_length(axis)),
                (1..comps.len()).map(|_| ';').collect::<String>()
            )
        }

        match self {
            Self::Single { cache, .. } => cache.view.get_space_attrs_in(axis, place),
            Self::Complex { format, comps, .. } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => one(comps, axis, place),
                    Axis::Horizontal => all(comps, axis, place, zero_length),
                },
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => one(comps, axis, place),
                    Axis::Vertical => all(comps, axis, place, zero_length),
                },
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

    fn subarea_count(&self, axis: Axis) -> usize {
        fn all(comps: &Vec<StrucComb>, axis: Axis) -> usize {
            comps.iter().map(|c| c.subarea_count(axis)).sum::<usize>()
        }

        fn one(comps: &Vec<StrucComb>, axis: Axis) -> usize {
            comps
                .iter()
                .map(|c| c.subarea_count(axis))
                .max()
                .unwrap_or_default()
        }

        match self {
            Self::Single { cache, .. } => cache.allocs.hv_get(axis).len(),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis),
                    Axis::Vertical => one(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis),
                    Axis::Horizontal => one(comps, axis),
                },
                _ => unreachable!(),
            },
        }
    }

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
