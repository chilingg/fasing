use crate::{
    axis::*,
    component::{
        comb::{CharInfo, StrucComb},
        struc::*,
    },
    config::Config,
    construct::{self, space::*, CharTree, Component, CpAttrs, CstError, CstTable, CstType},
    fas::FasFile,
};

pub mod combination {
    use super::*;

    fn check_name(
        name: &str,
        tp: CstType,
        in_tp: Place,
        adjacency: DataHV<[bool; 2]>,
        cfg: &Config,
    ) -> Option<Component> {
        match cfg.type_replace_name(name, tp, in_tp) {
            Some(comp) => cfg
                .place_replace_name(&comp.name(), adjacency)
                .or(Some(comp)),
            None => cfg.place_replace_name(name, adjacency),
        }
    }

    fn get_current_attr(
        name: String,
        tp: CstType,
        in_tp: Place,
        adjacency: DataHV<[bool; 2]>,
        table: &CstTable,
        cfg: &Config,
    ) -> (String, CpAttrs) {
        let comp = check_name(&name, tp, in_tp, adjacency, cfg).unwrap_or(Component::Char(name));

        match comp {
            Component::Char(c_name) => {
                let attrs = cfg
                    .supplement
                    .get(&c_name)
                    .or(table.get(&c_name))
                    .cloned()
                    .unwrap_or(CpAttrs::single());
                (c_name, attrs)
            }
            Component::Complex(attrs) => (attrs.comps_name(), attrs),
        }
    }

    pub fn get_char_tree(name: String, table: &CstTable, cfg: &Config) -> CharTree {
        get_component_in(
            name,
            CstType::Single,
            Place::Start,
            Default::default(),
            table,
            cfg,
        )
    }

    fn get_component_in(
        name: String,
        tp: CstType,
        in_tp: Place,
        adjacency: DataHV<[bool; 2]>,
        table: &CstTable,
        cfg: &Config,
    ) -> CharTree {
        let (name, attrs) = get_current_attr(name, tp, in_tp, adjacency, table, cfg);
        get_component_from_attr(name, attrs, adjacency, table, cfg)
    }

