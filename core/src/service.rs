use crate::{
    axis::*,
    component::{comb::StrucComb, struc::*},
    config::Config,
    construct::{self, Component, Components, Error},
    fas::FasFile,
};

#[derive(serde::Serialize, Clone)]
pub struct CharInfo {
    white_areas: DataHV<[f32; 2]>,
}

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

    pub fn export_combs(&self, list: &Vec<String>, path: &str) {
        use crate::construct::space::{KeyPointType, WorkPoint};

        const CHAR_BOX_PADDING: f32 = 8.;
        const AREA_LENGTH: f32 = 32.;
        const CHAR_BOX_SIZE: f32 = AREA_LENGTH + CHAR_BOX_PADDING * 2.0;
        const COLUMN: f32 = 16.;

        const NAME_HEIGHT: f32 = 20.;

        const STYLE: &str = r##"<style type="text/css">
.line{fill:none;stroke:#000000;stroke-width:2;stroke-linecap:round;stroke-linejoin:round;stroke-miterlimit:10;}
.char_box{fill:none;stroke:#DCDDDD;stroke-width:1;}
.name{fill:#000000;font-size:12px;}
</style>
"##;

        match &self.source {
            Some(source) => {
                let mut col = 0.0;
                let mut row = 0.0;
                let mut count = 0;

                let box_size = source.config.size;
                let area_length = box_size.map(|&v| v * AREA_LENGTH);
                let padding = area_length.map(|&v| (CHAR_BOX_SIZE - v) * 0.5);

                let comb_list: String = list
                    .iter()
                    .filter_map(|chr| match self.get_comb_struc(chr) {
                        Ok((comb, _)) => {
                            let paths: String = comb
                                .key_paths
                                .into_iter()
                                .filter_map(|path| {
                                    match path
                                        .points
                                        .first()
                                        .map_or(KeyPointType::Line, |ps| ps.p_type)
                                    {
                                        KeyPointType::Hide => None,
                                        _ => Some(
                                            path.points
                                                .into_iter()
                                                .map(|kp| {
                                                    format!(
                                                        "{},{} ",
                                                        ((kp.point.x)
                                                            * area_length.h
                                                            + padding.h)
                                                            + col * CHAR_BOX_SIZE,
                                                        ((kp.point.y)
                                                            * area_length.v
                                                            + padding.v)
                                                            + row * (CHAR_BOX_SIZE+NAME_HEIGHT)
                                                    )
                                                })
                                                .collect::<String>(),
                                        ),
                                    }
                                })
                                .map(|points: String| {
                                    format!("<polyline points=\"{}\" class=\"line\"/>", points)
                                })
                                .collect();

                            let offset = WorkPoint::new(col * CHAR_BOX_SIZE, row * (CHAR_BOX_SIZE+NAME_HEIGHT));
                            col += 1.0;
                            count += 1;
                            if col == COLUMN {
                                col = 0.0;
                                row += 1.0;
                            }

                            Some(format!(
                                "<g><rect class=\"char_box\" x=\"{}\" y=\"{}\" width=\"{CHAR_BOX_SIZE}\" height=\"{CHAR_BOX_SIZE}\"/>{}<text class=\"name\" x=\"{}\" y=\"{}\">{}</text></g>",
                                offset.x, offset.y, paths,
                                offset.x + (CHAR_BOX_SIZE - 12.)*0.5, offset.y + CHAR_BOX_SIZE + 14., chr
                            ))
                        }
                        Err(_) => None,
                    })
                    .collect();

                let contents = format!(
                    r##"<svg width="{}" height="{}" version="1.1" xmlns="http://www.w3.org/2000/svg">
    {STYLE}
    <text class="name" x="{}" y="{}">总计：{}</text>
    {comb_list}
</svg>
"##,
                    COLUMN * CHAR_BOX_SIZE + 200.,
                    row * (CHAR_BOX_SIZE + NAME_HEIGHT),
                    COLUMN * CHAR_BOX_SIZE + 20.,
                    CHAR_BOX_SIZE,
                    count
                );

                if let Err(e) = std::fs::write(path, contents) {
                    eprintln!("{}", e)
                }
            }
            None => {}
        }
    }

    pub fn export_all_combs(&self, size: f32, stroke_width: usize, padding: f32, list: &Vec<char>, path: &str) {
        use super::construct::space::KeyPointType;

        let style = format!(
            r##"<style type="text/css">.line{{fill:none;stroke:#000000;stroke-width:{stroke_width};stroke-linecap:round;stroke-linejoin:round;stroke-miterlimit:10;}}</style>"##
        );
        let view_size = size + 2.0 * padding;

        match &self.source {
            Some(source) => {
                let area_length = source.config.size.map(|&v| v * size);
                let padding = area_length.map(|&v| (size - v) * 0.5 + padding);

                list
                    .iter()
                    .for_each(|chr| match self.get_comb_struc(&chr.to_string()) {
                        Ok((comb, _)) => {
                            let paths: String = comb
                                .key_paths
                                .into_iter()
                                .filter_map(|path| {
                                    match path
                                        .points
                                        .first()
                                        .map_or(KeyPointType::Line, |ps| ps.p_type)
                                    {
                                        KeyPointType::Hide => None,
                                        _ => Some(
                                            path.points
                                                .into_iter()
                                                .map(|kp| {
                                                    format!(
                                                        "{},{} ",
                                                        ((kp.point.x)
                                                            * area_length.h
                                                            + padding.h),
                                                        ((kp.point.y)
                                                            * area_length.v
                                                            + padding.v)
                                                    )
                                                })
                                                .collect::<String>(),
                                        ),
                                    }
                                })
                                .map(|points: String| {
                                    format!("<polyline points=\"{}\" class=\"line\"/>", points)
                                })
                                .collect();

                            let contents = format!(
                                r##"<svg x="0" y="0" width="{view_size}" height="{view_size}" viewBox="0 0 {view_size} {view_size}" version="1.1" xmlns="http://www.w3.org/2000/svg">
    {style}
    {paths}
</svg>"##);
            
                            if let Err(e) = std::fs::write(format!("{path}/{chr}.svg"), contents) {
                                eprintln!("{}", e)
                            }
                        }
                        Err(_) => {},
                    });
            }
            None => {}
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

    pub fn get_struc_proto_all(&self) -> std::collections::BTreeMap<String, StrucProto> {
        match &self.source {
            Some(source) => source.components.clone(),
            None => Default::default(),
        }
    }

    fn gen_comb_proto(&self, name: &str) -> Result<StrucComb, Error> {
        match self.source() {
            Some(source) => Ok(combination::gen_comb_proto(
                name,
                DataHV::splat([false; 2]),
                &self.construct_table,
                &source.components,
                &source.config,
            )?),
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    fn assign_comb_space(&self, mut comb: StrucComb) -> Result<StrucComb, Error> {
        match self.source() {
            Some(source) => {
                let level = source.config.check_comb_proto(&mut comb)?;
                let char_box = comb.get_char_box();
                source
                    .config
                    .assign_comb_space(&mut comb, level, source.config.size.as_ref().zip(char_box.size().to_hv_data()).map(|(a, b)| **a * *b));

                Ok(comb)
            }
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    pub fn get_char_info(&self, name: &str) -> Result<CharInfo, Error> {
        let comb = self.gen_comb_proto(name)?;
        let config = &self.source().unwrap().config;
        let white_areas = Axis::hv_data().into_map(|axis| {
            Place::start_and_end()
                .map(|place| config.get_white_area_weight(&comb.read_edge_element(axis, place)))
        });

        Ok(CharInfo { white_areas })
    }

    pub fn get_comb_struc(&self, name: &str) -> Result<(StrucWork, Vec<String>), Error> {
        let comb = self.assign_comb_space(self.gen_comb_proto(name)?)?;
        let struc = comb.to_struc(comb.get_char_box().min);
        let names = comb.name_list();

        Ok((struc, names))
    }

    pub fn get_config(&self) -> Option<Config> {
        self.source().map(|source| source.config.clone())
    }

    pub fn set_config(&mut self, config: Config) -> bool {
        match &mut self.source {
            Some(source) => {
                self.changed = true;
                source.config = config;
                true
            }
            None => false,
        }
    }

    pub fn is_changed(&self) -> bool {
        self.source.is_some() && self.changed
    }
}
