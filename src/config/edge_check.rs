use serde::{Deserialize, Serialize};
use serde_json as sj;

use std::collections::BTreeMap;

use crate::{
    base::*,
    combination::{
        StrucProto,
        view::{EdgeShape, ShapeState, ShapeTrend},
    },
};

#[derive(Clone, Default)]
pub struct EdgeCheck<T> {
    pub conditions: Vec<(String, sj::Value)>,
    pub setup: T,

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

    pub fn is_match<F>(
        &self,
        blanks: &DataHV<[AssignVal; 2]>,
        proto: &StrucProto,
        axis: Axis,
        edges: &DataHV<[EdgeShape; 2]>,
        ad_edges: &DataHV<[Option<EdgeShape>; 2]>,
        ad_interval: &DataHV<[Option<usize>; 2]>,
        adjacencies: &DataHV<[bool; 2]>,
        mut supplement: F,
    ) -> Result<bool, CheckError>
    where
        F: FnMut(&str, &sj::Value) -> Option<Result<bool, CheckError>>,
    {
        for (k, v) in self.conditions.iter() {
            let k = k.as_str();
            let ok = match supplement(k, v) {
                Some(Ok(b)) => b,
                Some(Err(e)) => return Err(e),
                None => match k {
                    "axis" => match sj::from_value::<Axis>(v.clone()) {
                        Ok(t_axis) => axis == t_axis,
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
                                rule.is_match(&edges.hv_get(axis)[side.n()])
                            }
                            Err(e) => {
                                return Err(CheckError::ValueError {
                                    key: k.to_string(),
                                    value: e.to_string(),
                                });
                            }
                        }
                    }
                    "front_edge" | "back_edge" | "front_edge_cross" | "back_edge_cross" => {
                        match sj::from_value::<EdgeMatch>(v.clone()) {
                            Ok(rule) => {
                                let (axis, side) = match k {
                                    "front_edge" => (axis, Side::Front),
                                    "back_edge" => (axis, Side::Back),
                                    "front_edge_cross" => (axis.inverse(), Side::Front),
                                    "back_edge_cross" => (axis.inverse(), Side::Back),
                                    _ => unreachable!(),
                                };
                                if let Some(edge) = &ad_edges.hv_get(axis)[side.n()] {
                                    rule.is_match(edge)
                                } else {
                                    false
                                }
                            }
                            Err(e) => {
                                return Err(CheckError::ValueError {
                                    key: k.to_string(),
                                    value: e.to_string(),
                                });
                            }
                        }
                    }
                    "adjacencies" => super::place_match(v.as_str().unwrap_or(""), *adjacencies),
                    "blanks" => match sj::from_value::<[Option<bool>; 2]>(v.clone()) {
                        Ok(check) => check
                            .iter()
                            .zip(blanks.hv_get(axis).iter())
                            .all(|(c, v)| c.is_none() || v.base == 0.0),
                        Err(e) => {
                            return Err(super::CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "allocs_val_f" | "allocs_val_b" => {
                        match sj::from_value::<Vec<usize>>(v.clone()) {
                            Ok(check) => {
                                let mut alloc = proto.allocation_values().hv_get_into(axis);
                                if check.len() <= alloc.len() {
                                    if k == "allocs_b" {
                                        alloc.reverse();
                                    }
                                    alloc
                                        .into_iter()
                                        .zip(check.into_iter())
                                        .all(|(a, c)| a == c)
                                } else {
                                    false
                                }
                            }
                            Err(e) => {
                                return Err(super::CheckError::ValueError {
                                    key: k.to_string(),
                                    value: e.to_string(),
                                });
                            }
                        }
                    }
                    "interval_f" | "interval_b" | "interval_f_cross" | "interval_b_cross" => {
                        let (axis, idx) = match k {
                            "interval_f" => (axis, 0),
                            "interval_b" => (axis, 1),
                            "interval_f_cross" => (axis.inverse(), 0),
                            "interval_b_cross" => (axis.inverse(), 1),
                            _ => unreachable!(),
                        };

                        if let Some(iv) = ad_interval.hv_get(axis)[idx] {
                            match v.as_str().map(|str| {
                                super::BoolExps::try_from(str)
                                    .and_then(|exps| exps.eval(&[("val", iv as isize)].into()))
                            }) {
                                Some(Ok(b)) => b,
                                Some(Err(e)) => {
                                    return Err(super::CheckError::ValueError {
                                        key: k.to_string(),
                                        value: e.to_string(),
                                    });
                                }
                                None => {
                                    return Err(super::CheckError::ValueError {
                                        key: k.to_string(),
                                        value: "Required 'interval_*' is a string".to_string(),
                                    });
                                }
                            }
                        } else {
                            false
                        }
                    }
                    _ => return Err(CheckError::UnknowKey(k.to_string())),
                },
            };
            if ok == false {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn is_match_scale<F1, F2>(
        &self,
        axis: Axis,
        i: usize,
        length: usize,
        base_list: &Vec<usize>,
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
                "section" => {
                    let list = if v.is_array() {
                        sj::from_value::<Vec<Section>>(v.clone())
                    } else {
                        sj::from_value::<Section>(v.clone()).map(|s| vec![s])
                    };
                    match list {
                        Ok(sections) => {
                            if i == 0 {
                                sections.contains(&Section::Start)
                            } else if i + 1 == length {
                                sections.contains(&Section::End)
                            } else {
                                sections.contains(&Section::Middle)
                            }
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
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
                "base_val" => match v.as_str() {
                    Some(str) => {
                        match super::BoolExps::try_from(str)
                            .and_then(|exps| exps.eval(&[("val", base_list[i] as isize)].into()))
                        {
                            Ok(b) => b,
                            Err(e) => {
                                return Err(super::CheckError::ValueError {
                                    key: k.to_string(),
                                    value: e.to_string(),
                                });
                            }
                        }
                    }
                    None => {
                        return Err(super::CheckError::ValueError {
                            key: k.to_string(),
                            value: "Required 'base_val' is a string".to_string(),
                        });
                    }
                },
                _ => supplement(k, v)?,
            };
            if ok == false {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn is_match_interval<F>(
        &self,
        axis: Axis,
        interval: usize,
        edge1: &EdgeShape,
        edge2: &EdgeShape,
        mut supplement: F,
    ) -> Result<bool, CheckError>
    where
        F: FnMut(&str, &sj::Value) -> Option<Result<bool, CheckError>>,
    {
        for (k, v) in self.conditions.iter() {
            let k = k.as_str();
            let ok = match supplement(k, v) {
                Some(Ok(b)) => b,
                Some(Err(e)) => return Err(e),
                None => match k {
                    "axis" => match sj::from_value::<Axis>(v.clone()) {
                        Ok(t_axis) => axis == t_axis,
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "interval" => match sj::from_value::<usize>(v.clone()) {
                        Ok(val) => interval == val,
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "edge1" => match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => rule.is_match(edge1),
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "edge2" => match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => rule.is_match(edge2),
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "edge_all" => match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => [edge1, edge2].iter().all(|e| rule.is_match(e)),
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    "edge_any" => match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => [edge1, edge2].iter().any(|e| rule.is_match(e)),
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    },
                    _ => return Err(CheckError::UnknowKey(k.to_string())),
                },
            };
            if ok == false {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub fn is_match_surround<F>(
        &self,
        axis: Axis,
        surr_section: &DataHV<Section>,
        edges: &DataHV<[EdgeShape; 2]>,
        axis_edges: &DataHV<[[Option<EdgeShape>; 2]; 2]>,
        adjacency_edges: &DataHV<[Option<EdgeShape>; 2]>,
        adjacencies: &DataHV<[bool; 2]>,
        mut supplement: F,
    ) -> Result<bool, CheckError>
    where
        F: FnMut(&str, &sj::Value) -> Result<bool, CheckError>,
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
                    Ok(section_check) => surr_section.hv_get_into(axis) == section_check,
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "section_cross" => match sj::from_value::<Section>(v.clone()) {
                    Ok(section_check) => surr_section.hv_get_into(axis.inverse()) == section_check,
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "edge_f" | "edge_b" | "edge_cross_f" | "edge_cross_b" => {
                    match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => {
                            let (axis, side) = match k {
                                "edge_f" => (axis, Side::Front),
                                "edge_b" => (axis, Side::Back),
                                "edge_cross_f" => (axis.inverse(), Side::Front),
                                "edge_cross_b" => (axis.inverse(), Side::Back),
                                _ => unreachable!(),
                            };
                            rule.is_match(&edges.hv_get(axis)[side.n()])
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
                "front_edge" | "front_edge_cross" | "back_edge" | "back_edge_cross" => {
                    match sj::from_value::<EdgeMatch>(v.clone()) {
                        Ok(rule) => {
                            let (axis, side) = match k {
                                "front_edge" => (axis, Side::Front),
                                "back_edge" => (axis, Side::Back),
                                "front_edge_cross" => (axis.inverse(), Side::Front),
                                "back_edge_cross" => (axis.inverse(), Side::Back),
                                _ => unreachable!(),
                            };
                            adjacency_edges.hv_get(axis)[side.n()]
                                .as_ref()
                                .map(|edge| rule.is_match(edge))
                                .unwrap_or(false)
                        }
                        Err(e) => {
                            return Err(CheckError::ValueError {
                                key: k.to_string(),
                                value: e.to_string(),
                            });
                        }
                    }
                }
                "axis_front_edge1"
                | "axis_front_edge2"
                | "axis_back_edge1"
                | "axis_back_edge2"
                | "cross_axis_front_edge1"
                | "cross_axis_front_edge2"
                | "cross_axis_back_edge1"
                | "cross_axis_back_edge2" => match sj::from_value::<EdgeMatch>(v.clone()) {
                    Ok(rule) => {
                        let (axis, i, j) = match k {
                            "axis_front_edge1" => (axis, 0, 0),
                            "axis_front_edge2" => (axis, 0, 1),
                            "axis_back_edge1" => (axis, 1, 0),
                            "axis_back_edge2" => (axis, 1, 1),
                            "cross_axis_front_edge1" => (axis.inverse(), 0, 0),
                            "cross_axis_front_edge2" => (axis.inverse(), 0, 1),
                            "cross_axis_back_edge1" => (axis.inverse(), 1, 0),
                            "cross_axis_back_edge2" => (axis.inverse(), 1, 1),
                            _ => unreachable!(),
                        };
                        axis_edges.hv_get(axis)[i][j]
                            .as_ref()
                            .map(|edge| rule.is_match(edge))
                            .unwrap_or(false)
                    }
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "axis_primary_edge_all" => match sj::from_value::<EdgeMatch>(v.clone()) {
                    Ok(rule) => {
                        let axis_section = *surr_section.hv_get(axis);
                        if axis_section != Section::Middle {
                            axis_edges.hv_get(axis)[axis_section.to_side().inverse().n()]
                                .iter()
                                .all(|edge| edge.as_ref().map_or(true, |edge| rule.is_match(edge)))
                        } else {
                            false
                        }
                    }
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "axis_primary_edge_any" => match sj::from_value::<EdgeMatch>(v.clone()) {
                    Ok(rule) => {
                        let axis_section = *surr_section.hv_get(axis);
                        if axis_section != Section::Middle {
                            axis_edges.hv_get(axis)[axis_section.to_side().inverse().n()]
                                .iter()
                                .any(|edge| edge.as_ref().map_or(false, |edge| rule.is_match(edge)))
                        } else {
                            false
                        }
                    }
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "axis_edge" => match sj::from_value::<EdgeMatch>(v.clone()) {
                    Ok(rule) => {
                        let axis_section = *surr_section.hv_get(axis);
                        if axis_section != Section::Middle {
                            rule.is_match(&edges.hv_get(axis)[axis_section.to_side().inverse().n()])
                        } else {
                            false
                        }
                    }
                    Err(e) => {
                        return Err(CheckError::ValueError {
                            key: k.to_string(),
                            value: e.to_string(),
                        });
                    }
                },
                "adjacencies" => super::place_match(v.as_str().unwrap_or(""), *adjacencies),
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
        let mut data: BTreeMap<String, sj::Value> = Deserialize::deserialize(deserializer)?;
        let note = data
            .get("note")
            .map(|val| val.to_string())
            .unwrap_or_default();

        let key = "conditions";
        let conditions = data
            .remove(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|val| match val {
                serde_json::Value::Object(map) => Ok(map.into_iter().collect()),
                _ => sj::from_value(val.clone()),
            })
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
    trend: [Vec<(Option<usize>, Option<ShapeTrend>)>; 2],
    middle: Vec<ShapeState>,
    not: bool,
}

impl EdgeMatch {
    pub fn is_match(&self, shape: &EdgeShape) -> bool {
        for i in 0..2 {
            if !self.trend[i].is_empty()
                && !self.trend[i].iter().any(|(b, t)| {
                    let b_ok = if let Some(b) = b {
                        *b == shape.blank[i]
                    } else {
                        shape.blank[i] != 0
                    };
                    b_ok && t.map_or(true, |t| t == shape.trend[i])
                })
            {
                return self.not;
            }
        }
        if !self.middle.is_empty() && !self.middle.contains(&shape.middle) {
            return self.not;
        }

        return !self.not;
    }
}

impl Serialize for EdgeMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        fn to_blank_str(list: &Vec<(Option<usize>, Option<ShapeTrend>)>) -> String {
            fn trend_to_str(trend: ShapeTrend) -> char {
                match trend {
                    ShapeTrend::Acute => '/',
                    ShapeTrend::Right => '-',
                    ShapeTrend::Obtuse => 'z',
                    ShapeTrend::None => 'x',
                }
            }

            match list.is_empty() {
                true => "*".to_string(),
                false => {
                    let mut str = String::new();
                    for (i, (b, t)) in list.iter().enumerate() {
                        if let Some(b) = b {
                            str.push_str(&b.to_string());
                        }
                        if let Some(t) = t {
                            str.push(trend_to_str(*t));
                        }
                        if i + 1 != list.len() {
                            str.push(' ');
                        }
                    }
                    str
                }
            }
        }

        fn to_middle_str(list: &Vec<ShapeState>) -> String {
            match list.is_empty() {
                false => {
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
                true => "*".to_string(),
            }
        }

        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&to_blank_str(&self.trend[0]))?;
        seq.serialize_element(&to_middle_str(&self.middle))?;
        seq.serialize_element(&to_blank_str(&self.trend[1]))?;
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
        fn str_to_trend(chr: char) -> Option<ShapeTrend> {
            match chr {
                '/' => Some(ShapeTrend::Acute),
                '-' => Some(ShapeTrend::Right),
                'z' => Some(ShapeTrend::Obtuse),
                'x' => Some(ShapeTrend::None),
                _ => None,
            }
        }

        let from_blank_str = |s: &str| match s {
            "*" | "" => Ok(vec![]),
            str => {
                let mut trend = vec![];
                for c in str.split(' ') {
                    let chars: Vec<char> = c.chars().collect();
                    if chars.len() == 1 {
                        if let Ok(n) = c.parse::<usize>() {
                            if n < 3 {
                                trend.push((Some(n), None));
                                continue;
                            }
                        } else if let Some(t) = str_to_trend(chars[0]) {
                            trend.push((None, Some(t)));
                            continue;
                        }
                    } else if chars.len() == 2 {
                        match c {
                            "!0" => {
                                trend.push((None, None));
                                continue;
                            }
                            _ => {}
                        }
                        match (chars[0].to_digit(10), str_to_trend(chars[1])) {
                            (Some(n), Some(t)) => {
                                trend.push((Some(n as usize), Some(t)));
                                continue;
                            }
                            _ => {}
                        }
                    }
                    return Err(serde::de::Error::custom(format!(
                        "unknown Shape Trend `{c}`"
                    )));
                }
                Ok(trend)
            }
        };

        let from_middle_str = |s: &str| match s {
            "*" => Ok(vec![]),
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
                Ok(list)
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
                    let trend1 = from_blank_str(&list[0])?;
                    let trend2 = from_blank_str(&list[2])?;
                    let middle = from_middle_str(list[1])?;
                    let not = list
                        .get(3)
                        .filter(|str| **str == "!")
                        .map(|_| true)
                        .unwrap_or(false);

                    Ok(Self {
                        trend: [trend1, trend2],
                        middle,
                        not,
                    })
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
        let t_list = [
            ShapeTrend::Acute,
            ShapeTrend::Right,
            ShapeTrend::Obtuse,
            ShapeTrend::None,
        ];
        let s_list = [
            ShapeState::Empty,
            ShapeState::Acute,
            ShapeState::Sparse,
            ShapeState::Breach,
            ShapeState::Dense,
        ];

        let rule1: EdgeMatch = serde_json::from_value(json!(["-", "]", "x"])).unwrap();
        let rule2: EdgeMatch = serde_json::from_value(json!(["x", "= ]", "*"])).unwrap();
        let rule3: EdgeMatch = serde_json::from_value(json!(["/", "*", "*"])).unwrap();
        let rule4: EdgeMatch = serde_json::from_value(json!(["0z", "*", "0z"])).unwrap();
        let rule5: EdgeMatch = serde_json::from_value(json!(["0", "*", "0"])).unwrap();
        let rule6: EdgeMatch = serde_json::from_value(json!(["2-", ">", "2-"])).unwrap();
        let rule7: EdgeMatch = serde_json::from_value(json!(["2", "*", "1"])).unwrap();
        let rule8: EdgeMatch = serde_json::from_value(json!(["!0", "*", "0"])).unwrap();

        let shape = EdgeShape {
            blank: [1, 0],
            trend: [t_list[1], t_list[3]],
            middle: s_list[4],
        };
        assert!(rule8.is_match(&shape));

        for f_b_i in 0..3 {
            for b_b_i in 0..3 {
                for f_t_i in 0..t_list.len() {
                    for b_t_i in 0..t_list.len() {
                        for s_i in 0..s_list.len() {
                            let shape = EdgeShape {
                                blank: [f_b_i, b_b_i],
                                trend: [t_list[f_t_i], t_list[b_t_i]],
                                middle: s_list[s_i],
                            };

                            assert_eq!(
                                f_b_i != 0 && f_t_i == 1 && s_i == 4 && b_b_i != 0 && b_t_i == 3,
                                rule1.is_match(&shape)
                            );
                            assert_eq!(
                                f_b_i != 0 && f_t_i == 3 && (s_i == 4 || s_i == 2),
                                rule2.is_match(&shape)
                            );
                            assert_eq!(f_b_i != 0 && f_t_i == 0, rule3.is_match(&shape));
                            assert_eq!(
                                f_b_i == 0 && f_t_i == 2 && b_b_i == 0 && b_t_i == 2,
                                rule4.is_match(&shape)
                            );
                            assert_eq!(f_b_i == 0 && b_b_i == 0, rule5.is_match(&shape));
                            assert_eq!(
                                f_b_i == 2 && f_t_i == 1 && s_i == 1 && b_b_i == 2 && b_t_i == 1,
                                rule6.is_match(&shape)
                            );
                            assert_eq!(f_b_i == 2 && b_b_i == 1, rule7.is_match(&shape));
                            assert_eq!(f_b_i != 0 && b_b_i == 0, rule8.is_match(&shape));
                        }
                    }
                }
            }
        }
    }
}
