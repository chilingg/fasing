pub mod interval;
use interval::IntervalMatch;
pub mod edge_check;
pub use edge_check::{CheckError, EdgeCheck, EdgeMatch};

use crate::{
    base::*,
    combination::{StrucComb, attrs, view},
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

pub fn get_side_val(value: &sj::Value) -> [Option<&sj::Value>; 2] {
    match value {
        sj::Value::Array(array) => [array.get(0), array.get(1)],
        _ => [Some(value); 2],
    }
}

pub fn get_axis_val(value: &sj::Value) -> DataHV<Option<&sj::Value>> {
    match value {
        sj::Value::Object(obj) => Axis::hv().into_map(|axis| obj.get(axis.symbol())),
        _ => DataHV::splat(Some(value)),
    }
}

pub fn get_axis_side_val(value: &sj::Value) -> DataHV<[Option<&sj::Value>; 2]> {
    match value {
        sj::Value::Object(map) => DataHV::new(
            map.get("h").map(|v| get_side_val(v)).unwrap_or_default(),
            map.get("v").map(|v| get_side_val(v)).unwrap_or_default(),
        ),
        sj::Value::Array(_) => DataHV::splat(get_side_val(value)),
        _ => DataHV::splat([Some(value); 2]),
    }
}

mod keys {
    pub const SIZE: &str = "size";
    pub const UNITS: &str = "units";
    pub const ZIMIAN: &str = "zimian";
    pub const SUPPLEMENT: &str = "supplement";
    pub const TYPE_REPLACE: &str = "type_replace";
    pub const PLACE_REPLACE: &str = "place_replace";
    pub const REDUCE_TRIGGER: &str = "reduce_trigger";
    pub const REPLACE_TRIGGER: &str = "replace_trigger";
    pub const VISUAL_CORR: &str = "visual_corr";
    pub const REDUCE_REPLACE: &str = "reduce_replace";
    pub const SPACE_CTRLS: &str = "space_ctrls";
    pub const SPACE_ASSIGN: &str = "space_assign";
    pub const INTERVAL: &str = "interval";
    pub const MAIN_EDGE: &str = "main_edge";
}

#[derive(Clone)]
pub struct Config {
    pub size: DataHV<f32>,
    pub units: DataHV<Vec<f32>>,
    pub zimian: DataHV<ZiMian>,

    pub supplement: BTreeMap<String, CpAttrs>,
    // 结构-位-字-部件
    type_replace: BTreeMap<char, BTreeMap<Section, BTreeMap<String, String>>>,
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

        let key = keys::SIZE;
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

        let key = keys::UNITS;
        let parse = |val: &sj::Value| {
            sj::from_value::<Vec<f32>>(val.clone()).map_err(|e| {
                serde::de::Error::custom(format!("Config Error: Invalid `{val}` in `{key}`!\n{e}"))
            })
        };
        let units: DataHV<Vec<f32>> = match data.get(key) {
            None => DataHV::splat(vec![DEFAULT_MIN_VALUE]),
            Some(val) => match val {
                sj::Value::Object(obj) => DataHV::new(
                    parse(obj.get("h").unwrap_or(&json!([DEFAULT_MIN_VALUE])))?,
                    parse(obj.get("v").unwrap_or(&json!([DEFAULT_MIN_VALUE])))?,
                ),
                val => DataHV::splat(parse(val)?),
            },
        };

        let key = keys::ZIMIAN;
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

        let key = keys::SUPPLEMENT;
        let supplement = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        let key = keys::TYPE_REPLACE;
        let type_replace = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        let key = keys::PLACE_REPLACE;
        let place_replace = match data.get(key).map(|val| sj::from_value(val.clone())) {
            None => Default::default(),
            Some(r) => {
                r.map_err(|e| serde::de::Error::custom(format!("Config Error: `{key}` {e}")))?
            }
        };

        Ok(Self {
            size,
            units,
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
            .get(keys::REDUCE_TRIGGER)
            .and_then(|v| v.as_f64().or(v.get(axis.symbol()).and_then(|v| v.as_f64())))
            .unwrap_or(0.0) as f32
    }

    pub fn get_replace_trigger(&self, axis: Axis) -> f32 {
        self.data
            .get(keys::REPLACE_TRIGGER)
            .and_then(|v| v.as_f64().or(v.get(axis.symbol()).and_then(|v| v.as_f64())))
            .unwrap_or(0.0) as f32
    }

    pub fn get_visual_corr(&self, axis: Axis) -> f32 {
        self.data
            .get(keys::VISUAL_CORR)
            .and_then(|v| v.as_f64().or(v.get(axis.symbol()).and_then(|v| v.as_f64())))
            .unwrap_or(0.0) as f32
    }

    pub fn get_space_ctrls(&self) -> (Vec<&str>, Option<&sj::value::Map<String, sj::Value>>) {
        let obj = self.data.get(keys::SPACE_CTRLS).and_then(|d| d.as_object());
        let order = obj
            .and_then(|obj| obj.get("order"))
            .and_then(|order| order.as_array())
            .and_then(|list| list.iter().map(|v| v.as_str()).collect())
            .unwrap_or_default();
        (order, obj)
    }

    pub fn get_space_assign_settings(
        &self,
    ) -> (DataHV<[f32; 2]>, DataHV<[f32; 2]>, DataHV<Option<f32>>) {
        let mut settings = (
            DataHV::splat([1.0; 2]),
            DataHV::splat([0.0; 2]),
            DataHV::splat(None),
        );

        if let Some(data) = self.data.get(keys::SPACE_ASSIGN) {
            if let Some(data) = data.get("white") {
                settings
                    .0
                    .as_mut()
                    .zip(get_axis_side_val(data))
                    .into_iter()
                    .for_each(|(setting, val)| {
                        *setting = val.map(|v| v.and_then(|v| v.as_f64()).unwrap_or(1.0) as f32);
                    });
            }
            if let Some(data) = data.get("visual_corr") {
                settings
                    .1
                    .as_mut()
                    .zip(get_axis_side_val(data))
                    .into_iter()
                    .for_each(|(setting, val)| {
                        *setting = val.map(|v| v.and_then(|v| v.as_f64()).unwrap_or(0.0) as f32);
                    });
            }
            if let Some(data) = data.get("unit") {
                settings
                    .2
                    .as_mut()
                    .zip(get_axis_val(data))
                    .into_iter()
                    .for_each(|(setting, val)| {
                        *setting = val.and_then(|v| v.as_f64()).map(|v| v as f32);
                    });
            }
        }

        settings
    }

    pub fn get_interval_limit(&self, axis: Axis) -> Option<f32> {
        return self
            .data
            .get(keys::INTERVAL)
            .and_then(|val| val.get("limit"))
            .and_then(|val| get_axis_val(val).hv_get(axis).and_then(|val| val.as_f64()))
            .map(|val| val as f32);
    }

    pub fn reduce_replace_name(&self, axis: Axis, name: &str) -> Option<&str> {
        self.data
            .get(keys::REDUCE_REPLACE)
            .and_then(|table| table.get(axis.symbol()))
            .and_then(|table| table.get(name))
            .and_then(|r| r.as_str())
    }

    fn type_replace_name(&self, name: &str, in_tp: (CstType, Section)) -> Option<String> {
        fn process<'a>(
            cfg: &'a Config,
            name: &str,
            in_tp: (CstType, Section),
        ) -> Option<&'a String> {
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
        in_tp: (CstType, Section),
        adjacency: DataHV<[bool; 2]>,
    ) -> Option<String> {
        match self.type_replace_name(name, in_tp) {
            Some(comp) => self.place_replace_name(&comp, adjacency).or(Some(comp)),
            None => self.place_replace_name(name, adjacency),
        }
    }

    fn set_main_comp_axis_in_setting(
        &self,
        comps: &mut Vec<StrucComb>,
        axis: Axis,
        len_list: &Vec<usize>,
    ) -> Vec<[Option<bool>; 2]> {
        let mut status = vec![[Option::<bool>::None; 2]; comps.len()];
        let mut edge_datas: Vec<Vec<Option<view::EdgeShape>>> = vec![vec![None; 4]; comps.len()];
        let max_len = *len_list.iter().max().unwrap();
        let length = comps.len();

        fn get_edge(
            i: usize,
            axis: Axis,
            side: Side,
            edge_datas: &mut Vec<Vec<Option<view::EdgeShape>>>,
            comps: &Vec<StrucComb>,
        ) -> view::EdgeShape {
            let j = match (axis, side) {
                (Axis::Horizontal, Side::Front) => 0,
                (Axis::Horizontal, Side::Back) => 1,
                (Axis::Vertical, Side::Front) => 2,
                (Axis::Vertical, Side::Back) => 3,
            };
            if edge_datas[i][j].is_none() {
                edge_datas[i][j] = Some(comps[i].get_edge(axis, side, false).to_shape());
            }
            edge_datas[i][j].clone().unwrap()
        }

        match self
            .data
            .get(keys::MAIN_EDGE)
            .map(|val| sj::from_value::<Vec<EdgeCheck<bool>>>(val.clone()))
        {
            Some(Ok(settings)) => {
                for mcheck in settings.iter() {
                    let mut indexs: Vec<usize> = (0..comps.len()).collect();
                    let force = mcheck
                        .get_value::<bool>("force")
                        .map(|r| {
                            r.unwrap_or_else(|e| {
                                eprint!("Main edge check `fore`: {e}");
                                false
                            })
                        })
                        .unwrap_or(false);

                    if !force {
                        indexs.retain(|i| {
                            status[*i]
                                .iter()
                                .zip(mcheck.setup.iter())
                                .all(|(a, b)| a.is_none() || b.is_none())
                        })
                    }

                    let mut not_set = None;
                    for i in indexs {
                        let r = mcheck.is_match(
                            axis,
                            i,
                            length,
                            |i, axis, side| get_edge(i, axis, side, &mut edge_datas, comps),
                            |k, v| match k {
                                "not_set" => {
                                    let b = v.as_bool().unwrap_or(true);
                                    let r = *not_set.get_or_insert_with(|| {
                                        (0..2).all(|j| {
                                            mcheck.setup[j].is_none()
                                                || !status
                                                    .iter()
                                                    .any(|s| matches!(s[j], Some(true)))
                                        })
                                    });
                                    Ok(r == b)
                                }
                                "state_f" | "state_b" => {
                                    let b = v.as_bool();
                                    let side = match k {
                                        "state_f" => 0,
                                        "state_b" => 1,
                                        _ => unreachable!(),
                                    };
                                    Ok(status[i][side] == b)
                                }
                                "state_front_f" | "state_front_b" => {
                                    let b = v.as_bool();
                                    let side = match k {
                                        "state_front_f" => 0,
                                        "state_front_b" => 1,
                                        _ => unreachable!(),
                                    };
                                    Ok(i != 0 && status[i - 1][side] == b)
                                }
                                "state_back_f" | "state_back_b" => {
                                    let b = v.as_bool();
                                    let side = match k {
                                        "state_back_f" => 0,
                                        "state_back_b" => 1,
                                        _ => unreachable!(),
                                    };
                                    Ok(i + 1 != length && status[i + 1][side] == b)
                                }
                                "is_max" => {
                                    let b = v.as_bool().unwrap_or(true);
                                    Ok((len_list[i] == max_len) == b)
                                }
                                _ => Err(CheckError::UnknowKey(k.to_string())),
                            },
                        );

                        match r {
                            Ok(r) => {
                                if r {
                                    for j in 0..2 {
                                        if mcheck.setup[j].is_some() {
                                            status[i][j] = mcheck.setup[j];
                                        }
                                    }
                                }
                            }
                            Err(e) => eprintln!("In main edge setting: {e}"),
                        }
                    }
                }
            }
            Some(Err(e)) => eprintln!("Error in Main Edge settings: {e}"),
            None => {}
        }

        status
    }

    // fn set_main_comp_axis_in_setting(
    //     &self,
    //     comps: &mut Vec<StrucComb>,
    //     axis: Axis,
    //     len_list: &Vec<usize>,
    // ) -> Vec<[Option<bool>; 2]> {
    //     let mut status = vec![[Option::<bool>::None; 2]; comps.len()];
    //     let mut edge_datas: Vec<Vec<Option<view::EdgeShape>>> = vec![vec![None; 4]; comps.len()];
    //     fn get_edge<'a>(
    //         i: usize,
    //         axis: Axis,
    //         side: Side,
    //         edge_datas: &'a mut Vec<Vec<Option<view::EdgeShape>>>,
    //         comps: &Vec<StrucComb>,
    //     ) -> &'a view::EdgeShape {
    //         let j = match (axis, side) {
    //             (Axis::Horizontal, Side::Front) => 0,
    //             (Axis::Horizontal, Side::Back) => 1,
    //             (Axis::Vertical, Side::Front) => 2,
    //             (Axis::Vertical, Side::Back) => 3,
    //         };
    //         if edge_datas[i][j].is_none() {
    //             edge_datas[i][j] = Some(comps[i].get_edge(axis, side, false).to_shape());
    //         }
    //         edge_datas[i][j].as_ref().unwrap()
    //     }
    //     let max_len = *len_list.iter().max().unwrap();

    //     match self
    //         .data
    //         .get(keys::MAIN_EDGE)
    //         .map(|val| sj::from_value::<Vec<EdgeCheck>>(val.clone()))
    //     {
    //         Some(Ok(settings)) => {
    //             for mcheck in settings.iter() {
    //                 let mut indexs: Vec<usize> = (0..comps.len()).collect();
    //                 if !mcheck.force {
    //                     indexs.retain(|i| {
    //                         status[*i]
    //                             .iter()
    //                             .zip(mcheck.setup.iter())
    //                             .any(|(a, b)| a.is_none() && b.is_some())
    //                     })
    //                 }

    //                 for (k, v) in mcheck.conditions.iter() {
    //                     let k = k.as_str();
    //                     match k {
    //                         "axis" => match sj::from_value::<Axis>(v.clone()) {
    //                             Ok(t_axis) if axis != t_axis => indexs.clear(),
    //                             Err(_) => {
    //                                 eprintln!("Main edge setting: Unknown `{v}` in `{k}`!");
    //                                 indexs.clear();
    //                             }
    //                             _ => {}
    //                         },
    //                         "section" => match sj::from_value::<Section>(v.clone()) {
    //                             Ok(section) => indexs.retain(|&i| {
    //                                 if i == 0 {
    //                                     section == Section::Start
    //                                 } else if i + 1 == comps.len() {
    //                                     section == Section::End
    //                                 } else {
    //                                     section == Section::Middle
    //                                 }
    //                             }),
    //                             Err(_) => {
    //                                 eprintln!("Main edge setting: Unknown `{v}` in `{k}`!");
    //                                 indexs.clear();
    //                             }
    //                         },
    //                         "not_set" => {
    //                             let b = v.as_bool().unwrap_or(true);
    //                             (0..2).for_each(|i| {
    //                                 if mcheck.setup[i].is_some() {
    //                                     if status.iter().any(|s| matches!(s[i], Some(true))) == b {
    //                                         indexs.clear();
    //                                     }
    //                                 }
    //                             });
    //                         }
    //                         "edge1" | "edge2" | "edge1_cross" | "edge2_cross" => {
    //                             match sj::from_value::<EdgeMatch>(v.clone()) {
    //                                 Ok(rule) => {
    //                                     let (axis, side) = match k {
    //                                         "edge1" => (axis, Side::Front),
    //                                         "edge2" => (axis, Side::Back),
    //                                         "edge1_cross" => (axis.inverse(), Side::Front),
    //                                         "edge2_cross" => (axis.inverse(), Side::Back),
    //                                         _ => unreachable!(),
    //                                     };
    //                                     indexs.retain(|&i| {
    //                                         rule.is_match(&get_edge(
    //                                             i,
    //                                             axis,
    //                                             side,
    //                                             &mut edge_datas,
    //                                             comps,
    //                                         ))
    //                                     });
    //                                 }
    //                                 Err(e) => eprintln!("Main Edge Setting: {e}"),
    //                             }
    //                         }
    //                         "state_f" | "state_b" => {
    //                             let b = v.as_bool();
    //                             let side = match k {
    //                                 "state_f" => 0,
    //                                 "state_b" => 1,
    //                                 _ => unreachable!(),
    //                             };
    //                             indexs.retain(|&i| status[i][side] == b);
    //                         }
    //                         "state_front_f" | "state_front_b" => {
    //                             let b = v.as_bool();
    //                             let side = match k {
    //                                 "state_front_f" => 0,
    //                                 "state_front_b" => 1,
    //                                 _ => unreachable!(),
    //                             };
    //                             indexs.retain(|&i| i != 0 && status[i - 1][side] == b);
    //                         }
    //                         "front_edge" | "front_edge_f" | "front_edge_b" => {
    //                             match sj::from_value::<EdgeMatch>(v.clone()) {
    //                                 Ok(rule) => {
    //                                     let (axis, side) = match k {
    //                                         "front_edge" => (axis.inverse(), Side::Back),
    //                                         "front_edge_f" => (axis, Side::Front),
    //                                         "front_edge_b" => (axis, Side::Back),
    //                                         _ => unreachable!(),
    //                                     };
    //                                     indexs.retain(|&i| {
    //                                         i != 0
    //                                             && rule.is_match(&get_edge(
    //                                                 i - 1,
    //                                                 axis,
    //                                                 side,
    //                                                 &mut edge_datas,
    //                                                 comps,
    //                                             ))
    //                                     });
    //                                 }
    //                                 Err(e) => eprintln!("Main Edge Setting: {e}"),
    //                             }
    //                         }
    //                         "back_edge" | "back_edge_f" | "back_edge_b" => {
    //                             match sj::from_value::<EdgeMatch>(v.clone()) {
    //                                 Ok(rule) => {
    //                                     let (axis, side) = match k {
    //                                         "back_edge" => (axis.inverse(), Side::Front),
    //                                         "back_edge_f" => (axis, Side::Front),
    //                                         "back_edge_b" => (axis, Side::Back),
    //                                         _ => unreachable!(),
    //                                     };
    //                                     indexs.retain(|&i| {
    //                                         i + 1 != comps.len()
    //                                             && rule.is_match(&get_edge(
    //                                                 i + 1,
    //                                                 axis,
    //                                                 side,
    //                                                 &mut edge_datas,
    //                                                 comps,
    //                                             ))
    //                                     });
    //                                 }
    //                                 Err(e) => eprintln!("Main Edge Setting: {e}"),
    //                             }
    //                         }
    //                         "state_back_f" | "state_back_b" => {
    //                             let b = v.as_bool();
    //                             let side = match k {
    //                                 "state_back_f" => 0,
    //                                 "state_back_b" => 1,
    //                                 _ => unreachable!(),
    //                             };
    //                             indexs
    //                                 .retain(|&i| i + 1 != comps.len() && status[i + 1][side] == b);
    //                         }
    //                         "is_max" => {
    //                             let b = v.as_bool().unwrap_or(true);
    //                             indexs.retain(|&i| (len_list[i] == max_len) == b);
    //                         }
    //                         "note" => {}
    //                         _ => eprintln!("Unknown key `{k}` in main edge setting!"),
    //                     }
    //                     if indexs.is_empty() {
    //                         break;
    //                     }
    //                 }
    //                 indexs.into_iter().for_each(|i| {
    //                     for j in 0..2 {
    //                         if mcheck.setup[j].is_some() && (status[i][j].is_none() || mcheck.force)
    //                         {
    //                             status[i][j] = mcheck.setup[j];
    //                         }
    //                     }
    //                 });
    //             }
    //         }
    //         Some(Err(e)) => eprintln!("Error in Main Edge settings: {e}"),
    //         None => {}
    //     }

    //     return status;
    // }

    pub fn set_main_comp_axis(
        &self,
        comps: &mut Vec<StrucComb>,
        axis: Axis,
        len_list: &Vec<usize>,
    ) {
        let mut status = self.set_main_comp_axis_in_setting(comps, axis, len_list);

        for i in 0..comps.len() {
            if comps[i]
                .attrs
                .get::<attrs::MainComp>()
                .map(|data| *data.hv_get(axis))
                .unwrap_or_default()
            {
                status[i].iter_mut().for_each(|state| *state = Some(true));
            }

            if len_list[i] == 0 {
                status[i].iter_mut().for_each(|state| *state = Some(false));
            }
        }

        let mut default = [false; 2];
        for side in Side::fb() {
            let mut mark = 0;
            for state in status.iter().map(|status| status[side.n()]) {
                match state {
                    Some(true) => {
                        mark = 2;
                        break;
                    }
                    Some(false) => mark = 1,
                    None => {}
                }
            }
            match mark {
                1 => default[side.n()] = true,
                0 => status
                    .iter_mut()
                    .for_each(|state| state[side.n()] = Some(true)),
                _ => {}
            }
        }

        status
            .into_iter()
            .map(|state| [0, 1].map(|i| state[i].unwrap_or(default[i])))
            .zip(comps)
            .for_each(|(state, c)| {
                for side in Side::fb() {
                    if state[side.n()] {
                        c.blanks.hv_get_mut(axis)[side.n()] = Default::default();
                    } else {
                        c.blanks.hv_get_mut(axis)[side.n()] = AssignVal::new(1.0, 0.0);
                    }
                }
            });
    }

    pub fn set_intervals_axis(&self, comps: &mut Vec<StrucComb>, axis: Axis) -> Option<Vec<usize>> {
        let mut intervals = Vec::with_capacity(comps.len() - 1);
        for i1 in 0..comps.len() - 1 {
            let i2 = i1 + 1;
            let edge1 = comps[i1].get_edge(axis, Side::Back, true).to_shape();
            let edge2 = comps[i2].get_edge(axis, Side::Front, true).to_shape();
            let mut val = 0;

            let (l, r) = comps.split_at_mut(i2);
            let r = StrucComb::set_edge_alloc(&mut l[i1], &edge1, &mut r[0], &edge2, axis).ok()?;

            if let Some(i_val) = r {
                val = i_val;
            } else if let Some(rules) = self
                .data
                .get(keys::INTERVAL)
                .and_then(|v| v.get("rules"))
                .and_then(|v| v.as_array())
                .map(|v| {
                    v.iter()
                        .filter_map(|v| sj::from_value::<IntervalMatch>(v.clone()).ok())
                        .collect::<Vec<IntervalMatch>>()
                })
            {
                if let Some(i_val) = rules
                    .iter()
                    .find_map(|rule| rule.is_match(&edge1, &edge2, axis))
                {
                    val = i_val;
                }
            };

            intervals.push(val);
        }

        Some(intervals)
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
                1 => places.iter().flatten().all(|e| is_match(place_attr[0], *e)),
                2 => places
                    .iter()
                    .zip(place_attr.iter())
                    .all(|(place, attr)| place.iter().all(|e| is_match(attr, *e))),
                4 => places
                    .iter()
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
                Section::Start,
                std::collections::BTreeMap::from([("丯".to_string(), "丰".to_string())]),
            )]),
        );

        let r = cfg
            .type_replace_name("丯", (CstType::Single, Section::Start))
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
