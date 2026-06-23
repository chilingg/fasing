use crate::base::*;

use serde::{Deserialize, Serialize};
use serde_json as sj;

use std::collections::{BTreeMap, BTreeSet};

pub trait CompAttrData {
    type Data;

    fn key() -> &'static str;

    fn from_sj_value(attr: sj::Value) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        match sj::from_value::<Self::Data>(attr) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Error parsing attributes `{}`: \n{}", Self::key(), e);
                None
            }
        }
    }

    fn to_sj_value(attr: &Self::Data) -> Option<sj::Value>
    where
        Self::Data: serde::Serialize,
    {
        match sj::to_value(attr) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Error attributes `{}`: \n{}", Self::key(), e);
                None
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CompAttrs(BTreeMap<String, sj::Value>);

impl CompAttrs {
    pub fn get<T: CompAttrData>(&self) -> Option<<T as CompAttrData>::Data>
    where
        <T as CompAttrData>::Data: serde::de::DeserializeOwned,
    {
        self.0
            .get(T::key())
            .and_then(|v| <T as CompAttrData>::from_sj_value(v.clone()))
    }

    pub fn set<T: CompAttrData>(&mut self, attr: &T::Data)
    where
        <T as CompAttrData>::Data: serde::Serialize,
    {
        if let Some(value) = T::to_sj_value(attr) {
            self.0.insert(T::key().to_string(), value);
        }
    }
}

pub struct Allocs;
impl CompAttrData for Allocs {
    type Data = DataHV<Vec<usize>>;
    fn key() -> &'static str {
        "allocs"
    }
}

pub struct MainEdge;
impl CompAttrData for MainEdge {
    type Data = DataHV<[Option<bool>; 2]>;
    fn key() -> &'static str {
        "main_edge"
    }

    fn from_sj_value(attr: serde_json::Value) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        let data = Axis::hv().into_map(|axis| {
            match attr
                .get(axis.symbol())
                .map(|v| sj::from_value::<[Option<bool>; 2]>(v.clone()))
            {
                Some(Ok(data)) => data,
                Some(Err(e)) => {
                    log::error!("Error parsing attributes `{}`: \n{}", Self::key(), e);
                    Default::default()
                }
                None => Default::default(),
            }
        });

        Some(data)
    }
}

pub struct TrendSetting;
impl CompAttrData for TrendSetting {
    type Data = Vec<crate::config::EdgeCheck<[Option<String>; 2]>>;
    fn key() -> &'static str {
        "trend_setting"
    }
}

pub struct EdgeAreaSetting;
impl CompAttrData for EdgeAreaSetting {
    type Data = Vec<crate::config::EdgeCheck<[Option<String>; 2]>>;
    fn key() -> &'static str {
        "edge_area_setting"
    }
}

pub struct CurrAreaWeights;
impl CompAttrData for CurrAreaWeights {
    type Data = DataHV<Vec<f32>>;
    fn key() -> &'static str {
        "curr_area_Weights"
    }
}

pub struct AreaWeights {
    normal: DataHV<Vec<f32>>,
    variation: DataHV<Option<Vec<f32>>>,
}

impl AreaWeights {
    pub fn new(normal: DataHV<Vec<f32>>, variation: DataHV<Option<Vec<f32>>>) -> Self {
        Self { normal, variation }
    }

    pub fn get_weights(&self, size: DataHV<f32>) -> DataHV<Vec<f32>> {
        Axis::hv().into_map(|axis| {
            let variation = self
                .variation
                .hv_get(axis)
                .as_ref()
                .unwrap_or(self.normal.hv_get(axis));
            let t = (size.hv_get(axis.inverse()) / size.hv_get(axis)).min(1.0);

            self.normal
                .hv_get(axis)
                .iter()
                .zip(variation.iter())
                .map(|(&n, &v)| ((n - v) * t + v).max(0.0))
                .collect()
        })
    }
}