    fn get_component_from_attr(
        name: String,
        attrs: CpAttrs,
        adjacency: DataHV<[bool; 2]>,
        table: &CstTable,
        cfg: &Config,
    ) -> CharTree {
        match attrs.tp {
            CstType::Single => CharTree::new_single(name),
            CstType::Scale(axis) => {
                let end = attrs.components.len();
                let children = attrs
                    .components
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let mut c_in_place = adjacency.clone();
                        if i != 0 {
                            c_in_place.hv_get_mut(axis)[0] = true;
                        }
                        if i + 1 != end {
                            c_in_place.hv_get_mut(axis)[1] = true;
                        }
                        let in_tp = match i {
                            0 => Place::Start,
                            n if n + 1 == end => Place::End,
                            _ => Place::Middle,
                        };
                        match c {
                            Component::Char(c_name) => get_component_in(
                                c_name.to_string(),
                                attrs.tp,
                                in_tp,
                                c_in_place,
                                table,
                                cfg,
                            ),
                            Component::Complex(c_attrs) => match check_name(
                                &c_attrs.comps_name(),
                                attrs.tp,
                                in_tp,
                                adjacency,
                                cfg,
                            ) {
                                Some(Component::Char(map_name)) => get_component_in(
                                    map_name, attrs.tp, in_tp, c_in_place, table, cfg,
                                ),
                                Some(Component::Complex(map_attrs)) => get_component_from_attr(
                                    map_attrs.comps_name(),
                                    c_attrs,
                                    c_in_place,
                                    table,
                                    cfg,
                                ),
                                None => get_component_from_attr(
                                    c_attrs.comps_name(),
                                    c_attrs,
                                    c_in_place,
                                    table,
                                    cfg,
                                ),
                            },
                        }
                    })
                    .collect();
                CharTree {
                    name,
                    tp: attrs.tp,
                    children,
                }
            }
            CstType::Surround(surround_place) => {
                fn remap_comp(
                    comp: &Component,
                    tp: CstType,
                    in_tp: Place,
                    in_place: DataHV<[bool; 2]>,
                    table: &CstTable,
                    cfg: &Config,
                ) -> (String, CpAttrs) {
                    match comp {
                        Component::Char(c_name) => {
                            get_current_attr(c_name.to_string(), tp, in_tp, in_place, table, cfg)
                        }
                        Component::Complex(c_attrs) => {
                            match check_name(&c_attrs.comps_name(), tp, in_tp, in_place, cfg) {
                                Some(Component::Char(map_name)) => {
                                    get_current_attr(map_name, tp, in_tp, in_place, table, cfg)
                                }
                                Some(Component::Complex(map_attrs)) => {
                                    (map_attrs.comps_name(), map_attrs)
                                }
                                None => (c_attrs.comps_name(), c_attrs.clone()),
                            }
                        }
                    }
                }

                let mut primary: (String, CpAttrs) = remap_comp(
                    &attrs.components[0],
                    attrs.tp,
                    Place::Start,
                    adjacency,
                    table,
                    cfg,
                );
                match primary.1.tp {
                    CstType::Scale(c_axis) => {
                        let index = match surround_place.hv_get(c_axis) {
                            Place::Start => primary.1.components.len() - 1,
                            Place::End => 0,
                            Place::Middle => panic!(
                                "{} is surround component in {}",
                                primary.1.tp.symbol(),
                                CstType::Surround(surround_place).symbol()
                            ),
                        };
                        primary.1.components[index] = Component::Complex(CpAttrs {
                            tp: CstType::Surround(surround_place),
                            components: vec![
                                primary.1.components[index].clone(),
                                attrs.components[1].clone(),
                            ],
                        });
                        get_component_from_attr(name, primary.1, adjacency, table, cfg)
                    }
                    CstType::Surround(c_surround) => {
                        if c_surround == surround_place {
                            let sc1 = primary.1.components.pop().unwrap();
                            let pc = primary.1.components.pop().unwrap();
                            let sc = if c_surround.v == Place::End {
                                vec![attrs.components[1].clone(), sc1]
                            } else {
                                vec![sc1, attrs.components[1].clone()]
                            };
                            let new_attrs = CpAttrs {
                                tp: attrs.tp,
                                components: vec![
                                    pc,
                                    Component::Complex(CpAttrs {
                                        tp: CstType::Scale(Axis::Vertical),
                                        components: sc,
                                    }),
                                ],
                            };
                            get_component_from_attr(name, new_attrs, adjacency, table, cfg)
                        } else {
                            panic!(
                                "{} is surround component in {}",
                                primary.1.tp.symbol(),
                                CstType::Surround(surround_place).symbol()
                            )
                        }
                    }
                    CstType::Single => {
                        let mut in_place = [adjacency.clone(); 2];
                        Axis::list().into_iter().for_each(|axis| {
                            let surround_place = *surround_place.hv_get(axis);
                            if surround_place != Place::End {
                                in_place[0].hv_get_mut(axis)[1] = true;
                                in_place[1].hv_get_mut(axis)[0] = true;
                            }
                            if surround_place != Place::Start {
                                in_place[0].hv_get_mut(axis)[0] = true;
                                in_place[1].hv_get_mut(axis)[1] = true;
                            }
                        });
                        let secondery = remap_comp(
                            &attrs.components[1],
                            attrs.tp,
                            Place::End,
                            in_place[1],
                            table,
                            cfg,
                        );

                        CharTree {
                            name,
                            tp: attrs.tp,
                            children: vec![
                                get_component_from_attr(
                                    primary.0,
                                    primary.1,
                                    in_place[0],
                                    table,
                                    cfg,
                                ),
                                get_component_from_attr(
                                    secondery.0,
                                    secondery.1,
                                    in_place[1],
                                    table,
                                    cfg,
                                ),
                            ],
                        }
                    }
                }
            }
        }
    }

    pub fn gen_comb_proto(
        target: CharTree,
        table: &CstTable,
        fas: &FasFile,
    ) -> Result<StrucComb, CstError> {
        gen_comb_proto_in(target, DataHV::splat([false; 2]), table, fas)
    }

    fn gen_comb_proto_in(
        mut target: CharTree,
        adjacency: DataHV<[bool; 2]>,
        table: &CstTable,
        fas: &FasFile,
    ) -> Result<StrucComb, CstError> {
        let components = &fas.components;
        match target.tp {
            CstType::Single => {
                let mut proto = match components.get(&target.name) {
                    Some(proto) if !proto.paths.is_empty() => proto.clone(),
                    _ => {
                        return Err(CstError::Empty(target.name));
                    }
                };

                proto.set_allocs_in_adjacency(adjacency);

                Ok(StrucComb::new_single(target.name, proto))
            }
            CstType::Scale(axis) => {
                let mut combs = vec![];
                let children = target.children;

                let end = children.len();
                for (i, c_target) in children.into_iter().enumerate() {
                    let mut c_in_place = adjacency.clone();
                    if i != 0 {
                        c_in_place.hv_get_mut(axis)[0] = true;
                    }
                    if i + 1 != end {
                        c_in_place.hv_get_mut(axis)[1] = true;
                    }

                    combs.push(gen_comb_proto_in(c_target, c_in_place, table, fas)?);
                }
                Ok(StrucComb::new_complex(target.name, target.tp, combs))
            }
            CstType::Surround(surround_place) => {
                let mut in_place = [adjacency.clone(); 2];
                Axis::list().into_iter().for_each(|axis| {
                    let surround_place = *surround_place.hv_get(axis);
                    if surround_place != Place::End {
                        in_place[0].hv_get_mut(axis)[1] = true;
                        in_place[1].hv_get_mut(axis)[0] = true;
                    }
                    if surround_place != Place::Start {
                        in_place[0].hv_get_mut(axis)[0] = true;
                        in_place[1].hv_get_mut(axis)[1] = true;
                    }
                });

                let secondery = target.children.pop().unwrap();
                let primary = target.children.pop().unwrap();
                Ok(StrucComb::new_complex(
                    target.name,
                    target.tp,
                    vec![
                        gen_comb_proto_in(primary, in_place[0], table, fas)?,
                        gen_comb_proto_in(secondery, in_place[1], table, fas)?,
                    ],
                ))
            }
        }
    }
}

