use crate::axis::DataHV;

pub const NORMAL_OFFSET: f32 = 0.0001;

#[derive(Debug, Clone, Copy)]
enum CorrAction {
    Shrink,
    Expand,
    ShrinkCheck,
    ExpandCheck,
}

// return (x1, x2, height, area)
pub fn find_reactangle_three(length: &[usize]) -> (usize, usize, usize, usize) {
    if length.len() < 2 {
        (0, 0, 0, 0)
    } else {
        let (split_x, &min_height) = length
            .iter()
            .enumerate()
            .min_by_key(|(_, height)| *height)
            .unwrap();
        let x2 = length.len() - 1;
        let area = x2 * min_height;

        let (x1_l, x2_l, height_l, area_l) = find_reactangle_three(&length[..split_x]);

        let (x1_r, x2_r, height_r, area_r) = find_reactangle_three(&length[split_x + 1..]);

        if area >= area_r {
            if area >= area_l {
                (0, x2, min_height, area)
            } else {
                (x1_l, x2_l, height_l, area_l)
            }
        } else {
            if area_r > area_l {
                (x1_r + split_x + 1, x2_r + split_x + 1, height_r, area_r)
            } else {
                (x1_l, x2_l, height_l, area_l)
            }
        }
    }
}

fn base_value_correction(
    bases: &Vec<f32>,
    mut values: Vec<f32>,
    split_index: usize,
    action: CorrAction,
) -> Vec<f32> {
    let process = |mut difference: f32, (v, &b): (&mut f32, &f32)| {
        let v_abs = v.abs();
        let b_abs = b.abs();
        if v_abs < b_abs {
            difference += b_abs - v_abs;
            *v = 0.0;
        } else {
            let allowance = v_abs - b_abs;
            if allowance >= difference {
                *v = (allowance - difference) * v.signum();
                difference = 0.0;
            } else {
                difference -= allowance;
                *v = 0.0;
            }
        }
        difference
    };

    match action {
        CorrAction::Shrink => {
            values[0..split_index]
                .iter_mut()
                .zip(bases[0..split_index].iter())
                .rev()
                .fold(0.0, process);
            values[split_index..]
                .iter_mut()
                .zip(bases[split_index..].iter())
                .fold(0.0, process);
        }
        CorrAction::ShrinkCheck => {
            let debt = values[0..split_index]
                .iter_mut()
                .zip(bases[0..split_index].iter())
                .rev()
                .fold(0.0, process);
            if debt != 0.0 {
                let a_total = values[0..split_index].iter().sum::<f32>();
                debug_assert_ne!(a_total, 0.0);
                debug_assert!(a_total > debt);
                let scale = (a_total - debt) / a_total;
                values[0..split_index].iter_mut().for_each(|v| *v *= scale);
            }

            let debt = values[split_index..]
                .iter_mut()
                .zip(bases[split_index..].iter())
                .fold(0.0, process);
            if debt != 0.0 {
                let a_total = values[split_index..].iter().sum::<f32>();
                debug_assert_ne!(a_total, 0.0);
                debug_assert!(a_total > debt);
                let scale = (a_total - debt) / a_total;
                values[split_index..].iter_mut().for_each(|v| *v *= scale);
            }
        }
        CorrAction::Expand => {
            values[0..split_index]
                .iter_mut()
                .zip(bases[0..split_index].iter())
                .fold(0.0, process);
            values[split_index..]
                .iter_mut()
                .zip(bases[split_index..].iter())
                .rev()
                .fold(0.0, process);
        }
        CorrAction::ExpandCheck => {
            let debt = values[0..split_index]
                .iter_mut()
                .zip(bases[0..split_index].iter())
                .fold(0.0, process);
            if debt != 0.0 {
                let a_total = values[0..split_index].iter().sum::<f32>();
                debug_assert_ne!(a_total, 0.0);
                debug_assert!(a_total > debt);
                let scale = (a_total - debt) / a_total;
                values[0..split_index].iter_mut().for_each(|v| *v *= scale);
            }

            let debt = values[split_index..]
                .iter_mut()
                .zip(bases[split_index..].iter())
                .rev()
                .fold(0.0, process);
            if debt != 0.0 {
                let a_total = values[split_index..].iter().sum::<f32>();
                debug_assert_ne!(a_total, 0.0);
                debug_assert!(a_total > debt);
                let scale = (a_total - debt) / a_total;
                values[split_index..].iter_mut().for_each(|v| *v *= scale);
            }
        }
    }

    values
}

