use crate::{config::Config, service::Strucs};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub version: String,
    pub strucs: Strucs,
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

    pub fn versions(&self) -> [u32; 2] {
        let mut iter = self
            .version
            .split('.')
            .map(|n| n.parse::<u32>().unwrap_or_default());
        [
            iter.next().unwrap_or_default(),
            iter.next().unwrap_or_default(),
        ]
    }
}

impl std::default::Default for FasFile {
    fn default() -> Self {
        Self {
            name: "untile".to_string(),
            version: "0.1".to_string(),
            strucs: Default::default(),
            config: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fas_file() {
        use crate::{base::*, combination::struc::StrucProto};

        let mut test_file = FasFile::default();

        let struc = StrucProto {
            paths: vec![KeyPath::from([key_pos(0, 0), key_pos(4, 0)])],
            attrs: Default::default(),
        };
        test_file.strucs.insert("一".to_string(), struc);

        let tmp_dir = std::path::Path::new("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir(tmp_dir).unwrap();
        }
        std::fs::write(
            tmp_dir.join("fas_file.fas.json"),
            serde_json::to_string_pretty(&test_file).unwrap(),
        )
        .unwrap();
    }
}
