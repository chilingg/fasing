use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
};

use super::{
    construct::{self, fasing_1_0},
    hv::*,
    struc::{space::*, *},
};

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    Deserialize(serde_json::Error),
    Io(std::io::Error),
    Transform {
        alloc_len: usize,
        length: f32,
        min: f32,
    },
    AxisTransform {
        axis: Axis,
        alloc_len: usize,
        length: f32,
        min: f32,
    },
    Variety {
        name: String,
        fmt: construct::Format,
        in_fmt: usize,
        level: usize,
    },
    Empty(String),
    Surround(construct::Format, String, String),
    Message(String),
}

impl Error {
    pub fn marked_transform(self, axis: Axis) -> Self {
        match self {
            Error::Transform {
                alloc_len,
                length,
                min,
            } => Self::AxisTransform {
                axis,
                alloc_len,
                length,
                min,
            },
            _ => self,
        }
    }
}

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Self::Deserialize(e) => e.to_string(),
            Self::Io(e) => e.to_string(),
            Self::Transform {
                alloc_len,
                length,
                min,
            } => {
                format!(
                    "Length {}({}) requi a minimum value of {}({})!",
                    length,
                    alloc_len,
                    min,
                    (length / min).ceil()
                )
            }
            Self::Variety {
                name,
                fmt,
                in_fmt,
                level,
            } => {
                format!(
                    "\"{}\" cannot be variation to level{} in {}{}!",
                    name,
                    level,
                    fmt.to_symbol().unwrap_or_default(),
                    in_fmt
                )
            }
            Self::AxisTransform {
                axis,
                alloc_len,
                length,
                min,
            } => {
                format!(
                    "Length {}({}) requi a minimum value of {}({}) in {:?}!",
                    length,
                    alloc_len,
                    min,
                    (length / min).ceil(),
                    axis
                )
            }
            Self::Empty(name) => format!("\"{}\" is empty!", name),
            Self::Surround(fmt, primary, secondary) => {
                format!(
                    "\"{secondary}\" cannot be {} in \"{primary}\"",
                    fmt.to_symbol().unwrap()
                )
            }
            Self::Message(s) => s.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeightRegex<T = usize> {
    #[serde(with = "serde_regex")]
    pub regex: Regex,
    pub weight: T,
}

