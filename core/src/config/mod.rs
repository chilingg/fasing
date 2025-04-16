use crate::{
    axis::*,
    component::strategy::PlaceMain,
    construct::{Component, CpAttrs, CstType},
};
pub mod interval;
use interval::Interval;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Operation<O, E> {
    pub operation: O,
    pub execution: E,
}

impl<O, E> Operation<O, E> {
    pub fn new(operation: O, execution: E) -> Self {
        Self {
            operation,
            execution,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WhiteArea {
    pub fixed: f32,
    pub allocated: f32,
    pub value: f32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct TypeDate<A, S> {
    pub axis: A,
    pub surround: S,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum SpaceProcess {
    Center,
    CompCenter,
    CenterArea,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub min_val: DataHV<Vec<f32>>,
    pub white: DataHV<WhiteArea>,
    pub comp_white: DataHV<WhiteArea>,
    pub strok_width: f32,

    pub supplement: BTreeMap<String, CpAttrs>,
    // 结构-位-字-部件
    pub type_replace: BTreeMap<char, BTreeMap<Place, BTreeMap<String, Component>>>,
    pub place_replace: BTreeMap<String, Vec<(String, Component)>>,

    pub interval: Interval,

    pub center: DataHV<Operation<f32, f32>>,
    pub comp_center: DataHV<Operation<f32, f32>>,
    pub center_area: DataHV<Operation<(f32, f32), f32>>,
    pub process_control: Vec<SpaceProcess>,

    pub strategy: TypeDate<
        DataHV<BTreeMap<Place, BTreeMap<Place, BTreeSet<PlaceMain>>>>,
        BTreeMap<char, DataHV<BTreeSet<PlaceMain>>>,
    >,

    pub reduce_trigger: DataHV<f32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DataHV::splat(1.0),
            min_val: DataHV::splat(vec![Self::DEFAULT_MIN_VALUE]),
            white: Default::default(),
            comp_white: Default::default(),
            strok_width: Self::DEFAULT_MIN_VALUE,
            supplement: Default::default(),
            type_replace: Default::default(),
            place_replace: Default::default(),
            interval: Default::default(),
            center: DataHV::splat(Operation::new(0.5, 0.5)),
            comp_center: DataHV::splat(Operation::new(0.5, 0.5)),
            center_area: Default::default(),
            process_control: vec![SpaceProcess::Center, SpaceProcess::CompCenter],
            strategy: Default::default(),
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
    fn test_type_replace_name() {
        let mut cfg = Config::default();
        cfg.type_replace.insert(
            '□',
            std::collections::BTreeMap::from([(
                crate::axis::Place::Start,
                std::collections::BTreeMap::from([(
                    "丯".to_string(),
                    crate::construct::Component::from_name("丰"),
                )]),
            )]),
        );

        let r = cfg
            .type_replace_name("丯", CstType::Single, Place::Start)
            .unwrap();
        match r {
            Component::Char(name) => assert_eq!(name, "丰".to_string()),
            Component::Complex(_) => unreachable!(),
        }
    }

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
}
