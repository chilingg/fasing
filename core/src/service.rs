use crate::{
    construct,
    fas_file::{self, Error, FasFile, StrokeReplace},
    hv::*,
    struc::{
        space::{WorkPoint, WorkRect, WorkSize},
        StrokePath, StrucComb, StrucProto, StrucWork, TransformValue,
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
    intervals: Vec<f32>,
    intervals_attr: Vec<String>,
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
                intervals: vec![],
                intervals_attr: vec![],
            },
            StrucComb::Complex {
                name,
                format,
                comps,
                limit,
                intervals,
                ..
            } => CombInfos {
                name: name.clone(),
                format: *format,
                limit: limit.clone(),
                trans: None,
                comps: comps.iter().map(|comb| CombInfos::new(comb)).collect(),
                intervals: intervals.clone(),
                intervals_attr: StrucComb::read_connect(comps, *format),
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
    const EMPTY_LIST: DataHV<Vec<f32>> = DataHV {
        h: vec![],
        v: vec![],
    };

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
                let components = &source.components;
                let config = &source.config;

                let mut comb = StrucComb::new(
                    name.to_string(),
                    const_table,
                    // alloc_table,
                    components,
                    config,
                )?;

                let mut size = WorkSize::splat(1.0);
                match &comb {
                    StrucComb::Single { cache, .. } => {
                        if cache.proto.tags.contains("left") || cache.proto.tags.contains("right") {
                            size.width *= 0.5;
                        } else if cache.proto.tags.contains("top")
                            || cache.proto.tags.contains("bottom")
                        {
                            size.height *= 0.5;
                        }
                    }
                    _ => {}
                }

                let trans_value = comb.allocation(size, config, Default::default())?;
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
        let (comb, _) = self.get_comb_and_trans(name)?;

        let axis_length: Vec<f32> = Axis::list().map(|axis| comb.axis_length(axis)).collect();
        let offset = WorkPoint::new(
            match axis_length[0] == 0.0 {
                true => 0.5,
                false => 0.5 - axis_length[0] * 0.5,
            },
            match axis_length[1] == 0.0 {
                true => 0.5,
                false => 0.5 - axis_length[1] * 0.5,
            },
        );

        Ok(comb.to_work(
            offset,
            WorkRect::new(WorkPoint::origin(), WorkSize::splat(1.0)),
            self.source()
                .map(|source| &source.config.min_values)
                .unwrap_or(&Self::EMPTY_LIST),
        ))
    }

    pub fn get_skeleton(&self, name: char) -> Result<Vec<StrokePath>, Error> {
        const EMPTY_STROKE_MATCHS: Vec<StrokeReplace> = vec![];

        let (comb, trans) = self.get_comb_and_trans(name)?;

        let axis_length: Vec<f32> = Axis::list().map(|axis| comb.axis_length(axis)).collect();
        let offset = WorkPoint::new(
            match axis_length[0] == 0.0 {
                true => 0.5,
                false => 0.5 - axis_length[0] * 0.5,
            },
            match axis_length[1] == 0.0 {
                true => 0.5,
                false => 0.5 - axis_length[1] * 0.5,
            },
        );

        Ok(comb.to_skeleton(
            trans.map(|t| t.level),
            self.source
                .as_ref()
                .map_or(&EMPTY_STROKE_MATCHS, |source| &source.stroke_matchs),
            offset,
            WorkRect::new(WorkPoint::origin(), WorkSize::splat(1.0)),
            self.source()
                .map(|source| &source.config.min_values)
                .unwrap_or(&Self::EMPTY_LIST),
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

    pub fn save_struc_in_cells(&mut self, name: String, struc: StrucWork, unit: WorkSize) {
        if let Some(source) = &mut self.source {
            source
                .components
                .insert(name, struc.to_prototype_cells(unit));
            self.changed = true;
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

    pub fn export_combs(&self, list: &Vec<char>, path: &str) {
        use super::struc::space::KeyPointType;

        const CHAR_BOX_SIZE: f32 = 44.;
        const CHAR_BOX_PADDING: f32 = 8.;
        const AREA_LENGTH: f32 = CHAR_BOX_SIZE - CHAR_BOX_PADDING * 2.;
        const COLUMN: f32 = 20.;

        const NAME_HEIGHT: f32 = 20.;

        const STYLE: &str = r##"<style type="text/css">
.line{fill:none;stroke:#000000;stroke-width:2;stroke-linecap:square;stroke-linejoin:round;stroke-miterlimit:10;}
.char_box{fill:none;stroke:#DCDDDD;stroke-width:1;}
.name{fill:#000000;font-size:12px;}
</style>
"##;

        match &self.source.is_some() {
            true => {
                let mut col = 0.0;
                let mut row = 0.0;
                let mut count = 0;

                let comb_list: String = list
                    .iter()
                    .filter_map(|chr| match self.get_struc_comb(*chr) {
                        Ok(comb) => {
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
                                                            + CHAR_BOX_PADDING)
                                                            + col * CHAR_BOX_SIZE,
                                                        ((kp.point.y)
                                                            * AREA_LENGTH
                                                            + CHAR_BOX_PADDING)
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
            false => {}
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

    pub fn align_cells(mut struc: StrucWork, unit: WorkSize) -> StrucWork {
        struc.align_cells(unit);
        struc
    }

    pub fn statistical_stroke_types(&self, list: &Vec<char>) -> BTreeMap<String, Vec<char>> {
        list.iter().fold(Default::default(), |mut counter, &name| {
            if let Ok((comb, _)) = self.get_comb_and_trans(name) {
                comb.stroke_types(Default::default())
                    .into_iter()
                    .for_each(|stroke| {
                        counter
                            .entry(stroke)
                            .and_modify(|list| list.push(name))
                            .or_insert(vec![name]);
                    })
            }
            counter
        })
    }
}
