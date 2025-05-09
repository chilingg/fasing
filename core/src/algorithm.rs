use crate::{
    axis::{Axis, DataHV, ValueHV},
    component::comb::AssignVal,
    construct::space::*,
};

pub const NORMAL_OFFSET: f32 = 0.0001;

#[derive(Debug, Clone, Copy)]
enum CorrAction {
    Shrink,
    Expand,
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

pub fn intersection(
    p11: WorkPoint,
    p12: WorkPoint,
    p21: WorkPoint,
    p22: WorkPoint,
) -> Option<(WorkPoint, [f32; 2])> {
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
                    WorkPoint::new(x1 + t1 * (x2 - x1), y1 + t1 * (y2 - y1)),
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

pub fn split_intersect(paths: &mut Vec<KeyWorkPath>, min_len: f32) {
    let min_len_square = min_len.powi(2);
    let mut paths: Vec<_> = paths.iter_mut().map(|path| &mut path.points).collect();

    for i in (1..paths.len()).rev() {
        for j in 0..i {
            let mut p1_i = 1;
            while p1_i < paths[i].len() {
                let p11 = paths[i][p1_i - 1];
                let mut p12 = paths[i][p1_i];

                let mut p2_i = 1;
                while p2_i < paths[j].len() {
                    let p21 = paths[j][p2_i - 1];
                    let p22 = paths[j][p2_i];

                    match intersection(p11, p12, p21, p22) {
                        Some((new_point, t)) => {
                            let p = new_point;

                            if 0.0 < t[0]
                                && t[0] < 1.0
                                && (p11.x - p.x).powi(2) + (p11.y - p.y).powi(2) + NORMAL_OFFSET
                                    >= min_len_square
                                && (p.x - p12.x).powi(2) + (p.y - p12.y).powi(2) + NORMAL_OFFSET
                                    >= min_len_square
                            {
                                p12 = p;
                                paths[i].insert(p1_i, p);
                            }
                            if 0.0 < t[1]
                                && t[1] < 1.0
                                && (p21.x - p.x).powi(2) + (p21.y - p.y).powi(2) + NORMAL_OFFSET
                                    >= min_len_square
                                && (p.x - p22.x).powi(2) + (p.y - p22.y).powi(2) + NORMAL_OFFSET
                                    >= min_len_square
                            {
                                paths[j].insert(p2_i, p);
                                p2_i += 1;
                            }
                        }
                        _ => {}
                    }
                    p2_i += 1;
                }
                p1_i += 1;
            }
        }
    }
}

pub fn visual_center(paths: &Vec<KeyWorkPath>) -> WorkPoint {
    let mut size = DataHV::splat([f32::MAX, f32::MIN]);
    let (pos, count) =
        paths
            .iter()
            .fold((DataHV::splat(0.0), 0.0), |(mut pos, mut count), path| {
                path.points
                    .iter()
                    .zip(path.points.iter().skip(1))
                    .for_each(|(kp1, kp2)| {
                        Axis::list().into_iter().for_each(|axis| {
                            let val1 = *kp1.hv_get(axis);
                            let val2 = *kp2.hv_get(axis);

                            if !path.hide {
                                *pos.hv_get_mut(axis) += (val1 + val2) * 0.5;
                            }

                            let len = size.hv_get_mut(axis);
                            len[0] = len[0].min(val1).min(val2);
                            len[1] = len[1].max(val1).max(val2);
                        });

                        if !path.hide {
                            count += 1.0;
                        }
                    });

                (pos, count)
            });

    let center = pos
        .zip(size)
        .into_map(|(v, len)| {
            let l = len[1] - len[0];
            if count == 0.0 || l <= 0.0 {
                0.0
            } else {
                let center = (v / count - len[0]) / l;
                if (center - 0.5).abs() < NORMAL_OFFSET {
                    0.5
                } else {
                    center
                }
            }
        })
        .to_array()
        .into();

    center
}

pub fn visual_center_length(
    mut paths: Vec<KeyWorkPath>,
    min_len: f32,
    stroke_width: f32,
) -> WorkPoint {
    split_intersect(&mut paths, min_len);

    let mut size = DataHV::splat([f32::MAX, f32::MIN]);
    let (pos, count) =
        paths
            .iter()
            .fold((DataHV::splat(0.0), 0.0), |(mut pos, mut count), path| {
                path.points
                    .iter()
                    .zip(path.points.iter().skip(1))
                    .for_each(|(&kp1, &kp2)| {
                        let length = (kp1 - kp2).length() + stroke_width;
                        Axis::list().into_iter().for_each(|axis| {
                            let val1 = *kp1.hv_get(axis);
                            let val2 = *kp2.hv_get(axis);

                            if !path.hide {
                                *pos.hv_get_mut(axis) += (val1 + val2) * 0.5 * length;
                            }

                            let len = size.hv_get_mut(axis);
                            len[0] = len[0].min(val1).min(val2);
                            len[1] = len[1].max(val1).max(val2);
                        });

                        if !path.hide {
                            count += length;
                        }
                    });

                (pos, count)
            });

    let center = pos
        .zip(size)
        .into_map(|(v, len)| {
            let l = len[1] - len[0];
            if count == 0.0 || l <= 0.0 {
                0.0
            } else {
                let center = (v / count - len[0]) / l;
                if (center - 0.5).abs() < NORMAL_OFFSET {
                    0.5
                } else {
                    center
                }
            }
        })
        .to_array()
        .into();

    center
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
    }

    values
}

pub fn scale_correction(vlist: &mut Vec<AssignVal>, assign: f32) -> bool {
    let old_assign: AssignVal = vlist.iter().sum();
    if old_assign.base > assign {
        vlist.iter_mut().for_each(|v| v.excess = 0.0);
        false
    } else {
        let total = old_assign.total();
        let scale = assign / total;
        let mut debt = 0.0;
        vlist.iter_mut().for_each(|v| {
            v.excess = v.total() * scale - v.base;
            if v.excess < 0.0 {
                debt -= v.excess;
                v.excess = 0.0;
            }
        });

        while debt != 0.0 {
            let targets: Vec<_> = vlist.iter_mut().filter(|v| v.excess != 0.0).collect();
            let sub_val = debt / targets.len() as f32;
            debt = 0.0;
            targets.into_iter().for_each(|v| {
                v.excess -= sub_val;
                if v.excess < 0.0 {
                    debt -= v.excess;
                    v.excess = 0.0;
                }
            });
        }
        true
    }
}

// center & target: range = 0..1
// deviation: range = -1..1
pub fn center_correction(
    vlist: &Vec<f32>,
    bases: &Vec<f32>,
    center: f32,
    target: f32,
    corr_val: f32,
) -> Vec<f32> {
    if center == 0.0 {
        return vlist.iter().zip(bases).map(|(v, a)| v - a).collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_center_in_split() {
        let mut paths = vec![
            KeyWorkPath::from([WorkPoint::new(1.0, 0.0), WorkPoint::new(1.0, 2.0)]),
            KeyWorkPath {
                points: vec![WorkPoint::new(0.0, 1.0), WorkPoint::new(2.0, 1.0)],
                hide: true,
            },
        ];
        split_intersect(&mut paths, 0.0);
        assert_eq!(paths[0].points.len(), 3);
        assert_eq!(paths[0].points.len(), paths[1].points.len());
        let center = visual_center(&paths);
        assert_eq!(center, WorkPoint::new(0.5, 0.5));

        let mut paths = vec![
            KeyWorkPath::from([WorkPoint::new(1.0, 0.0), WorkPoint::new(4.0, 2.0)]),
            KeyWorkPath::from([WorkPoint::new(0.0, 2.0), WorkPoint::new(3.0, 0.0)]),
        ];
        split_intersect(&mut paths, 0.0);
        assert_eq!(paths[0].points.len(), 3);
        assert_eq!(paths[0].points.len(), paths[1].points.len());
        let center = visual_center(&paths);
        assert_eq!(center.x, 0.5);
        assert!(center.y < 0.5);

        let mut paths = vec![
            KeyWorkPath::from([WorkPoint::new(1.0, 0.0), WorkPoint::new(1.0, 2.0)]),
            KeyWorkPath::from([WorkPoint::new(0.0, 1.0), WorkPoint::new(2.0, 1.0)]),
        ];
        split_intersect(&mut paths, 1.1);
        assert_eq!(paths[0].points.len(), 2);
        assert_eq!(paths[0].points.len(), paths[1].points.len());
    }

    #[test]
    fn test_scale_correction() {
        fn check_eq(a: f32, b: f32) -> bool {
            (a - b).abs() < NORMAL_OFFSET
        }

        let mut list = vec![
            AssignVal::new(1.0, 3.0),
            AssignVal::new(2.0, 2.0),
            AssignVal::new(3.0, 1.0),
        ];

        scale_correction(&mut list, 9.0);
        assert!(check_eq(list.iter().sum::<AssignVal>().total(), 9.0));
        assert!(check_eq(list[0].excess, 2.0));
        assert!(check_eq(list[1].excess, 1.0));
        assert!(check_eq(list[2].excess, 0.0));

        scale_correction(&mut list, 8.0);
        assert!(check_eq(list.iter().sum::<AssignVal>().total(), 8.0));
        assert!(check_eq(list[0].excess, 1.5));
        assert!(check_eq(list[1].excess, 0.5));
        assert!(check_eq(list[2].excess, 0.0));

        scale_correction(&mut list, 4.0);
        assert!(check_eq(list.iter().sum::<AssignVal>().total(), 6.0));
        assert!(check_eq(list[0].excess, 0.0));
        assert!(check_eq(list[1].excess, 0.0));
        assert!(check_eq(list[2].excess, 0.0));

        scale_correction(&mut list, 12.0);
        assert!(check_eq(list.iter().sum::<AssignVal>().total(), 12.0));
        assert!(check_eq(list[0].excess, 1.0));
        assert!(check_eq(list[1].excess, 2.0));
        assert!(check_eq(list[2].excess, 3.0));
    }
}