// center & target: range = 0..1
// deviation: range = -1..1
pub fn center_correction(
    vlist: &[f32],
    bases: &Vec<f32>,
    center: f32,
    target: f32,
    corr_val: f32,
) -> Vec<f32> {
    if center == 0.0 {
        return vlist.to_vec();
    }

    let total = vlist.iter().sum::<f32>();
    let split_val = total * center;
    let deviation = {
        let (target, corr_val) = if corr_val < 0.0 {
            let mut t = (target - 0.5).abs();
            t = if center <= 0.5 { 0.5 - t } else { 0.5 + t };
            (t, -corr_val)
        } else {
            (target, corr_val)
        };

        match target - center {
            v if v.is_sign_negative() => v / center * corr_val,
            v => v / (1.0 - center) * corr_val,
        }
    };

    let (l_ratio, r_ratio) = if deviation.is_sign_negative() {
        let ratio = deviation + 1.0;
        (ratio, (1.0 - center * ratio) / (1.0 - center))
    } else {
        let ratio = 1.0 - deviation;
        ((1.0 - (1.0 - center) * ratio) / center, ratio)
    };

    let mut advance = 0.0;
    let mut pre = 0.0;
    let r = vlist
        .iter()
        .map(|&v| {
            advance += v;
            advance
        })
        .map(|v| {
            let new_val = if v < split_val {
                v * l_ratio - pre
            } else {
                (v - split_val) * r_ratio + split_val * l_ratio - pre
            };
            pre += new_val;
            new_val
        })
        .collect();

    match deviation.partial_cmp(&0.0).unwrap() {
        std::cmp::Ordering::Greater => base_value_correction(bases, r, 0, CorrAction::Expand),
        std::cmp::Ordering::Less => base_value_correction(bases, r, 0, CorrAction::Shrink),
        std::cmp::Ordering::Equal => base_value_correction(bases, r, 0, CorrAction::Shrink),
    }
}

