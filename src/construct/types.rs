use crate::base::*;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CstType {
    Single,
    Scale(Axis),
    Surround(DataHV<Section>),
}

impl CstType {
    pub fn symbol(&self) -> char {
        match self {
            Self::Single => '□',
            Self::Scale(Axis::Horizontal) => '⿰',
            Self::Scale(Axis::Vertical) => '⿱',
            Self::Surround(DataHV {
                h: Section::Start,
                v: Section::Start,
            }) => '⿸',
            Self::Surround(DataHV {
                h: Section::End,
                v: Section::Start,
            }) => '⿹',
            Self::Surround(DataHV {
                h: Section::Start,
                v: Section::End,
            }) => '⿺',
            Self::Surround(DataHV {
                h: Section::End,
                v: Section::End,
            }) => '⿽',
            Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::Start,
            }) => '⿵',
            Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::End,
            }) => '⿶',
            Self::Surround(DataHV {
                h: Section::Start,
                v: Section::Middle,
            }) => '⿷',
            Self::Surround(DataHV {
                h: Section::End,
                v: Section::Middle,
            }) => '⿼',
            Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::Middle,
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
                h: Section::Start,
                v: Section::Start,
            }),
            "⿹" => Self::Surround(DataHV {
                h: Section::End,
                v: Section::Start,
            }),
            "⿺" => Self::Surround(DataHV {
                h: Section::Start,
                v: Section::End,
            }),
            "⿽" => Self::Surround(DataHV {
                h: Section::End,
                v: Section::End,
            }),
            "⿵" => Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::Start,
            }),
            "⿶" => Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::End,
            }),
            "⿷" => Self::Surround(DataHV {
                h: Section::Start,
                v: Section::Middle,
            }),
            "⿼" => Self::Surround(DataHV {
                h: Section::End,
                v: Section::Middle,
            }),
            "⿴" => Self::Surround(DataHV {
                h: Section::Middle,
                v: Section::Middle,
            }),
            _ => return None,
        };
        Some(tp)
    }
}

impl Serialize for CstType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.symbol().to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for CstType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let symbol = String::deserialize(deserializer)?;
        Self::from_symbol(&symbol).ok_or(serde::de::Error::custom(format!(
            "Unkonw construct type `{symbol}`!"
        )))
    }
}
