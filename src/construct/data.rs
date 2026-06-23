use super::types::CstType;

use serde::{Deserialize, Serialize, ser::SerializeStruct};
use serde_json as sj;
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub struct CpAttrs {
    pub tp: CstType,
    pub components: Vec<Component>,
}

impl CpAttrs {
    pub fn new(tp: CstType, components: Vec<Component>) -> Self {
        CpAttrs { tp, components }
    }

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
                .join(", ")
        )
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseAttrError {
    MissingType,
    MissingStartParen,
    MissingEndParen,
    ErrorCompNumber,
    EmptyCompName,
}

impl std::fmt::Display for ParseAttrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingType => write!(f, "Missing type!"),
            Self::MissingStartParen => write!(f, "Missing start parenthesis operator \"(\"!"),
            Self::MissingEndParen => write!(f, "Missing end parenthesis operator \"(\"!"),
            Self::ErrorCompNumber => write!(f, "The number of components is error!"),
            Self::EmptyCompName => write!(f, "The component name is empty!"),
        }
    }
}

impl std::str::FromStr for CpAttrs {
    type Err = ParseAttrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn skip_white<T: std::iter::Iterator<Item = char>>(iter: &mut T) -> Option<char> {
            while let Some(chr) = iter.next() {
                if chr.is_whitespace() {
                    continue;
                } else {
                    return Some(chr);
                }
            }
            None
        }

        fn get_comp_parameter<T: std::iter::Iterator<Item = char>>(
            iter: &mut T,
            tp: CstType,
        ) -> Result<Vec<Component>, ParseAttrError> {
            let mut components = vec![];
            let mut name = String::new();
            while let Some(chr) = iter.next() {
                if name.is_empty() {
                    if chr.is_whitespace() {
                        continue;
                    } else if let Some(tp) = CstType::from_symbol_char(chr) {
                        let c_attrs = get_cpattrs(iter, tp)?;
                        components.push(Component::Complex(c_attrs));
                    } else if chr == ')' {
                        break;
                    } else if chr == ',' {
                        return Err(ParseAttrError::EmptyCompName);
                    } else {
                        name.push(chr);
                    }
                } else {
                    if chr == ',' || chr == ')' {
                        let c_name = name.trim().to_string();
                        if c_name.is_empty() {
                            return Err(ParseAttrError::EmptyCompName);
                        }
                        components.push(Component::Char(c_name));

                        name = String::new();
                        if chr == ')' {
                            break;
                        }
                    } else {
                        name.push(chr);
                    }
                }
            }
            if !name.is_empty() {
                return Err(ParseAttrError::MissingEndParen);
            }

            match components.len() {
                2 => Ok(components),
                0 | 1 => Err(ParseAttrError::ErrorCompNumber),
                _ if matches!(tp, CstType::Scale(_)) => Ok(components),
                _ => Err(ParseAttrError::ErrorCompNumber),
            }
        }

        fn get_cpattrs<T: std::iter::Iterator<Item = char>>(
            iter: &mut T,
            tp: CstType,
        ) -> Result<CpAttrs, ParseAttrError> {
            match skip_white(iter) {
                Some(chr) if chr == '(' => Ok(CpAttrs::new(tp, get_comp_parameter(iter, tp)?)),
                _ => Err(ParseAttrError::MissingStartParen),
            }
        }

        let mut iter = s.chars();
        match skip_white(&mut iter).and_then(|chr| CstType::from_symbol_char(chr)) {
            Some(tp) if tp != CstType::Single => get_cpattrs(&mut iter, tp),
            _ => Err(ParseAttrError::MissingType),
        }
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Serialize, Deserialize)]
pub struct CstTable(BTreeMap<String, CpAttrs>);

impl Deref for CstTable {
    type Target = BTreeMap<String, CpAttrs>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CstTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CstTable {
    pub fn standard() -> Self {
        const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));
        Self::from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
    }

    pub fn empty() -> Self {
        Self(Default::default())
    }

    fn from_json_array(obj: sj::Value) -> Self {
        let obj = match obj {
            sj::Value::Object(obj) => obj,
            _ => panic!(),
        };
        let table = Self(BTreeMap::new());

        obj.into_iter().fold(table, |mut table, (chr, attr)| {
            table.insert(chr, Self::attr_from_json_array(attr.as_array().unwrap()));
            table
        })
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str() {
        let attrs = "".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::MissingType)));

        let attrs = "x".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::MissingType)));

        let attrs = "⿱".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::MissingStartParen)));

        let attrs = "⿱ (".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::ErrorCompNumber)));

        let attrs = "⿱ (x".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::MissingEndParen)));

        let attrs = "⿱ (x)".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::ErrorCompNumber)));

        let attrs = "⿱ (x,,)".parse::<CpAttrs>();
        assert_eq!(attrs.unwrap_err(), ParseAttrError::EmptyCompName);

        let attrs = "⿴ (x, y, z)".parse::<CpAttrs>();
        assert!(matches!(attrs, Err(ParseAttrError::ErrorCompNumber)));

        let attrs = "⿱ ( x , y )".parse::<CpAttrs>().unwrap();
        assert_eq!(attrs.comps_name().as_str(), "⿱(x, y)");
        let attrs = "⿱ ( x , y, )".parse::<CpAttrs>().unwrap();
        assert_eq!(attrs.comps_name().as_str(), "⿱(x, y)");

        let attrs = "⿱ ( x , y,  ⿴(  a, b))".parse::<CpAttrs>().unwrap();
        assert_eq!(attrs.tp, CstType::Scale(crate::base::Axis::Vertical));
        assert_eq!(attrs.components.len(), 3);
        assert!(matches!(attrs.components[2], Component::Complex(_)));
        assert_eq!(attrs.components[2].name().as_str(), "⿴(a, b)");
    }

    #[test]
    fn test_completeness() {
        use std::collections::HashSet;

        fn find_until(
            comp: &Component,
            table: &BTreeMap<String, CpAttrs>,
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
                Component::Complex(attrs) => attrs
                    .components
                    .iter()
                    .for_each(|comp| find_until(comp, table, requis)),
            }
        }

        let table = CstTable::standard();
        let requests = table
            .iter()
            .fold(HashSet::new(), |mut requis, (chr, attrs)| {
                if attrs.tp == CstType::Single {
                    requis.insert(chr.to_string());
                } else {
                    attrs
                        .components
                        .iter()
                        .for_each(|comp| find_until(comp, &table, &mut requis));
                }

                requis
            });

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
}
