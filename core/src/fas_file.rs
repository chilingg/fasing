use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use super::construct::fasing_1_0;
use super::struc::*;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    Deserialize(serde_json::Error),
    Io(std::io::Error),
}

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Self::Deserialize(e) => e.to_string(),
            Self::Io(e) => e.to_string(),
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
    use space::*;

    use crate::construct;

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
