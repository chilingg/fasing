use crate::{
    construct,
    fas_file::{self, Error, FasFile},
    hv::*,
    struc::{
        space::{WorkPoint, WorkRect, WorkSize},
        StrucComb, StrucProto, StrucWork, TransformValue,
    },
};

use std::collections::BTreeMap;

#[derive(serde::Serialize, Clone)]
pub struct CombInfos {
    name: String,
    format: construct::Format,
    limit: Option<WorkSize>,
    trans: Option<DataHV<TransformValue>>,
    comps: Vec<CombInfos>,
}

impl CombInfos {
    pub fn new(comb: &StrucComb) -> Self {
        match comb {
            StrucComb::Single {
                name, limit, trans, ..
            } => CombInfos {
                name: name.clone(),
                format: construct::Format::Single,
                limit: limit.clone(),
                trans: trans.clone(),
                comps: vec![],
            },
            StrucComb::Complex {
                name,
                format,
                comps,
                limit,
            } => CombInfos {
                name: name.clone(),
                format: *format,
                limit: limit.clone(),
                trans: None,
                comps: comps.iter().map(|comb| CombInfos::new(comb)).collect(),
            },
        }
    }
}

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

    fn get_comb_and_trans(&self, name: char) -> Result<(StrucComb, DataHV<TransformValue>), Error> {
        match &self.source {
            Some(source) => {
                let const_table = &self.construct_table;
                let alloc_table = &source.alloc_tab;
                let components = &source.components;
                let config = &source.config;

                let mut comb = StrucComb::new(
                    name.to_string(),
                    const_table,
                    alloc_table,
                    components,
                    config,
                )?;
                let trans_value =
                    comb.allocation(WorkSize::splat(1.0), WorkPoint::zero(), config)?;

                if trans_value.hv_iter().all(|t| t.allocs.is_empty()) {
                    Err(Error::Empty(name.to_string()))
                } else {
                    Ok((comb, trans_value))
                }
            }
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    pub fn get_struc_comb(&self, name: char) -> Result<StrucWork, Error> {
        let (comb, trans_value) = self.get_comb_and_trans(name)?;

        let offset = WorkPoint::new(
            match trans_value.h.allocs.iter().all(|&n| n == 0) {
                true => trans_value.h.length * 0.5,
                false => 0.5 - trans_value.h.length * 0.5,
            },
            match trans_value.v.allocs.iter().all(|&n| n == 0) {
                true => trans_value.v.length * 0.5,
                false => 0.5 - trans_value.v.length * 0.5,
            },
        );

        Ok(comb.to_work(
            offset,
            WorkRect::new(WorkPoint::origin(), WorkSize::splat(1.0)),
        ))
    }

    pub fn get_config(&self) -> Option<fas_file::ComponetConfig> {
        self.source().map(|source| source.config.clone())
    }

    pub fn get_comb_info(&self, name: char) -> Result<CombInfos, Error> {
        match self.get_comb_and_trans(name) {
            Ok((comb, _)) => Ok(CombInfos::new(&comb)),
            Err(e) => Err(e),
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
