use crate::{
    axis::*,
    component::{comb::StrucComb, struc::*},
    config::Config,
    construct::{self, Component, Components, Error},
    fas::FasFile,
};

pub use crate::config::CharInfo;

pub mod combination {
    use super::*;

    fn check_name(
        name: &str,
        tp: construct::Type,
        in_tp: Place,
        in_place: DataHV<[bool;2]>,
        cfg: &Config,
    ) -> Option<Component> {
        let r = if let Some(mut map_comp) = cfg.type_replace_name(name, tp, in_tp) {
            while let Some(mc) = cfg.type_replace_name(&map_comp.name(), tp, in_tp) {
                map_comp = mc
            }
            Some(map_comp)
        } else {
            None
        };

        if let Some(mc) = &r {
            cfg.place_replace_name(&mc.name(), in_place).or(r)
        } else {
            cfg.place_replace_name(name, in_place)
        }
    }

    fn get_current_attr(
        name: String,
        tp: construct::Type,
        in_tp: Place,
        in_place: DataHV<[bool;2]>,
        table: &construct::Table,
        cfg: &Config,
    ) -> (String, construct::Attrs) {
        let comp = check_name(&name, tp, in_tp,in_place, cfg).unwrap_or(Component::Char(name));
        
        match comp {
            Component::Char(c_name) => {
                let attrs = cfg
                    .correction_table
                    .get(&c_name)
                    .or(table.data.get(&c_name))
                    .unwrap_or(construct::Attrs::single())
                    .clone();
                (c_name, attrs)
            }
            Component::Complex(attrs) => (attrs.comps_name(), attrs)
        }
    }

