use crate::axis::DataHV;
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
    type Data = BTreeMap<[Option<bool>; 4], DataHV<Vec<usize>>>;

    fn attr_name() -> &'static str {
        "in_place_allocs"
    }
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
