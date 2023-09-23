// return (x1, x2, height, area)
pub fn find_reactangle_three(height_list: &[usize]) -> (usize, usize, usize, usize) {
    if height_list.len() < 2 {
        (0, 0, 0, 0)
    } else {
        let (split_x, &min_height) = height_list
            .iter()
            .enumerate()
            .min_by_key(|(_, height)| *height)
            .unwrap();
        let x2 = height_list.len() - 1;
        let area = x2 * min_height;

        let (x1_l, x2_l, height_l, area_l) = find_reactangle_three(&height_list[..split_x]);

        let (x1_r, x2_r, height_r, area_r) = find_reactangle_three(&height_list[split_x + 1..]);

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

#[cfg(test)]
mod tests {
    use super::*;

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
