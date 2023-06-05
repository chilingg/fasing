use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
};

use super::{
    construct::{self, fasing_1_0},
    hv::*,
    struc::{attribute::StrucAllocates, space::*, *},
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

#[derive(Clone, Default, Serialize)]
pub struct TransformValue {
    pub allocs: Vec<usize>,
    pub length: f32,
    pub min_step: f32,
    pub step: f32,
}

impl TransformValue {
    pub fn from_step(allocs: Vec<usize>, min_step: f32, step: f32) -> Self {
        Self {
            length: allocs
                .iter()
                .map(|&n| match n {
                    1 => min_step,
                    n => n as f32 * step,
                })
                .sum(),
            allocs: allocs.clone(),
            min_step,
            step,
        }
    }

    pub fn from_allocs(
        allocs: Vec<usize>,
        length: f32,
        min: f32,
        increment: f32,
        limit: &BTreeMap<usize, f32>,
    ) -> Result<Self, Error> {
        Self::from_allocs_interval(allocs, length, min, increment, 0.0, limit)
    }

    pub fn from_allocs_interval(
        mut allocs: Vec<usize>,
        length: f32,
        min: f32,
        increment: f32,
        interval_times: f32,
        limit: &BTreeMap<usize, f32>,
    ) -> Result<Self, Error> {
        // attribute::StrucAttributes::compact(&mut allocs);

        let mut alloc_length = allocs.iter().sum::<usize>();
        let mut alloc_max = allocs.iter().cloned().max().unwrap_or_default();

        if length == 0.0 || alloc_length == 0 {
            allocs.fill(0);
            return Ok(Self {
                allocs,
                length: 0.0,
                min_step: min + increment,
                step: min + increment,
            });
        }

        let step_limit = match limit.get(&allocs.iter().filter(|n| **n != 0).count()) {
            None => length,
            Some(&limit) => limit.min(length).max(min),
        };

        loop {
            if (alloc_length as f32 + interval_times) * min > length
                || step_limit / (alloc_max as f32) < min
            {
                let mut can = false;
                alloc_length = allocs.iter_mut().fold(0, |len, n| {
                    if *n > 1 {
                        *n -= 1;
                        can = true;
                    }
                    len + *n
                });
                if !can {
                    return Err(Error::Transform {
                        alloc_len: alloc_length,
                        length: length - interval_times * min,
                        min,
                    });
                }
                alloc_max -= 1;
            } else {
                break;
            }
        }

        let alloc_length = alloc_length as f32 + interval_times;
        let min_max = min + increment;
        Ok(if allocs.iter().all(|&n| n == 0 || n == alloc_max) {
            match length / alloc_length {
                step if step * alloc_max as f32 <= step_limit => Self {
                    allocs,
                    length,
                    min_step: min_max.min(step),
                    step,
                },
                _ => {
                    let step_limit = step_limit / alloc_max as f32;
                    Self {
                        allocs,
                        length: alloc_length * step_limit,
                        min_step: min_max.min(step_limit),
                        step: step_limit,
                    }
                }
            }
        } else {
            let step_limit = step_limit / (alloc_max as f32);
            let min_max = min_max.min(step_limit);
            let mut one_num = interval_times;
            let other_size = allocs
                .iter()
                .filter(|&&n| {
                    if n == 1 {
                        one_num += n as f32;
                        false
                    } else {
                        true
                    }
                })
                .sum::<usize>() as f32;

            let mut result = if alloc_length * min_max <= length {
                Self {
                    allocs,
                    length,
                    min_step: min_max,
                    step: (length - one_num * min_max) / other_size,
                }
            } else {
                let val = length / alloc_length;
                Self {
                    allocs,
                    length,
                    min_step: val,
                    step: val,
                }
            };
            if result.step > step_limit {
                result.step = step_limit;
                result.length = other_size * step_limit + one_num * result.min_step;
            }

            result
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ComponetConfig {
    pub min_space: f32,
    pub increment: f32,
    pub limit: DataHV<BTreeMap<usize, f32>>,

    pub interval_judge: Vec<WeightRegex<f32>>,
    pub replace_list: BTreeMap<construct::Format, BTreeMap<usize, BTreeMap<String, String>>>,
    pub format_limit:
        BTreeMap<construct::Format, BTreeMap<usize, Vec<(BTreeSet<String>, WorkSize)>>>,

    #[serde(with = "serde_regex")]
    pub reduce_check: Regex,
    pub reduce_targger: f32,
}

impl Default for ComponetConfig {
    fn default() -> Self {
        Self {
            min_space: 0.06,
            increment: 0.12,

            limit: Default::default(),
            interval_judge: Default::default(),
            replace_list: Default::default(),
            format_limit: Default::default(),

            reduce_targger: 0.06,
            reduce_check: Regex::new("^$").unwrap(),
        }
    }
}

impl ComponetConfig {
    pub fn gen_comp_format(name: String, format: construct::Format, in_fmt: usize) -> String {
        format!(
            "{},{},{}",
            format.to_symbol().unwrap_or_default(),
            in_fmt,
            name
        )
    }

    pub fn min_max_step(&self) -> f32 {
        self.min_space + self.increment
    }

    pub fn single_allocation(
        &self,
        allocs: StrucAllocates,
        size: WorkSize,
    ) -> Result<DataHV<TransformValue>, Error> {
        Ok(DataHV {
            h: TransformValue::from_allocs(
                allocs.h,
                size.width,
                self.min_space,
                self.increment,
                &self.limit.h,
            )
            .map_err(|e| e.marked_transform(Axis::Horizontal))?,
            v: TransformValue::from_allocs(
                allocs.v,
                size.height,
                self.min_space,
                self.increment,
                &self.limit.v,
            )
            .map_err(|e| e.marked_transform(Axis::Horizontal))?,
        })
    }

    // pub fn allocation_components()
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
pub struct FasFile {
    pub name: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub alloc_tab: AllocateTable,
    pub components: BTreeMap<String, StrucProto>,
    pub config: ComponetConfig,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::construct;

    #[test]
    fn test_transform() {
        let limit: BTreeMap<usize, f32> = BTreeMap::from([(2, 0.8)]);
        let TransformValue {
            allocs,
            length,
            min_step,
            step,
        } = TransformValue::from_allocs(vec![2, 1], 1.0, 0.06, 0.12, &limit).unwrap();
        assert_eq!(allocs, vec![2, 1]);
        assert_eq!(length, 0.98);
        assert!((min_step - 0.18).abs() < 0.001);
        assert_eq!(step, 0.4);

        let limit: BTreeMap<usize, f32> = BTreeMap::from([(1, 0.8)]);
        let TransformValue {
            allocs,
            length,
            min_step,
            step,
        } = TransformValue::from_allocs(vec![2], 1.0, 0.06, 0.12, &limit).unwrap();
        assert_eq!(allocs, vec![2]);
        assert_eq!(length, 0.8);
        assert!((min_step - 0.18).abs() < 0.001);
        assert_eq!(step, 0.4);

        let limit: BTreeMap<usize, f32> = BTreeMap::from([(1, 0.8), (2, 0.7)]);
        let TransformValue {
            allocs,
            length,
            min_step,
            step,
        } = TransformValue::from_allocs(vec![3, 2], 1.0, 0.05, 0.05, &limit).unwrap();
        assert_eq!(allocs, vec![3, 2]);
        assert_eq!(length, 1.0);
        assert_eq!(min_step, 0.1);
        assert_eq!(step, 0.2);

        let TransformValue {
            allocs,
            length,
            min_step,
            step,
        } = TransformValue::from_allocs(vec![2, 3, 2], 0.5, 0.06, 0.12, &BTreeMap::new()).unwrap();
        assert_eq!(allocs, vec![2, 3, 2]);
        assert_eq!(length, 0.5);
        assert_eq!(min_step, 0.5 / 7.0);
        assert_eq!(step, 0.5 / 7.0);
    }

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
