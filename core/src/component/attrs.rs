use crate::{axis::*, construct::space::*};

use serde::{Deserialize, Serialize};
extern crate serde_json as sj;

use std::collections::BTreeMap;

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

pub struct InPlaceAllocs;
impl CompAttrData for InPlaceAllocs {
    type Data = Vec<(String, DataHV<Vec<usize>>)>;
    fn key() -> &'static str {
        "in_place"
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

pub struct ReduceAllc;
impl CompAttrData for ReduceAllc {
    type Data = DataHV<Vec<Vec<usize>>>;
    fn key() -> &'static str {
        "ruduce_alloc"
    }
}

pub struct PresetCenter;
impl CompAttrData for PresetCenter {
    type Data = DataHV<Option<f32>>;
    fn key() -> &'static str {
        "preset_center"
    }

    fn from_sj_value(attr: serde_json::Value) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        attr.as_object().map(|data| {
            DataHV::new(
                data.get("h").and_then(|v| v.as_f64().map(|f| f as f32)),
                data.get("v").and_then(|v| v.as_f64().map(|f| f as f32)),
            )
        })
    }

    fn to_sj_value(attr: &Self::Data) -> Option<serde_json::Value>
    where
        Self::Data: serde::Serialize,
    {
        let mut data = sj::Map::new();
        if let Some(val) = attr.h {
            data.insert("h".to_string(), sj::json!(val));
        }
        if let Some(val) = attr.v {
            data.insert("v".to_string(), sj::json!(val));
        }
        Some(sj::Value::Object(data))
    }
}
