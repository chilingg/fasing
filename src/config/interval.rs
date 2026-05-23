use serde::{Deserialize, Serialize};

use super::EdgeMatch;
use crate::{base::Axis, combination::view::EdgeShape};

#[derive(Serialize, Deserialize, Clone)]
pub struct IntervalMatch {
    pub inverse: bool,
    pub axis: Option<Axis>,
    pub val: usize,
    pub note: String,
    pub rule1: EdgeMatch,
    pub rule2: EdgeMatch,
}

impl IntervalMatch {
    pub fn is_match(&self, edge1: &EdgeShape, edge2: &EdgeShape, axis: Axis) -> Option<usize> {
        let mut r = None;
        if self.axis.unwrap_or(axis) == axis {
            if self.rule1.is_match(edge1) && self.rule2.is_match(edge2) {
                r = Some(self.val)
            } else if self.inverse && self.rule1.is_match(edge2) && self.rule2.is_match(edge1) {
                r = Some(self.val)
            }
        }
        r
    }
}
