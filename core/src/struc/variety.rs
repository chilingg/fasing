use crate::{
    construct::{self, Component, Format},
    fas_file::{AllocateRule, AllocateTable, ComponetConfig, Error},
    hv::*,
    struc::{
        space::*, view::StrucAllAttrView, StrucAllocates, StrucAttributes, StrucProto, StrucWork,
    },
};

use once_cell::sync::Lazy;
use serde::Serialize;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

pub struct StrucDataCache {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub view: StrucAllAttrView,
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
            view: StrucAllAttrView::new(&proto),
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
            view: StrucAllAttrView::new(&proto),
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
            self.view = StrucAllAttrView::new(&self.proto);
            self.attrs = self.proto.attributes();
            true
        }
    }
}

#[derive(Clone, Default, Serialize)]
pub struct TransformValue {
    pub length: f32,
    pub level: usize,
    pub allocs: Vec<usize>,
    pub assign: Vec<f32>,
}

impl TransformValue {
    pub const DEFAULT_MIN_VALUE: f32 = 0.1;

    pub fn from_allocs(
        mut allocs: Vec<usize>,
        length: f32,
        assign_values: &Vec<f32>,
        min_values: &Vec<f32>,
    ) -> Result<Self, Error> {
        let mut alloc_max = allocs.iter().cloned().max().unwrap_or_default();
        let min = min_values.last().unwrap_or(&Self::DEFAULT_MIN_VALUE);

        if alloc_max == 0 {
            return Ok(Self {
                length,
                level: 0,
                assign: vec![0.0; allocs.len()],
                allocs,
            });
        }

        for _ in 1..=alloc_max {
            let assign: Vec<f32> = allocs
                .iter()
                .map(|&n| match n {
                    0 => 0.0,
                    n => assign_values
                        .get(n - 1)
                        .or(assign_values.last())
                        .cloned()
                        .unwrap_or(Self::DEFAULT_MIN_VALUE),
                })
                .collect();

            match assign.iter().sum::<f32>() {
                assign_length if assign_length * min <= length => {
                    let level = min_values
                        .iter()
                        .position(|assign| assign_length * assign <= length)
                        .unwrap();
                    let ratio = length / assign_length;

                    return Ok(Self {
                        level,
                        length,
                        allocs,
                        assign: assign.into_iter().map(|n| n * ratio).collect(),
                    });
                }
                _ => {
                    allocs.iter_mut().for_each(|n| {
                        if *n == alloc_max {
                            *n -= 1;
                        }
                    });
                    alloc_max -= 1;
                }
            }
        }

        Err(Error::Transform {
            alloc_len: allocs.iter().sum(),
            length,
            min: *min_values.last().unwrap_or(&Self::DEFAULT_MIN_VALUE),
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
        format: Format,
        comps: Vec<StrucComb>,
        limit: Option<WorkSize>,
    },
}

impl StrucComb {
    pub fn new(
        name: String,
        const_table: &construct::Table,
        // alloc_table: &AllocateTable,
        components: &BTreeMap<String, StrucProto>,
        config: &ComponetConfig,
    ) -> Result<Self, Error> {
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
                    let comp_name =
                        match get_real_name(comp.name().as_str(), const_attrs.format, in_fmt) {
                            Some(map_name) => map_name.to_owned(),
                            None => match comp {
                                Component::Char(comp_name) => comp_name.clone(),
                                Component::Complex(ref complex_attrs) => {
                                    format!("{}", complex_attrs)
                                }
                            },
                        };

                    let comp_attrs = get_const_attr(&comp_name);
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

    pub fn to_work(&self, offset: WorkPoint, rect: WorkRect) -> StrucWork {
        let mut struc = Default::default();
        self.merge(&mut struc, offset, rect);
        struc
    }

    pub fn merge(&self, struc: &mut StrucWork, offset: WorkPoint, rect: WorkRect) -> WorkSize {
        // fn merge_in_axis(
        //     comps: &Vec<StrucComb>,
        //     struc: &mut StrucWork,
        //     offset: WorkPoint,
        //     rect: WorkRect,
        //     axis: Axis,
        // ) -> WorkSize {
        //     let max_length = comps
        //         .iter()
        //         .map(|vc| vc.axis_length(axis.inverse()))
        //         .reduce(f32::max)
        //         .unwrap_or_default();
        //     let mut advence = WorkSize::zero();

        //     comps
        //         .iter()
        //         .fold(offset, |mut offset, vc| {
        //             let mut sub_offset = offset;
        //             *sub_offset.hv_get_mut(axis.inverse()) +=
        //                 (max_length - vc.axis_length(axis.inverse())) * 0.5;

        //             let sub_advence = vc.merge(struc, sub_offset, rect);
        //             *offset.hv_get_mut(axis) += sub_advence.hv_get(axis);

        //             *advence.hv_get_mut(axis.inverse()) = sub_advence
        //                 .hv_get(axis.inverse())
        //                 .max(*advence.hv_get(axis.inverse()));
        //             *advence.hv_get_mut(axis) += sub_advence.hv_get(axis);

        //             offset
        //         })
        //         .hv_get(axis);

        //     advence
        // }

        match self {
            Self::Single { cache, trans, .. } => {
                let trans = trans.as_ref().unwrap();
                let struc_work = cache.proto.to_work_in_transform(trans).transform(
                    rect.size.to_vector(),
                    WorkVec::new(
                        rect.origin.x + (offset.x) * rect.width(),
                        rect.origin.y + (offset.y) * rect.height(),
                    ),
                );
                let advence = WorkSize::new(trans.h.length, trans.v.length);
                struc.meger(struc_work);
                advence
            }
            Self::Complex { format, comps, .. } => match format {
                // Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                //     merge_in_axis(comps, struc, offset, rect, Axis::Vertical)
                // }
                // Format::LeftToMiddleAndRight | Format::LeftToRight => {
                //     merge_in_axis(comps, struc, offset, rect, Axis::Horizontal)
                // }
                _ => Default::default(),
            },
        }
    }

    pub fn allocation(
        &mut self,
        size_limit: WorkSize,
        offset: WorkPoint,
        config: &ComponetConfig,
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
                        if limit.width > 1.0 {
                            other_options.h = Some(size_limit.width);
                        }
                        if limit.height > 1.0 {
                            other_options.v = Some(size_limit.height);
                        }
                        WorkSize::new(
                            size_limit.width * limit.width,
                            size_limit.height * limit.height,
                        )
                    }
                    None => size_limit,
                };

                let mut results = Vec::with_capacity(2);
                for ((allocs, length), other) in cache
                    .allocs
                    .hv_iter()
                    .zip(size.hv_iter())
                    .zip(other_options.hv_iter())
                {
                    match TransformValue::from_allocs(
                        allocs.clone(),
                        *length,
                        &config.assign_values,
                        &config.min_values,
                    ) {
                        Ok(tv) => results.push(tv),
                        Err(_) if other.is_some() => results.push(TransformValue::from_allocs(
                            allocs.clone(),
                            other.unwrap(),
                            &config.assign_values,
                            &config.min_values,
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
                ..
            } => match format {
                // Format::LeftToMiddleAndRight | Format::LeftToRight => {
                //     Self::allocation_axis(comps, size_limit, offset, config, Axis::Horizontal)
                // }
                // Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                //     Self::allocation_axis(comps, size_limit, offset, config, Axis::Vertical)
                // }
                _ => Err(Error::Empty(format.to_symbol().unwrap().to_string())),
            },
        }
    }

    fn allocation_axis(
        comps: &mut Vec<StrucComb>,
        size_limit: WorkSize,
        offset: WorkPoint,
        config: &ComponetConfig,
        axis: Axis,
    ) -> Result<DataHV<TransformValue>, Error> {
        let min_top = *config
            .min_values
            .first()
            .unwrap_or(&TransformValue::DEFAULT_MIN_VALUE);

        let mut max_level: Option<usize> = None;
        let mut reduce = Self::axis_reduce_comps(comps, axis, &config.reduce_check);

        let err = 'composing: loop {
            let intervals: Vec<f32> = Self::axis_read_connect(comps, axis.inverse())
                .iter()
                .map(|connect| {
                    for wr in &config.interval_rule {
                        if wr.regex.is_match(connect) {
                            return wr.weight;
                        }
                    }
                    0.0
                })
                .collect();

            let mut size = size_limit;
            *size.hv_get_mut(axis) -= intervals.iter().sum::<f32>();

            let mut segments = Vec::with_capacity(comps.len());
            let mut allocs: Vec<usize> = comps
                .iter()
                .flat_map(|c| {
                    let mut allocs = c.axis_allocs(axis).clone();
                    allocs.iter_mut().for_each(|n| {
                        if let Some(max) = max_level {
                            if *n > max {
                                *n = max;
                            }
                        }
                    });
                    segments.push(allocs.len());
                    allocs
                })
                .collect();

            let mut primary_trans = match TransformValue::from_allocs(
                allocs,
                *size.hv_get(axis),
                &config.assign_values,
                &vec![min_top],
            ) {
                Ok(tfv) => tfv,
                Err(e) => {
                    if reduce {
                        if !Self::axis_reduce_comps(comps, axis, &config.reduce_check) {
                            reduce = false;
                        }
                        continue 'composing;
                    } else {
                        let max_level = max_level.get_or_insert(
                            comps
                                .iter()
                                .map(|c| c.max_alloc_level(axis))
                                .max()
                                .unwrap_or_default(),
                        );

                        if *max_level > 1 {
                            *max_level -= 1;
                            continue 'composing;
                        } else {
                            break 'composing e;
                        }
                    }
                }
            };

            let denominator = segments.iter().sum::<usize>() as f32;
            comps.iter_mut().zip(segments).for_each(|(c, n)| {
                let mut size = size;
                *size.hv_get_mut(axis) *= n as f32 / denominator;
            })
        };

        Err(err)
    }

    fn axis_reduce(&mut self, axis: Axis, regex: &regex::Regex) -> bool {
        match self {
            Self::Single { cache, .. } => cache.reduce(axis, regex),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight
                | Format::LeftToRight
                | Format::AboveToBelow
                | Format::AboveToMiddleAndBelow => Self::axis_reduce_comps(comps, axis, regex),
                _ => (0..comps.len())
                    .find(|i| comps[*i].axis_reduce(axis, regex))
                    .is_some(),
            },
        }
    }

    fn axis_reduce_comps(comps: &mut Vec<StrucComb>, axis: Axis, regex: &regex::Regex) -> bool {
        let list: Vec<(usize, usize)> = comps
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (c.subarea_count(axis), i))
            .collect();
        let min_len = list
            .iter()
            .min_by(|a, b| a.0.cmp(&b.0))
            .map(|m| m.0)
            .unwrap_or_default();
        list.into_iter()
            .filter(|(l, _)| *l == min_len)
            .map(|(_, i)| comps[i].axis_reduce(axis, regex))
            .fold(false, |ok, rsl| ok | rsl)
    }

