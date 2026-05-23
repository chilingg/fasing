use serde::{Deserialize, Serialize};
use serde_json as sj;

use std::collections::BTreeMap;

use crate::{
    base::*,
    combination::view::{EdgeShape, ShapeState, ShapeTrend},
};

#[derive(Clone, Default)]
pub struct EdgeCheck<T> {
    pub conditions: BTreeMap<String, sj::Value>,
    pub setup: [Option<T>; 2],

    #[allow(dead_code)]
    note: String,

    data: BTreeMap<String, sj::Value>,
}

pub enum CheckError {
    ValueError { key: String, value: String },
    UnknowKey(String),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValueError { key, value } => {
                write!(f, "Error condition `{key}`: {value}")
            }
            Self::UnknowKey(key) => write!(f, "Conditions for Edge check `{key}`: Unknow key!"),
        }
    }
}

impl<T> EdgeCheck<T> {
    pub fn get_value<Target>(&self, key: &str) -> Option<Result<Target, serde_json::Error>>
    where
        Target: serde::de::DeserializeOwned,
    {
        self.data.get(key).map(|val| sj::from_value(val.clone()))
    }

    pub fn is_match<F1, F2>(
        &self,
        axis: Axis,
        i: usize,
        length: usize,
        mut get_edge: F1,
        mut supplement: F2,
    ) -> Result<bool, CheckError>
    where
        F1: FnMut(usize, Axis, Side) -> EdgeShape,
        F2: FnMut(&str, &sj::Value) -> Result<bool, CheckError>,
    {
        for (k, v) in self.conditions.iter() {
            let k = k.as_str();
            let ok = match k {
                "axis" => match sj::from_value::<Axis>(v.clone()) {
                    Ok(t_axis) => axis == t_axis,
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "section" => match sj::from_value::<Section>(v.clone()) {
                    Ok(section) => {
                        if i == 0 {
                            section == Section::Start
                        } else if i + 1 == length {
                            section == Section::End
                        } else {
                            section == Section::Middle
                        }
                    }
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "edge1" | "edge2" | "edge1_cross" | "edge2_cross" => {
                    match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => {
                            let (axis, side) = match k {
                                "edge1" => (axis, Side::Front),
                                "edge2" => (axis, Side::Back),
                                "edge1_cross" => (axis.inverse(), Side::Front),
                                "edge2_cross" => (axis.inverse(), Side::Back),
                                _ => unreachable!(),
                            };
                            rule.is_match(&get_edge(i, axis, side))
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
                "front_edge" | "front_edge_f" | "front_edge_b" => {
                    match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => {
                            let (axis, side) = match k {
                                "front_edge" => (axis.inverse(), Side::Back),
                                "front_edge_f" => (axis, Side::Front),
                                "front_edge_b" => (axis, Side::Back),
                                _ => unreachable!(),
                            };
                            i != 0 && rule.is_match(&get_edge(i - 1, axis, side))
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
                "back_edge" | "back_edge_f" | "back_edge_b" => {
                    match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => {
                            let (axis, side) = match k {
                                "back_edge" => (axis.inverse(), Side::Front),
                                "back_edge_f" => (axis, Side::Front),
                                "back_edge_b" => (axis, Side::Back),
                                _ => unreachable!(),
                            };
                            i + 1 != length && rule.is_match(&get_edge(i + 1, axis, side))
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
                _ => supplement(k, v)?,
            };
            if ok == false {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

impl<'de, T> Deserialize<'de> for EdgeCheck<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: BTreeMap<String, sj::Value> = Deserialize::deserialize(deserializer)?;
        let note = data
            .get("note")
            .map(|val| val.to_string())
            .unwrap_or_default();

        let key = "conditions";
        let conditions = data
            .get(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|val| sj::from_value(val.clone()))
            .map_err(|e| serde::de::Error::custom(e))?;

        let key = "setup";
        let setup = data
            .get(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|val| Deserialize::deserialize(val.clone()))
            .map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self {
            conditions,
            setup,
            note,
            data,
        })
    }
}

#[derive(Clone)]
pub struct EdgeMatch {
    blank: [Option<Vec<ShapeTrend>>; 2],
    middle: Option<Vec<ShapeState>>,
    not: bool,
}

impl EdgeMatch {
    pub fn is_match(&self, shape: &EdgeShape) -> bool {
        for i in 0..2 {
            if let Some(list) = self.blank[i].as_ref() {
                if !list.iter().any(|trend| match trend {
                    ShapeTrend::Square if shape.blank[i] == ShapeTrend::SquareLarg => true,
                    ShapeTrend::Triangle if shape.blank[i] == ShapeTrend::TriangleLarg => true,
                    _ => *trend == shape.blank[i],
                }) {
                    return self.not;
                }
            }
        }
        let r = self
            .middle
            .as_ref()
            .map(|m| m.iter().find(|s| **s == shape.middle).is_some())
            .unwrap_or(true);
        return r != self.not;
    }
}

impl Serialize for EdgeMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        fn to_blank_str(d: &Option<Vec<ShapeTrend>>) -> String {
            match d {
                None => "*".to_string(),
                Some(list) => {
                    let mut str = String::new();
                    for (i, s) in list.iter().enumerate() {
                        let c = match s {
                            ShapeTrend::None => "x",
                            ShapeTrend::Square => "o",
                            ShapeTrend::SquareLarg => "2o",
                            ShapeTrend::Triangle => "/",
                            ShapeTrend::TriangleLarg => "2/",
                        };
                        str.push_str(c);
                        if i + 1 != list.len() {
                            str.push(' ');
                        }
                    }
                    str
                }
            }
        }

        fn to_middle_str(m: &Option<Vec<ShapeState>>) -> String {
            match m {
                Some(list) => {
                    let mut str = String::with_capacity(list.len() * 2 - 1);
                    for (i, s) in list.iter().enumerate() {
                        let c = match s {
                            ShapeState::Empty => 'x',
                            ShapeState::Acute => '>',
                            ShapeState::Sparse => '=',
                            ShapeState::Breach => ';',
                            ShapeState::Dense => ']',
                        };
                        str.push(c);
                        if i + 1 != list.len() {
                            str.push(' ');
                        }
                    }
                    str
                }
                None => "*".to_string(),
            }
        }

        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&to_blank_str(&self.blank[0]))?;
        seq.serialize_element(&to_middle_str(&self.middle))?;
        seq.serialize_element(&to_blank_str(&self.blank[1]))?;
        if self.not {
            seq.serialize_element("!")?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for EdgeMatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let from_blank_str = |s: &str| match s {
            "*" => Ok(None),
            str => {
                let mut list = vec![];
                for c in str.split(' ') {
                    let shape = match c {
                        "x" => ShapeTrend::None,
                        "o" => ShapeTrend::Square,
                        "2o" => ShapeTrend::SquareLarg,
                        "/" => ShapeTrend::Triangle,
                        "2/" => ShapeTrend::TriangleLarg,
                        c => {
                            return Err(serde::de::Error::custom(format!(
                                "unknown Shape Trend `{c}`"
                            )));
                        }
                    };
                    list.push(shape);
                }
                Ok(Some(list))
            }
        };

        let from_middle_str = |s: &str| match s {
            "*" => Ok(None),
            str => {
                let mut list = vec![];
                for c in str.split(' ') {
                    let shape = match c {
                        "x" => ShapeState::Empty,
                        ">" => ShapeState::Acute,
                        "=" => ShapeState::Sparse,
                        ";" => ShapeState::Breach,
                        "]" => ShapeState::Dense,
                        c => {
                            return Err(serde::de::Error::custom(format!(
                                "unknown Shape State `{c}`"
                            )));
                        }
                    };
                    list.push(shape);
                }
                Ok(Some(list))
            }
        };

        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::Array(array) => {
                if array.len() != 3 && array.len() != 4 {
                    Err(serde::de::Error::custom(format!(
                        "Standard edge element is not {}",
                        array.len()
                    )))
                } else if !array.iter().all(|ele| ele.is_string()) {
                    Err(serde::de::Error::custom(format!(
                        "Failed convert to IntervalRule in {:?}",
                        array
                    )))
                } else {
                    let list: Vec<_> = array.iter().map(|ele| ele.as_str().unwrap()).collect();
                    let blank = [from_blank_str(list[0])?, from_blank_str(list[2])?];
                    let middle = from_middle_str(list[1])?;
                    let not = list
                        .get(3)
                        .filter(|str| **str == "!")
                        .map(|_| true)
                        .unwrap_or(false);

                    Ok(Self { blank, middle, not })
                }
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to IntervalRule in {}",
                val
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_edge_match() {
        let mut instances = vec![];
        for f in [ShapeTrend::Square, ShapeTrend::None, ShapeTrend::Triangle] {
            for b in [ShapeTrend::Square, ShapeTrend::None, ShapeTrend::Triangle] {
                for m in [
                    ShapeState::Empty,
                    ShapeState::Acute,
                    ShapeState::Sparse,
                    ShapeState::Dense,
                ] {
                    instances.push(EdgeShape {
                        blank: [f, b],
                        middle: m,
                    });
                }
            }
        }

        let js = json!(["o", "]", "x"]);
        let rule: EdgeMatch = serde_json::from_value(js).unwrap();
        assert_eq!(
            rule.blank,
            [Some(vec![ShapeTrend::Square]), Some(vec![ShapeTrend::None])]
        );
        assert_eq!(rule.middle, Some(vec![ShapeState::Dense]));
        instances.iter().enumerate().for_each(|(i, shape)| {
            let b = match i {
                7 => true,
                _ => false,
            };
            assert_eq!(rule.is_match(shape), b, "{i}");
        });

        let js = json!(["x", "= ]", "*"]);
        let rule: EdgeMatch = serde_json::from_value(js).unwrap();
        assert_eq!(rule.blank, [Some(vec![ShapeTrend::None]), None]);
        assert_eq!(
            rule.middle,
            Some(vec![ShapeState::Sparse, ShapeState::Dense])
        );
        instances.iter().enumerate().for_each(|(i, shape)| {
            let b = match i {
                14 | 15 | 18 | 19 | 22 | 23 => true,
                _ => false,
            };
            assert_eq!(rule.is_match(shape), b, "{i}");
        });

        let js = json!(["/", "*", "*"]);
        let rule: EdgeMatch = serde_json::from_value(js).unwrap();
        assert_eq!(rule.blank, [Some(vec![ShapeTrend::Triangle]), None]);
        assert_eq!(rule.middle, None);
        instances.iter().enumerate().for_each(|(i, shape)| {
            let b = i >= 24;
            assert_eq!(rule.is_match(shape), b, "{i}");
        });
    }
}
