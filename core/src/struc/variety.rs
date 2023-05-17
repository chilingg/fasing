use super::{
    space::*, view::StrucAllAttrView, StrucAllocates, StrucAttributes, StrucProto, StrucWork,
};
use crate::{
    construct::Format,
    fas_file::{AllocateTable, ComponetConfig, Error, TransformValue},
    hv::*,
};

#[derive(Default, Clone)]
pub struct StrucVarietys {
    pub proto: StrucProto,
    pub attrs: StrucAttributes,
    pub allocs: StrucAllocates,
    pub view: StrucAllAttrView,
}

impl StrucVarietys {
    pub fn from_attrs(
        proto: StrucProto,
        attrs: StrucAttributes,
        alloc_tab: &AllocateTable,
    ) -> Self {
        Self {
            view: StrucAllAttrView::new(&proto),
            proto,
            allocs: attrs.get_space_allocates(alloc_tab),
            attrs,
        }
    }

    pub fn from_allocs(proto: StrucProto, allocs: StrucAllocates) -> Self {
        Self {
            view: StrucAllAttrView::new(&proto),
            attrs: proto.attributes(),
            proto,
            allocs,
        }
    }

    pub fn can_reduce(&self, regex: &regex::Regex, axis: Axis) -> bool {
        self.attrs
            .hv_get(axis)
            .iter()
            .find(|a| regex.is_match(a))
            .is_some()
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

#[derive(Clone)]
pub enum VarietysComb {
    Single {
        name: String,
        size: WorkSize,
        interval: WorkSize,
        varietys: StrucVarietys,
        trans_value: Option<DataHV<TransformValue>>,
    },
    Complex {
        name: String,
        format: Format,
        comps: Vec<VarietysComb>,
        // size: WorkSize, // 对于格式限制是需要的
    },
}

impl VarietysComb {
    pub fn from_complex(format: Format, comps: Vec<VarietysComb>, name: String) -> Self {
        Self::Complex {
            name,
            format,
            comps,
        }
    }

    pub fn from_single(varietys: StrucVarietys, size: WorkSize, name: String) -> Self {
        Self::Single {
            name,
            size,
            interval: WorkSize::zero(),
            varietys,
            trans_value: Default::default(),
        }
    }

    pub fn to_work(&self, offset: WorkPoint, rect: WorkRect) -> StrucWork {
        let mut struc = Default::default();
        self.merge(&mut struc, offset, rect);
        struc
    }

    pub fn merge(&self, struc: &mut StrucWork, offset: WorkPoint, rect: WorkRect) -> WorkSize {
        fn merge_in_axis(
            comps: &Vec<VarietysComb>,
            struc: &mut StrucWork,
            offset: WorkPoint,
            rect: WorkRect,
            axis: Axis,
        ) -> WorkSize {
            let max_length = comps
                .iter()
                .map(|vc| vc.axis_length(axis.inverse()))
                .reduce(f32::max)
                .unwrap_or_default();
            let mut advence = WorkSize::zero();

            comps
                .iter()
                .fold(offset, |mut offset, vc| {
                    let mut sub_offset = offset;
                    *sub_offset.hv_get_mut(axis.inverse()) +=
                        (max_length - vc.axis_length(axis.inverse())) * 0.5;

                    let sub_advence = vc.merge(struc, sub_offset, rect);
                    *offset.hv_get_mut(axis) += sub_advence.hv_get(axis);

                    *advence.hv_get_mut(axis.inverse()) = sub_advence
                        .hv_get(axis.inverse())
                        .max(*advence.hv_get(axis.inverse()));
                    *advence.hv_get_mut(axis) += sub_advence.hv_get(axis);

                    offset
                })
                .hv_get(axis);

            advence
        }

        match self {
            Self::Single {
                interval,
                varietys,
                trans_value,
                ..
            } => {
                let trans = trans_value.as_ref().unwrap();
                let mut struc_work = varietys.proto.to_work_in_transform(trans);
                let advence = WorkSize::new(
                    trans.h.length + interval.width,
                    trans.v.length + interval.height,
                );
                struc_work.transform(
                    rect.size.to_vector(),
                    WorkVec::new(
                        rect.origin.x + (offset.x + interval.width) * rect.width(),
                        rect.origin.y + (offset.y + interval.height) * rect.height(),
                    ),
                );
                struc.meger(struc_work);
                advence
            }
            Self::Complex { format, comps, .. } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                    merge_in_axis(comps, struc, offset, rect, Axis::Vertical)
                }
                Format::LeftToMiddleAndRight | Format::LeftToRight => {
                    merge_in_axis(comps, struc, offset, rect, Axis::Horizontal)
                }
                _ => Default::default(),
            },
        }
    }

