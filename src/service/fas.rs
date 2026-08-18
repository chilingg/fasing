use crate::{combination::StrucProto, config::Config};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Strucs(BTreeMap<String, StrucProto>);

impl std::ops::Deref for Strucs {
    type Target = BTreeMap<String, StrucProto>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Strucs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Strucs {
    pub fn filter_stroke(&self, stroke: &str) -> Vec<&str> {
        self.iter()
            .filter_map(
                |(name, proto)| match proto.strokes().iter().any(|s| s == stroke) {
                    true => Some(name.as_str()),
                    false => None,
                },
            )
            .collect()
    }
}

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

    pub fn from_folder(path: &str) -> Result<Self> {
        let folder = std::path::Path::new(path);
        match folder.try_exists() {
            Err(e) => Err(e)?,
            Ok(true) if folder.is_dir() => {}
            _ => Err(std::io::Error::new(std::io::ErrorKind::NotFound, path))?,
        }

        let info_path = folder.join("info.json");
        let mut info: BTreeMap<String, String> =
            serde_json::from_str(&std::fs::read_to_string(&info_path)?)?;

        let mut strucs = Strucs::default();
        let strucs_path = folder.join("strucs");
        for entry in std::fs::read_dir(&strucs_path)? {
            match entry.map(|e| e.path()) {
                Err(e) => log::error!("{e}"),
                Ok(path)
                    if path.is_file()
                        && matches!(path.extension().and_then(|s| s.to_str()), Some("json")) =>
                {
                    if let Some(char_name) = path.file_prefix().and_then(|s| s.to_str()) {
                        strucs.insert(
                            char_name.to_string(),
                            serde_json::from_str(&std::fs::read_to_string(path)?)?,
                        );
                    } else {
                        Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Unable to parse file name: {:?}", path),
                        ))?
                    }
                }
                _ => {}
            }
        }

        let config_path = folder.join("config");
        let mut config_datas = Vec::with_capacity(3);
        for name in ["assign.json", "ctrls.json", "chars.json"] {
            let path = config_path.join(name);
            config_datas.push(serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(path)?,
            )?);
        }
        let chars = config_datas.pop().unwrap();
        let ctrls = config_datas.pop().unwrap();
        let assign = config_datas.pop().unwrap();

        Ok(Self {
            name: info.remove("name").ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Missing info `name`",
            ))?,
            version: info.remove("version").ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Missing info `version`",
            ))?,
            strucs,
            config: Config::from_datas(assign, ctrls, chars),
        })
    }

    pub fn get_info(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("name".to_string(), self.name.to_string()),
            ("version".to_string(), self.version.to_string()),
        ])
    }

    pub fn save(&self, path: &str) -> Result<usize> {
        let texts = serde_json::to_string(self)?;
        Ok(std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))?)
    }

    pub fn save_pretty(&self, path: &str) -> Result<usize> {
        let texts = serde_json::to_string_pretty(self)?;
        Ok(std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))?)
    }

    pub fn save_to_folder(&self, path: &str) -> Result<()> {
        let folder = std::path::Path::new(path);
        match folder.try_exists() {
            Err(e) => Err(e)?,
            Ok(true) if folder.is_dir() => {}
            _ => Err(std::io::Error::new(std::io::ErrorKind::NotFound, path))?,
        }

        let info = serde_json::to_string_pretty(&self.get_info()).unwrap();
        std::fs::write(folder.join("info.json"), info)?;

        let strucs_path = folder.join("strucs");
        if !strucs_path.exists() {
            std::fs::create_dir(&strucs_path)?;
        }
        let mut old_files = std::collections::HashSet::new();
        for entry in std::fs::read_dir(&strucs_path)? {
            match entry.map(|e| e.path()) {
                Err(e) => log::error!("{e}"),
                Ok(path)
                    if path.is_file()
                        && matches!(path.extension().and_then(|s| s.to_str()), Some("json")) =>
                {
                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        old_files.insert(file_name.to_string());
                    }
                }
                _ => {}
            }
        }
        for (name, struc) in self.strucs.iter() {
            let name = format!("{name}.json");
            if old_files.contains(&name) {
                old_files.remove(&name);
            }
            let struc_file = strucs_path.join(name);
            let struc_data = serde_json::to_string_pretty(&struc).unwrap();
            std::fs::write(struc_file, struc_data)?;
        }
        for old_file in old_files {
            std::fs::remove_file(strucs_path.join(old_file))?;
        }

        let config_path = folder.join("config");
        if !config_path.exists() {
            std::fs::create_dir(&config_path)?;
        }
        let list = [
            ("assign.json", self.config.assign.get_data()),
            ("ctrls.json", self.config.ctrls.get_data()),
            ("chars.json", self.config.chars.get_data()),
        ];
        for (file_name, data) in list {
            let data = serde_json::to_string_pretty(&data).unwrap();
            std::fs::write(config_path.join(file_name), data)?;
        }

        Ok(())
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
