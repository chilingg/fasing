use crate::{
    axis::*,
    component::{comb::StrucComb, struc::*},
    config::Config,
    construct::{self, Component, Components, Error},
    fas::FasFile,
};

pub mod combination {
    use super::*;

    pub fn gen_comb_proto(
        name: &str,
        in_place: DataHV<[bool; 2]>,
        table: &construct::Table,
        components: &Components,
        cfg: &Config,
    ) -> Result<StrucComb, Error> {
        let attrs = cfg
            .correction_table
            .data
            .get(name)
            .or(table.data.get(name))
            .unwrap_or(construct::Attrs::single());

        gen_comb_proto_from_attr(&name, attrs, in_place, table, components, cfg)
    }

    pub fn gen_comb_proto_from_attr(
        name: &str,
        attrs: &construct::Attrs,
        in_place: DataHV<[bool; 2]>,
        table: &construct::Table,
        components: &Components,
        cfg: &Config,
    ) -> Result<StrucComb, Error> {
        match attrs.tp {
            construct::Type::Single => match cfg.place_replace_name(&name, in_place) {
                Some(Component::Char(map_name)) if map_name != name => {
                    gen_comb_proto(map_name, in_place, table, components, cfg)
                }
                Some(Component::Complex(attrs)) => gen_comb_proto_from_attr(
                    &attrs.comps_name(),
                    attrs,
                    in_place,
                    table,
                    components,
                    cfg,
                ),
                _ => {
                    let mut proto = components
                        .get(name)
                        .cloned()
                        .ok_or(Error::Empty(name.to_string()))?;
                    proto.set_allocs_in_place(&in_place);

                    Ok(StrucComb::new_single(name.to_string(), proto))
                }
            },
            _ => Err(Error::Empty(attrs.tp.symbol().to_string())),
            // construct::Type::Scale(axis) => {
            //     let mut combs = vec![];
            //     let end = components.len();
            //     for (i, c) in attrs.components.iter().enumerate() {
            //         let mut c_in_place = in_place.clone();
            //         if i != 0 {
            //             c_in_place.hv_get_mut(axis)[0] = true;
            //         } else if i + 1 != end {
            //             c_in_place.hv_get_mut(axis)[1] = true;
            //         }
            //         let comb = match c {
            //             Component::Char(c_name) => {
            //                 gen_comb_proto(c_name, c_in_place, table, components, cfg)?
            //             }
            //             Component::Complex(c_attrs) => gen_comb_proto_from_attr(
            //                 &c_attrs.comps_name(),
            //                 c_attrs,
            //                 c_in_place,
            //                 table,
            //                 components,
            //                 cfg,
            //             )?,
            //         };
            //         combs.push(comb);
            //     }
            //     Ok(StrucComb::new_complex(name.to_string(), attrs.tp, combs))
            // }
            // construct::Type::Surround(surround_place) => {
            //     todo!()
            //     let mut in_place_0 = in_place.clone();
            //     let mut in_place_1 = in_place.clone();
            //     Axis::list().for_each(|axis| {
            //         let surround_place = *surround_place.hv_get(axis);
            //         if surround_place != Place::Start {
            //             in_place_0.hv_get_mut(axis)[0] = true;
            //             in_place_1.hv_get_mut(axis)[1] = true;
            //         }
            //         if surround_place != Place::End {
            //             in_place_0.hv_get_mut(axis)[1] = true;
            //             in_place_1.hv_get_mut(axis)[0] = true;
            //         }
            //     });

            //     let mut combs = vec![];
            //     let iter = [
            //         cfg.surround_replace_name(&attrs.components[0].name(), surround_place)
            //             .unwrap_or(&attrs.components[0]),
            //         &attrs.components[1],
            //     ]
            //     .into_iter();

            //     for c in iter {
            //         let comb = match c {
            //             Component::Char(p_name) => {
            //                 gen_comb_proto(p_name, in_place, table, components, cfg)?
            //             }
            //             Component::Complex(attrs) => gen_comb_proto_from_attr(
            //                 &attrs.comps_name(),
            //                 attrs,
            //                 in_place,
            //                 table,
            //                 components,
            //                 cfg,
            //             )?,
            //         };
            //         combs.push(comb);
            //     }
            //     Ok(StrucComb::new_complex(name.to_string(), attrs.tp, combs))
            // }
        }
    }
}

pub struct LocalService {
    changed: bool,
    source: Option<FasFile>,
    pub construct_table: construct::Table,
}

impl LocalService {
    pub fn new() -> Self {
        Self {
            changed: false,
            source: None,
            construct_table: construct::Table::default(),
        }
    }

    pub fn save(&mut self, path: &str) -> Result<(), std::io::Error> {
        match &self.source {
            Some(source) => match source.save_pretty(path) {
                Ok(_) => {
                    self.changed = false;
                    Ok(())
                }
                Err(e) => Err(e),
            },
            None => Ok(()),
        }
    }

    pub fn save_struc(&mut self, name: String, struc: StrucProto) {
        if let Some(source) = &mut self.source {
            source.components.insert(name, struc);
            self.changed = true;
        }
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        match FasFile::from_file(path) {
            Ok(data) => {
                self.source = Some(data);
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

    pub fn get_struc_comb(&self, name: &str) -> Result<(StrucWork, Vec<String>), Error> {
        match self.source() {
            Some(source) => {
                let mut comb = combination::gen_comb_proto(
                    name,
                    DataHV::splat([false; 2]),
                    &self.construct_table,
                    &source.components,
                    &source.config,
                )?;
                let level = source.config.check_comb_proto(&mut comb)?;
                let char_box = comb.get_char_box();
                source
                    .config
                    .assign_comb_space(&mut comb, level, char_box.size().to_hv_data());

                let struc = comb.to_struc(char_box.min);
                let names = comb.name_list();

                Ok((struc, names))
            }
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    pub fn get_config(&self) -> Option<Config> {
        self.source().map(|source| source.config.clone())
    }

    pub fn is_changed(&self) -> bool {
        self.source.is_some() && self.changed
    }
}