impl Serialize for AreaWeights {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut len = 2;
        len += [self.variation.h.is_some(), self.variation.v.is_some()]
            .map(|b| if b { 1 } else { 0 })
            .into_iter()
            .sum::<usize>();

        let mut state = serializer.serialize_struct("AreaWeights", len)?;
        state.serialize_field("h", &self.normal.h)?;
        state.serialize_field("v", &self.normal.v)?;

        if let Some(allocs) = self.variation.h.as_ref() {
            state.serialize_field("wide", allocs)?;
        } else {
            state.skip_field("wide")?;
        }
        if let Some(allocs) = self.variation.v.as_ref() {
            state.serialize_field("tall", allocs)?;
        } else {
            state.skip_field("tall")?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for AreaWeights {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn to_vec(val: serde_json::Value) -> Option<Vec<f32>> {
            sj::from_value::<Vec<f32>>(val).ok()
        }

        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::Object(mut obj) => {
                let h = obj.remove("h").and_then(to_vec);
                let v = obj.remove("v").and_then(to_vec);
                if h.is_some() || v.is_some() {
                    let normal = DataHV::new(h.unwrap_or_default(), v.unwrap_or_default());
                    let variation = DataHV::new(
                        obj.remove("wide").and_then(to_vec),
                        obj.remove("tall").and_then(to_vec),
                    );
                    Ok(AreaWeights { normal, variation })
                } else {
                    Err(serde::de::Error::missing_field("h or v"))
                }
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to AreaWeights in {}",
                val
            ))),
        }
    }
}

impl CompAttrData for AreaWeights {
    type Data = AreaWeights;
    fn key() -> &'static str {
        "area_weights"
    }
}

pub struct InPlaceAllocs;
impl CompAttrData for InPlaceAllocs {
    type Data = Vec<(String, DataHV<Vec<usize>>)>;
    fn key() -> &'static str {
        "in_place"
    }
}

pub struct InSurroundAllocs;
impl CompAttrData for InSurroundAllocs {
    type Data = Vec<(String, DataHV<Vec<usize>>)>;
    fn key() -> &'static str {
        "in_surround"
    }
}

pub struct CharBox;
impl CompAttrData for CharBox {
    type Data = WorkBox;
    fn key() -> &'static str {
        "char_box"
    }

    fn from_sj_value(attr: serde_json::Value) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        if let Some(cbox_str) = attr.as_str() {
            match cbox_str {
                "left" => Some(WorkBox::new(
                    WorkPoint::new(0.0, 0.0),
                    WorkPoint::new(0.5, 1.0),
                )),
                "right" => Some(WorkBox::new(
                    WorkPoint::new(0.5, 0.0),
                    WorkPoint::new(1.0, 1.0),
                )),
                "top" => Some(WorkBox::new(
                    WorkPoint::new(0.0, 0.0),
                    WorkPoint::new(1.0, 0.5),
                )),
                "bottom" => Some(WorkBox::new(
                    WorkPoint::new(0.0, 0.5),
                    WorkPoint::new(1.0, 1.0),
                )),
                _ => {
                    eprintln!("Unknown character box label: {}", cbox_str);
                    None
                }
            }
        } else if let Ok(cbox) = serde_json::from_value::<Self::Data>(attr) {
            Some(cbox)
        } else {
            None
        }
    }
}

pub struct ReduceAlloc;
impl CompAttrData for ReduceAlloc {
    type Data = DataHV<Vec<Vec<usize>>>;
    fn key() -> &'static str {
        "reduce_alloc"
    }
}

pub struct FixedAlloc;
impl CompAttrData for FixedAlloc {
    type Data = DataHV<BTreeSet<usize>>;
    fn key() -> &'static str {
        "fixed_Alloc"
    }
}

pub struct IntervalAlloc {
    pub rules: Vec<crate::config::EdgeMatch>,
    pub blanks: Option<[bool; 2]>,
    pub in_surround: BTreeSet<String>,
    pub setup: BTreeMap<String, sj::Value>,
}