fn origin_distance<F>(
    vlist: &Vec<f32>,
    bases: &Vec<f32>,
    center: f32,
    action: CorrAction,
    op: F,
) -> Vec<f32>
where
    F: Fn(f32) -> f32,
{
    fn offset_corection(mut list: Vec<&mut f32>) {
        list.sort_by(|a, b| a.partial_cmp(b).unwrap());
        list.iter_mut().for_each(|v| {
            if **v < NORMAL_OFFSET {
                **v = 0.0
            }
        });
        list.retain(|v| **v == 0.0);

        let mut eq_list: Vec<usize> = vec![];
        let mut cur_val = f32::MAX;
        let mut count = 0.0;
        for i in 0..list.len() {
            if (*list[i] - cur_val).abs() < NORMAL_OFFSET {
                count += 1.0;
                cur_val += (*list[i] - cur_val) / count;
            } else {
                eq_list.drain(..).for_each(|index| *list[index] = cur_val);
                cur_val = *list[i];
                count = 1.0;
            }
            eq_list.push(i);
        }
    }

    let total = vlist.iter().sum::<f32>();
    let split_val = total * center;
    let r_val = total - split_val;
    let mut bases = bases.clone();

    if vlist.is_empty() {
        return vec![];
    }

    let (pre, back, segment) = {
        let mut advance = 0.0;
        let list: Vec<f32> = vlist
            .iter()
            .map(|&v| {
                advance += v;
                advance
            })
            .collect();
        let mut pre: Vec<f32> = list
            .iter()
            .take_while(|v| **v < split_val)
            .copied()
            .collect();
        let mut back: Vec<f32> = list
            .iter()
            .skip_while(|v| **v < split_val)
            .copied()
            .collect();
        let l_val = *pre.last().unwrap_or(&0.0);
        let split_area = *back.first().unwrap() - l_val;
        assert_ne!(split_val, l_val);
        let insert_val = split_val - l_val;

        let l_ratio = insert_val / split_area;
        let this_b_total = bases[pre.len()];
        let segment = pre.len();
        bases[segment] = (1.0 - l_ratio) * this_b_total;
        bases.insert(segment, l_ratio * this_b_total);
        pre.push(split_val);

        pre.iter_mut().for_each(|v| *v = 1.0 - *v / split_val);
        back.iter_mut().for_each(|v| *v = (*v - split_val) / r_val);
        offset_corection(pre.iter_mut().chain(back.iter_mut()).collect());

        (pre, back, segment)
    };

    let mut pre_val = 0.0;
    let mut r: Vec<f32> = pre
        .into_iter()
        .chain(back.into_iter())
        .enumerate()
        .map(|(i, v)| {
            let new_val = if i < segment {
                (1.0 - op(v)) * split_val - pre_val
            } else {
                op(v) * r_val + split_val - pre_val
            };
            pre_val += new_val;
            new_val
        })
        .collect();

    // offset correction
    // let total = r.iter().sum::<f32>();
    // if total != 0.0 {
    //     let scale = vlist.iter().sum::<f32>() / total;
    //     r.iter_mut().for_each(|v| *v *= scale);
    // }

    r = base_value_correction(&bases, r, segment + 1, action);
    let val = r.remove(segment);
    r[segment] += val;
    r
}

pub fn peripheral_and_central(
    vlist: &Vec<f32>,
    bases: &Vec<f32>,
    center: f32,
    p_t: f32,
    c_t: f32,
) -> Vec<f32> {
    let [p_t_v, c_t_v] = [p_t, c_t].map(|t| if t > 1.0 { t } else { 1.0 / t });
    let action = if p_t >= 1.0 && c_t >= 1.0 {
        let t = if p_t_v > c_t_v { p_t } else { c_t };
        match t.partial_cmp(&1.0).unwrap() {
            std::cmp::Ordering::Greater => CorrAction::Shrink,
            std::cmp::Ordering::Less => CorrAction::Expand,
            std::cmp::Ordering::Equal => {
                return vlist
                    .iter()
                    .zip(bases.iter())
                    .map(|(v, a)| *v - *a)
                    .collect()
            }
        }
    } else {
        let t = if p_t_v > c_t_v { p_t } else { c_t };
        match t.partial_cmp(&1.0).unwrap() {
            std::cmp::Ordering::Greater => CorrAction::ShrinkCheck,
            std::cmp::Ordering::Less => CorrAction::ExpandCheck,
            std::cmp::Ordering::Equal => {
                return vlist
                    .iter()
                    .zip(bases.iter())
                    .map(|(v, a)| *v - *a)
                    .collect()
            }
        }
    };
    origin_distance(vlist, bases, center, action, |x| {
        (1.0 - (1.0 - x).powf(1.0 / p_t)).powf(c_t)
    })
}

pub fn central_unit_correction(
    vlist: &Vec<f32>,
    bases: &Vec<f32>,
    center: f32,
    t: f32,
) -> Vec<f32> {
    let action = match t.partial_cmp(&1.0).unwrap() {
        std::cmp::Ordering::Greater => CorrAction::Shrink,
        std::cmp::Ordering::Less => CorrAction::Expand,
        std::cmp::Ordering::Equal => {
            return vlist
                .iter()
                .zip(bases.iter())
                .map(|(v, a)| *v - *a)
                .collect()
        }
    };
    origin_distance(vlist, bases, center, action, |x| x.powf(t))
}

