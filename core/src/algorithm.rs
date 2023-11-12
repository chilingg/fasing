use crate::axis::DataHV;

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

// center & require: range = 0..1
// deviation: range = -1..1
pub fn center_correction(vlist: &Vec<f32>, center: f32, deviation: f32) -> Vec<f32> {
    if center == 0.0 {
        return vlist.clone();
    }

    let total = vlist.iter().sum::<f32>();
    let split_val = total * center;

    let (l_ratio, r_ratio) = if deviation.is_sign_negative() {
        let ratio = deviation + 1.0;
        (ratio, (1.0 - center * ratio) / (1.0 - center))
    } else {
        let ratio = 1.0 - deviation;
        ((1.0 - (1.0 - center) * ratio) / center, ratio)
    };

    let mut advance = 0.0;
    let mut pre = 0.0;
    vlist
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
        .collect()
}

pub fn centripetal_correction(vlist: &Vec<f32>, center: f32, power: f32) -> Vec<f32> {
    let total = vlist.iter().sum::<f32>();
    let split_val = total * center;
    let r_val = total - split_val;

    let mut advance = 0.0;
    let mut pre = 0.0;
    vlist
        .iter()
        .map(|&v| {
            advance += v;
            advance
        })
        .map(|v| {
            let new_val = if v < split_val {
                let difference = v;
                (difference / split_val).powf(power) * split_val - pre
            } else {
                let difference = total - v;
                total - (difference / r_val).powf(power) * r_val - pre
            };
            pre += new_val;
            new_val
        })
        .collect()
}

pub fn intersection(
    p11: DataHV<f32>,
    p12: DataHV<f32>,
    p21: DataHV<f32>,
    p22: DataHV<f32>,
) -> Option<(DataHV<f32>, [f32; 2])> {
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
        let t1 = ((x1 - x3) * b2 - a2 * (y1 - y3)) / (b1 * a2 - a1 * b2);
        if t1 >= 0.0 && t1 <= 1.0 {
            let t2 = ((x3 - x1) * b1 - (y3 - y1) * a1) / (b2 * a1 - a2 * b1);
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
    fn test_centripetal_correction() {
        let vlist = vec![1.0, 1.0, 1.0, 1.0, 2.0];
        let total = vlist.iter().sum::<f32>();

        assert_eq!(centripetal_correction(&vlist, 0.5, 1.0), vlist);

        let center = 0.5;
        let c_list = centripetal_correction(&vlist, center, 2.0);
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
        let c_list = centripetal_correction(&vlist, center, 0.5);
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
        let c_list = centripetal_correction(&vlist, center, 0.5);
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

        assert_eq!(center_correction(&vlist, 0.5, 0.0), vlist);

        let c_list = center_correction(&vlist, 0.5, -1.0);
        assert_eq!(c_list, vec![0.0, 0.0, 0.0, 2.0, 4.0]);

        let c_list = center_correction(&vlist, 0.5, 1.0);
        assert_eq!(c_list, vec![2.0, 2.0, 2.0, 0.0, 0.0]);

        let c_list = center_correction(&vlist, 0.2, 1.0);
        assert_eq!(c_list[0], 5.0);
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let c_list = center_correction(&vlist, 0.5, 0.5);
        assert!(vlist[..3].iter().zip(c_list.iter()).all(|(&a, &b)| a < b));
        assert_eq!(c_list.iter().sum::<f32>(), total);

        let c_list = center_correction(&vlist, 0.5, -0.5);
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
}
