use crate::{
    axis::*,
    component::{strategy::PlaceMain, view::Element},
    construct::{Component, CpAttrs, CstType},
};

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Clone)]
pub struct Interval {
    pub rules: BTreeMap<String, usize>,
    pub limit: f32,
}

impl Default for Interval {
    fn default() -> Self {
        Self {
            rules: Default::default(),
            limit: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Operation<O, E> {
    pub operation: O,
    pub execution: E,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WhiteArea {
    pub fixed: f32,
    pub allocated: f32,
    pub value: f32,
    pub weights: BTreeMap<Element, f32>,
}

impl WhiteArea {
    pub fn get_weight(&self, elements: &Vec<Vec<Element>>) -> f32 {
        let weight: BTreeMap<Element, f32> = BTreeMap::from([
            (
                Element::Dot,
                self.weights.get(&Element::Dot).cloned().unwrap_or(0.9),
            ),
            (
                Element::Diagonal,
                self.weights.get(&Element::Diagonal).cloned().unwrap_or(0.9),
            ),
        ]);

        elements
            .iter()
            .map(|els| {
                1.0 - if els.is_empty() {
                    1.0
                } else if els.contains(&Element::Face) {
                    0.0
                } else {
                    els.iter().map(|e| weight[e]).product()
                }
            })
            .sum::<f32>()
            / elements.len() as f32
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TypeDate<A, S> {
    pub axis: A,
    pub surround: S,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub min_val: DataHV<Vec<f32>>,
    pub white: DataHV<WhiteArea>,

    pub supplement: BTreeMap<String, CpAttrs>,
    // 结构-位-字-部件
    pub type_replace: BTreeMap<char, BTreeMap<Place, BTreeMap<String, Component>>>,
    pub place_replace: BTreeMap<String, Vec<(String, Component)>>,

    pub interval: DataHV<Interval>,

    pub center: DataHV<Operation<f32, f32>>,
    pub comp_center: DataHV<Operation<f32, f32>>,
    pub center_area: DataHV<Operation<f32, f32>>,

    pub strategy: TypeDate<
        DataHV<BTreeMap<Place, BTreeMap<Place, BTreeSet<PlaceMain>>>>,
        BTreeMap<char, DataHV<BTreeSet<PlaceMain>>>,
    >,
    pub align: TypeDate<DataHV<f32>, DataHV<BTreeMap<Place, f32>>>,

    pub reduce_trigger: DataHV<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DataHV::splat(1.0),
            min_val: DataHV::splat(vec![Self::DEFAULT_MIN_VALUE]),
            white: Default::default(),
            supplement: Default::default(),
            type_replace: Default::default(),
            place_replace: Default::default(),
            interval: Default::default(),
            center: Default::default(),
            comp_center: Default::default(),
            center_area: Default::default(),
            strategy: Default::default(),
            align: Default::default(),
            reduce_trigger: DataHV::splat(0.0),
        }
    }
}

impl Config {
    pub const DEFAULT_MIN_VALUE: f32 = 0.05;

    pub fn type_replace_name(&self, name: &str, tp: CstType, in_tp: Place) -> Option<Component> {
        fn process<'a>(
            cfg: &'a Config,
            name: &str,
            tp: CstType,
            in_tp: Place,
        ) -> Option<&'a Component> {
            cfg.type_replace
                .get(&tp.symbol())
                .and_then(|pm| pm.get(&in_tp).and_then(|map| map.get(name)))
        }

        process(self, name, tp, in_tp)
            .map(|mut map_comp| {
                while let Some(mc) = process(self, &map_comp.name(), tp, in_tp) {
                    map_comp = mc
                }
                map_comp
            })
            .cloned()
    }

    pub fn place_replace_name(&self, name: &str, places: DataHV<[bool; 2]>) -> Option<Component> {
        self.place_replace
            .get(name)
            .and_then(|pm| {
                pm.iter().find_map(|(r, c)| match place_match(r, places) {
                    true => Some(c),
                    false => None,
                })
            })
            .cloned()
    }
}

pub fn place_match(rule: &str, places: DataHV<[bool; 2]>) -> bool {
    fn is_match(attr: &str, exist: bool) -> bool {
        match attr {
            "x" => !exist,
            "o" => exist,
            "*" => true,
            _ => false,
        }
    }

    rule.split(';')
        .into_iter()
        .find(|r| {
            let place_attr: Vec<&str> = r.split(' ').collect();
            match place_attr.len() {
                1 => places
                    .hv_iter()
                    .flatten()
                    .all(|e| is_match(place_attr[0], *e)),
                2 => places
                    .hv_iter()
                    .zip(place_attr.iter())
                    .all(|(place, attr)| place.iter().all(|e| is_match(attr, *e))),
                4 => places
                    .hv_iter()
                    .flatten()
                    .zip(place_attr.iter())
                    .all(|(e, attr)| is_match(attr, *e)),
                _ => false,
            }
        })
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_place_match() {
        let mut places = DataHV::from(([true, false], [true, false]));
        assert!(place_match("*", places));
        assert!(!place_match("x", places));
        assert!(!place_match("o", places));
        assert!(place_match("o x o x", places));
        assert!(!place_match("o x", places));

        places.h = [true, true];
        places.v = [false, false];
        assert!(place_match("*", places));
        assert!(!place_match("x", places));
        assert!(!place_match("o", places));
        assert!(place_match("o o x x", places));
        assert!(place_match("o x", places));

        places.v = [true, true];
        assert!(place_match("*", places));
        assert!(!place_match("x", places));
        assert!(place_match("o", places));
        assert!(!place_match("o o x x", places));
        assert!(place_match("o o", places));
    }

    #[test]
    fn test_white_weight() {
        let white = WhiteArea::default();

        let elements1 = vec![vec![], vec![]];
        assert_eq!(white.get_weight(&elements1), 0.0);
        let elements1 = vec![vec![Element::Face, Element::Diagonal], vec![Element::Face]];
        assert_eq!(white.get_weight(&elements1), 1.0);

        let elements1 = vec![vec![], vec![Element::Face], vec![]];
        assert!((white.get_weight(&elements1) - 1.0 / 3.0).abs() < 0.0001);

        let elements1 = vec![vec![Element::Face, Element::Diagonal], vec![Element::Face]];
        let elements2 = vec![vec![Element::Face], vec![Element::Diagonal]];
        assert!(white.get_weight(&elements1) > white.get_weight(&elements2));

        let elements1 = vec![
            vec![Element::Diagonal, Element::Diagonal],
            vec![Element::Diagonal],
        ];
        let elements2 = vec![vec![Element::Diagonal], vec![Element::Diagonal]];
        assert!(white.get_weight(&elements1) > white.get_weight(&elements2));
    }
}
