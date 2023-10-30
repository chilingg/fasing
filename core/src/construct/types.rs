use crate::axis::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Type {
    Single,
    Scale(Axis),
    Surround(DataHV<Place>),
}

impl Type {
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
                h: Place::Mind,
                v: Place::Start,
            }) => '⿵',
            Self::Surround(DataHV {
                h: Place::Mind,
                v: Place::End,
            }) => '⿶',
            Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Mind,
            }) => '⿷',
            Self::Surround(DataHV {
                h: Place::Mind,
                v: Place::Mind,
            }) => '⿴',
            _ => panic!("Unkonw construct type: {:?}", self),
        }
    }

    pub fn from_symbol(symbol: &str) -> Self {
        match symbol {
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
            "⿵" => Self::Surround(DataHV {
                h: Place::Mind,
                v: Place::Start,
            }),
            "⿶" => Self::Surround(DataHV {
                h: Place::Mind,
                v: Place::End,
            }),
            "⿷" => Self::Surround(DataHV {
                h: Place::Start,
                v: Place::Mind,
            }),
            "⿴" => Self::Surround(DataHV {
                h: Place::Mind,
                v: Place::Mind,
            }),
            _ => Self::Single,
        }
    }
}
