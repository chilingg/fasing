mod algorithm;
mod combination;
pub mod fas;
pub mod local;

use crate::{
    combination::{StrucComb, StrucProto},
    config::Config,
    construct::{CharTree, CstError, CstTable},
};

pub type Strucs = std::collections::BTreeMap<String, StrucProto>;

pub trait Service {
    fn get_table(&self) -> &CstTable;
    fn get_config(&self) -> &Config;
    fn get_strucs(&self) -> &Strucs;

    fn get_struc_proto(&self, name: &str) -> Option<&StrucProto> {
        self.get_strucs().get(name)
    }

    fn get_char_tree(&self, name: String) -> CharTree
    where
        Self: Sized,
    {
        combination::get_char_tree(self, name)
    }

    fn get_struc_comb(&self, target: CharTree) -> Result<StrucComb, CstError>
    where
        Self: Sized,
    {
        let mut comb = combination::get_comb_proto_in(self, target, Default::default())?;
        let (assigns, offsets, levels) = combination::check_space(self, &mut comb)?;
        combination::assign_space(self, &mut comb, assigns, offsets, levels);
        combination::process_space(self, &mut comb);

        Ok(comb)
    }

    fn target_chars(&self) -> Vec<char> {
        self.get_table()
            .keys()
            .chain(self.get_config().supplement.keys())
            .filter_map(|key| {
                let mut iter = key.chars();
                iter.next().and_then(|chr| match iter.next() {
                    Some(_) => None,
                    None => Some(chr),
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
}

pub use local::LocalService;

pub struct SimpleService {
    pub config: Config,
    pub strucs: Strucs,
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
    fn get_strucs(&self) -> &Strucs {
        &self.strucs
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
    fn test_tartget_chars() {
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

        let service = SimpleService::new(table);
        assert_eq!(service.target_chars(), vec!['艹']);
    }

    #[test]
    fn test_surround_remap() {
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
            }
        });

        let service = SimpleService::new(serde_json::from_value(table).unwrap());
        let tree = service.get_char_tree("岸".to_string());
        assert_eq!(tree.tp, CstType::Scale(Axis::Vertical));
        assert_eq!(&tree.children[0].name, "山");
        assert_eq!(&tree.children[1].name, "⿸(厂+干)");
        assert_eq!(tree.children[1].children.len(), 2);

        let tree = service.get_char_tree("魔".to_string());
        assert_eq!(tree.tp, CstType::Surround(DataHV::splat(Place::Start)));
        assert_eq!(&tree.children[0].name, "广");
        assert_eq!(&tree.children[1].name, "⿱(林+鬼)");
        assert_eq!(tree.children[1].tp, CstType::Scale(Axis::Vertical));
        assert_eq!(&tree.children[1].children[0].name, "林");
        assert_eq!(&tree.children[1].children[1].name, "鬼");
    }

    #[test]
    fn test_comb() {
        use crate::combination::{CompData, attrs};
        use serde_json::json;

        const OFFSET: f32 = 0.001;

        let mut service = SimpleService::new(CstTable::empty());

        let blank = 0.1;
        let config = json!({
            "size": 1.0,
            "min_val": [0.1, 0.05],
            "zimian": [[2, 0.2], [5, 0.5], [8, 1.0 - 2.0 * blank]],
            "reduce_trigger": 0.099,
            "visual_corr": 0.1,
        });
        service.config = serde_json::from_value(config).unwrap();

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
        )
        .unwrap();
        let (assigns, offsets, levels) = combination::check_space(&service, &mut comb_t1).unwrap();

        assert!((assigns.h - 0.3).abs() < OFFSET, "{}", assigns.h);
        assert!((assigns.v - 0.2).abs() < OFFSET, "{}", assigns.v);
        offsets.into_iter().flatten().for_each(|av| {
            assert!((av.base - blank).abs() < OFFSET, "{}", av.base);
        });
        assert_eq!(offsets.h[0].excess, offsets.h[1].excess);
        assert_eq!(offsets.v[0].excess, offsets.v[1].excess);
        Axis::list().into_iter().for_each(|axis| {
            let length = offsets
                .hv_get(axis)
                .iter()
                .map(|av| av.total())
                .sum::<f32>()
                + assigns.hv_get(axis);
            assert!((length - 1.0).abs() < OFFSET, "{}", length);
        });

        assert_eq!(levels.h, levels.v);
        assert_eq!(levels.h, 0);

        combination::assign_space(&service, &mut comb_t1, assigns, offsets, levels);
        match &comb_t1.cdata {
            CompData::Single { assigns, .. } => {
                let mut assigns: Vec<f32> = assigns
                    .hv_get(Axis::Horizontal)
                    .iter()
                    .map(|av| av.total())
                    .collect();

                assert!(
                    (comb_t1.offsets.h[0].total() - 0.35).abs() < OFFSET,
                    "{}",
                    comb_t1.offsets.h[0].total()
                );
                assert!(
                    (comb_t1.offsets.h[1].total() - 0.25).abs() < OFFSET,
                    "{}",
                    comb_t1.offsets.h[1].total()
                );

                assert_eq!(assigns.len(), 3);
                assigns.dedup();
                assert_eq!(assigns.len(), 1);
                assert!((assigns[0] * 3.0 - 0.4).abs() < OFFSET, "{}", assigns[0]);
            }
            CompData::Complex { .. } => unreachable!(),
        }

        // =======================================

        service.strucs.insert(
            "level2".to_string(),
            StrucProto::from(vec![
                KeyPath::from([key_pos(2, 0), key_pos(2, 2)]),
                KeyPath::from([key_pos(1, 1), key_pos(11, 1)]),
            ]),
        );

        let mut comb_level2 = combination::get_comb_proto_in(
            &service,
            CharTree::new_single("level2".to_string()),
            Default::default(),
        )
        .unwrap();
        let (assigns, offsets, levels) =
            combination::check_space(&service, &mut comb_level2).unwrap();

        assert!((assigns.h - 0.8).abs() < OFFSET, "{}", assigns.h);
        offsets.h.iter().for_each(|av| {
            assert!((av.base - blank).abs() < OFFSET, "{}", av.base);
        });
        assert_eq!(offsets.h[0].excess, offsets.h[1].excess);
        let length = offsets
            .hv_get(Axis::Horizontal)
            .iter()
            .map(|av| av.total())
            .sum::<f32>()
            + assigns.hv_get(Axis::Horizontal);
        assert!((length - 1.0).abs() < OFFSET, "{}", length);
        assert_eq!(levels.h, 1);
        assert_eq!(comb_level2.get_bases_length(Axis::Horizontal), 10);

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
        )
        .unwrap();
        let (assigns, offsets, levels) =
            combination::check_space(&service, &mut comb_level2).unwrap();
        assert_eq!(comb_level2.get_bases_length(Axis::Horizontal), 8);
        assert_eq!(levels.h, 0);
        assert!((assigns.h - 0.8).abs() < OFFSET, "{}", assigns.h);
        let length = offsets
            .hv_get(Axis::Horizontal)
            .iter()
            .map(|av| av.total())
            .sum::<f32>()
            + assigns.hv_get(Axis::Horizontal);
        assert!((length - 1.0).abs() < OFFSET, "{}", length);
    }
}
