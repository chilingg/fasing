pub mod interval;
use interval::{IntervalMatch, SurrIntervalMatch};
pub mod edge_check;
pub use edge_check::{CheckError, EdgeCheck, EdgeMatch};
mod exprs;
pub use exprs::{BoolExps, Exps};

use crate::{
    base::*,
    combination::{StrucComb, attrs, comb, view},
    construct::{CpAttrs, CstType},
};

use serde::{Deserialize, Serialize};
use serde_json as sj;
use sj::json;

use std::collections::BTreeMap;

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
        sj::Value::Object(obj) if obj.contains_key("h") || obj.contains_key("v") => {
            Axis::hv().into_map(|axis| obj.get(axis.symbol()).or(None))
        }
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

pub struct WhiteAssignSetting {
    pub white: DataHV<[f32; 2]>,
    pub corr: DataHV<[sj::Value; 2]>,
    pub units: DataHV<Option<Vec<f32>>>,
}

impl Default for WhiteAssignSetting {
    fn default() -> Self {
        Self {
            white: DataHV::splat([1.0; 2]),
            corr: DataHV::splat([json!(0.0), json!(0.0)]),
            units: DataHV::splat(None),
        }
    }
}

pub struct AssignConfig {
    data: sj::Value,
}

impl AssignConfig {
    pub fn new(data: sj::Value) -> Self {
        Self { data }
    }

    pub fn get_data(&self) -> &sj::Value {
        &self.data
    }

    pub fn size(&self) -> DataHV<f32> {
        self.data
            .get("size")
            .map(|size| {
                get_axis_val(size).into_map(|val| match val {
                    Some(val) => val.as_f64().unwrap_or(1.0) as f32,
                    None => 1.0,
                })
            })
            .unwrap_or(DataHV::splat(1.0))
    }

    pub fn units(&self) -> DataHV<Vec<f32>> {
        const DEFAULT: [f32; 1] = [1.0];

        self.data
            .get("units")
            .map(|size| {
                get_axis_val(size).into_map(|val| match val {
                    Some(val) => sj::from_value(val.clone())
                        .inspect_err(|e| log::error!("Error in config assign units: {e}"))
                        .unwrap_or(DEFAULT.into()),
                    None => DEFAULT.into(),
                })
            })
            .unwrap_or(DataHV::splat(DEFAULT.into()))
    }

    pub fn zimian(&self) -> DataHV<ZiMian> {
        const DEFAULT: [(usize, f32); 1] = [(0, 1.0)];

        self.data
            .get("zimian")
            .map(|size| {
                get_axis_val(size).into_map(|val| match val {
                    Some(val) => sj::from_value::<Vec<(usize, f32)>>(val.clone())
                        .inspect_err(|e| log::error!("Error in config assign zimian: {e}"))
                        .unwrap_or(DEFAULT.into()),
                    None => DEFAULT.into(),
                })
            })
            .unwrap_or(DataHV::splat(DEFAULT.into()))
            .into_map(|list| ZiMian(list))
    }

    pub fn visual_corr(&self) -> DataHV<f32> {
        self.data
            .get("visual_corr")
            .map(|v| {
                get_axis_val(v).into_map(|val| match val {
                    Some(val) => sj::from_value(val.clone())
                        .inspect_err(|e| log::error!("Error in config visual corr: {e}"))
                        .unwrap_or(0.0),
                    None => 0.0,
                })
            })
            .unwrap_or(DataHV::splat(0.0))
    }

    pub fn reduce_trigger(&self) -> DataHV<f32> {
        self.data
            .pointer("/trigger/reduce")
            .map(|v| {
                get_axis_val(v).into_map(|val| match val {
                    Some(val) => sj::from_value(val.clone())
                        .inspect_err(|e| log::error!("Error in config reduce trigger: {e}"))
                        .unwrap_or(0.0),
                    None => 0.0,
                })
            })
            .unwrap_or(DataHV::splat(0.0))
    }

