use crate::{config::Config, construct::Components};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub version: String,
    pub components: Components,
    pub config: Config,
}

impl FasFile {
    pub fn from_file(path: &str) -> Result<Self> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn save(&self, path: &str) -> Result<usize> {
        let texts = serde_json::to_string(self)?;
        Ok(std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))?)
    }

    pub fn save_pretty(&self, path: &str) -> Result<usize> {
        let texts = serde_json::to_string_pretty(self)?;
        Ok(std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))?)
    }
}

impl std::default::Default for FasFile {
    fn default() -> Self {
        Self {
            name: "untile".to_string(),
            version: "0.1".to_string(),
            components: Default::default(),
            config: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fas_file() {
        use crate::{component::struc::StrucProto, construct::space::*};

        let mut test_file = FasFile::default();

        let struc = StrucProto {
            paths: vec![KeyPath::from([
                IndexPoint::new(0, 0),
                IndexPoint::new(2, 0),
                IndexPoint::new(2, 2),
                IndexPoint::new(0, 2),
                IndexPoint::new(0, 0),
            ])],
            attrs: Default::default(),
        };
        test_file.components.insert("口".to_string(), struc);

        test_file.config.type_replace.insert(
            '⿰',
            std::collections::BTreeMap::from([(
                crate::axis::Place::Start,
                std::collections::BTreeMap::from([(
                    "王".to_string(),
                    crate::construct::Component::from_name("王字旁"),
                )]),
            )]),
        );

        test_file.config.supplement.insert(
            "無".to_string(),
            crate::construct::CpAttrs {
                tp: crate::construct::CstType::Scale(crate::axis::Axis::Vertical),
                components: vec![
                    crate::construct::Component::from_name("無字头"),
                    crate::construct::Component::from_name("灬"),
                ],
            },
        );

        test_file
            .config
            .interval
            .rules
            .push(crate::config::interval::IntervalMatch {
                axis: None,
                rule1: serde_json::from_str("\"*;>0;*;>0;*;>0;*;>0;*\"").unwrap(),
                rule2: serde_json::from_str("\"*;>0;*;>0;*;>0;*;>0;*\"").unwrap(),
                val: 2,
            });

        test_file
            .config
            .interval
            .rules
            .push(crate::config::interval::IntervalMatch {
                axis: Some(crate::axis::Axis::Horizontal),
                rule1: serde_json::from_str("\"*;>0;*;>0;*;>0;*;>0;*\"").unwrap(),
                rule2: serde_json::from_str("\"*;>0;*;>0;*;>0;*;>0;*\"").unwrap(),
                val: 2,
            });

        let tmp_dir = std::path::Path::new("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir(tmp_dir).unwrap();
        }
        std::fs::write(
            tmp_dir.join("fas_file.fas"),
            serde_json::to_string_pretty(&test_file).unwrap(),
        )
        .unwrap();
    }
}
