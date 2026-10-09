pub mod algorithm;
mod combination;
mod space;

pub mod fas;
pub mod local;

pub mod char_tree;
pub use char_tree::CharTree;

use crate::{
    base,
    combination::{CompTree, StrucComb, StrucProto},
    config::Config,
    construct::{Component, CpAttrs, CstError, CstTable, CstType},
};

use std::collections::BTreeMap;

pub trait Service {
    fn get_table(&self) -> &CstTable;
    fn get_config(&self) -> &Config;
    fn get_struc_proto(&self, name: &str) -> Option<&StrucProto>;

    fn get_char_tree(&self, name: String) -> CharTree
    where
        Self: Sized,
    {
        char_tree::get_char_tree(self, name)
    }

    fn get_struc_comb(&self, target: CharTree) -> Result<StrucComb, CstError>
    where
        Self: Sized,
    {
        let mut comb = combination::get_comb_proto_in(
            self,
            target,
            Default::default(),
            Default::default(),
            (CstType::Single.symbol(), base::Section::Start),
        )?;
        let (assigns, _) = combination::check_space(self, &mut comb)?;
        combination::assign_space(self, &mut comb, assigns, Default::default());
        combination::process_space(self, &mut comb);

        Ok(comb)
    }

    fn target_chars(&self) -> Vec<char> {
        let supplement = &self.get_config().chars.supplement;
        self.get_table()
            .keys()
            .filter(|name| !supplement.contains_key(name.as_str()))
            .chain(supplement.keys())
            .filter_map(|key| {
                let mut iter = key.chars();
                iter.next().and_then(|chr| match iter.next() {
                    Some(_) => None,
                    None => Some(chr),
                })
            })
            .collect()
    }

    fn mete_components(&self) -> Vec<String>
    where
        Self: Sized,
    {
        let supplement = &self.get_config().chars.supplement;
        self.get_table()
            .keys()
            .filter(|name| !supplement.contains_key(name.as_str()))
            .chain(supplement.keys())
            .filter_map(|name| {
                char_tree::get_comp_attrs(self, name).and_then(|attrs| match attrs.tp {
                    CstType::Single => Some(name.clone()),
                    _ => None,
                })
            })
            .collect()
    }

    fn taget_char_trees(&self) -> Vec<CharTree>
    where
        Self: Sized,
    {
        self.target_chars()
            .into_iter()
            .map(|chr| self.get_char_tree(String::from(chr)))
            .collect()
    }

    fn filter_comps_relate(&self, target: &str) -> Vec<char>
    where
        Self: Sized,
    {
        fn recursion(attrs: &CpAttrs, target: &str, service: &impl Service) -> bool {
            match attrs {
                CpAttrs {
                    tp: CstType::Single,
                    ..
                } => false,
                CpAttrs { components, .. } => components.iter().any(|c| match c {
                    Component::Char(name) => {
                        name == target
                            || recursion(
                                char_tree::get_comp_attrs(service, name)
                                    .unwrap_or(&CpAttrs::single()),
                                target,
                                service,
                            )
                    }
                    Component::Complex(attrs) => recursion(attrs, target, service),
                }),
            }
        }

        self.target_chars()
            .into_iter()
            .filter(|chr| {
                let chr = &chr.to_string();
                if chr == target {
                    true
                } else {
                    recursion(
                        char_tree::get_comp_attrs(self, chr).unwrap_or(&CpAttrs::single()),
                        target,
                        self,
                    )
                }
            })
            .collect()
    }

    fn generate_chars_data(&self, targets: Option<Vec<char>>) -> BTreeMap<char, CompTree>
    where
        Self: Sized,
    {
        let targets = targets.unwrap_or_else(|| self.target_chars());
        let length = targets.len();
        let mut char_data = BTreeMap::new();
        let mut errors = vec![];

        log::info!("目标 {} 个字符，开始生成骨架数据...", length);
        for char in targets {
            let target = self.get_char_tree(char.to_string());
            match self.get_struc_comb(target).map(|comp| comp.get_comp_tree()) {
                Ok(comp) => {
                    char_data.insert(char, comp);
                }
                Err(e) => {
                    let msg = format!("为字符`{char}`生成骨架数据时错误：{e}");
                    log::error!("{msg}");
                    errors.push(msg);
                }
            }
        }

        log::info!("{}个错误发生", errors.len());
        errors.iter().for_each(|msg| log::info!("{msg}"));

        log::info!(
            "目标{}, 成功{}, 失败{}",
            length,
            char_data.len(),
            errors.len()
        );

        char_data
    }