    pub fn allocation(
        &mut self,
        size_limit: WorkSize,
        offset: WorkSize,
        config: &ComponetConfig,
    ) -> Result<DataHV<TransformValue>, Error> {
        match self {
            Self::Single {
                varietys,
                size,
                interval,
                trans_value,
                ..
            } => {
                let mut cur_size = WorkSize::new(
                    size_limit.width * size.width,
                    size_limit.height * size.height,
                );
                let mut other_option = DataHV::new(
                    match size.width < 1.0 {
                        true => Some(size_limit.width),
                        false => None,
                    },
                    match size.height < 1.0 {
                        true => Some(size_limit.height),
                        false => None,
                    },
                );

                loop {
                    match config
                        .single_allocation(varietys.allocs.clone(), cur_size)
                        .and_then(|tv| {
                            *trans_value = Some(tv.clone());
                            *interval = offset;
                            Ok(tv)
                        }) {
                        Err(Error::AxisTransform { axis, .. })
                            if other_option.hv_get(axis).is_some() =>
                        {
                            *cur_size.hv_get_mut(axis) =
                                other_option.hv_get_mut(axis).take().unwrap()
                        }
                        res => return res,
                    }
                }
            }
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => {
                    Self::allocation_axis(comps, size_limit, offset, config, Axis::Horizontal)
                }
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                    Self::allocation_axis(comps, size_limit, offset, config, Axis::Vertical)
                }
                _ => Err(Error::Empty(format.to_symbol().unwrap().to_string())),
            },
        }
    }

    fn allocation_axis(
        comps: &mut Vec<VarietysComb>,
        size_limit: WorkSize,
        offset: WorkSize,
        config: &ComponetConfig,
        axis: Axis,
    ) -> Result<DataHV<TransformValue>, Error> {
        'composing: loop {
            let intervals: Vec<f32> = Self::axis_read_connect(comps, axis.inverse())
                .into_iter()
                .map(|connect| {
                    let mut interval = 0.0;
                    for wr in &config.interval_judge {
                        if wr.regex.is_match(connect.as_str()) {
                            interval = wr.weight;
                            break;
                        }
                    }
                    interval
                })
                .collect();

            let mut segments = Vec::with_capacity(comps.len());
            let mut allocs: Vec<_> = comps
                .iter()
                .flat_map(|vc| {
                    let allocs = vc.axis_allocs(axis).clone();
                    segments.push(allocs.len());
                    allocs
                })
                .collect();
            let mut primary_trans = {
                loop {
                    match TransformValue::from_allocs_interval(
                        allocs.clone(),
                        *size_limit.hv_get(axis),
                        config.min_space,
                        config.increment,
                        intervals.iter().sum(),
                        &config.limit.h,
                    ) {
                        Ok(tvs) => {
                            if tvs.min_step < config.reduce_targger {
                                if allocs.iter_mut().fold(false, |mut reduced, n| {
                                    if *n > 1 {
                                        *n -= 1;
                                        reduced = true;
                                    }
                                    reduced
                                }) {
                                    continue;
                                } else if Self::axis_reduce_in_axis(
                                    comps,
                                    axis,
                                    &config.reduce_check,
                                ) {
                                    continue 'composing;
                                } else {
                                    break tvs;
                                }
                            } else {
                                break tvs;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            };

            let equally = primary_trans.allocs.iter().all(|n| *n < 2);
            let mut interval_iter = intervals.into_iter();
            let mut offset = offset;
            let mut real_tv = DataHV::<TransformValue>::default();
            for (comp, n) in comps.iter_mut().zip(segments) {
                let allocs: Vec<usize> = primary_trans.allocs.drain(0..n).collect();
                let (min_step, step) = match allocs.iter().all(|n| *n < 2) {
                    true if !equally => (primary_trans.min_step, primary_trans.min_step),
                    _ => (primary_trans.min_step, primary_trans.step),
                };

                let mut size_limit = size_limit;
                *size_limit.hv_get_mut(axis) =
                    TransformValue::from_step(allocs.clone(), min_step, step).length;
                let mut tv = comp.allocation(size_limit, offset, config)?;
                Axis::list().for_each(|axis| {
                    if tv.hv_get(axis).allocs.is_empty() {
                        tv.hv_get_mut(axis).min_step = min_step;
                        tv.hv_get_mut(axis).step = step;
                        comp.for_each_mut(&mut |vc: &mut VarietysComb| match vc {
                            VarietysComb::Single { trans_value, .. } => {
                                trans_value.as_mut().unwrap().hv_get_mut(axis).min_step = min_step;
                                trans_value.as_mut().unwrap().hv_get_mut(axis).step = step;
                            }
                            _ => {}
                        });
                    }
                });
                *offset.hv_get_mut(axis) = interval_iter.next().unwrap_or_default() * min_step;

                let (primary, sub_primary) = (real_tv.hv_get_mut(axis), tv.hv_get(axis));
                primary.allocs.extend(sub_primary.allocs.iter());
                primary.length += sub_primary.length + offset.hv_get(axis);
                let (secondary, sub_secondary) = (
                    real_tv.hv_get_mut(axis.inverse()),
                    tv.hv_get(axis.inverse()),
                );
                if secondary.allocs.iter().sum::<usize>()
                    < sub_secondary.allocs.iter().sum::<usize>()
                {
                    secondary.allocs = sub_secondary.allocs.clone();
                }
                secondary.length = secondary.length.max(sub_secondary.length);

                Axis::list().for_each(|axis| {
                    real_tv.hv_get_mut(axis).min_step =
                        real_tv.hv_get(axis).min_step.max(tv.hv_get(axis).min_step);
                    real_tv.hv_get_mut(axis).step =
                        real_tv.hv_get(axis).step.max(tv.hv_get(axis).step);
                })
            }

            return Ok(real_tv);
        }
    }

    fn axis_reduce(&mut self, axis: Axis, regex: &regex::Regex) -> bool {
        match self {
            Self::Single { varietys, .. } => varietys.reduce(axis, regex),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight
                | Format::LeftToRight
                | Format::AboveToBelow
                | Format::AboveToMiddleAndBelow => Self::axis_reduce_in_axis(comps, axis, regex),
                _ => (0..comps.len())
                    .find(|i| comps[*i].axis_reduce(axis, regex))
                    .is_some(),
            },
        }
    }

    fn axis_reduce_in_axis(
        comps: &mut Vec<VarietysComb>,
        axis: Axis,
        regex: &regex::Regex,
    ) -> bool {
        let list: Vec<(usize, usize)> = comps
            .iter_mut()
            .enumerate()
            .map(|(i, c)| (c.axis_alloc_length(axis), i))
            .collect();
        let max_len = list
            .iter()
            .max_by(|a, b| a.0.cmp(&b.0))
            .map(|m| m.0)
            .unwrap_or_default();
        list.into_iter()
            .filter(|(l, _)| *l == max_len)
            .map(|(_, i)| comps[i].axis_reduce(axis, regex))
            .fold(false, |ok, rsl| ok | rsl)
    }

    fn axis_allocs(&self, axis: Axis) -> Vec<usize> {
        fn all(comps: &Vec<VarietysComb>, axis: Axis) -> Vec<usize> {
            comps.iter().flat_map(|c| c.axis_allocs(axis)).collect()
        }

        fn one(comps: &Vec<VarietysComb>, axis: Axis) -> Vec<usize> {
            let mut allocs_list: Vec<(usize, Vec<usize>)> = comps
                .iter()
                .map(|c| {
                    let allocs = c.axis_allocs(axis);
                    (allocs.iter().sum::<usize>(), allocs)
                })
                .collect();
            allocs_list.sort_by(|a, b| a.0.cmp(&b.0));
            allocs_list.pop().unwrap().1
        }

        match self {
            Self::Single { varietys, .. } => varietys.allocs.hv_get(axis).clone(),
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

    fn axis_length(&self, axis: Axis) -> f32 {
        fn all(comps: &Vec<VarietysComb>, axis: Axis) -> f32 {
            comps.iter().map(|c| c.axis_length(axis)).sum()
        }

        fn one(comps: &Vec<VarietysComb>, axis: Axis) -> f32 {
            comps
                .iter()
                .map(|c| c.axis_length(axis))
                .reduce(f32::max)
                .unwrap_or_default()
        }

        match self {
            Self::Single {
                trans_value,
                interval,
                ..
            } => {
                trans_value
                    .as_ref()
                    .expect("Unallocate transform value!")
                    .hv_get(axis)
                    .length
                    + interval.hv_get(axis)
            }
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis),
                    Axis::Vertical => one(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis),
                    Axis::Horizontal => one(comps, axis),
                },
                _ => 0.0,
            },
        }
    }

    fn axis_alloc_length(&self, axis: Axis) -> usize {
        fn all(comps: &Vec<VarietysComb>, axis: Axis) -> usize {
            comps.iter().map(|c| c.axis_alloc_length(axis)).sum()
        }

        fn one(comps: &Vec<VarietysComb>, axis: Axis) -> usize {
            comps
                .iter()
                .map(|c| c.axis_alloc_length(axis))
                .max()
                .unwrap_or_default()
        }

        match self {
            Self::Single { varietys, .. } => varietys.allocs.hv_get(axis).iter().sum(),
            Self::Complex { comps, format, .. } => match format {
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Horizontal => all(comps, axis),
                    Axis::Vertical => one(comps, axis),
                },
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Vertical => all(comps, axis),
                    Axis::Horizontal => one(comps, axis),
                },
                _ => 0,
            },
        }
    }

    pub fn read_connect(&self) -> Vec<String> {
        match self {
            Self::Single { .. } => vec![],
            Self::Complex { format, comps, .. } => match format {
                Format::LeftToRight | Format::LeftToMiddleAndRight => {
                    Self::axis_read_connect(comps, Axis::Vertical)
                }
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => {
                    Self::axis_read_connect(comps, Axis::Horizontal)
                }
                _ => vec![],
            },
        }
    }

    fn axis_read_connect(comps: &Vec<VarietysComb>, axis: Axis) -> Vec<String> {
        comps
            .iter()
            .zip(comps.iter().skip(1))
            .map(|(vc1, vc2)| {
                let (len1, len2) = (
                    vc1.axis_alloc_length(axis.inverse()),
                    vc2.axis_alloc_length(axis.inverse()),
                );

                let axis_symbol = match axis {
                    Axis::Horizontal => 'h',
                    Axis::Vertical => 'v',
                };
                format!(
                    "{}:{}{}:{}",
                    axis_symbol,
                    vc1.axis_read_edge(axis, len1, false),
                    axis_symbol,
                    vc2.axis_read_edge(axis, len2, true)
                )
            })
            .collect()
    }

    fn axis_read_edge(&self, axis: Axis, other_axis_max_len: usize, front: bool) -> String {
        fn all(
            comps: &Vec<VarietysComb>,
            axis: Axis,
            other_axis_max_len: usize,
            front: bool,
        ) -> String {
            comps
                .iter()
                .filter_map(|c| {
                    match c.axis_alloc_length(axis.inverse()) > 1 || other_axis_max_len == 1 {
                        true => Some(c.axis_read_edge(axis, other_axis_max_len, front)),
                        false => None,
                    }
                })
                .collect()
        }

        fn one(comps: &Vec<VarietysComb>, axis: Axis, front: bool) -> String {
            let vc = match front {
                true => comps.first(),
                false => comps.last(),
            }
            .unwrap();

            vc.axis_read_edge(axis, vc.axis_alloc_length(axis.inverse()), front)
        }

        match self {
            Self::Single { varietys, .. } => match axis {
                Axis::Horizontal => match front {
                    true => varietys.view.read_column(0, 0..varietys.view.width()),
                    false => varietys
                        .view
                        .read_column(varietys.view.height() - 1, 0..varietys.view.width()),
                },
                Axis::Vertical => match front {
                    true => varietys.view.read_row(0, 0..varietys.view.height()),
                    false => varietys
                        .view
                        .read_row(varietys.view.width() - 1, 0..varietys.view.height()),
                },
            },
            Self::Complex { comps, format, .. } => match format {
                Format::AboveToBelow | Format::AboveToMiddleAndBelow => match axis {
                    Axis::Horizontal => one(comps, axis, front),
                    Axis::Vertical => all(comps, axis, other_axis_max_len, front),
                },
                Format::LeftToMiddleAndRight | Format::LeftToRight => match axis {
                    Axis::Vertical => one(comps, axis, front),
                    Axis::Horizontal => all(comps, axis, other_axis_max_len, front),
                },
                _ => String::new(),
            },
        }
    }

    pub fn for_each_mut<F>(&mut self, f: &mut F)
    where
        F: FnMut(&mut Self),
    {
        f(self);
        match self {
            Self::Single { .. } => {}
            Self::Complex { comps, .. } => {
                for vc in comps.iter_mut() {
                    vc.for_each_mut(f);
                }
            }
        }
    }

    pub fn for_each<F>(&self, f: &mut F)
    where
        F: FnMut(&Self),
    {
        f(self);
        match self {
            Self::Single { .. } => {}
            Self::Complex { comps, .. } => {
                for vc in comps.iter() {
                    vc.for_each(f);
                }
            }
        }
    }
}