pub struct LocalService {
    changed: bool,
    source: Option<FasFile>,
    pub construct_table: construct::CstTable,
}

impl LocalService {
    pub fn new() -> Self {
        Self {
            changed: false,
            source: None,
            construct_table: construct::CstTable::default(),
        }
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
            source.components.insert(name, struc);
            self.changed = true;
        }
    }

    pub fn set_config(&mut self, cfg: Config) {
        if let Some(source) = &mut self.source {
            source.config = cfg;
            self.changed = true;
        }
    }

    pub fn load_fas(&mut self, data: FasFile) {
        data.config.supplement.iter().for_each(|(ch, attr)| {
            self.construct_table.insert(ch.to_string(), attr.clone());
        });
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

    pub fn source(&self) -> Option<&FasFile> {
        self.source.as_ref()
    }

    pub fn get_struc_proto(&self, name: &str) -> StrucProto {
        match &self.source {
            Some(source) => source.components.get(name).cloned().unwrap_or_default(),
            None => Default::default(),
        }
    }

    pub fn gen_char_tree(&self, name: String) -> CharTree {
        combination::get_char_tree(
            name,
            &self.construct_table,
            self.source()
                .map(|s| &s.config)
                .unwrap_or(&Default::default()),
        )
    }

    pub fn gen_comp_visible_path(
        &self,
        target: CharTree,
    ) -> Result<(Vec<Vec<WorkPoint>>, CharTree), CstError> {
        match self.source() {
            Some(source) => {
                let mut comb = combination::gen_comb_proto(target, &self.construct_table, &source)?;
                comb.expand_comb_proto(&source, false)?;
                let paths = comb
                    .to_paths()
                    .into_iter()
                    .filter_map(|path| match path.hide {
                        true => None,
                        false => Some(path.points),
                    })
                    .collect();
                let tree = comb.get_char_tree();

                Ok((paths, tree))
            }
            None => Err(CstError::Empty("Source".to_string())),
        }
    }

    pub fn gen_char_info(&self, name: String) -> Result<CharInfo, CstError> {
        match self.source() {
            Some(source) => {
                let target = self.gen_char_tree(name);
                let comp_name = target.get_comb_name();

                let info = combination::gen_comb_proto(target, &self.construct_table, &source)
                    .map(|mut comb| match comb.expand_comb_proto(&source, true) {
                        Ok(info) => info.unwrap(),
                        Err(_) => {
                            let mut info = CharInfo::default();
                            info.comb_name = comb.get_comb_name();
                            info
                        }
                    })
                    .unwrap_or_else(|_| {
                        let mut info = CharInfo::default();
                        info.comb_name = comp_name;
                        info
                    });

                Ok(info)
            }
            None => Err(CstError::Empty("Source".to_string())),
        }
    }
}

mod tests {
    #[test]
    fn test_correction_table() {
        use super::*;

        let mut data = FasFile::default();
        let attr = CpAttrs {
            tp: CstType::Scale(Axis::Horizontal),
            components: vec![
                Component::Char("一".to_string()),
                Component::Char("一".to_string()),
            ],
        };
        data.config
            .supplement
            .insert(String::from("二"), attr.clone());

        let mut service = LocalService::new();
        service.load_fas(data);

        let cur_attr = &service.construct_table["二"];
        assert_eq!(cur_attr.tp, attr.tp);
    }
}