    pub fn replace_trigger(&self) -> DataHV<f32> {
        self.data
            .pointer("/trigger/replace")
            .map(|v| {
                get_axis_val(v).into_map(|val| match val {
                    Some(val) => sj::from_value(val.clone())
                        .inspect_err(|e| log::error!("Error in config replace trigger: {e}"))
                        .unwrap_or(0.0),
                    None => 0.0,
                })
            })
            .unwrap_or(DataHV::splat(0.0))
    }

    pub fn white_setting(&self, size: DataHV<f32>, surround: bool) -> WhiteAssignSetting {
        let mut settings = WhiteAssignSetting::default();

        if let Some(data) = self.data.get("white_area") {
            let surr_setting = if surround { data.get("surround") } else { None };

            let key = "white";
            if let Some(setting) = surr_setting.or(Some(data)).unwrap().get(key).map(|value| {
                get_axis_side_val(value)
                    .into_map(|value| value.map(|v| v.and_then(|v| v.as_str()).unwrap_or("1.0")))
            }) {
                settings.white = setting.zip(size).into_map(|(setting, size)| {
                    let vars = std::collections::HashMap::from([("total", size as f64)]);
                    setting
                        .map(|str| -> anyhow::Result<f32> {
                            let exps = Exps::try_from(str)?;
                            Ok(exps.eval(&vars)? as f32)
                        })
                        .map(|r| {
                            r.inspect_err(|e| log::error!("Error in config assign white: {e}"))
                                .unwrap_or(1.0)
                        })
                });
            }

            let key = "visual_corr";
            if let Some(setting) = surr_setting.or(Some(data)).unwrap().get(key).map(|value| {
                get_axis_side_val(value)
                    .into_map(|value| value.map(|v| v.cloned().unwrap_or(json!(0.0))))
            }) {
                settings.corr = setting;
            }

            let key = "units";
            if let Some(setting) = surr_setting.or(Some(data)).unwrap().get(key).map(|value| {
                get_axis_val(value).into_map(|value| {
                    value.and_then(|v| sj::from_value(v.clone()).unwrap_or_default())
                })
            }) {
                settings.units = setting;
            }
        }

        settings
    }