    fn generate_chars_data_thread(&self, targets: Option<Vec<char>>) -> BTreeMap<char, CompTree>
    where
        Self: Sized + Sync,
    {
        use std::{
            sync::{Arc, Mutex},
            thread,
        };

        const NUM: usize = 999;
        let targets = targets.unwrap_or_else(|| self.target_chars());
        if targets.len() < 4 * NUM {
            return self.generate_chars_data(Some(targets));
        }

        let length = targets.len();
        log::info!("目标 {} 个字符，开始生成骨架数据...", length);
        let targets = Arc::new(Mutex::new(targets));
        let char_data = Arc::new(Mutex::new(BTreeMap::default()));

        fn get_tasks(targets: &Arc<Mutex<Vec<char>>>) -> Vec<char> {
            let mut targets = targets.lock().unwrap();
            let len = targets.len();
            targets.split_off(len.checked_sub(NUM).unwrap_or(0))
        }

        thread::scope(|s| {
            for _ in 0..6 {
                let mut tasks = get_tasks(&targets);

                if tasks.is_empty() {
                    break;
                } else {
                    let targets = targets.clone();
                    let char_data = char_data.clone();
                    s.spawn(move || {
                        let mut rs: BTreeMap<_, _> = Default::default();
                        loop {
                            for char in tasks.iter() {
                                let target = self.get_char_tree(char.to_string());
                                match self.get_struc_comb(target).map(|comp| comp.get_comp_tree()) {
                                    Ok(comb) => {
                                        rs.insert(*char, comb);
                                    }
                                    Err(e) => log::error!("{char}: {e}"),
                                }
                            }

                            tasks = get_tasks(&targets);
                            if tasks.is_empty() {
                                char_data.lock().unwrap().append(&mut rs);
                                break;
                            }
                        }
                    });
                }
            }
        });

        let char_data = match Arc::try_unwrap(char_data) {
            Err(_) => unreachable!(),
            Ok(char_data) => char_data.into_inner().unwrap(),
        };

        log::info!(
            "目标{}, 成功{}, 失败{}",
            length,
            char_data.len(),
            length - char_data.len()
        );

        char_data
    }
}

pub use local::LocalService;

pub struct SimpleService {
    pub config: Config,
    pub strucs: fas::Strucs,
    table: CstTable,
}

impl SimpleService {
    pub fn new(table: CstTable) -> Self {
        Self {
            config: Default::default(),
            strucs: Default::default(),
            table,
        }
    }

    pub fn standard() -> Self {
        Self {
            config: Default::default(),
            strucs: Default::default(),
            table: CstTable::standard(),
        }
    }
}

