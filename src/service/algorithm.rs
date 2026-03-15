use crate::base::AssignVal;

pub const NORMAL_OFFSET: f32 = 0.0001;

pub fn scale_in_weights(assigns: &mut [AssignVal], weights: &[f32], factor: f32) {
    if assigns.len() < 2 || factor == 0.0 || weights.iter().all(|w| *w == 0.0) {
        return;
    }

    let mut targets: Vec<usize> = (0..assigns.len()).collect();
    while !targets.is_empty() {
        let weights_sum = targets.iter().map(|i| weights[*i]).sum::<f32>();
        let length = targets.iter().map(|i| assigns[*i].total()).sum::<f32>();

        let mut overstep: Option<f32> = None;
        let tlist: Vec<f32> = targets
            .iter()
            .map(|&i| {
                let new_val = weights[i] / weights_sum * length;
                if new_val < assigns[i].base {
                    let t = assigns[i].excess / (assigns[i].total() - new_val) * factor;
                    overstep = overstep.map(|t2| t2.min(t)).or(Some(t));
                    t
                } else {
                    factor
                }
            })
            .collect();
        let t = overstep.unwrap_or(factor);
        targets = targets
            .into_iter()
            .zip(tlist)
            .filter_map(|(i, it)| {
                let new_val = (weights[i] / weights_sum * length - assigns[i].total()) * t
                    + assigns[i].total();
                assigns[i].check_set(new_val);
                if it == t { None } else { Some(i) }
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_in_weights() {
        let mut assigns = vec![
            AssignVal::new(0.1, 0.1),
            AssignVal::new(0.2, 0.2),
            AssignVal::new(0.1, 0.1),
        ];
        let weights = vec![1.0, 0.0, 1.0];
        scale_in_weights(&mut assigns, &weights, 1.0);
        assert!(
            (assigns[0].excess - 0.2).abs() < NORMAL_OFFSET,
            "{} != {}",
            assigns[0].excess,
            0.2
        );
        assert_eq!(assigns[1].excess, 0.0);
        assert!(
            (assigns[2].excess - 0.2).abs() < NORMAL_OFFSET,
            "{} != {}",
            assigns[0].excess,
            0.2
        );
    }
}
