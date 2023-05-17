use crate::{
    fas_file::{self, FasFile},
    struc::{StrucProto, StrucWork},
};

use std::collections::BTreeMap;

#[derive(Default)]
pub struct Service {
    changed: bool,
    source: Option<FasFile>,
}

impl Service {
    pub fn new(path: &str) -> Result<Self, fas_file::Error> {
        match FasFile::from_file(path) {
            Ok(fas) => Ok(Self {
                changed: false,
                source: Some(fas),
            }),
            Err(e) => Err(e),
        }
    }

    pub fn source(&self) -> Option<&FasFile> {
        self.source.as_ref()
    }

    pub fn get_struc_proto(&self, name: &str) -> StrucProto {
        match &self.source {
            Some(source) => source.components.get(name).cloned().unwrap_or_default(),
            None => Default::default(),
        }
    }

    pub fn get_struc_standerd(&self, name: &str) -> StrucWork {
        match &self.source {
            Some(source) => source
                .components
                .get(name)
                .cloned()
                .unwrap_or_default()
                .to_standard(&source.alloc_tab),
            None => Default::default(),
        }
    }

    pub fn get_struc_standerd_all(&self) -> BTreeMap<String, StrucWork> {
        match &self.source {
            Some(source) => source
                .components
                .iter()
                .map(|(name, struc)| (name.clone(), struc.to_standard(&source.alloc_tab)))
                .collect(),
            None => Default::default(),
        }
    }

    pub fn is_changed(&self) -> bool {
        self.source.is_some() && self.changed
    }

    pub fn comp_name_list(&self) -> Vec<String> {
        match &self.source {
            Some(source) => source.components.keys().cloned().collect(),
            None => vec![],
        }
    }
}