    pub fn gen_comb_proto(
        name: String,
        tp: construct::Type,
        in_tp: Place,
        in_place: DataHV<[bool; 2]>,
        table: &construct::Table,
        components: &Components,
        cfg: &Config,
    ) -> Result<StrucComb, Error> {
        let (name, attrs) = get_current_attr(name, tp, in_tp, in_place, table, cfg);
        gen_comb_proto_from_attr(&name, &attrs, in_place, table, components, cfg)
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
            construct::Type::Single => {
                let mut proto = match components
                    .get(name) {
                        Some(proto) => proto.clone(),
                        None => {
                            eprintln!("Missing component {}", name);
                            return Err(Error::Empty(name.to_string()))
                        }
                    };
                proto.set_allocs_in_place(&in_place);

                Ok(StrucComb::new_single(name.to_string(), proto))
            }
            construct::Type::Scale(axis) => {
                let mut combs = vec![];
                let end = attrs.components.len();
                for (i, c) in attrs.components.iter().enumerate() {
                    let mut c_in_place = in_place.clone();
                    if i != 0 {
                        c_in_place.hv_get_mut(axis)[0] = true;
                    }
                    if i + 1 != end {
                        c_in_place.hv_get_mut(axis)[1] = true;
                    }
                    let in_tp = match i {
                        0 => Place::Start,
                        n if n + 1 == end => Place::End,
                        _ => Place::Mind,
                    };

                    let comb = match c {
                        Component::Char(c_name) => {
                            gen_comb_proto(c_name.to_string(), attrs.tp, in_tp, c_in_place, table, components, cfg)?
                        }
                        Component::Complex(c_attrs) => {
                            match check_name(&c_attrs.comps_name(), attrs.tp, in_tp, in_place, cfg) {
                                Some(Component::Char(map_name)) => gen_comb_proto(map_name, attrs.tp, in_tp, c_in_place, table, components, cfg),
                                Some(Component::Complex(map_attrs)) => gen_comb_proto_from_attr(
                                    &map_attrs.comps_name(),
                                    c_attrs,
                                    c_in_place,
                                    table,
                                    components,
                                    cfg,
                                ),
                                None => gen_comb_proto_from_attr(
                                    &c_attrs.comps_name(),
                                    c_attrs,
                                    c_in_place,
                                    table,
                                    components,
                                    cfg,
                                )
                            }?
                        },
                    };
                    combs.push(comb);
                }
                Ok(StrucComb::new_complex(name.to_string(), attrs.tp, combs))
            }
            construct::Type::Surround(surround_place) => {
                fn remap_comp(comp: &Component, tp: construct::Type, in_tp: Place, in_place: DataHV<[bool;2]>, table: &construct::Table, cfg: &Config) -> (String, construct::Attrs) {
                    match comp {
                        Component::Char(c_name) => get_current_attr(c_name.to_string(), tp, in_tp, in_place, table, cfg),
                        Component::Complex(c_attrs) => match check_name(&c_attrs.comps_name(), tp, in_tp, in_place, cfg) {
                            Some(Component::Char(map_name)) => get_current_attr(map_name, tp, in_tp, in_place, table, cfg),
                            Some(Component::Complex(map_attrs)) => (map_attrs.comps_name(), map_attrs),
                            None => (c_attrs.comps_name(), c_attrs.clone())
                        }
                    }
                }

                let mut primary: (String, construct::Attrs) = remap_comp(&attrs.components[0], attrs.tp, Place::Start, in_place, table, cfg);
                match primary.1.tp {
                    construct::Type::Scale(c_axis) => {
                        let index = match surround_place.hv_get(c_axis) {
                            Place::Start => primary.1.components.len() - 1,
                            Place::End => 0,
                            Place::Mind => return Err(Error::Empty(format!("{} is surround component in {}", primary.1.tp.symbol(), construct::Type::Surround(surround_place).symbol()))),
                        };
                        primary.1.components[index] = Component::Complex(construct::Attrs { tp: construct::Type::Surround(surround_place), components: vec![primary.1.components[index].clone(), attrs.components[1].clone()] });
                        gen_comb_proto_from_attr(name, &primary.1, in_place, table, components, cfg)
                    }
                    construct::Type::Surround(c_surround) => {
                        if c_surround == surround_place {
                            let sc1 = primary.1.components.pop().unwrap();
                            let pc = primary.1.components.pop().unwrap();
                            let sc = if c_surround.v == Place::End {
                                vec![attrs.components[1].clone(), sc1]
                            } else {
                                vec![sc1, attrs.components[1].clone()]
                            };
                            let new_attrs = construct::Attrs { tp: attrs.tp, components: vec![
                                pc,
                                Component::Complex(construct::Attrs { tp: construct::Type::Scale(Axis::Vertical), components: sc })
                            ] };
                            gen_comb_proto_from_attr(name, &new_attrs, in_place, table, components, cfg)
                        } else {
                            Err(Error::Empty(format!("{} is surround component in {}", primary.1.tp.symbol(), construct::Type::Surround(surround_place).symbol())))
                        }
                    }
                    construct::Type::Single => {
                        let mut in_place = [in_place.clone();2];
                        Axis::list().into_iter().for_each(|axis| {
                            let surround_place = *surround_place.hv_get(axis);
                            if surround_place == Place::Mind {
                                *in_place[1].hv_get_mut(axis) = [true, true];
                            }
                        });
                        let secondery = remap_comp(&attrs.components[1], attrs.tp, Place::End, in_place[1], table, cfg);

                        Ok(StrucComb::new_complex(name.to_string(), attrs.tp, vec![
                            gen_comb_proto_from_attr(&primary.0, &primary.1, in_place[0], table, components, cfg)?,
                            gen_comb_proto_from_attr(&secondery.0, &secondery.1, in_place[1], table, components, cfg)?,
                        ]))
                    }
                }
            }
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

        const CHAR_BOX_PADDING: f32 = 0.;
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

                let area_length = source.config.size.map(|&v| v * AREA_LENGTH);
                let padding = area_length.map(|&v| (CHAR_BOX_SIZE - v) * 0.5);

                let comb_list: String = list
                    .iter()
                    .filter_map(|chr| match self.get_comb_struc(chr.clone()) {
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
                                                            * AREA_LENGTH
                                                            + padding.h)
                                                            + col * CHAR_BOX_SIZE,
                                                        ((kp.point.y)
                                                            * AREA_LENGTH
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

    pub fn export_comb_datas(&self, list: &Vec<char>, path: &str) {
        if self.source.is_some() {
            let mut data = serde_json::Map::new();
            let mut error_list = vec![];

            list.iter().for_each(|chr| match self.gen_comb_proto(chr.to_string()).and_then(|comb| {
                let mut info = CharInfo::default();
                self.assign_comb_space(comb, Some(&mut info)).and_then(|comb| Ok((comb, info)))
            }) {
                Ok((comb, info)) => {
                    let mut attrs = serde_json::Map::new();
                    let min_vals = info.level.zip(self.get_config().as_ref().unwrap().min_values.as_ref()).into_map(|(l, list)| {
                        *list.get(l).unwrap_or(&Config::DEFAULT_MIN_VALUE)
                    });
                    attrs.insert("comb".to_string(), serde_json::to_value(comb.to_struc(comb.get_char_box().min, min_vals)).unwrap());
                    attrs.insert("info".to_string(), serde_json::to_value(info).unwrap());
                    data.insert(chr.to_string(), serde_json::Value::Object(attrs));
                }
                Err(e) => error_list.push(format!("{}: {}", chr, e.to_string()))
            });

            if !error_list.is_empty() {
                error_list.into_iter().for_each(|e| eprintln!("{}",e));
            }
            
            if let Err(e) = std::fs::write(path, serde_json::to_string(&data).unwrap()) {
                eprintln!("{}", e)
            }
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
                    .for_each(|chr| match self.get_comb_struc(chr.to_string()) {
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
                                                            * size
                                                            + padding.h),
                                                        ((kp.point.y)
                                                            * size
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
            
                            let filename = if chr.is_ascii() {
                                format!("{}", *chr as u32)
                            } else {
                                chr.to_string()
                            };
                            if let Err(e) = std::fs::write(format!("{path}/{filename}.svg"), contents) {
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

    fn gen_comb_proto(&self, name: String) -> Result<StrucComb, Error> {
        match self.source() {
            Some(source) => Ok(combination::gen_comb_proto(
                name,
                construct::Type::Single,
                Place::Start,
                DataHV::splat([false; 2]),
                &self.construct_table,
                &source.components,
                &source.config,
            )?),
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    fn assign_comb_space(&self, mut comb: StrucComb, char_info: Option<&mut CharInfo>) -> Result<StrucComb, Error> {
        match self.source() {
            Some(source) => {
                let char_box = comb.get_char_box();
                let info = source.config.expand_comb_proto(
                    &mut comb,
                    source.config.size.as_ref().zip(char_box.size().to_hv_data()).into_map(|(a, b)| *a * b)
                )?;
                if let Some(char_info) = char_info {
                    *char_info = info;
                }
                
                Ok(comb)
            }
            None => Err(Error::Empty("Source".to_string())),
        }
    }

    pub fn get_char_info(&self, name: String) -> Result<CharInfo, Error> {
        let mut info = CharInfo::default();
        self.assign_comb_space(self.gen_comb_proto(name)?, Some(&mut info))?;
        Ok(info)
    }

    pub fn get_comb_struc(&self, name: String) -> Result<(StrucWork, Vec<String>), Error> {
        let mut info = CharInfo::default();
        let comb = self.assign_comb_space(self.gen_comb_proto(name)?, Some(&mut info))?;
        let min_vals = info.level.zip(self.get_config().as_ref().unwrap().min_values.as_ref()).into_map(|(l, list)| {
            *list.get(l).unwrap_or(&Config::DEFAULT_MIN_VALUE)
        });
        let struc = comb.to_struc(comb.get_char_box().min, min_vals);
        let names = comb.name_list();

        Ok((struc, names))
    }

    pub fn get_comb_name_list(&self, name: String) -> Result<Vec<String>, Error> {
        let comb = self.assign_comb_space(self.gen_comb_proto(name)?, None)?;
        Ok(comb.name_list())
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
