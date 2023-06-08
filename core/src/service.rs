use crate::{
    construct,
    fas_file::{self, Error, FasFile},
    hv::*,
    struc::{
        space::{WorkPoint, WorkRect, WorkSize},
        StrucProto, StrucWork, VarietysComb,
    },
};

use std::collections::BTreeMap;

#[derive(Default)]
pub struct Service {
    changed: bool,
    source: Option<FasFile>,
    pub construct_table: construct::Table,
}

impl Service {
    pub fn new(path: &str) -> Result<Self, fas_file::Error> {
        match FasFile::from_file(path) {
            Ok(fas) => Ok(Self {
                changed: false,
                source: Some(fas),
                construct_table: construct::fasing_1_0::generate_table(),
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

    pub fn get_struc_proto_all(&self) -> BTreeMap<String, StrucProto> {
        match &self.source {
            Some(source) => source
                .components
                .iter()
                .map(|(name, struc)| (name.clone(), struc.clone()))
                .collect(),
            None => Default::default(),
        }
    }

    pub fn get_struc_comb(&self, name: char) -> Result<StrucWork, Error> {
        match &self.source {
            Some(source) => {
                let const_table = &self.construct_table;
                let alloc_table = &source.alloc_tab;
                let components = &source.components;
                let config = &source.config;

                let mut varitys = VarietysComb::new(
                    name.to_string(),
                    const_table,
                    alloc_table,
                    components,
                    config,
                )?;
                let trans_value =
                    varitys.allocation(WorkSize::splat(1.0), WorkSize::zero(), config)?;

                if trans_value.hv_iter().all(|t| t.allocs.is_empty()) {
                    return Err(Error::Empty(name.to_string()));
                }

                let offset = WorkPoint::new(
                    0.5 - trans_value.h.length * 0.5,
                    0.5 - trans_value.v.length * 0.5,
                );

                Ok(varitys.to_work(
                    offset,
                    WorkRect::new(WorkPoint::origin(), WorkSize::splat(1.0)),
                ))
            }
            None => Err(Error::Empty("Source".to_string())),
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

    pub fn save_struc(&mut self, name: String, struc: &StrucWork) {
        if let Some(source) = &mut self.source {
            source
                .components
                .insert(name, struc.to_prototype_offset(0.001));
            self.changed = true;
        }
    }

    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        match &self.source {
            Some(source) => match source.save(path) {
                Ok(_) => {
                    self.changed = false;
                    Ok(())
                }
                Err(e) => Err(e),
            },
            None => Ok(()),
        }
    }

    pub fn reload(&mut self, path: &str) {
        if let Ok(fas) = FasFile::from_file(path) {
            self.source = Some(fas);
            self.changed = false;
        }
    }

    pub fn normalization(struc: &StrucWork, offset: f32) -> StrucWork {
        struc.to_prototype_offset(offset).to_normal()
    }
}
