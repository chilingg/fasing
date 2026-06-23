use crate::base::AssignVal;

pub const NORMAL_OFFSET: f32 = 0.0001;

pub fn zero_one<T>(n: T) -> usize
where
    T: euclid::num::Zero + PartialEq,
{
    let zero = T::zero();
    match n == zero {
        true => 0,
        false => 1,
    }
}

pub fn correction_val(target: f32, val: f32, offset: f32) -> f32 {
    if (target - val).abs() < offset {
        target
    } else {
        val
    }
}

pub fn reallocate_on_weights<'a>(
    assigns: impl std::iter::Iterator<Item = &'a mut AssignVal>,
    weights: &[f32],
    factor: f32,
) {
    let mut assigns: Vec<_> = assigns.collect();
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

pub fn reassign_in<'a>(
    assigns: impl std::iter::Iterator<Item = &'a mut AssignVal>,
    changes: &[bool],
    new_val: f32,
) -> Result<(), f32> {
    let mut iter = changes.iter();
    let mut unchanging: Vec<_> = vec![];
    let mut changing: Vec<_> = vec![];
    for val in assigns {
        if *iter.next().unwrap() {
            changing.push(val);
        } else {
            unchanging.push(val);
        }
    }

    let unchanging_len = unchanging.iter().map(|v| **v).sum::<AssignVal>().total();

    match reassign(changing.into_iter(), new_val - unchanging_len) {
        Ok(_) => Ok(()),
        Err(val) => {
            let new_val = unchanging_len - val;
            reassign(unchanging.into_iter(), new_val)
        }
    }
}

pub fn reassign<'a>(
    assigns: impl std::iter::Iterator<Item = &'a mut AssignVal>,
    new_val: f32,
) -> Result<(), f32> {
    let mut assigns: Vec<_> = assigns.collect();
    let base_total = assigns.iter().map(|av| av.base).sum::<f32>();
    let weights: Vec<f32> = assigns.iter().map(|av| av.total()).collect();
    assigns.iter_mut().for_each(|av| av.uncheck_set(0.0));

    let new_val = correction_val(base_total, new_val, 0.0001);
    if base_total > new_val {
        Err(base_total - new_val)
    } else {
        if !assigns.is_empty() {
            *assigns[0] = AssignVal::new(assigns[0].base, new_val - base_total);
            reallocate_on_weights(assigns.into_iter(), &weights, 1.0);
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
    fn test_reassign_in() {
        let mut assigns = vec![
            AssignVal::new(0.1, 0.2),
            AssignVal::new(0.2, 0.2),
            AssignVal::new(0.1, 0.2),
        ];
        reassign_in(assigns.iter_mut(), &[false, true, false], 0.6).unwrap();
        assert_flot(assigns[0].excess, 0.1);
        assert_flot(assigns[1].excess, 0.0);
        assert_flot(assigns[2].excess, 0.1);
    }

    #[test]
    fn test_reallocate_on_weights() {
        let mut assigns = vec![
            AssignVal::new(0.1, 0.1),
            AssignVal::new(0.2, 0.2),
            AssignVal::new(0.1, 0.1),
        ];
        let weights = vec![1.0, 0.0, 1.0];
        reallocate_on_weights(assigns.iter_mut(), &weights, 1.0);
        assert_flot(assigns[0].excess, 0.2);
        assert_eq!(assigns[1].excess, 0.0);
        assert_flot(assigns[2].excess, 0.2);

        let mut assigns = vec![
            AssignVal::new(0.1, 0.1),
            AssignVal::new(0.2, 0.2),
            AssignVal::new(0.1, 0.2),
        ];
        let weights = vec![1.0, 1.0, 1.0];
        reallocate_on_weights(assigns.iter_mut(), &weights, 0.5);
        assert_flot(assigns[0].excess, 0.15);
        assert_flot(assigns[1].excess, 0.15);
        assert_flot(assigns[2].excess, 0.2);
    }

    #[test]
    fn test_reassign() {
        let mut assigns = vec![
            AssignVal::new(1.0, 0.1),
            AssignVal::new(2.0, 0.1),
            AssignVal::new(1.0, 0.1),
        ];
        reassign(assigns.iter_mut(), 8.6).unwrap_or_default();
        assert_flot(assigns[0].total(), 2.2);
        assert_flot(assigns[1].total(), 4.2);
        assert_flot(assigns[2].total(), 2.2);

        reassign(assigns.iter_mut(), 3.8).unwrap_or_default();
        assert_flot(assigns[0].total(), 1.0);
        assert_flot(assigns[1].total(), 2.0);
        assert_flot(assigns[2].total(), 1.0);
    }
}
