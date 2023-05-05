use crate::{
    fas_file::{Error, FasFile},
    struc::StrucProto,
};

// use std::collections::BTreeMap;

#[derive(Default)]
pub struct Service {
    changed: bool,
    source: Option<FasFile>,
}

impl Service {
    pub fn new(path: &str) -> Result<Self, Error> {
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
