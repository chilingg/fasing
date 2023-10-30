use super::types::Type;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
extern crate serde_json as sj;

use std::collections::{HashMap, HashSet};

#[derive(Clone, Serialize, Deserialize)]
pub struct Attrs {
    pub tp: Type,
    pub components: Vec<Component>,
}

impl Attrs {
    pub fn single() -> &'static Self {
        static SINGLE_ATTRS: OnceCell<Attrs> = OnceCell::new();
        SINGLE_ATTRS.get_or_init(|| Attrs {
            tp: Type::Single,
            components: vec![],
        })
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

#[derive(Clone, Serialize, Deserialize)]
pub enum Component {
    Char(String),
    Complex(Attrs),
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Table {
    pub data: HashMap<String, Attrs>,
}

impl Table {
    fn attr_from_json_array(array: &Vec<sj::Value>) -> Attrs {
        let format = Type::from_symbol(array[0].as_str().unwrap());
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

        Attrs {
            tp: format,
            components,
        }
    }

    pub fn empty() -> Self {
        Self {
            data: Default::default(),
        }
    }

    pub fn from_json_array(obj: sj::Value) -> Table {
        let obj = obj.as_object().unwrap();
        let table = Table {
            data: HashMap::with_capacity(obj.len()),
        };

        obj.into_iter().fold(table, |mut table, (chr, attr)| {
            if let Some(a) = table.data.insert(
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
            table: &HashMap<String, Attrs>,
            requis: &mut HashSet<String>,
        ) {
            match comp {
                Component::Char(str) => match table.get(str) {
                    Some(attrs) => {
                        if attrs.tp == Type::Single {
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

        self.data
            .iter()
            .fold(HashSet::new(), |mut requis, (chr, attrs)| {
                if attrs.tp == Type::Single {
                    requis.insert(chr.to_string());
                } else {
                    attrs
                        .components
                        .iter()
                        .for_each(|comp| find_until(comp, &self.data, &mut requis));
                }

                requis
            })
    }
}

impl Default for Table {
    fn default() -> Self {
        const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));
        Self::from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completeness() {
        let table = Table::default();
        let requests = table.all_necessary_components();

        let mut misses = std::collections::HashSet::new();

        requests.into_iter().for_each(|name| {
            let mut chars = name.chars();
            let chr = chars.next().unwrap();
            if chars.next().is_none() && !table.data.contains_key(&name) {
                misses.insert(chr);
            }
        });

        assert_eq!(misses, std::collections::HashSet::new());
    }
}
