use crate::base::AssignVal;

pub const NORMAL_OFFSET: f32 = 0.0001;

pub fn reallocate_on_weights(assigns: &mut [AssignVal], weights: &[f32], factor: f32) {
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
                assigns[i].uncheck_set(new_val);
                if it == t { None } else { Some(i) }
            })
            .collect();
    }
}

pub fn reassign(assigns: &mut [AssignVal], new_val: f32) -> Result<(), f32> {
    let base_total = assigns.iter().map(|av| av.base).sum::<f32>();
    let weights: Vec<f32> = assigns.iter().map(|av| av.total()).collect();
    assigns.iter_mut().for_each(|av| av.uncheck_set(0.0));

    if base_total > new_val {
        Err(new_val - base_total)
    } else {
        if !assigns.is_empty() {
            assigns[0] = AssignVal::new(assigns[0].base, new_val - base_total);
            reallocate_on_weights(assigns, &weights, 1.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_flot(a: f32, b: f32) {
        assert!((a - b).abs() < NORMAL_OFFSET, "{a} != {b}");
    }

    #[test]
    fn test_reallocate_on_weights() {
        let mut assigns = vec![
            AssignVal::new(0.1, 0.1),
            AssignVal::new(0.2, 0.2),
            AssignVal::new(0.1, 0.1),
        ];
        let weights = vec![1.0, 0.0, 1.0];
        reallocate_on_weights(&mut assigns, &weights, 1.0);
        assert_flot(assigns[0].excess, 0.2);
        assert_eq!(assigns[1].excess, 0.0);
        assert_flot(assigns[2].excess, 0.2);
    }

    #[test]
    fn test_reassign() {
        let mut assigns = vec![
            AssignVal::new(1.0, 0.1),
            AssignVal::new(2.0, 0.1),
            AssignVal::new(1.0, 0.1),
        ];
        reassign(&mut assigns, 8.6).unwrap_or_default();
        assert_flot(assigns[0].total(), 2.2);
        assert_flot(assigns[1].total(), 4.2);
        assert_flot(assigns[2].total(), 2.2);

        reassign(&mut assigns, 3.8).unwrap_or_default();
        assert_flot(assigns[0].total(), 1.0);
        assert_flot(assigns[1].total(), 2.0);
        assert_flot(assigns[2].total(), 1.0);
    }
}
