use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use super::construct::fasing_1_0;
use super::{
    struc::{attribute::StrucAllocates, space::*, *},
    DataHV,
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
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WeightRegex {
    #[serde(with = "serde_regex")]
    pub regex: Regex,
    pub weight: usize,
}

impl WeightRegex {
    pub fn from_str(regex: &str, weight: usize) -> Result<Self, regex::Error> {
        Ok(Self {
            regex: Regex::new(regex)?,
            weight,
        })
    }

    pub fn new(regex: Regex, weight: usize) -> Self {
        Self { regex, weight }
    }
}

pub struct TransformValue {
    pub allocs: Vec<usize>,
    pub length: f32,
    pub min_step: f32,
    pub step: f32,
}

impl TransformValue {
    pub fn new(
        mut allocs: Vec<usize>,
        length: f32,
        min: f32,
        increment: f32,
        limit: &BTreeMap<usize, f32>,
    ) -> Result<Self, Error> {
        let step_limit = match limit.get(&allocs.iter().filter(|&&n| n != 0).count()) {
            None => length,
            Some(&limit) => limit.min(length).max(min),
        };

        let mut alloc_length = allocs.iter().sum::<usize>();
        let mut alloc_max = allocs.iter().cloned().max().unwrap_or_default();

        while alloc_length as f32 * min > length || step_limit / (alloc_max as f32) < min {
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
                    length,
                    min,
                });
            }
            alloc_max -= 1;
        }

        let alloc_length = alloc_length as f32;
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
            let mut one_num = 0.0;
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

#[derive(Serialize, Deserialize)]
pub struct ComponetConfig {
    pub min_space: f32,
    pub increment: f32,
    pub limit: DataHV<BTreeMap<usize, f32>>,
}

impl Default for ComponetConfig {
    fn default() -> Self {
        Self {
            min_space: 0.06,
            increment: 0.12,

            limit: Default::default(),
        }
    }
}

impl ComponetConfig {
    pub fn single_allocation(
        &self,
        allocs: StrucAllocates,
        size: WorkSize,
    ) -> Result<DataHV<TransformValue>, Error> {
        Ok(DataHV {
            h: TransformValue::new(
                allocs.h,
                size.width,
                self.min_space,
                self.increment,
                &self.limit.h,
            )?,
            v: TransformValue::new(
                allocs.v,
                size.height,
                self.min_space,
                self.increment,
                &self.limit.v,
            )?,
        })
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct AllocateTable(Vec<WeightRegex>);

impl Deref for AllocateTable {
    type Target = Vec<WeightRegex>;

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
    pub fn new(table: Vec<WeightRegex>) -> Self {
        Self(table)
    }

    pub fn get_weight(&self, attr: &str) -> usize {
        for wr in self.0.iter() {
            if wr.regex.is_match(attr) {
                return wr.weight;
            }
        }
        1
    }

    pub fn get_weight_in(&self, attr: &str) -> (usize, usize) {
        for (i, wr) in self.0.iter().enumerate() {
            if wr.regex.is_match(attr) {
                return (i, wr.weight);
            }
        }
        (self.0.len(), 1)
    }

    pub fn match_in(&self, attr: &str) -> usize {
        for (i, wr) in self.0.iter().enumerate() {
            if wr.regex.is_match(attr) {
                return i;
            }
        }
        self.0.len()
    }

    pub fn match_in_regex(&self, attr: &str) -> Option<usize> {
        for (i, wr) in self.0.iter().enumerate() {
            if wr.regex.is_match(attr) {
                return Some(i);
            }
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub alloc_tab: AllocateTable,
    pub components: BTreeMap<String, StrucProto>,
}

impl std::default::Default for FasFile {
    fn default() -> Self {
        Self {
            name: "untile".to_string(),
            major_version: 0,
            minor_version: 1,
            alloc_tab: Default::default(),
            components: Default::default(),
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
        } = TransformValue::new(vec![2, 1], 1.0, 0.06, 0.12, &limit).unwrap();
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
        } = TransformValue::new(vec![2], 1.0, 0.06, 0.12, &limit).unwrap();
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
        } = TransformValue::new(vec![3, 2], 1.0, 0.05, 0.05, &limit).unwrap();
        assert_eq!(allocs, vec![3, 2]);
        assert_eq!(length, 1.0);
        assert_eq!(min_step, 0.1);
        assert_eq!(step, 0.2);

        let TransformValue {
            allocs,
            length,
            min_step,
            step,
        } = TransformValue::new(vec![2, 3, 2], 0.5, 0.06, 0.12, &BTreeMap::new()).unwrap();
        assert_eq!(allocs, vec![2, 3, 2]);
        assert_eq!(length, 0.5);
        assert_eq!(min_step, 0.5 / 7.0);
        assert_eq!(step, 0.5 / 7.0);
    }

    #[test]
    fn test_allocate() {
        let table = AllocateTable::new(vec![WeightRegex::from_str(r"[hv](..M..;)+$", 0).unwrap()]);
        assert_eq!(table.get_weight("hA1M2O;"), 0);
        assert_eq!(table.get_weight("vX0M2L;X0L2L;"), 1);
        let table = AllocateTable::default();
        assert_eq!(table.get_weight("hA1M2O;"), 1);
    }

    #[test]
    fn test_fas_file() {
        let mut test_file = FasFile::default();
        let table = construct::fasing_1_0::generate_table();

        let requis = construct::all_requirements(&table);
        requis.into_iter().for_each(|comp| {
            test_file.components.insert(comp, StrucProto::default());
        });

        let mut key_points = StrucWokr::default();
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
