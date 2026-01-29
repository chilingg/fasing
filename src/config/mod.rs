use crate::{
    base::*,
    construct::{CpAttrs, CstType},
};

use serde::{Deserialize, Serialize};
use serde_json as sj;
use sj::json;

use std::collections::BTreeMap;

const DEFAULT_MIN_VALUE: f32 = 0.05;

#[derive(Clone, Default)]
pub struct ZiMian(Vec<(usize, f32)>);

impl ZiMian {
    pub fn val_in(&self, size: usize) -> f32 {
        let first = self.0.first().unwrap_or(&(usize::MAX, 1.0));

        if size <= first.0 {
            first.1
        } else if size >= self.0.last().unwrap().0 {
            self.0.last().unwrap().1
        } else {
            self.0
                .windows(2)
                .find_map(|vec| {
                    if (vec[0].0..=vec[1].0).contains(&size) {
                        let (x1, x2) = (vec[0].0 as f32, vec[1].0 as f32);
                        let (y1, y2) = (vec[0].1, vec[1].1);

                        Some(((size as f32 - x2) / (x1 - x2)) * (y1 - y2) + y2)
                    } else {
                        None
                    }
                })
                .unwrap_or(1.0)
        }
    }

    pub fn max_val(&self) -> f32 {
        self.0.iter().map(|e| e.1).reduce(f32::max).unwrap_or(1.0)
    }
}

#[derive(Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub min_val: DataHV<Vec<f32>>,
    pub zimian: DataHV<ZiMian>,

    pub supplement: BTreeMap<String, CpAttrs>,
    // 结构-位-字-部件
    type_replace: BTreeMap<char, BTreeMap<Place, BTreeMap<String, String>>>,
    place_replace: BTreeMap<String, Vec<(String, String)>>,

    data: sj::Value,
}

impl Serialize for Config {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Serialize::serialize(&self.data, serializer)
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: sj::Value = Deserialize::deserialize(deserializer)?;

        let key = "size";
        let parse = |val: &sj::Value| match val.as_f64() {
            Some(size) => Ok::<f64, D::Error>(size),
            None => Err(serde::de::Error::custom(format!(
                "Config Error: Invalid `{val}` in `{key}`!"
            ))),
        };
        let size = match data.get(key) {
            None => DataHV::splat(1.0),
            Some(val) => match val {
                sj::Value::Object(obj) => DataHV::new(
                    parse(obj.get("h").unwrap_or(&json!(1)))?,
                    parse(obj.get("v").unwrap_or(&json!(1)))?,
                ),
                val => DataHV::splat(parse(val)?),
            },
        }
        .into_map(|v| v as f32);

        let key = "min_val";
        let parse = |val: &sj::Value| {
            sj::from_value::<Vec<f32>>(val.clone()).map_err(|e| {
                serde::de::Error::custom(format!("Config Error: Invalid `{val}` in `{key}`!\n{e}"))
            })
        };
        let min_val: DataHV<Vec<f32>> = match data.get(key) {
            None => DataHV::splat(vec![DEFAULT_MIN_VALUE]),
            Some(val) => match val {
                sj::Value::Object(obj) => DataHV::new(
                    parse(obj.get("h").unwrap_or(&json!([DEFAULT_MIN_VALUE])))?,
                    parse(obj.get("v").unwrap_or(&json!([DEFAULT_MIN_VALUE])))?,
                ),
                val => DataHV::splat(parse(val)?),
            },
        };