impl Service for SimpleService {
    fn get_struc_proto(&self, name: &str) -> Option<&StrucProto> {
        self.strucs.get(name)
    }

    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_table(&self) -> &CstTable {
        &self.table
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{base::*, construct::CstType};

    #[test]
    fn test_target_chars() {
        use crate::construct::CpAttrs;

        let mut table = CstTable::empty();
        table.insert(
            String::from("艹"),
            CpAttrs {
                tp: CstType::Single,
                components: vec![],
            },
        );
        table.insert(
            String::from("艹字头"),
            CpAttrs {
                tp: CstType::Single,
                components: vec![],
            },
        );

        assert_eq!(table.len(), 2);

        let mut service = SimpleService::new(table);
        assert_eq!(service.target_chars(), vec!['艹']);

        service.config.chars.supplement.insert(
            String::from("艹"),
            CpAttrs {
                tp: CstType::Single,
                components: vec![],
            },
        );
        assert_eq!(service.target_chars(), vec!['艹']);
    }

    #[test]
    fn test_type_remap() {
        use serde_json::json;

        let table = json!({
            "岸": {
                "tp": "⿸",
                "components": [
                    "屵",
                    "干"
                ]
            },
            "屵": {
                "tp": "⿱",
                "components": [
                    "山",
                    "厂"
                ]
            },
            "魔": {
                "tp": "⿸",
                "components": [
                    "麻",
                    "鬼"
                ]
            },
            "麻": {
                "tp": "⿸",
                "components": [
                    "广",
                    "林"
                ]
            },
            "系": {
                "tp": "⿱",
                "components": [
                    "丿",
                    "糸"
                ]
            },
            "糸": {
                "tp": "⿱",
                "components": [
                    "幺",
                    "小"
                ]
            },
        });

        let service = SimpleService::new(serde_json::from_value(table).unwrap());
        let tree = service.get_char_tree("岸".to_string());
        assert_eq!(tree.tp, CstType::Scale(Axis::Vertical));
        assert_eq!(&tree.children[0].name, "山");
        assert_eq!(&tree.children[1].name, "⿸(厂, 干)");
        assert_eq!(tree.children[1].children.len(), 2);

        let tree = service.get_char_tree("魔".to_string());
        assert_eq!(tree.tp, CstType::Surround(DataHV::splat(Section::Start)));
        assert_eq!(&tree.children[0].name, "广");
        assert_eq!(&tree.children[1].name, "⿱(林, 鬼)");
        assert_eq!(tree.children[1].tp, CstType::Scale(Axis::Vertical));
        assert_eq!(&tree.children[1].children[0].name, "林");
        assert_eq!(&tree.children[1].children[1].name, "鬼");

        let tree = service.get_char_tree("系".to_string());
        assert_eq!(tree.tp, CstType::Scale(Axis::Vertical));
        assert_eq!(&tree.children[0].name, "丿");
        assert_eq!(&tree.children[1].name, "糸");
    }

    #[test]
    fn test_comb() {
        use crate::combination::{CompData, attrs};
        use serde_json::json;

        const OFFSET: f32 = 0.001;
        fn offset_val(v1: f32, v2: f32) -> bool {
            (v1 - v2).abs() < OFFSET
        }

        let mut service = SimpleService::new(CstTable::empty());
        let in_type = (CstType::Single.symbol(), Section::Start);

        let blank = 0.1;
        let config = json!({
            "assign": {
                "size": 1.0,
                "units": [0.1, 0.05],
                "zimian": [[2, 0.2], [5, 0.5], [8, 1.0 - 2.0 * blank]],
                "trigger": {
                    "reduce": 0.099
                },
                "visual_corr": 0.1,
            }
        });
        service.config = Config::new(config);

        service.strucs.insert(
            "t1".to_string(),
            StrucProto::from(vec![
                KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(3, 0), key_pos(3, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(4, 1)]),
            ]),
        );

        let mut comb_t1 = combination::get_comb_proto_in(
            &service,
            CharTree::new_single("t1".to_string()),
            Default::default(),
            Default::default(),
            in_type,
        )
        .unwrap();
        let (assigns, levels) = combination::check_space(&service, &mut comb_t1).unwrap();
        let offsets = comb_t1.get_white_area().unwrap();

        assert!((assigns.h - 0.4).abs() < OFFSET, "{}", assigns.h);
        assert!((assigns.v - 0.4).abs() < OFFSET, "{}", assigns.v);
        assert!(offset_val(offsets.h[0], 0.35), "{}", offsets.h[0]);
        assert!(offset_val(offsets.h[1], 0.25), "{}", offsets.h[1]);
        assert!(offset_val(offsets.v[0], 0.3), "{}", offsets.v[0]);
        assert!(offset_val(offsets.v[1], 0.3), "{}", offsets.v[1]);
        Axis::list().into_iter().for_each(|axis| {
            let length = offsets.hv_get(axis).iter().sum::<f32>() + assigns.hv_get(axis);
            assert!((length - 1.0).abs() < OFFSET, "{}", length);
        });

        assert_eq!(levels.h, levels.v);
        assert_eq!(levels.h, 0);

        combination::assign_space(&service, &mut comb_t1, assigns, Default::default());
        match &comb_t1.cdata {
            CompData::Single { assigns, .. } => {
                let mut assigns: Vec<f32> = assigns
                    .hv_get(Axis::Horizontal)
                    .iter()
                    .map(|av| av.total())
                    .collect();

                assert_eq!(comb_t1.blanks.h.map(|v| v.total()), [0.0; 2]);
                assert_eq!(comb_t1.blanks.v.map(|v| v.total()), [0.0; 2]);

                assert_eq!(assigns.len(), 3);
                assigns.dedup();
                assert_eq!(assigns.len(), 1);
                assert!((assigns[0] * 3.0 - 0.4).abs() < OFFSET, "{}", assigns[0]);
            }
            _ => unreachable!(),
        }

        // =======================================

        service.strucs.insert(
            "level2".to_string(),
            StrucProto::from(vec![
                KeyPath::from([key_pos(1, 0), key_pos(1, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(11, 1)]),
                KeyPath::from([key_pos(11, 0), key_pos(11, 2)]),
            ]),
        );

        let mut comb_level2 = combination::get_comb_proto_in(
            &service,
            CharTree::new_single("level2".to_string()),
            Default::default(),
            Default::default(),
            in_type,
        )
        .unwrap();
        let (assigns, levels) = combination::check_space(&service, &mut comb_level2).unwrap();
        let offsets = comb_level2.get_white_area().unwrap();

        assert!((assigns.h - 0.8).abs() < OFFSET, "{}", assigns.h);
        assert_eq!(offsets.h[0], offsets.h[1]);
        let length =
            offsets.hv_get(Axis::Horizontal).iter().sum::<f32>() + assigns.hv_get(Axis::Horizontal);
        assert!((length - 1.0).abs() < OFFSET, "{}", length);
        assert_eq!(levels.h, 1);
        assert_eq!(comb_level2.get_bases_length(Axis::Horizontal, false), 10);

        service
            .strucs
            .get_mut("level2")
            .unwrap()
            .attrs
            .set::<attrs::ReduceAlloc>(&DataHV::new(vec![vec![1, 1]], vec![]));
        let mut comb_level2 = combination::get_comb_proto_in(
            &service,
            CharTree::new_single("level2".to_string()),
            Default::default(),
            Default::default(),
            in_type,
        )
        .unwrap();
        let (assigns, levels) = combination::check_space(&service, &mut comb_level2).unwrap();
        let offsets = comb_level2.get_white_area().unwrap();
        assert_eq!(comb_level2.get_bases_length(Axis::Horizontal, false), 8);
        assert_eq!(levels.h, 0);
        assert!((assigns.h - 0.8).abs() < OFFSET, "{}", assigns.h);
        let length =
            offsets.hv_get(Axis::Horizontal).iter().sum::<f32>() + assigns.hv_get(Axis::Horizontal);
        assert!((length - 1.0).abs() < OFFSET, "{}", length);
    }
}
