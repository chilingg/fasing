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

#[derive(Serialize, Deserialize, Clone)]
pub struct ComponetConfig {
    pub min_values: Vec<f32>,
    pub assign_values: Vec<f32>,

    pub interval_rule: Vec<WeightRegex<f32>>,
    pub replace_list: BTreeMap<construct::Format, BTreeMap<usize, BTreeMap<String, String>>>,
    pub format_limit:
        BTreeMap<construct::Format, BTreeMap<usize, Vec<(BTreeSet<String>, WorkSize)>>>,

    pub reduce_checks: Vec<WeightRegex<usize>>,
    pub reduce_targger: f32,
}

impl Default for ComponetConfig {
    fn default() -> Self {
        Self {
            min_values: vec![0.1],
            assign_values: vec![1.0],

            interval_rule: Default::default(),
            replace_list: Default::default(),
            format_limit: Default::default(),

            reduce_targger: 0.05,
            reduce_checks: Default::default(),
        }
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
