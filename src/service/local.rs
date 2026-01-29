use crate::{
    combination::struc::StrucProto,
    construct::CstTable,
    service::{Service, fas::FasFile},
};

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

    pub fn save(&mut self, path: &str) -> anyhow::Result<()> {
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

    pub fn set_config(&mut self, setting: &str, _value: serde_json::Value) {
        if let Some(_) = &mut self.source {
            self.changed |= match setting {
                _ => false,
            };
        }
    }

    pub fn load_fas(&mut self, data: FasFile) {
        self.source = Some(data);
        self.changed = false;
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        match FasFile::from_file(path) {
            Ok(data) => {
                self.load_fas(data);
                Ok(())
            }
            Err(e) => Err(format!("{:?}", e)),
        }
    }
}

impl Service for LocalService {
    fn get_strucs(&self) -> &super::Strucs {
        &self.source.as_ref().unwrap().strucs
    }

    fn get_config(&self) -> &crate::config::Config {
        &self.source.as_ref().unwrap().config
    }

    fn get_table(&self) -> &CstTable {
        &self.table
    }
}