pub fn peripheral_correction(vlist: &Vec<f32>, bases: &Vec<f32>, center: f32, t: f32) -> Vec<f32> {
    let action = match t.partial_cmp(&1.0).unwrap() {
        std::cmp::Ordering::Greater => CorrAction::Shrink,
        std::cmp::Ordering::Less => CorrAction::Expand,
        std::cmp::Ordering::Equal => {
            return vlist
                .iter()
                .zip(bases.iter())
                .map(|(v, a)| *v - *a)
                .collect()
        }
    };
    origin_distance(vlist, bases, center, action, |x| {
        -(1.0 - x).powf(1.0 / t) + 1.0
    })
}

pub fn intersection(
    p11: DataHV<f32>,
    p12: DataHV<f32>,
    p21: DataHV<f32>,
    p22: DataHV<f32>,
) -> Option<(DataHV<f32>, [f32; 2])> {
    fn offset_corection(val: f32) -> f32 {
        if val > 0.0 - NORMAL_OFFSET && val < 1.0 + NORMAL_OFFSET {
            val.max(0.0).min(1.0)
        } else {
            val
        }
    }

    let [x1, y1] = p11.to_array();
    let [x2, y2] = p12.to_array();
    let [x3, y3] = p21.to_array();
    let [x4, y4] = p22.to_array();

    let a1 = x2 - x1;
    let b1 = y2 - y1;
    let a2 = x4 - x3;
    let b2 = y4 - y3;

    if a1 * b2 == b1 * a2 {
        return None;
    } else {
        let t1 = offset_corection(((x1 - x3) * b2 - a2 * (y1 - y3)) / (b1 * a2 - a1 * b2));
        if t1 >= 0.0 && t1 <= 1.0 {
            let t2 = offset_corection(((x3 - x1) * b1 - (y3 - y1) * a1) / (b2 * a1 - a2 * b1));
            if t2 >= 0.0 && t2 <= 1.0 {
                Some((
                    DataHV::new(x1 + t1 * (x2 - x1), y1 + t1 * (y2 - y1)),
                    [t1, t2],
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection() {
        assert_eq!(
            intersection(
                DataHV::new(0.0, 0.0),
                DataHV::new(2.0, 0.0),
                DataHV::new(1.0, 1.0),
                DataHV::new(1.0, -1.0)
            ),
            Some((DataHV::new(1.0, 0.0), [0.5, 0.5]))
        );
        assert_eq!(
            intersection(
                DataHV::new(0.0, 0.0),
                DataHV::new(2.0, 0.0),
                DataHV::new(1.0, -4.0),
                DataHV::new(1.0, -1.0)
            ),
            None
        );
        assert_eq!(
            intersection(
                DataHV::new(0.0, 0.0),
                DataHV::new(2.0, 2.0),
                DataHV::new(0.0, 2.0),
                DataHV::new(2.0, 0.0)
            ),
            Some((DataHV::new(1.0, 1.0), [0.5, 0.5]))
        );
        assert_eq!(
            intersection(
                DataHV::new(0.0, 0.0),
                DataHV::new(0.0, 2.0),
                DataHV::new(0.0, 1.0),
                DataHV::new(0.0, 2.0)
            ),
            None
        );
        assert_eq!(
            intersection(
                DataHV::new(1.0, 0.0),
                DataHV::new(1.0, 2.0),
                DataHV::new(0.0, 0.0),
                DataHV::new(2.0, 2.0)
            ),
            Some((DataHV::new(1.0, 1.0), [0.5, 0.5]))
        );
    }

    #[test]
    fn test_central_correction() {
        let vlist = central_unit_correction(
            &vec![0.12345789, 0.234567899, 0.234567899, 0.12345789],
            &vec![0.0; 10],
            0.5,
            0.1,
        );
        assert!((vlist[0] - vlist[3]).abs() < NORMAL_OFFSET);
        assert!((vlist[1] - vlist[2]).abs() < NORMAL_OFFSET);

        let vlist = vec![1.0, 1.0, 1.0, 1.0, 2.0];
        let total = vlist.iter().sum::<f32>();

        assert!(central_unit_correction(&vlist, &vec![0.0; 10], 0.5, 1.0)
            .iter()
            .zip(vlist.iter())
            .all(|(a, b)| (a - b).abs() < 0.0001));

        let center = 0.5;
        let c_list = central_unit_correction(&vlist, &vec![0.0; 10], center, 0.5);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Less => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Greater => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let center = 0.5;
        let c_list = central_unit_correction(&vlist, &vec![0.0; 10], center, 2.0);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Greater => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Less => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let center = 0.4;
        let c_list = central_unit_correction(&vlist, &vec![0.0; 10], center, 2.0);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Greater => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Less => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);
    }

    #[test]
    fn test_peripheral_correction() {
        let vlist = peripheral_correction(
            &vec![0.12345789, 0.234567899, 0.234567899, 0.12345789],
            &vec![0.0; 10],
            0.5,
            0.1,
        );
        assert!((vlist[0] - vlist[3]).abs() < NORMAL_OFFSET);
        assert!((vlist[1] - vlist[2]).abs() < NORMAL_OFFSET);

        let vlist = vec![1.0, 1.0, 1.0, 1.0, 2.0];
        let total = vlist.iter().sum::<f32>();

        assert!(peripheral_correction(&vlist, &vec![0.0; 10], 0.5, 1.0)
            .iter()
            .zip(vlist.iter())
            .all(|(a, b)| (a - b).abs() < 0.0001));

        let center = 0.5;
        let c_list = peripheral_correction(&vlist, &vec![0.0; 10], center, 0.5);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Less => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Greater => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let center = 0.5;
        let c_list = peripheral_correction(&vlist, &vec![0.0; 10], center, 2.0);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Greater => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Less => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let center = 0.4;
        let c_list = peripheral_correction(&vlist, &vec![0.0; 10], center, 2.0);
        let (mut advance1, mut advance2) = (0.0, 0.0);
        assert!(c_list[..4].iter().zip(vlist.iter()).all(|(&c, &v)| {
            advance1 += c;
            advance2 += v;

            match advance2.partial_cmp(&(total * center)).unwrap() {
                std::cmp::Ordering::Greater => advance1 < advance2,
                std::cmp::Ordering::Equal => (advance1 - advance2).abs() < 0.0001,
                std::cmp::Ordering::Less => advance1 > advance2,
            }
        }));
        assert_eq!(c_list.iter().sum::<f32>(), total);
    }

    #[test]
    fn test_center_correction() {
        let vlist = vec![1.0, 1.0, 1.0, 1.0, 2.0];
        let total = vlist.iter().sum::<f32>();

        assert_eq!(
            center_correction(&vlist, &vec![0.0; 5], 0.5, 0.5, 1.0),
            vlist
        );

        let c_list = center_correction(&vlist, &vec![0.0; 5], 0.5, 0.0, 1.0);
        assert_eq!(c_list, vec![0.0, 0.0, 0.0, 2.0, 4.0]);
        let c_list = center_correction(&vlist, &vec![1.0; 5], 0.5, 0.0, 1.0);
        assert_eq!(c_list, vec![0.0, 0.0, 0.0, 0.0, 1.0]);

        let c_list = center_correction(&vlist, &vec![0.0; 5], 0.5, 1.0, 1.0);
        assert_eq!(c_list, vec![2.0, 2.0, 2.0, 0.0, 0.0]);
        let c_list = center_correction(&vlist, &vec![1.0; 5], 0.5, 1.0, 1.0);
        assert_eq!(c_list, vec![1.0, 0.0, 0.0, 0.0, 0.0]);

        let c_list = center_correction(&vlist, &vec![0.0; 5], 0.2, 1.0, 1.0);
        assert_eq!(c_list[0], 5.0);
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let c_list = center_correction(&vlist, &vec![0.0; 5], 0.5, 0.75, 1.0);
        assert!(vlist[..3].iter().zip(c_list.iter()).all(|(&a, &b)| a < b));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let c_list = center_correction(&vlist, &vec![0.0; 5], 0.5, 0.25, 1.0);
        assert!(vlist[3..].iter().zip(c_list.iter()).all(|(&a, &b)| a > b));
        assert_eq!(c_list.iter().sum::<f32>(), total);
    }

    #[test]
    fn test_find_reactangle_three() {
        // three
        let test_cases = vec![
            (vec![1, 2, 3, 4, 1], (0, 4, 1, 4)),
            (vec![1, 2, 4, 4, 1], (0, 4, 1, 4)),
            (vec![1, 2, 5, 5], (2, 3, 5, 5)),
            (vec![5, 5, 5, 3, 2], (0, 2, 5, 10)),
        ];
        test_cases
            .into_iter()
            .for_each(|(case, result)| assert_eq!(find_reactangle_three(&case[..]), result))
    }

    #[test]
    fn test_base_value_correction() {
        let bases = vec![1.0; 6];

        let values = vec![3.0, 1.0, 2.0, 1.0, 1.5, 1.5];
        assert!(bases.iter().sum::<f32>() < values.iter().sum::<f32>());
        assert_eq!(
            base_value_correction(&bases, values.clone(), 0, CorrAction::Expand),
            vec![2.0, 0.0, 1.0, 0.0, 0.5, 0.5]
        );

        let values = vec![3.0, 1.0, 2.0, 1.0, 0.5, 0.5];
        assert!(bases.iter().sum::<f32>() < values.iter().sum::<f32>());
        assert_eq!(
            base_value_correction(&bases, values.clone(), 6, CorrAction::Shrink),
            vec![2.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 0, CorrAction::Expand),
            vec![2.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        );

        let values = vec![0.5, 1.0, 2.0, 2.0, 1.5, 0.8];
        assert!(bases.iter().sum::<f32>() < values.iter().sum::<f32>());
        assert_eq!(
            base_value_correction(&bases, values.clone(), 3, CorrAction::Expand),
            vec![0.0, 0.0, 0.5, 1.0, 0.3, 0.0]
        );

        let values = vec![0.5, 1.0, 2.0, 0.0, 1.5, 1.5];
        assert!(bases.iter().sum::<f32>() < values.iter().sum::<f32>());
        assert_eq!(
            base_value_correction(&bases, values.clone(), 6, CorrAction::Expand),
            vec![0.0, 0.0, 0.5, 0.0, 0.0, 0.0]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 0, CorrAction::Shrink),
            vec![0.0, 0.0, 0.5, 0.0, 0.0, 0.0]
        );

        let values = vec![1.0; 6];
        assert_eq!(
            base_value_correction(&bases, values.clone(), 0, CorrAction::Expand),
            vec![0.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 4, CorrAction::Shrink),
            vec![0.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 2, CorrAction::Shrink),
            vec![0.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 3, CorrAction::Expand),
            vec![0.0; 6]
        );

        let values = vec![10.0; 6];
        assert_eq!(
            base_value_correction(&bases, values.clone(), 0, CorrAction::Shrink),
            vec![9.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 4, CorrAction::Expand),
            vec![9.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 2, CorrAction::Expand),
            vec![9.0; 6]
        );
        assert_eq!(
            base_value_correction(&bases, values.clone(), 3, CorrAction::Shrink),
            vec![9.0; 6]
        );
    }
}