impl Serialize for IntervalAlloc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut len = 2;
        if self.blanks.is_some() {
            len += 1;
        }
        if !self.in_surround.is_empty() {
            len += 1;
        }

        let mut state = serializer.serialize_struct("IntervalAlloc", len)?;
        state.serialize_field("rules", &self.rules)?;
        state.serialize_field("setup", &self.setup)?;

        if let Some(blanks) = self.blanks.as_ref() {
            state.serialize_field("blanks", blanks)?;
        } else {
            state.skip_field("blanks")?;
        }

        if self.in_surround.is_empty() {
            state.skip_field("in_surround")?;
        } else {
            state.serialize_field("in_surround", &self.in_surround)?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for IntervalAlloc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut obj: sj::value::Map<_, _> = Deserialize::deserialize(deserializer)?;

        let key = "rules";
        let rules = obj
            .remove(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|value| sj::from_value(value))
            .map_err(|e| serde::de::Error::custom(e))?;
        let key = "setup";
        let setup = obj
            .remove(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|value| sj::from_value(value))
            .map_err(|e| serde::de::Error::custom(e))?;

        let blanks = obj
            .remove("blanks")
            .and_then(|val| sj::from_value::<[bool; 2]>(val).ok());
        let in_surround = obj
            .remove("in_surround")
            .and_then(|val| sj::from_value(val).ok())
            .unwrap_or_default();

        Ok(Self {
            rules,
            blanks,
            in_surround,
            setup,
        })
    }
}

impl CompAttrData for IntervalAlloc {
    type Data = BTreeMap<Axis, BTreeMap<Side, Vec<IntervalAlloc>>>;
    fn key() -> &'static str {
        "interval_alloc"
    }
}

pub struct IntervalSurrAlloc;
impl CompAttrData for IntervalSurrAlloc {
    type Data = BTreeMap<String, BTreeMap<Axis, BTreeMap<Side, Vec<IntervalAlloc>>>>;
    fn key() -> &'static str {
        "interval_surr_alloc"
    }
}

pub struct LineWeight;
impl CompAttrData for LineWeight {
    type Data = DataHV<Option<f32>>;
    fn key() -> &'static str {
        "line_weight"
    }

    fn from_sj_value(attr: sj::Value) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        Some(Axis::hv().into_map(|axis| {
            attr.get(axis.symbol())
                .and_then(|val| val.as_f64())
                .map(|val| val as f32)
        }))
    }

    fn to_sj_value(attr: &Self::Data) -> Option<sj::Value>
    where
        Self::Data: serde::Serialize,
    {
        let mut json = sj::json!({});
        let obj = json.as_object_mut().unwrap();
        for axis in Axis::list() {
            if let Some(val) = *attr.hv_get(axis) {
                obj.insert(axis.symbol().to_string(), val.into());
            }
        }
        Some(json)
    }
}

// ================================= comp

pub struct Adjacencies;
impl CompAttrData for Adjacencies {
    type Data = DataHV<[bool; 2]>;
    fn key() -> &'static str {
        "adjacencies"
    }
}

pub struct InSurround;
impl CompAttrData for InSurround {
    type Data = DataHV<[bool; 2]>;
    fn key() -> &'static str {
        "in_surround"
    }
}

pub struct WhiteArea;
impl CompAttrData for WhiteArea {
    type Data = DataHV<[f32; 2]>;
    fn key() -> &'static str {
        "white_area"
    }
}

pub struct ReduceTarget;
impl CompAttrData for ReduceTarget {
    type Data = DataHV<Option<usize>>;
    fn key() -> &'static str {
        "reduce_target"
    }
}

pub struct MainComp;
impl CompAttrData for MainComp {
    type Data = DataHV<bool>;
    fn key() -> &'static str {
        "main_comp"
    }
}
