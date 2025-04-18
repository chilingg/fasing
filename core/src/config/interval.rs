use crate::{axis::*, component::view::StandardEdge};

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone)]
pub struct IntervalRule {
    dots: [Option<bool>; 5],
    faces: [(Ordering, f32); 4],
}

impl IntervalRule {
    pub fn match_edge(&self, edge: &StandardEdge) -> bool {
        for i in 0..5 {
            if let Some(b) = self.dots[i] {
                if b != edge.dots[i] {
                    return false;
                }
            }
        }
        for i in 0..4 {
            if edge.faces[i].partial_cmp(&self.faces[i].1).unwrap() != self.faces[i].0 {
                return false;
            }
        }

        true
    }
}

impl Serialize for IntervalRule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        fn to_dot_str(d: Option<bool>) -> &'static str {
            match d {
                None => "*",
                Some(true) => "|",
                Some(false) => "x",
            }
        }

        fn to_fase_str(ord: Ordering) -> &'static str {
            match ord {
                Ordering::Less => "<",
                Ordering::Greater => ">",
                Ordering::Equal => "",
            }
        }

        let mut seq = serializer.serialize_seq(Some(9))?;
        for i in 0..4 {
            seq.serialize_element(to_dot_str(self.dots[i]))?;
            seq.serialize_element(&format!(
                "{}{:.3}",
                to_fase_str(self.faces[i].0),
                self.faces[i].1
            ))?;
        }
        seq.serialize_element(to_dot_str(self.dots[4]))?;
        seq.end()
    }
}

impl<'de> Deserialize<'de> for IntervalRule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        fn from_dot_str(s: &str) -> Option<bool> {
            match s {
                "|" => Some(true),
                "x" => Some(false),
                _ => None,
            }
        }

        fn from_fase_str(s: &str) -> Result<(Ordering, f32), std::num::ParseFloatError> {
            let r = match s.chars().next() {
                Some('<') => (Ordering::Less, s[1..].parse::<f32>()?),
                Some('>') => (Ordering::Greater, s[1..].parse::<f32>()?),
                _ => (Ordering::Equal, s.parse::<f32>()?),
            };
            Ok(r)
        }

        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::Array(array) => {
                if array.len() != 9 {
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

                    let dots = [
                        from_dot_str(list[0]),
                        from_dot_str(list[2]),
                        from_dot_str(list[4]),
                        from_dot_str(list[6]),
                        from_dot_str(list[8]),
                    ];
                    let faces = [
                        from_fase_str(list[1])
                            .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                        from_fase_str(list[3])
                            .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                        from_fase_str(list[5])
                            .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                        from_fase_str(list[7])
                            .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                    ];
                    Ok(Self { dots, faces })
                }
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to IntervalRule in {}",
                val
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IntervalMatch {
    pub axis: Option<Axis>,
    pub val: usize,
    pub note: String,
    pub rule1: IntervalRule,
    pub rule2: IntervalRule,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Interval {
    pub rules: Vec<IntervalMatch>,
    pub limit: f32,
}

impl Default for Interval {
    fn default() -> Self {
        Self {
            rules: Default::default(),
            limit: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_interval_rule() {
        let js = json!(["x", "1.000", "|", ">2.000", "*", "<3.000", "*", "4.000", "x"]);
        let str = serde_json::to_string(&js).unwrap();
        let rule: IntervalRule = serde_json::from_value(js).unwrap();
        assert_eq!(
            rule.dots,
            [Some(false), Some(true), None, None, Some(false)]
        );
        assert_eq!(
            rule.faces,
            [
                (Ordering::Equal, 1.0),
                (Ordering::Greater, 2.0),
                (Ordering::Less, 3.0),
                (Ordering::Equal, 4.0),
            ]
        );
        assert_eq!(str, serde_json::to_string(&rule).unwrap());
    }

    #[test]
    fn test_interval_match() {
        let edge = StandardEdge {
            dots: [true, false, false, false, true],
            faces: [0., 0.5, 1., 0.],
        };
        let val = json!(["|", "0", "x", "<0.501", "x", "1", "x", "0", "|"]);
        let rule: IntervalRule = serde_json::from_value(val).unwrap();
        assert!(rule.match_edge(&edge));

        let val = json!(["|", "0", "x", "0.501", "x", "1", "x", "0", "|"]);
        let rule: IntervalRule = serde_json::from_value(val).unwrap();
        assert!(!rule.match_edge(&edge));

        let val = json!(["|", "0", "x", "<0.501", "x", "1", "x", "0", "*"]);
        let rule: IntervalRule = serde_json::from_value(val).unwrap();
        assert!(rule.match_edge(&edge));

        let val = json!(["|", "0", "x", "<0.501", "|", "1", "x", "0", "|"]);
        let rule: IntervalRule = serde_json::from_value(val).unwrap();
        assert!(!rule.match_edge(&edge));
    }
}
