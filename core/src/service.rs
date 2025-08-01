use crate::{
    axis::*,
    component::{
        comb::{CharInfo, StrucComb},
        struc::*,
    },
    config::{setting, Config},
    construct::{self, space::*, CharTree, Component, CpAttrs, CstError, CstTable, CstType},
    fas::FasFile,
    svg,
};

use std::collections::BTreeSet;

pub mod path {
    use super::*;

    pub fn process_path(mut paths: Vec<Vec<WorkPoint>>, connect: bool) -> Vec<Vec<WorkPoint>> {
        if connect {
            let mut i = 0;
            while i + 1 < paths.len() {
                let mut j = i + 1;
                while j < paths.len() {
                    if paths[i][0] == paths[j][0] {
                        paths[i] = paths
                            .remove(j)
                            .into_iter()
                            .rev()
                            .chain(paths[i].iter().copied())
                            .collect();
                    } else if paths[i][0] == *paths[j].last().unwrap() {
                        paths[i] = paths
                            .remove(j)
                            .into_iter()
                            .chain(paths[i].iter().copied())
                            .collect();
                    } else if *paths[i].last().unwrap() == paths[j][0] {
                        let temp = paths.remove(j);
                        paths[i].extend(temp);
                    } else if *paths[i].last().unwrap() == *paths[j].last().unwrap() {
                        let temp = paths.remove(j);
                        paths[i].extend(temp.into_iter().rev());
                    } else {
                        j += 1;
                    }
                }
                i += 1;
            }
        }

        paths.iter_mut().for_each(|path| {
            debug_assert!(!path.is_empty());
            if path.len() > 1 {
                let mut i = 1;
                while i + 1 < path.len() {
                    if (path[i - 1].x == path[i].x && path[i].x == path[i + 1].x)
                        || (path[i - 1].y == path[i].y && path[i].y == path[i + 1].y)
                    {
                        path.remove(i);
                    } else {
                        i += 1;
                    }
                }
            }
        });

        let mut dots: Vec<usize> = vec![];
        let mut indexes: DataHV<BTreeSet<i32>> = Default::default();
        let integer_paths: Vec<Vec<euclid::Point2D<i32, IndexSpace>>> = paths
            .iter()
            .map(|path| {
                path.iter()
                    .map(|p| {
                        euclid::Point2D::new(
                            (p.x / crate::algorithm::NORMAL_OFFSET) as i32,
                            (p.y / crate::algorithm::NORMAL_OFFSET) as i32,
                        )
                    })
                    .collect()
            })
            .collect();
        integer_paths.iter().enumerate().for_each(|(i, path)| {
            if path.iter().all(|p| path[0].eq(p)) {
                dots.push(i);
            }
            path.iter().for_each(|p| {
                indexes.h.insert(p.x);
                indexes.v.insert(p.y);
            });
        });
        if !dots.is_empty() {
            let mut view: Vec<Vec<bool>> = vec![vec![false; indexes.h.len()]; indexes.v.len()];
            integer_paths.iter().enumerate().for_each(|(i, path)| {
                if !dots.contains(&i) {
                    path.windows(2).for_each(|slice| {
                        let x1 = indexes.h.iter().position(|val| slice[0].x.eq(val)).unwrap();
                        let y1 = indexes.v.iter().position(|val| slice[0].y.eq(val)).unwrap();
                        if slice[0].x == slice[1].x {
                            let y2 = indexes.v.iter().position(|val| slice[1].y.eq(val)).unwrap();
                            for idx in y1.min(y2)..=y2.min(y1) {
                                view[idx][x1] = true;
                            }
                        } else if slice[0].y == slice[1].y {
                            let x2 = indexes.h.iter().position(|val| slice[1].x.eq(val)).unwrap();
                            for idx in x1.min(x2)..=x2.max(x1) {
                                view[y1][idx] = true;
                            }
                        }
                    });
                }
            });
            dots.into_iter().rev().for_each(|i| {
                let p = integer_paths[i][0];
                let coor = Axis::hv().into_map(|axis| {
                    indexes
                        .hv_get(axis)
                        .iter()
                        .position(|val| p.hv_get(axis).eq(val))
                        .unwrap()
                });
                if view[coor.v][coor.h] {
                    paths.remove(i);
                }
            });
        }

        paths
    }

    pub fn get_path(comb: &StrucComb, connect: bool) -> Vec<Vec<WorkPoint>> {
        let paths: Vec<Vec<WorkPoint>> = comb
            .to_paths()
            .into_iter()
            .filter_map(|path| match path.hide {
                false if path.points.len() > 1 => Some(path.points),
                _ => None,
            })
            .collect();
        process_path(paths, connect)
    }

