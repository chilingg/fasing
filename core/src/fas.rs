use crate::{config::*, construct::Components};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    Deserialize(serde_json::Error),
    Io(std::io::Error),
}

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub version: String,
    pub components: Components,
    pub config: Config,
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

    #[test]
    fn test_fas_file() {
        let mut test_file = FasFile::default();

        test_file.config.place_replace.insert(
            '⿰',
            std::collections::BTreeMap::from([(
                crate::axis::Place::Start,
                std::collections::BTreeMap::from([(
                    "王".to_string(),
                    crate::construct::Component::from_name("王字旁"),
                )]),
            )]),
        );

        test_file.config.correction_table.insert(
            "無".to_string(),
            crate::construct::Attrs {
                tp: crate::construct::Type::Scale(crate::axis::Axis::Vertical),
                components: vec![
                    crate::construct::Component::from_name("無字头"),
                    crate::construct::Component::from_name("灬"),
                ],
            },
        );

        let mut proto = crate::component::struc::StrucProto::default();
        proto.set_attr::<crate::component::attrs::InPlaceAllocs>(&vec![(
            "*".to_string(),
            crate::axis::DataHV::splat(vec![0usize]),
        )]);
        test_file.components.insert("test".to_string(), proto);

        test_file.config.white_weights = std::collections::BTreeMap::from([
            (crate::component::view::Element::Diagonal, 0.72),
            (crate::component::view::Element::Dot, 0.72),
            (crate::component::view::Element::Face, 0.2),
        ]);

        test_file
            .config
            .interval_rule
            .push(MatchValue::new(regex::Regex::new(".*").unwrap(), 1));

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