    pub fn interval_rules(&self) -> Vec<IntervalMatch> {
        self.data
            .pointer("/interval/rules")
            .and_then(|v| sj::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn interval_rules_surr(&self) -> Vec<SurrIntervalMatch> {
        self.data
            .pointer("/interval/surround_rules")
            .and_then(|v| v.as_array())
            .map(|v| {
                v.iter()
                    .filter_map(|v| sj::from_value::<SurrIntervalMatch>(v.clone()).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub fn interval_limit(&self, axis: Axis, surround: bool) -> Option<Exps<'_>> {
        return self
            .data
            .get("interval")
            .and_then(|val| {
                if surround {
                    val.get("surr_limit").or(val.get("limit"))
                } else {
                    val.get("limit")
                }
            })
            .and_then(|val| get_axis_val(val).hv_get(axis).and_then(|val| val.as_str()))
            .and_then(|val| match Exps::try_from(val) {
                Ok(exps) => Some(exps),
                Err(e) => {
                    log::error!(
                        "Error in config assign interval limit, expression parsing error: {e}"
                    );
                    None
                }
            });
    }

    pub fn main_edge(&self) -> Vec<EdgeCheck<[Option<bool>; 2]>> {
        self.data
            .pointer("/main_edge/scale")
            .and_then(|v| {
                sj::from_value(v.clone())
                    .inspect_err(|e| log::error!("Error in config assign main edge: {e}"))
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn main_edge_surround(&self) -> Vec<EdgeCheck<[Option<bool>; 2]>> {
        self.data
            .pointer("/main_edge/surround")
            .and_then(|v| {
                sj::from_value(v.clone())
                    .inspect_err(|e| log::error!("Error in config assign main edge surround: {e}"))
                    .ok()
            })
            .unwrap_or_default()
    }
}

pub struct CtrlsConfig {
    data: sj::Value,
}

impl CtrlsConfig {
    pub fn new(data: sj::Value) -> Self {
        Self { data }
    }

    pub fn get_data(&self) -> &sj::Value {
        &self.data
    }

    pub fn ctrl_order(&self) -> Vec<&str> {
        self.data
            .get("order")
            .and_then(|order| order.as_array())
            .map(|list| list.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn ctrl_data(&self, name: &str) -> Option<&sj::Value> {
        self.data.get(name)
    }
}

pub struct CharsConfig {
    pub supplement: BTreeMap<String, CpAttrs>,
    // 结构-位-字-部件
    // type_replace: BTreeMap<char, BTreeMap<Section, BTreeMap<String, sj::Value>>>,
    // place_replace: BTreeMap<String, Vec<(String, String)>>,
    data: sj::Value,
}

impl CharsConfig {
    pub fn new(data: sj::Value) -> Self {
        let supplement = match data
            .get("supplement")
            .map(|val| sj::from_value(val.clone()))
        {
            None => Default::default(),
            Some(r) => match r {
                Ok(data) => data,
                Err(e) => {
                    log::error!("Error in config char supplement: {e}");
                    Default::default()
                }
            },
        };
        Self { supplement, data }
    }

    pub fn get_data(&self) -> &sj::Value {
        &self.data
    }

    pub fn reduce_replace_name(&self, axis: Axis, name: &str) -> Option<&str> {
        self.data
            .get("reduce_replace")
            .and_then(|table| table.get(axis.symbol()))
            .and_then(|table| table.get(name))
            .and_then(|r| r.as_str())
    }

    pub fn adjacency_replace_name(&self, name1: &str, name2: &str, axis: Axis) -> Option<String> {
        self.data
            .get("adjacency_replace")
            .and_then(|v| v.get(axis.symbol()))
            .and_then(|v| v.get(name1))
            .and_then(|v| v.get(name2))
            .and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    pub fn check_name_replace(
        &self,
        name: &str,
        in_tp: (CstType, Section),
        adjacency: DataHV<[bool; 2]>,
    ) -> Option<String> {
        match self.type_replace_name(name, in_tp, adjacency) {
            Some(comp) => self.place_replace_name(&comp, adjacency).or(Some(comp)),
            None => self.place_replace_name(name, adjacency),
        }
    }

    fn place_replace_name(&self, name: &str, places: DataHV<[bool; 2]>) -> Option<String> {
        self.data
            .get("place_replace")
            .and_then(|v| v.get(name))
            .and_then(|v| {
                v.as_array().and_then(|list| {
                    list.iter().find_map(|v| match (v.get(0), v.get(1)) {
                        (Some(sj::Value::String(r)), Some(sj::Value::String(c)))
                            if place_match(r, places) =>
                        {
                            Some(c.clone())
                        }
                        _ => None,
                    })
                })
            })
    }

    pub fn type_replace_name(
        &self,
        name: &str,
        in_tp: (CstType, Section),
        adjacency: DataHV<[bool; 2]>,
    ) -> Option<String> {
        fn process<'a>(
            table: &'a sj::value::Map<String, sj::Value>,
            name: &str,
            in_tp: (CstType, Section),
            adjacency: DataHV<[bool; 2]>,
        ) -> Option<&'a str> {
            let value = table
                .get(&in_tp.0.symbol().to_string())
                .and_then(|pm| pm.get(in_tp.1.as_str()).and_then(|map| map.get(name)));

            value.and_then(|value| {
                if value.is_string() {
                    value.as_str()
                } else {
                    if let Some(rule) = value.get("adjacency").and_then(|rule| rule.as_str()) {
                        if place_match(rule, adjacency) {
                            return value.get("name").and_then(|name| name.as_str());
                        }
                    }
                    None
                }
            })
        }

        match self.data.get("type_replace") {
            Some(sj::Value::Object(table)) => process(table, name, in_tp, adjacency)
                .map(|mut map_comp| {
                    while let Some(mc) = process(table, map_comp, in_tp, adjacency) {
                        map_comp = mc
                    }
                    map_comp
                })
                .map(|str| str.to_string()),
            _ => None,
        }
    }
}

pub struct Config {
    pub assign: AssignConfig,
    pub ctrls: CtrlsConfig,
    pub chars: CharsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            assign: AssignConfig::new(Default::default()),
            ctrls: CtrlsConfig::new(Default::default()),
            chars: CharsConfig::new(Default::default()),
        }
    }
}

impl Serialize for Config {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut data = BTreeMap::new();

        data.insert("assign", self.assign.data.clone());
        data.insert("ctrls", self.ctrls.data.clone());
        data.insert("chars", self.chars.data.clone());

        Serialize::serialize(&data, serializer)
    }
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = Deserialize::deserialize(deserializer)?;
        Ok(Self::new(data))
    }
}

impl Config {
    pub fn new(data: sj::Value) -> Self {
        let mut data = match data {
            serde_json::Value::Object(obj) => obj,
            _ => Default::default(),
        };

        let assign = AssignConfig::new(data.remove("assign").unwrap_or_default());
        let ctrls = CtrlsConfig::new(data.remove("ctrls").unwrap_or_default());
        let chars = CharsConfig::new(data.remove("chars").unwrap_or_default());
        Self {
            assign,
            ctrls,
            chars,
        }
    }

    pub fn from_datas(assign: sj::Value, ctrls: sj::Value, chars: sj::Value) -> Self {
        let assign = AssignConfig::new(assign);
        let ctrls = CtrlsConfig::new(ctrls);
        let chars = CharsConfig::new(chars);
        Self {
            assign,
            ctrls,
            chars,
        }
    }

    fn set_main_comp_axis_in_setting(
        &self,
        comps: &mut Vec<StrucComb>,
        mut status: Vec<[Option<bool>; 2]>,
        axis: Axis,
        len_list: &Vec<usize>,
    ) -> Vec<[Option<bool>; 2]> {
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

        for idx in 0..comps.len() {
            let settings = match &comps[idx].cdata {
                comb::CompData::Single { proto, .. } => proto
                    .attrs
                    .get::<attrs::MainEdgeSetting>()
                    .unwrap_or_default(),
                _ => vec![],
            };
            for mcheck in settings.iter() {
                let force = mcheck
                    .get_value::<bool>("force")
                    .map(|r| {
                        r.unwrap_or_else(|e| {
                            eprint!("Main edge check `fore`: {e}");
                            false
                        })
                    })
                    .unwrap_or(false);
                if status[idx]
                    .iter()
                    .zip(mcheck.setup.iter())
                    .all(|(a, b)| a.is_some() && b.is_some())
                    && !force
                {
                    continue;
                }

                let r = mcheck.is_match_scale(
                    axis,
                    idx,
                    length,
                    len_list,
                    |i, axis, side| get_edge(i, axis, side, &mut edge_datas, comps),
                    |k, v| match k {
                        "adjacencies" => Ok(place_match(
                            v.as_str().unwrap_or(""),
                            comps[idx].attrs.get::<attrs::Adjacencies>().unwrap(),
                        )),
                        "is_max" => {
                            let b = v.as_bool().unwrap_or(true);
                            Ok((len_list[idx] == max_len) == b)
                        }
                        _ => Err(CheckError::UnknowKey(k.to_string())),
                    },
                );

                match r {
                    Ok(r) => {
                        if r {
                            for j in 0..2 {
                                if mcheck.setup[j].is_some() {
                                    status[idx][j] = mcheck.setup[j];
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("In main edge setting: {e}"),
                }
            }
        }

        let settings = self.assign.main_edge();
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

            let mut curr_status = None;
            for i in indexs {
                let r = mcheck.is_match_scale(
                    axis,
                    i,
                    length,
                    len_list,
                    |i, axis, side| get_edge(i, axis, side, &mut edge_datas, comps),
                    |k, v| match k {
                        "not_set" => {
                            let status = curr_status.get_or_insert(status.clone());
                            let b = v.as_bool().unwrap_or(true);
                            let r = (0..2).all(|j| {
                                mcheck.setup[j].is_none()
                                    || !status.iter().any(|s| matches!(s[j], Some(true)))
                            });
                            Ok(r == b)
                        }
                        "other_set" => {
                            let status = curr_status.get_or_insert(status.clone());
                            let b = v.as_bool().unwrap_or(true);
                            let r = status
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .any(|(_, s)| s.iter().any(|s| matches!(s, Some(true))));
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
                        "adjacencies" => Ok(place_match(
                            v.as_str().unwrap_or(""),
                            comps[i].attrs.get::<attrs::Adjacencies>().unwrap(),
                        )),
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

        status
    }

    fn set_main_comp_surround_in_setting(
        &self,
        primary: &mut StrucComb,
        secondary: &mut StrucComb,
        surround: &DataHV<Section>,
        mut status: DataHV<[Option<bool>; 2]>,
        len_list: [DataHV<usize>; 2],
    ) -> DataHV<[Option<bool>; 2]> {
        let edges: DataHV<[view::EdgeShape; 2]> = Axis::hv().into_map(|axis| {
            Side::fb().map(|from| secondary.get_edge(axis, from, false).to_shape())
        });
        let adjacency_edges: DataHV<[Option<view::EdgeShape>; 2]> =
            comb::surround::get_surround_inside_edge(primary, surround)
                .into_map(|list| list.map(|e| e.map(|e| e.to_shape())));

        let axis_edges = Axis::hv().into_map(|axis| {
            Side::fb().map(|from| {
                if surround.hv_get_into(axis) == from.inverse().to_section() {
                    let (p1, _, p2) =
                        comb::surround::get_primary_area_edge(primary, surround, axis, from);
                    [p1, p2]
                } else {
                    [const { None }; 2]
                }
                .map(|edge| edge.map(|e| e.to_shape()))
            })
        });

        for axis in Axis::list() {
            if surround.hv_get_into(axis) == Section::Middle {
                continue;
            }
            let status = status.hv_get_mut(axis);
            let len_list = len_list.map(|l| l.hv_get_into(axis));
            let max = *len_list.iter().max().unwrap();

            for mcheck in self.assign.main_edge_surround().iter() {
                let force = mcheck
                    .get_value::<bool>("force")
                    .map(|r| {
                        r.unwrap_or_else(|e| {
                            eprint!("Surround Main edge check `fore`: {e}");
                            false
                        })
                    })
                    .unwrap_or(false);

                if !force {
                    if status
                        .iter()
                        .zip(mcheck.setup.iter())
                        .any(|(a, b)| a.is_some() && b.is_some())
                    {
                        continue;
                    }
                }

                let r = mcheck.is_match_surround(
                    axis,
                    surround,
                    &edges,
                    &axis_edges,
                    &adjacency_edges,
                    &primary.attrs.get::<attrs::Adjacencies>().unwrap(),
                    |k, v| match k {
                        "not_set" => {
                            let b = v.as_bool().unwrap_or(true);
                            let r = status.iter().all(|s| s.is_none());
                            Ok(r == b)
                        }
                        "state_p" => {
                            let b = v.as_bool();
                            Ok(status[0] == b)
                        }
                        "state_s" => {
                            let b = v.as_bool();
                            Ok(status[1] == b)
                        }
                        "is_max_p" => {
                            let b = v.as_bool().unwrap_or(true);
                            Ok((len_list[0] == max) == b)
                        }
                        "is_max_s" => {
                            let b = v.as_bool().unwrap_or(true);
                            Ok((len_list[1] == max) == b)
                        }
                        _ => Err(CheckError::UnknowKey(k.to_string())),
                    },
                );

                match r {
                    Ok(r) => {
                        if r {
                            for j in 0..2 {
                                if mcheck.setup[j].is_some() {
                                    status[j] = mcheck.setup[j];
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("In surround main edge setting: {e}"),
                }
            }
        }

        status
    }

    pub fn set_main_comp_surround(
        &self,
        primary: &mut StrucComb,
        secondary: &mut StrucComb,
        surround: &DataHV<Section>,
        len_list: [DataHV<usize>; 2],
        s_len: DataHV<usize>,
    ) {
        let mut status = DataHV::splat([Option::<bool>::None; 2]);
        if let comb::CompData::Single { proto, .. } = &primary.cdata {
            if let Some(state) = proto.attrs.get::<attrs::MainEdge>().and_then(|mut data| {
                data.remove(&CstType::Surround(*surround).symbol().to_string())
            }) {
                for axis in Axis::list() {
                    let i = match axis {
                        Axis::Horizontal => 0,
                        Axis::Vertical => 1,
                    };
                    status.hv_get_mut(axis)[0] = state[i];
                }
            }
        }
        let mut status =
            self.set_main_comp_surround_in_setting(primary, secondary, surround, status, len_list);

        let mut comps = [primary, secondary];
        for axis in Axis::list() {
            let status = status.hv_get_mut(axis);
            for (i, c) in comps.iter().enumerate() {
                if c.attrs
                    .get::<attrs::MainComp>()
                    .map(|data| *data.hv_get(axis))
                    .unwrap_or_default()
                {
                    status[i] = Some(true);
                }
            }

            let default = !status.iter().any(|status| matches!(status, Some(true)));
            if status.iter().all(|state| matches!(state, Some(false))) {
                *status = [Some(true); 2];
            }

            if s_len.hv_get_into(axis) == 0 {
                status[1] = Some(false);
            }

            for side in Side::fb() {
                if surround.hv_get_into(axis) == side.inverse().to_section() {
                    comps.iter_mut().zip(status.iter()).for_each(|(c, s)| {
                        if s.unwrap_or(default) {
                            c.blanks.hv_get_mut(axis)[side.n()] = Default::default();
                        } else {
                            c.blanks.hv_get_mut(axis)[side.n()] = AssignVal::new(1.0, 0.0);
                        }
                    });
                }
            }
        }
    }

    pub fn set_main_comp_axis(
        &self,
        comps: &mut Vec<StrucComb>,
        axis: Axis,
        len_list: &Vec<usize>,
    ) {
        let mut status = vec![[Option::<bool>::None; 2]; comps.len()];
        for i in 0..comps.len() {
            for side in Side::fb() {
                if let Some(comp) = comps[i].get_edge_main_comb(axis, side) {
                    if let comb::CompData::Single { proto, .. } = &comp.cdata {
                        if let Some(state) =
                            proto
                                .attrs
                                .get::<attrs::MainEdge>()
                                .and_then(|data| -> Option<bool> {
                                    data.get(axis.symbol()).and_then(|list| list[side.n()])
                                })
                        {
                            status[i][side.n()] = Some(state);
                        }
                    }
                }
            }
        }

        status = self.set_main_comp_axis_in_setting(comps, status, axis, len_list);
        for i in 0..comps.len() {
            if comps[i]
                .attrs
                .get::<attrs::MainComp>()
                .map(|data| *data.hv_get(axis))
                .unwrap_or_default()
            {
                status[i].iter_mut().for_each(|state| *state = Some(true));
            }
        }

        for i in 0..comps.len() {
            if len_list[i] == 0 {
                status[i].iter_mut().for_each(|state| *state = Some(false));
            }
        }

        let default = [0, 1].map(|i| !status.iter().any(|status| matches!(status[i], Some(true)))); // 谵
        // for i in [0, 1] {
        //     if !status.iter().any(|status| matches!(status[i], Some(true))) {
        //         status.iter_mut().enumerate().for_each(|(j, status)| {
        //             if len_list[j] != 0 {
        //                 status[i] = Some(true)
        //             }
        //         });
        //     }
        // }

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
        let data = json!({
            "chars": {
                "type_replace": {
                    "□": {
                        "Start": {
                            "丯": "丰"
                        }
                    },
                    "⿱": {
                        "End": {
                            "土": {
                        "adjacency": "* o * *",
                        "name": "提土旁"
                            }
                        }
                    }
                }
            }
        });
        let cfg = Config::new(data);

        let r = cfg
            .chars
            .type_replace_name("丯", (CstType::Single, Section::Start), Default::default())
            .unwrap();
        assert_eq!(r, "丰".to_string());

        let r = cfg.chars.type_replace_name(
            "土",
            (CstType::Single, Section::Start),
            DataHV::splat([false, true]),
        );
        assert!(r.is_none());
        let r = cfg.chars.type_replace_name(
            "土",
            (CstType::Scale(Axis::Vertical), Section::End),
            DataHV::splat([false, false]),
        );
        assert!(r.is_none());
        let r = cfg
            .chars
            .type_replace_name(
                "土",
                (CstType::Scale(Axis::Vertical), Section::End),
                DataHV::splat([false, true]),
            )
            .unwrap();
        assert_eq!(r, "提土旁".to_string());
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
        assert!(place_match("x;o o", places));
    }
}