    pub fn gen_stroke(paths: &Vec<Vec<WorkPoint>>, stroke_width: f32) -> Vec<svg::Path> {
        fn temp_stroking(path: &Vec<WorkPoint>, stroke_width: f32, cap: [bool; 2]) -> svg::Path {
            let half = stroke_width / 2.0;
            let cap_val = cap.map(|b| match b {
                true => 1.0,
                false => 0.0,
            });

            let vec = path[1] - path[0];
            let tangent = vec.normalize() * half;
            let mut pre_point = path[1];
            let mut pre_normal = WorkVec::new(tangent.y, -tangent.x);

            let line = svg::Draw::line(vec + tangent * cap_val[0]);
            let mut path1 = svg::Path::new(path[0] + pre_normal - tangent * cap_val[0], vec![line]);
            let mut path2 = svg::Path::new(
                path[0] + pre_normal - tangent * cap_val[0],
                vec![svg::Draw::line(pre_normal * -2.0), line],
            );

            path[2..].iter().for_each(|&p| {
                let vec = p - pre_point;
                let tangent = vec.normalize() * half;
                let normal = WorkVec::new(tangent.y, -tangent.x);
                let list = if pre_normal.angle_to(normal).radians < 0.0 {
                    [&mut path1, &mut path2]
                } else {
                    [&mut path2, &mut path1]
                };

                list[0].extend(-half);
                list[0].commands.push(svg::Draw::line(vec));
                list[0].extend(-half);

                list[1].extend(half);
                list[1].commands.push(svg::Draw::line(vec));
                list[1].extend(half);

                pre_normal = normal;
                pre_point = p;
            });

            path1.extend(half * cap_val[1]);
            path2.extend(half * cap_val[1]);
            path1.line_of(-pre_normal * 2.0);
            path1.connect(path2.reverse());
            path1
        }

        let cap_state: Vec<[bool; 2]> = {
            let mut indexes: DataHV<BTreeSet<i32>> = Default::default();
            let integer_paths: Vec<Vec<euclid::Point2D<i32, IndexSpace>>> = paths
                .iter()
                .map(|path| {
                    path.iter()
                        .map(|p| {
                            euclid::Point2D::new(
                                (p.x / crate::algorithm::NORMAL_OFFSET) as i32,
                                (p.y / crate::algorithm::NORMAL_OFFSET) as i32,
                            )
                        })
                        .collect()
                })
                .collect();
            integer_paths.iter().for_each(|path| {
                path.iter().for_each(|p| {
                    indexes.h.insert(p.x);
                    indexes.v.insert(p.y);
                });
            });
            let mut view: Vec<Vec<bool>> = vec![vec![false; indexes.h.len()]; indexes.v.len()];
            integer_paths.iter().for_each(|path| {
                path.windows(2).for_each(|slice| {
                    let x1 = indexes.h.iter().position(|val| slice[0].x.eq(val)).unwrap();
                    let y1 = indexes.v.iter().position(|val| slice[0].y.eq(val)).unwrap();
                    if slice[0].x == slice[1].x {
                        let y2 = indexes.v.iter().position(|val| slice[1].y.eq(val)).unwrap();
                        for idx in y1.min(y2) + 1..y2.max(y1) {
                            view[idx][x1] = true;
                        }
                    } else if slice[0].y == slice[1].y {
                        let x2 = indexes.h.iter().position(|val| slice[1].x.eq(val)).unwrap();
                        for idx in x1.min(x2) + 1..x2.max(x1) {
                            view[y1][idx] = true;
                        }
                    }
                });
            });
            integer_paths
                .iter()
                .map(|path| {
                    let x1 = indexes.h.iter().position(|val| path[0].x.eq(val)).unwrap();
                    let y1 = indexes.v.iter().position(|val| path[0].y.eq(val)).unwrap();
                    let x2 = indexes
                        .h
                        .iter()
                        .position(|val| path.last().unwrap().x.eq(val))
                        .unwrap();
                    let y2 = indexes
                        .v
                        .iter()
                        .position(|val| path.last().unwrap().y.eq(val))
                        .unwrap();

                    [!view[y1][x1], !view[y2][x2]]
                })
                .collect()
        };

        paths
            .iter()
            .zip(cap_state)
            .map(|(path, cap)| {
                if path.iter().all(|p| path[0].eq(p)) {
                    svg::Path::rect(path[0], stroke_width)
                } else {
                    temp_stroking(path, stroke_width, cap)
                }
            })
            .collect()
    }
}

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
                                "{} {} is surround component in {}",
                                primary.1.tp.symbol(),
                                primary.0,
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
                                "{}{} is surround component in {}",
                                primary.1.tp.symbol(),
                                primary.0,
                                CstType::Surround(surround_place).symbol()
                            )
                        }
                    }
                    CstType::Single => {
                        let mut in_place = [adjacency.clone(); 2];
                        Axis::list().into_iter().for_each(|axis| {
                            let surround_place = *surround_place.hv_get(axis);
                            if surround_place != Place::End {
                                // in_place[0].hv_get_mut(axis)[1] = true;
                                in_place[1].hv_get_mut(axis)[0] = true;
                            }
                            if surround_place != Place::Start {
                                // in_place[0].hv_get_mut(axis)[0] = true;
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

    pub fn gen_comb_proto_in(
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
                let children = target.children;
                let mut combs = Vec::with_capacity(children.len());

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

                if fas.config.setting.contains(setting::SAME_HORIZONTAL) {
                    let mut map = std::collections::HashMap::new();
                    combs.iter().for_each(|c| {
                        if let StrucComb::Single { name, proto, .. } = c {
                            let allocs = proto.allocation_size();
                            let size = IndexSize::new(allocs.h, allocs.v);
                            map.entry(name.to_string())
                                .and_modify(|s| *s = size.min(*s))
                                .or_insert(size);
                        }
                    });
                    combs.iter_mut().for_each(|c| {
                        if let StrucComb::Single {
                            name, proto, view, ..
                        } = c
                        {
                            let size = map.get(name).unwrap().to_hv_data();
                            let mut c_size = proto.allocation_size();
                            let modify = c_size != size;

                            for axis in [Axis::Horizontal] {
                                while c_size.hv_get(axis) > size.hv_get(axis) {
                                    if !proto.reduce(axis, false) {
                                        break;
                                    }
                                    c_size = proto.allocation_size();
                                }
                            }
                            if modify {
                                *view = crate::component::view::StrucView::new(&proto);
                            }
                        }
                    });
                }

                Ok(StrucComb::new_complex(target.name, target.tp, combs))
            }
            CstType::Surround(surround_place) => {
                let mut in_place = [adjacency.clone(); 2];
                Axis::list().into_iter().for_each(|axis| {
                    let surround_place = *surround_place.hv_get(axis);
                    if surround_place != Place::End {
                        // in_place[0].hv_get_mut(axis)[1] = true;
                        in_place[1].hv_get_mut(axis)[0] = true;
                    }
                    if surround_place != Place::Start {
                        // in_place[0].hv_get_mut(axis)[0] = true;
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
                comb.expand_comb_proto(&source, &self.construct_table, false)?;
                let paths = path::get_path(&comb, false);
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
                    .map(|mut comb| {
                        match comb.expand_comb_proto(&source, &self.construct_table, true) {
                            Ok(info) => info.unwrap(),
                            Err(_) => {
                                let mut info = CharInfo::default();
                                info.comb_name = comb.get_comb_name();
                                info
                            }
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

    pub fn export_chars(
        &self,
        list: Vec<char>,
        width: usize,
        height: usize,
        path: &str,
    ) -> Vec<String> {
        fn process(
            service: &LocalService,
            source: &FasFile,
            chr: char,
            width: usize,
            height: usize,
            path: &std::path::Path,
        ) -> anyhow::Result<()> {
            let mut comb = combination::gen_comb_proto(
                service.gen_char_tree(chr.to_string()),
                &service.construct_table,
                &source,
            )?;
            comb.expand_comb_proto(&source, &service.construct_table, false)?;
            let glyph = path::gen_stroke(&path::get_path(&comb, true), source.config.strok_width);
            let img = svg::to_svg_img(&glyph, IndexSize::new(width, height));

            let file_name = path.join(format!("{}.svg", chr));
            std::fs::write(file_name, img)?;
            Ok(())
        }

        match &self.source {
            Some(source) => {
                let targets = if list.is_empty() {
                    self.construct_table.target_chars()
                } else {
                    list
                };
                let path = std::path::Path::new(path);
                let mut message = vec![];
                let mut info = vec![];

                for chr in targets {
                    match process(self, source, chr, width, height, path) {
                        Err(e) => message.push(format!("{}: {}", chr, e)),
                        Ok(_) => info.push(chr.to_string()),
                    }
                }

                if let Err(e) = std::fs::write(path.join("char_list.txt"), info.join("\n")) {
                    eprintln!("{e}");
                }
                if let Err(e) = std::fs::write(path.join("error_list.txt"), message.join("\n")) {
                    eprintln!("{e}");
                }

                message
            }
            None => vec![],
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

    #[test]
    fn test_get_path() {
        use super::*;

        let paths = vec![
            vec![WorkPoint::new(0.5, 0.0), WorkPoint::new(0.5, 0.0)],
            vec![WorkPoint::new(0.5, 0.0), WorkPoint::new(0.5, 0.0)],
            vec![WorkPoint::new(0.5, 0.0), WorkPoint::new(0.5, 0.0)],
            vec![
                WorkPoint::new(0.0, 0.3),
                WorkPoint::new(0.5, 0.3),
                WorkPoint::new(0.9, 0.3),
                WorkPoint::new(0.9, 0.6),
                WorkPoint::new(0.9, 0.9),
            ],
            vec![WorkPoint::new(0.0, 0.3), WorkPoint::new(0.0, 0.3)],
            vec![WorkPoint::new(0.5, 0.3), WorkPoint::new(0.5, 0.3)],
            vec![WorkPoint::new(0.9, 0.3), WorkPoint::new(0.9, 0.3)],
            vec![WorkPoint::new(0.5, 0.9), WorkPoint::new(0.9, 0.9)],
        ];
        let paths = path::process_path(paths, true);
        assert_eq!(
            paths,
            vec![
                vec![WorkPoint::new(0.5, 0.0), WorkPoint::new(0.5, 0.0)],
                vec![
                    WorkPoint::new(0.0, 0.3),
                    WorkPoint::new(0.9, 0.3),
                    WorkPoint::new(0.9, 0.9),
                    WorkPoint::new(0.5, 0.9),
                ],
            ]
        );
    }
}