        let key = "zimian";
        let parse = |val: &sj::Value| {
            sj::from_value::<Vec<(usize, f32)>>(val.clone())
                .map(|setting| ZiMian(setting))
                .map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Config Error: Invalid `{val}` in `{key}`!\n{e}"
                    ))
                })
        };
        let zimian: DataHV<ZiMian> = match data.get(key) {
            None => DataHV::splat(ZiMian(vec![(0, 1.0)])),
            Some(val) => match val {
                sj::Value::Object(obj) if obj.contains_key("h") && obj.contains_key("v") => {
                    DataHV::new(parse(obj.get("h").unwrap())?, parse(obj.get("v").unwrap())?)
                }
                val => DataHV::splat(parse(val)?),
            },
        };

        let key = "supplement";
        let supplement = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        let key = "type_replace";
        let type_replace = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        let key = "place_replace";
        let place_replace = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        Ok(Self {
            size,
            min_val,
            zimian,
            supplement,
            type_replace,
            place_replace,
            data,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        let default_data = json!({
            "type_replace": {
              "⿺": {
                "Start": {
                    "虎": "虎字包围",
                    "尺": "尺字包围",
                    "风": "风字包围",
                    "风": "风字包围"
                },
              },
              "⿵": {
                "Start": {
                    "尺": "尺下包围",
                    "戕": "戕下包围",
                },
              },
            }
        });
        sj::from_value(default_data).unwrap()
    }
}

impl Config {
    pub fn get_reduce_trigger(&self, axis: Axis) -> f32 {
        self.data
            .get("reduce_trigger")
            .and_then(|v| v.as_f64().or(v.get(axis.symbol()).and_then(|v| v.as_f64())))
            .unwrap_or(0.0) as f32
    }

    pub fn get_visual_corr(&self, axis: Axis) -> f32 {
        self.data
            .get("visual_corr")
            .and_then(|v| v.as_f64().or(v.get(axis.symbol()).and_then(|v| v.as_f64())))
            .unwrap_or(0.0) as f32
    }

    pub fn reduce_replace_name(&self, axis: Axis, name: &str) -> Option<&str> {
        self.data
            .get("reduce_replace")
            .and_then(|table| table.get(axis.symbol()))
            .and_then(|table| table.get(name))
            .and_then(|r| r.as_str())
    }

    fn type_replace_name(&self, name: &str, in_tp: (CstType, Place)) -> Option<String> {
        fn process<'a>(cfg: &'a Config, name: &str, in_tp: (CstType, Place)) -> Option<&'a String> {
            cfg.type_replace
                .get(&in_tp.0.symbol())
                .and_then(|pm| pm.get(&in_tp.1).and_then(|map| map.get(name)))
        }

        process(self, name, in_tp)
            .map(|mut map_comp| {
                while let Some(mc) = process(self, map_comp, in_tp) {
                    map_comp = mc
                }
                map_comp
            })
            .cloned()
    }

    fn place_replace_name(&self, name: &str, places: DataHV<[bool; 2]>) -> Option<String> {
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

    pub fn check_name_replace(
        &self,
        name: &str,
        in_tp: (CstType, Place),
        adjacency: DataHV<[bool; 2]>,
    ) -> Option<String> {
        match self.type_replace_name(name, in_tp) {
            Some(comp) => self.place_replace_name(&comp, adjacency).or(Some(comp)),
            None => self.place_replace_name(name, adjacency),
        }
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
                _ => {
                    eprintln!(
                        "Excess number {} of symbols! {:?}",
                        place_attr.len(),
                        place_attr
                    );
                    false
                }
            }
        })
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zimian() {
        let setting = vec![(2, 0.2), (5, 0.5), (16, 0.9)];
        let zimian = ZiMian(setting);

        assert_eq!(zimian.val_in(0), 0.2);
        assert_eq!(zimian.val_in(2), 0.2);
        assert_eq!(zimian.val_in(5), 0.5);
        assert_eq!(zimian.val_in(16), 0.9);
        assert_eq!(zimian.val_in(20), 0.9);

        assert!(zimian.val_in(3) < 0.5);
        assert!(zimian.val_in(8) > 0.5);
        assert!(zimian.val_in(15) < 0.9);
    }

    #[test]
    fn test_type_replace_name() {
        let mut cfg = Config::default();
        cfg.type_replace.insert(
            '□',
            std::collections::BTreeMap::from([(
                Place::Start,
                std::collections::BTreeMap::from([("丯".to_string(), "丰".to_string())]),
            )]),
        );

        let r = cfg
            .type_replace_name("丯", (CstType::Single, Place::Start))
            .unwrap();
        assert_eq!(r, "丰".to_string())
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
