use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::EdgeMatch;
use crate::{base::*, combination::view::EdgeShape};

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

#[derive(Clone, Serialize, Deserialize)]
pub struct SurrIntervalMatch {
    pub val: BTreeMap<String, usize>,
    pub surr: DataHV<Option<Section>>,
    pub rules: BTreeMap<String, [EdgeMatch; 2]>,
    pub note: String,
}

impl SurrIntervalMatch {
    fn get_axis_side(symbol: &str) -> Option<(Axis, Side)> {
        match symbol {
            "l" => Some((Axis::Horizontal, Side::Front)),
            "r" => Some((Axis::Horizontal, Side::Back)),
            "t" => Some((Axis::Vertical, Side::Front)),
            "b" => Some((Axis::Vertical, Side::Back)),
            _ => None,
        }
    }

    pub fn is_match(
        &self,
        surround: &DataHV<Section>,
        primary: &DataHV<[Option<EdgeShape>; 2]>,
        secondary: &DataHV<[EdgeShape; 2]>,
    ) -> DataHV<[Option<usize>; 2]> {
        let hit = self.val.get("hit").copied();
        let mut result = DataHV::splat([None; 2]);

        if Axis::list().into_iter().any(|axis| matches!(self.surr.hv_get_into(axis), Some(section) if section != surround.hv_get_into(axis))) 
        {
            return result;
        }

        let is_match = |axis, side, rules: &[EdgeMatch;2]| {
            let mut ok = false;
            if let Some(p_edge) = &primary.hv_get(axis)[side] && rules[0].is_match(p_edge) && rules[1].is_match(&secondary.hv_get(axis)[side]) {
                ok = true;
            }
            ok
        };

        for (key, rules) in self.rules.iter() {
            match key.as_str() {
                "any" => {
                    for axis in Axis::list() {
                        for side in [0,1] {
                            if is_match(axis, side, rules) && let Some(hit) = hit {
                                result.hv_get_mut(axis)[side] = Some(hit);
                            }
                        }
                    }
                }
                symbol => if let Some((axis, side)) = Self::get_axis_side(symbol) {
                    if !is_match(axis, side.n(), rules) {
                        return Default::default();
                    }
                }
            }
        }

        for (key, val) in self.val.iter() {
            match key.as_str() {
                "hit" => {}
                "all" => result = DataHV::splat([Some(*val); 2]),
                symbol => if let Some((axis, side)) = Self::get_axis_side(symbol) {
                    result.hv_get_mut(axis)[side.n()] = Some(*val);
                }
            }
        }

        result
    }
}
