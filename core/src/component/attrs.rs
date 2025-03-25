use crate::{axis::*, construct::space::WorkBox};

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
    type Data = sj::Value;
    fn key() -> &'static str {
        "char_box"
    }
}

pub struct ReduceAllc;
impl CompAttrData for ReduceAllc {
    type Data = DataHV<Vec<Vec<usize>>>;
    fn key() -> &'static str {
        "ruduce_alloc"
    }
}
