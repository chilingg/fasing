use crate::{
    combination::StrucProto,
    construct::CstTable,
    service::{
        Service,
        fas::{FasFile, Strucs},
    },
};
use anyhow::Result;

pub struct LocalService {
    changed: bool,
    source: Option<FasFile>,
    table: CstTable,
}

impl LocalService {
    pub fn new(table: CstTable) -> Self {
        Self {
            changed: false,
            table: table,
            source: None,
        }
    }

    pub fn standard() -> Self {
        Self {
            changed: false,
            table: CstTable::standard(),
            source: None,
        }
    }

    pub fn source(&self) -> Option<&FasFile> {
        self.source.as_ref()
    }

    pub fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn save(&mut self, path: &str) -> Result<()> {
        match &self.source {
            Some(source) => source.save_pretty(path).map(|_| {
                self.changed = false;
                ()
            }),
            None => Ok(()),
        }
    }

    pub fn save_struc(&mut self, name: String, struc: StrucProto) {
        if let Some(source) = &mut self.source {
            source.strucs.insert(name, struc);
            self.changed = true;
        }
    }

    pub fn load_fas(&mut self, data: FasFile) {
        self.source = Some(data);
        self.changed = false;
    }

    pub fn load_file(&mut self, path: &str) -> Result<()> {
        match FasFile::from_file(path) {
            Ok(data) => {
                self.load_fas(data);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl Service for LocalService {
    fn get_strucs(&self) -> &Strucs {
        &self.source.as_ref().unwrap().strucs
    }

    fn get_config(&self) -> &crate::config::Config {
        &self.source.as_ref().unwrap().config
    }

    fn get_table(&self) -> &CstTable {
        &self.table
    }
}