    fn axis_allocs(&self, axis: Axis) -> Vec<usize> {
        fn all(comps: &Vec<StrucComb>, axis: Axis) -> Vec<usize> {
            comps.iter().flat_map(|c| c.axis_allocs(axis)).collect()
        }

        fn one(comps: &Vec<StrucComb>, axis: Axis) -> Vec<usize> {
            let mut allocs_list: Vec<(usize, usize, Vec<usize>)> = comps
                .iter()
                .map(|c| {
                    let allocs = c.axis_allocs(axis);
                    (allocs.len(), allocs.iter().sum::<usize>(), allocs)
                })
                .collect();
            allocs_list
                .into_iter()
                .reduce(|item1, item2| match item1.0.cmp(&item2.0) {
                    Ordering::Less => item2,
                    Ordering::Greater => item1,
                    Ordering::Equal => match item1.1.cmp(&item2.1) {
                        Ordering::Less => item2,
                        _ => item1,
                    },
                })
                .unwrap()
                .2
        }

        match self {
            Self::Single { cache, .. } => cache.allocs.hv_get(axis).clone(),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis),
                    Axis::Vertical => one(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis),
                    Axis::Horizontal => one(comps, axis),
                },
                _ => vec![],
            },
        }
    }

    fn axis_read_connect(comps: &mut Vec<StrucComb>, axis: Axis) -> Vec<String> {
        comps
            .iter()
            .zip(comps.iter().skip(1))
            .map(|(comp1, comp2)| {
                let axis_symbol = match axis {
                    Axis::Horizontal => 'h',
                    Axis::Vertical => 'v',
                };
                format!(
                    "{}:{}{}:{}",
                    axis_symbol,
                    comp1.axis_read_edge(axis, Place::End, comp1.is_zero_length(axis)),
                    axis_symbol,
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
                    if c.is_zero_length(axis.inverse()) && !zero_length {
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

            vc.axis_read_edge(axis, place, vc.is_zero_length(axis))
        }

        match self {
            Self::Single { cache, .. } => match axis {
                Axis::Horizontal => match place {
                    Place::Start => cache.view.read_column(0, 0..cache.view.width()),
                    Place::End => cache
                        .view
                        .read_column(cache.view.height() - 1, 0..cache.view.width()),
                },
                Axis::Vertical => match place {
                    Place::Start => cache.view.read_row(0, 0..cache.view.height()),
                    Place::End => cache
                        .view
                        .read_row(cache.view.width() - 1, 0..cache.view.height()),
                },
            },
            Self::Complex { format, comps, .. } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Horizontal => one(comps, axis, place),
                    Axis::Vertical => all(comps, axis, place, zero_length),
                },
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Vertical => one(comps, axis, place),
                    Axis::Horizontal => all(comps, axis, place, zero_length),
                },
                _ => Default::default(),
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
                _ => comps.first().unwrap().subarea_count(axis),
            },
        }
    }

    fn max_alloc_level(&self, axis: Axis) -> usize {
        match self {
            Self::Single { cache, .. } => cache
                .allocs
                .hv_get(axis)
                .iter()
                .max()
                .cloned()
                .unwrap_or_default(),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight
                | Format::LeftToRight
                | Format::AboveToBelow
                | Format::AboveToMiddleAndBelow => comps
                    .iter()
                    .map(|c| c.max_alloc_level(axis))
                    .max()
                    .unwrap_or_default(),
                _ => 0,
            },
        }
    }
}
