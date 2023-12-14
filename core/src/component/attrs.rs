use crate::axis::*;
extern crate serde_json as sj;

use std::collections::BTreeMap;

pub trait CompAttr {
    type Data: Sized;

    fn attr_name() -> &'static str;

    fn parse_str(attr: &str) -> Option<Self::Data>
    where
        Self::Data: serde::de::DeserializeOwned,
    {
        match sj::from_str::<Self::Data>(attr) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Error parsing attributes `{}`: \n{}", Self::attr_name(), e);
                None
            }
        }
    }

    fn attr_str(data: &Self::Data) -> Option<String>
    where
        Self::Data: serde::Serialize,
    {
        match sj::to_string::<Self::Data>(data) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("Error attributes `{}`: \n{}", Self::attr_name(), e);
                None
            }
        }
    }
}

pub struct Allocs();
impl CompAttr for Allocs {
    type Data = DataHV<Vec<usize>>;

    fn attr_name() -> &'static str {
        "allocs"
    }
}

pub struct InPlaceAllocs();

impl CompAttr for InPlaceAllocs {
    type Data = Vec<(String, DataHV<Vec<usize>>)>;

    fn attr_name() -> &'static str {
        "in_place"
    }
}

pub fn xplace_match<T: Clone>(data: &Vec<(String, T)>, in_place: &DataHV<[bool; 2]>) -> Option<T> {
    fn is_match(attr: &str, exist: bool) -> bool {
        match attr {
            "x" => !exist,
            "o" => exist,
            "*" => true,
            _ => false,
        }
    }

    data.iter().find_map(|(attrs, alloc)| {
        attrs.split(';').find_map(|attr| {
            let place_attr: Vec<&str> = attr.split(' ').collect();
            let ok = match place_attr.len() {
                1 => in_place
                    .hv_iter()
                    .flatten()
                    .all(|e| is_match(place_attr[0], *e)),
                2 => in_place
                    .hv_iter()
                    .zip(place_attr.iter())
                    .all(|(place, attr)| place.iter().all(|e| is_match(attr, *e))),
                4 => in_place
                    .hv_iter()
                    .flatten()
                    .zip(place_attr.iter())
                    .all(|(e, attr)| is_match(attr, *e)),
                _ => false,
            };

            match ok {
                true => Some(alloc.clone()),
                false => None,
            }
        })
    })
}

pub fn place_matchs<T: Clone>(data: &Vec<(String, T)>, in_place: &DataHV<[bool; 2]>) -> Vec<T> {
    fn is_match(attr: &str, exist: bool) -> bool {
        match attr {
            "x" => !exist,
            "o" => exist,
            "*" => true,
            _ => false,
        }
    }

    data.iter()
        .filter_map(|(attrs, alloc)| {
            attrs.split(';').find_map(|attr| {
                let place_attr: Vec<&str> = attr.split(' ').collect();
                let ok = match place_attr.len() {
                    1 => in_place
                        .hv_iter()
                        .flatten()
                        .all(|e| is_match(place_attr[0], *e)),
                    2 => in_place
                        .hv_iter()
                        .zip(place_attr.iter())
                        .all(|(place, attr)| place.iter().all(|e| is_match(attr, *e))),
                    4 => in_place
                        .hv_iter()
                        .flatten()
                        .zip(place_attr.iter())
                        .all(|(e, attr)| is_match(attr, *e)),
                    _ => false,
                };

                match ok {
                    true => Some(alloc.clone()),
                    false => None,
                }
            })
        })
        .collect()
}

pub struct ReduceAlloc();
impl CompAttr for ReduceAlloc {
    type Data = DataHV<Vec<Vec<usize>>>;

    fn attr_name() -> &'static str {
        "reduce_alloc"
    }
}

pub struct CharBox();
impl CompAttr for CharBox {
    type Data = [f32; 4];

    fn attr_name() -> &'static str {
        "char_box"
    }

    fn parse_str(attr: &str) -> Option<Self::Data> {
        match attr {
            "left" => Some([0.0, 0.0, 0.5, 1.0]),
            "right" => Some([0.5, 0.0, 1.0, 1.0]),
            "top" => Some([0.0, 0.0, 1.0, 0.5]),
            "bottom" => Some([0.0, 0.5, 1.0, 1.0]),
            _ => {
                let mut iter = attr.split(' ').map(|num| num.parse::<f32>());
                let mut data = [0.0; 4];
                for i in 0..4 {
                    match iter.next() {
                        Some(Ok(n)) => data[i] = n,
                        _ => return None,
                    }
                }
                Some(data)
            }
        }
    }
}
