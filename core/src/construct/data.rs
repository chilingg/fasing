use super::types::CstType;

use serde::{ser::SerializeStruct, Deserialize, Serialize};
extern crate serde_json as sj;
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
};

#[derive(Clone)]
pub struct CpAttrs {
    pub tp: CstType,
    pub components: Vec<Component>,
}

impl CpAttrs {
    pub fn single() -> Self {
        CpAttrs {
            tp: CstType::Single,
            components: vec![],
        }
    }

    pub fn comps_name(&self) -> String {
        format!(
            "{}({})",
            self.tp.symbol(),
            self.components
                .iter()
                .map(|comp| comp.name())
                .collect::<Vec<String>>()
                .join("+")
        )
    }
}

impl Serialize for CpAttrs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.tp {
            CstType::Single => serializer.serialize_str(""),
            tp => {
                let mut s = serializer.serialize_struct("CpAttrs", 2)?;
                s.serialize_field("tp", &tp.symbol())?;
                s.serialize_field("components", &self.components)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for CpAttrs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::String(symbol) => {
                let tp = CstType::from_symbol(&symbol).ok_or(serde::de::Error::custom(format!(
                    "Unkonw construct type: {}",
                    symbol
                )))?;
                Ok(Self {
                    tp,
                    components: vec![],
                })
            }
            serde_json::Value::Object(data) => {
                let tp = match data.get("tp") {
                    Some(val) if val.is_string() => {
                        CstType::from_symbol(val.as_str().unwrap()).ok_or(
                            serde::de::Error::custom(format!("Unkonw construct type: {}", val)),
                        )?
                    }
                    _ => Err(serde::de::Error::custom("Missing field `tp`!"))?,
                };
                let components = match data.get("components") {
                    Some(val) if val.is_array() => sj::from_value(val.clone())
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                    _ => Err(serde::de::Error::custom("Missing field `components`!"))?,
                };
                Ok(Self { tp, components })
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to CpAttrs in {}",
                val
            ))),
        }
    }
}

#[derive(Clone)]
pub enum Component {
    Char(String),
    Complex(CpAttrs),
}

impl Serialize for Component {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Char(name) => serializer.serialize_str(name),
            Self::Complex(attrs) => serializer.serialize_newtype_struct("attrs", attrs),
        }
    }
}

impl<'de> Deserialize<'de> for Component {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::String(str) => Ok(Self::Char(str)),
            serde_json::Value::Object(data) => {
                match sj::from_value::<CpAttrs>(sj::Value::Object(data)) {
                    Ok(attrs) => Ok(Self::Complex(attrs)),
                    _ => Err(serde::de::Error::custom("Conversion fails!")),
                }
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to Component in {}",
                val
            ))),
        }
    }
}

impl Component {
    pub fn from_name<T: ToString>(value: T) -> Self {
        return Self::Char(value.to_string());
    }

    pub fn name(&self) -> String {
        match self {
            Self::Char(name) => name.clone(),
            Self::Complex(attr) => attr.comps_name(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CharTree {
    pub name: String,
    pub tp: CstType,
    pub children: Vec<CharTree>,
}

impl CharTree {
    pub fn new_single(name: String) -> Self {
        Self {
            name,
            tp: CstType::Single,
            children: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CstTable(HashMap<String, CpAttrs>);

impl Deref for CstTable {
    type Target = HashMap<String, CpAttrs>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CstTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for CstTable {
    fn default() -> Self {
        const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));
        Self::from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
    }
}

impl CstTable {
    pub fn target_chars(&self) -> Vec<char> {
        self.keys()
            .filter_map(|key| {
                let mut iter = key.chars();
                iter.next().and_then(|chr| match iter.next() {
                    Some(_) => None,
                    None => Some(chr),
                })
            })
            .collect()
    }

    fn attr_from_json_array(array: &Vec<sj::Value>) -> CpAttrs {
        let format = CstType::from_symbol(array[0].as_str().unwrap()).unwrap();
        let components = array[1]
            .as_array()
            .unwrap()
            .iter()
            .fold(vec![], |mut comps, v| {
                match v {
                    sj::Value::String(c) => comps.push(Component::Char(c.clone())),
                    sj::Value::Array(array) => {
                        comps.push(Component::Complex(Self::attr_from_json_array(array)))
                    }
                    _ => panic!("Unknow data: {}", v.to_string()),
                }
                comps
            });

        CpAttrs {
            tp: format,
            components,
        }
    }

    pub fn empty() -> Self {
        Self(Default::default())
    }

    pub fn from_json_array(obj: sj::Value) -> CstTable {
        let obj = obj.as_object().unwrap();
        let table = CstTable(HashMap::with_capacity(obj.len()));

        obj.into_iter().fold(table, |mut table, (chr, attr)| {
            if let Some(a) = table.insert(
                chr.clone(),
                Self::attr_from_json_array(attr.as_array().unwrap()),
            ) {
                eprintln!(
                    "Duplicate character `{}`:\n{}\n{:?}",
                    chr,
                    attr,
                    a.comps_name()
                );
            }
            table
        })
    }

    pub fn all_necessary_components(&self) -> HashSet<String> {
        fn find_until(
            comp: &Component,
            table: &HashMap<String, CpAttrs>,
            requis: &mut HashSet<String>,
        ) {
            match comp {
                Component::Char(str) => match table.get(str) {
                    Some(attrs) => {
                        if attrs.tp == CstType::Single {
                            requis.insert(str.clone());
                        } else {
                            attrs
                                .components
                                .iter()
                                .for_each(|comp| find_until(comp, table, requis));
                        }
                    }
                    None => {
                        requis.insert(str.clone());
                    }
                },
                Component::Complex(ref attrs) => attrs
                    .components
                    .iter()
                    .for_each(|comp| find_until(comp, table, requis)),
            }
        }

        self.iter()
            .fold(HashSet::new(), |mut requis, (chr, attrs)| {
                if attrs.tp == CstType::Single {
                    requis.insert(chr.to_string());
                } else {
                    attrs
                        .components
                        .iter()
                        .for_each(|comp| find_until(comp, self, &mut requis));
                }

                requis
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completeness() {
        let table = CstTable::default();
        let requests = table.all_necessary_components();

        let mut misses = std::collections::HashSet::new();

        requests.into_iter().for_each(|name| {
            let mut chars = name.chars();
            let chr = chars.next().unwrap();
            if chars.next().is_none() && !table.contains_key(&name) {
                misses.insert(chr);
            }
        });

        assert_eq!(misses, std::collections::HashSet::new());
    }

    #[test]
    fn test_tartget_chars() {
        let mut table = CstTable::empty();
        table.insert(
            String::from("艹"),
            CpAttrs {
                tp: CstType::Single,
                components: vec![],
            },
        );
        table.insert(
            String::from("艹字头"),
            CpAttrs {
                tp: CstType::Single,
                components: vec![],
            },
        );

        assert_eq!(table.len(), 2);
        assert_eq!(table.target_chars(), vec!['艹']);
    }
}