impl<T> WeightRegex<T> {
    pub fn from_str(regex: &str, weight: T) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: Regex::new(regex)?,
            weight,
        })
    }

    pub fn new(regex: Regex, weight: T) -> Self {
        Self { regex, weight }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AllocateRule {
    pub weight: usize,
    pub filter: BTreeSet<String>,
    #[serde(with = "serde_regex")]
    pub regex: Regex,
}

impl AllocateRule {
    pub fn new(regex: Regex, weight: usize, filter: BTreeSet<String>) -> Self {
        Self {
            regex,
            weight,
            filter,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComponetConfig {
    pub min_values: DataHV<Vec<f32>>,
    pub base_values: DataHV<Vec<f32>>,
    pub assign_values: DataHV<Vec<f32>>,

    pub interval_rule: Vec<WeightRegex<i32>>,
    pub replace_list: BTreeMap<construct::Format, BTreeMap<usize, BTreeMap<String, String>>>,
    pub format_limit:
        BTreeMap<construct::Format, BTreeMap<usize, Vec<(BTreeSet<String>, WorkSize)>>>,

    pub reduce_checks: Vec<WeightRegex<usize>>,
    pub reduce_trigger: f32,
}

impl Default for ComponetConfig {
    fn default() -> Self {
        Self {
            min_values: DataHV::splat(vec![0.1]),
            base_values: DataHV::splat(vec![1.0]),
            assign_values: DataHV::splat(vec![1.0]),

            interval_rule: Default::default(),
            replace_list: Default::default(),
            format_limit: Default::default(),

            reduce_trigger: 0.08,
            reduce_checks: Default::default(),
        }
    }
}

fn integer_index_last_get<'a, T>(
    list: &'a Vec<T>,
    index: usize,
    zero_value: &'a T,
) -> Option<&'a T> {
    if index == 0 {
        Some(zero_value)
    } else {
        list.get(index - 1).or(list.last())
    }
}

impl ComponetConfig {
    pub fn vh(&self) -> Self {
        Self {
            min_values: self.min_values.vh(),
            base_values: self.base_values.vh(),
            assign_values: self.assign_values.vh(),
            interval_rule: self.interval_rule.clone(),
            replace_list: self.replace_list.clone(),
            format_limit: self.format_limit.clone(),
            reduce_checks: self.reduce_checks.clone(),
            reduce_trigger: self.reduce_trigger.clone(),
        }
    }

    fn get_base_list(&self, axis: Axis, allocs: &Vec<usize>) -> Vec<f32> {
        let base_values = self.base_values.hv_get(axis);
        allocs
            .iter()
            .map(|&v| *integer_index_last_get(base_values, v, &0.0).unwrap_or(&1.0))
            .collect()
    }

    pub fn get_base_total(&self, axis: Axis, allocs: &Vec<usize>) -> f32 {
        self.get_base_list(axis, allocs).iter().sum()
    }

    pub fn get_interval_value(&self, axis: Axis, interval: i32) -> f32 {
        let base_values = self.base_values.hv_get(axis);
        let mut val = if interval == 0 {
            0.0
        } else {
            base_values
                .get((interval.abs() - 1) as usize)
                .or(base_values.last())
                .cloned()
                .unwrap_or(1.0)
        };
        if interval.is_negative() {
            val = -val;
        }
        val
    }

    pub fn get_interval_list(&self, axis: Axis, intervals: &Vec<i32>) -> Vec<f32> {
        intervals
            .iter()
            .map(|&v| self.get_interval_value(axis, v))
            .collect()
    }

    pub fn get_interval_base_total(&self, axis: Axis, intervals: &Vec<i32>) -> f32 {
        self.get_interval_list(axis, intervals).iter().sum()
    }

    pub fn get_trans_and_interval(
        &self,
        axis: Axis,
        length: f32,
        allocs: Vec<usize>,
        intervals: &Vec<i32>,
        level: Option<usize>,
        calculate: Option<&Vec<usize>>,
    ) -> Result<(TransformValue, Vec<f32>, Option<Vec<f32>>), Error> {
        let assign_values = self.assign_values.hv_get(axis);
        let min_values = self.min_values.hv_get(axis);

        let assign_list: Vec<f32> = allocs
            .iter()
            .map(|n| *integer_index_last_get(assign_values, *n, &0.0).unwrap_or(&0.0))
            .collect();
        let bases_list = self.get_base_list(axis, &allocs);

        let interval_assign_list: Vec<f32> = intervals
            .iter()
            .map(|n| {
                let mut val =
                    *integer_index_last_get(assign_values, n.abs() as usize, &0.0).unwrap_or(&0.0);
                if n.is_negative() {
                    val = -val;
                }
                val
            })
            .collect();
        let interval_list = self.get_interval_list(axis, intervals);

        let base_total = bases_list.iter().chain(interval_list.iter()).sum::<f32>();

        let level = {
            let val = match min_values
                .iter()
                .position(|v| length - v * base_total > -0.001)
            {
                Some(level) => level,
                None => {
                    return Err(Error::Transform {
                        alloc_len: allocs.iter().sum(),
                        length,
                        min: min_values.last().cloned().unwrap_or_default(),
                    });
                }
            };

            if let Some(level) = level {
                level.max(val)
            } else {
                val
            }
        };

        let min = *min_values.get(level).or(min_values.last()).unwrap();
        let assign_total = length - base_total * min;
        let assign_count: f32 = assign_list.iter().chain(interval_assign_list.iter()).sum();

        let one_assign = if assign_count == 0.0 {
            let number = assign_list.len() + interval_assign_list.len();
            if number == 0 {
                return Ok((
                    TransformValue {
                        length: 0.0,
                        level: 0,
                        allocs,
                        assign: vec![],
                    },
                    vec![],
                    None,
                ));
            }
            assign_total / number as f32
        } else {
            assign_total / assign_count
        };

        let assign: Vec<f32> = bases_list
            .iter()
            .zip(assign_list.iter())
            .map(|(&n, &a)| min * n + one_assign * a)
            .collect();
        let interval_assign: Vec<f32> = interval_list
            .iter()
            .zip(interval_assign_list.iter())
            .map(|(&n, &a)| min * n + one_assign * a)
            .collect();

        Ok((
            TransformValue {
                level,
                length: assign.iter().sum(),
                allocs,
                assign,
            },
            interval_assign,
            calculate.map(|list| {
                let assigns = list
                    .iter()
                    .map(|n| *integer_index_last_get(assign_values, *n, &0.0).unwrap_or(&0.0));
                self.get_base_list(axis, list)
                    .into_iter()
                    .zip(assigns)
                    .map(|(n, a)| min * n + one_assign * a)
                    .collect()
            }),
        ))
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AllocateTable(Vec<AllocateRule>);

impl Deref for AllocateTable {
    type Target = Vec<AllocateRule>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AllocateTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AllocateTable {
    pub fn new(table: Vec<AllocateRule>) -> Self {
        Self(table)
    }
}

#[derive(Serialize, Deserialize)]
pub struct StrokeMatch {
    pub stroke: String,
    pub min_size: DataHV<Option<f32>>,
    pub min_level: DataHV<Option<usize>>,
    pub collision: Vec<Option<Vec<char>>>,
    pub pos_types: Vec<Option<KeyPointType>>,
}

#[derive(Serialize, Deserialize)]
pub struct StrokeReplace {
    pub matchs: StrokeMatch,
    pub replace: StrokePath,
}

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub alloc_tab: AllocateTable,
    pub components: BTreeMap<String, StrucProto>,
    pub config: ComponetConfig,
    pub stroke_matchs: Vec<StrokeReplace>,
}

impl std::default::Default for FasFile {
    fn default() -> Self {
        Self {
            name: "untile".to_string(),
            major_version: 0,
            minor_version: 1,
            alloc_tab: Default::default(),
            components: Default::default(),
            config: Default::default(),
            stroke_matchs: Default::default(),
        }
    }
}

impl FasFile {
    pub fn from_file(path: &str) -> Result<Self, Error> {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<Self>(&content) {
                Ok(obj) => Ok(obj),
                Err(e) => Err(Error::Deserialize(e)),
            },
            Err(e) => Err(Error::Io(e)),
        }
    }

    pub fn from_template_fasing_1_0() -> Self {
        fasing_1_0::generate_fas_file()
    }

    pub fn save(&self, path: &str) -> std::io::Result<usize> {
        let texts = serde_json::to_string(self).unwrap();
        std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))
    }

    pub fn save_pretty(&self, path: &str) -> std::io::Result<usize> {
        let texts = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::construct;

    #[test]
    fn test_fas_file() {
        let mut test_file = FasFile::default();
        let table = construct::fasing_1_0::generate_table();

        let requis = construct::all_requirements(&table);
        requis.into_iter().for_each(|comp| {
            test_file.components.insert(comp, StrucProto::default());
        });

        let mut key_points = StrucWork::default();
        key_points.add_lines([WorkPoint::new(0.0, 1.0), WorkPoint::new(1.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(2.0, 0.0), WorkPoint::new(2.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(4.0, 1.0), WorkPoint::new(3.0, 2.0)], false);
        assert_eq!(
            key_points.key_paths[0].points[0],
            KeyPoint::new_line_point(WorkPoint::new(0.0, 1.0))
        );
        assert_eq!(
            key_points.key_paths[1].points[1],
            KeyPoint::new_line_point(WorkPoint::new(2.0, 2.0))
        );

        test_file
            .components
            .insert("⺌".to_string(), key_points.to_prototype());

        test_file.alloc_tab.push(AllocateRule::new(
            regex::Regex::new(".*").unwrap(),
            1,
            BTreeSet::from(["default".to_string()]),
        ));

        test_file
            .config
            .reduce_checks
            .push(WeightRegex::new(Regex::new("^$").unwrap(), 1));

        test_file.stroke_matchs.push(StrokeReplace {
            matchs: {
                StrokeMatch {
                    stroke: String::from("3"),
                    min_size: Default::default(),
                    min_level: Default::default(),
                    collision: Default::default(),
                    pos_types: Default::default(),
                }
            },
            replace: StrokePath {
                start: WorkPoint::zero(),
                segment: vec![BezierCtrlPointF::from_to(WorkPoint::splat(1.0))],
            },
        });

        let tmp_dir = std::path::Path::new("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir(tmp_dir.clone()).unwrap();
        }
        std::fs::write(
            tmp_dir.join("fas_file.fas"),
            serde_json::to_string_pretty(&test_file).unwrap(),
        )
        .unwrap();
    }
}
