use crate::axis::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum CstType {
    Single,
    Scale(Axis),
    Surround(DataHV<Place>),
}

impl CstType {
    pub fn symbol(&self) -> char {
        match self {
            Self::Single => '□',
            Self::Scale(Axis::Horizontal) => '⿰',
            Self::Scale(Axis::Vertical) => '⿱',
            Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Start,
            }) => '⿸',
            Self::Surround(DataHV {
                h: Place::End,
                v: Place::Start,
            }) => '⿹',
            Self::Surround(DataHV {
                h: Place::Start,
                v: Place::End,
            }) => '⿺',
            Self::Surround(DataHV {
                h: Place::End,
                v: Place::End,
            }) => '⿽',
            Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::Start,
            }) => '⿵',
            Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::End,
            }) => '⿶',
            Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Middle,
            }) => '⿷',
            Self::Surround(DataHV {
                h: Place::End,
                v: Place::Middle,
            }) => '⿼',
            Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::Middle,
            }) => '⿴',
            // _ => panic!("Unkonw construct type: {:?}", self),
        }
    }

    pub fn from_symbol(symbol: &str) -> Option<Self> {
        let tp = match symbol {
            "" | "□" => Self::Single,
            "⿰" | "⿲" => Self::Scale(Axis::Horizontal),
            "⿱" | "⿳" => Self::Scale(Axis::Vertical),
            "⿸" => Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Start,
            }),
            "⿹" => Self::Surround(DataHV {
                h: Place::End,
                v: Place::Start,
            }),
            "⿺" => Self::Surround(DataHV {
                h: Place::Start,
                v: Place::End,
            }),
            "⿽" => Self::Surround(DataHV {
                h: Place::End,
                v: Place::End,
            }),
            "⿵" => Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::Start,
            }),
            "⿶" => Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::End,
            }),
            "⿷" => Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Middle,
            }),
            "⿼" => Self::Surround(DataHV {
                h: Place::End,
                v: Place::Middle,
            }),
            "⿴" => Self::Surround(DataHV {
                h: Place::Middle,
                v: Place::Middle,
            }),
            _ => return None,
        };
        Some(tp)
    }
}
