use super::Service;
use crate::{
    base::*,
    construct::{Component, CpAttrs, CstType},
};

use serde::{Deserialize, Serialize, ser::SerializeStruct};
use serde_json as sj;

use std::collections::BTreeMap;

pub struct CharTree {
    pub name: String,
    pub tp: CstType,
    pub children: Vec<CharTree>,
}

impl CharTree {
    pub fn parse_name(name: String) -> (String, BTreeMap<String, sj::Value>) {
        if let Some((c_name, attrs)) = name.split_once(':') {
            let attrs = sj::from_str::<BTreeMap<String, sj::Value>>(attrs)
                .inspect_err(|e| {
                    log::error!("The attribute `{attrs}` setting of `{c_name}` is error: {e}")
                })
                .ok()
                .unwrap_or_default();
            (c_name.to_string(), attrs)
        } else {
            (name, Default::default())
        }
    }

    pub fn new_single(name: String) -> Self {
        Self {
            name,
            tp: CstType::Single,
            children: vec![],
        }
    }

    pub fn get_comb_name(&self) -> String {
        match self.tp {
            CstType::Single => self.name.clone(),
            tp => {
                format!(
                    "{}: {}({})",
                    self.name.clone(),
                    tp.symbol(),
                    self.children
                        .iter()
                        .map(|c| c.get_comb_name())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
        }
    }
}

impl Serialize for CharTree {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("CharTree", 3)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("tp", &self.tp.symbol())?;
        s.serialize_field("children", &self.children)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for CharTree {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match Deserialize::deserialize(deserializer)? {
            serde_json::Value::Object(data) => {
                let name = match data.get("name") {
                    Some(val) if val.is_string() => val.as_str().unwrap().to_string(),
                    _ => Err(serde::de::Error::custom("Missing field `name`!"))?,
                };
                let tp = match data.get("tp") {
                    Some(val) if val.is_string() => {
                        CstType::from_symbol(val.as_str().unwrap()).ok_or(
                            serde::de::Error::custom(format!("Unkonw construct type: {}", val)),
                        )?
                    }
                    _ => Err(serde::de::Error::custom("Missing field `tp`!"))?,
                };
                let children = match data.get("children") {
                    Some(val) if val.is_array() => sj::from_value(val.clone())
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?,
                    _ => Err(serde::de::Error::custom("Missing field `children`!"))?,
                };
                Ok(Self { name, tp, children })
            }
            val => Err(serde::de::Error::custom(format!(
                "Failed convert to CharTree in {}",
                val
            ))),
        }
    }
}

// start point
pub fn get_char_tree(service: &impl Service, name: String) -> CharTree {
    get_char_tree_in(
        service,
        Component::Char(name),
        (CstType::Single, Section::Start),
        Default::default(),
    )
}

fn get_char_tree_in(
    service: &impl Service,
    comp: Component,
    in_tp: (CstType, Section),
    adjacency: DataHV<[bool; 2]>,
) -> CharTree {
    let (name, mut attrs) = get_char_attrs_in(service, comp, in_tp, adjacency);
    match attrs.tp {
        CstType::Single => CharTree::new_single(name),
        CstType::Scale(axis) => {
            let end = attrs.components.len();
            let children = attrs
                .components
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    let mut c_adjacency = adjacency.clone();
                    if i != 0 {
                        c_adjacency.hv_get_mut(axis)[0] = true;
                    }
                    if i + 1 != end {
                        c_adjacency.hv_get_mut(axis)[1] = true;
                    }

                    let c_in_tp = match i {
                        0 => {
                            if in_tp.0 == attrs.tp && in_tp.1 != Section::Start {
                                Section::Middle
                            } else {
                                Section::Start
                            }
                        }
                        n if n + 1 == end => {
                            if in_tp.0 == attrs.tp && in_tp.1 != Section::End {
                                Section::Middle
                            } else {
                                Section::End
                            }
                        }
                        _ => Section::Middle,
                    };
                    get_char_tree_in(service, c, (attrs.tp, c_in_tp), c_adjacency)
                })
                .collect();
            CharTree {
                name,
                tp: attrs.tp,
                children,
            }
        }
        CstType::Surround(_) => {
            let sc = get_char_tree_in(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Section::End),
                adjacency,
            );
            let pc = get_char_tree_in(
                service,
                attrs.components.pop().unwrap(),
                (attrs.tp, Section::Start),
                adjacency,
            );

            CharTree {
                name,
                tp: attrs.tp,
                children: vec![pc, sc],
            }
        }
    }
}

fn get_char_attrs_in(
    service: &impl Service,
    comp: Component,
    in_tp: (CstType, Section),
    adjacency: DataHV<[bool; 2]>,
) -> (String, CpAttrs) {
    let mut name = comp.name();
    let mut attrs = match service
        .get_config()
        .check_name_replace(&name, in_tp, adjacency)
    {
        Some(new_name) => {
            name = new_name;
            get_comp_attrs(service, &name)
                .cloned()
                .unwrap_or(CpAttrs::single())
        }
        None => match comp {
            Component::Char(name) => get_comp_attrs(service, &name)
                .cloned()
                .unwrap_or(CpAttrs::single()),
            Component::Complex(attrs) => attrs,
        },
    };
    comb_remap(service, &mut attrs, adjacency);

    (name, attrs)
}

pub fn get_comp_attrs<'a, 'b>(service: &'a impl Service, name: &'b str) -> Option<&'a CpAttrs> {
    service
        .get_config()
        .supplement
        .get(name)
        .or(service.get_table().get(name))
}

fn comb_remap(service: &impl Service, attrs: &mut CpAttrs, adjacency: DataHV<[bool; 2]>) {
    if let CstType::Scale(axis) = attrs.tp {
        let config = service.get_config();

        let mut pre: Vec<Component> = vec![];
        let mut next = attrs.components.clone();
        next.reverse();

        let mut ok = false;
        while !next.is_empty() {
            let mut c_adjacency = adjacency.clone();

            let section = if pre.is_empty() {
                c_adjacency.hv_get_mut(axis)[1] = true;
                Section::Start
            } else if next.len() == 1 {
                c_adjacency.hv_get_mut(axis)[0] = true;
                Section::End
            } else {
                c_adjacency.hv_get_mut(axis)[0] = true;
                c_adjacency.hv_get_mut(axis)[1] = true;
                Section::Middle
            };

            let mut c = next.pop().unwrap();
            let (c_name, c_attrs) =
                get_char_attrs_in(service, c.clone(), (attrs.tp, section), c_adjacency);
            while !pre.is_empty() {
                if let Some(new_name) =
                    config.adjacency_replace_name(&pre.last().unwrap().name(), &c_name, axis)
                {
                    pre.pop();
                    c = Component::Char(new_name);
                    ok = true;
                } else {
                    break;
                }
            }

            match c_attrs.tp {
                CstType::Scale(c_axis) if c_axis == axis => {
                    next.extend(c_attrs.components.into_iter().rev());
                }
                _ => pre.push(c),
            }
        }

        if ok {
            attrs.components = pre;
        }
    }

    if let CstType::Surround(surround_place) = attrs.tp {
        let (p_name, mut p_attrs) = get_char_attrs_in(
            service,
            attrs.components[0].clone(),
            (attrs.tp, Section::Start),
            adjacency,
        );

        if let CstType::Scale(c_axis) = p_attrs.tp {
            let index = match surround_place.hv_get(c_axis) {
                Section::Start => p_attrs.components.len() - 1,
                Section::End => 0,
                Section::Middle => {
                    // TODO suround in scale
                    eprintln!(
                        "{} {} surround component in {}",
                        p_name,
                        p_attrs.comps_name(),
                        CstType::Surround(surround_place).symbol()
                    );
                    return;
                }
            };
            let secondary = attrs.components.pop().unwrap();
            *attrs = p_attrs;
            attrs.components[index] = Component::Complex(CpAttrs {
                tp: CstType::Surround(surround_place),
                components: vec![attrs.components[index].clone(), secondary],
            })
        } else if let CstType::Surround(c_surround) = p_attrs.tp {
            if c_surround == surround_place {
                let sc1 = p_attrs.components.pop().unwrap();
                let pc = p_attrs.components.pop().unwrap();
                let sc = if c_surround.v == Section::End {
                    vec![attrs.components[1].clone(), sc1]
                } else {
                    vec![sc1, attrs.components[1].clone()]
                };
                *attrs = CpAttrs {
                    tp: attrs.tp,
                    components: vec![
                        pc,
                        Component::Complex(CpAttrs {
                            tp: CstType::Scale(Axis::Vertical),
                            components: sc,
                        }),
                    ],
                };
            } else {
                // TODO suround in surround
                eprintln!(
                    "{} {} surround component in {}",
                    p_name,
                    p_attrs.comps_name(),
                    CstType::Surround(surround_place).symbol()
                );
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name() {
        let chr = "name";
        let (name, attrs) = CharTree::parse_name(chr.to_string());
        assert_eq!(&name, "name");
        assert!(attrs.is_empty());

        let chr = "name:";
        let (name, attrs) = CharTree::parse_name(chr.to_string());
        assert_eq!(&name, "name");
        assert!(attrs.is_empty());

        let chr = "name: {}";
        let (name, attrs) = CharTree::parse_name(chr.to_string());
        assert_eq!(&name, "name");
        assert!(attrs.is_empty());

        let chr = "name: {x}";
        let (name, attrs) = CharTree::parse_name(chr.to_string());
        assert_eq!(&name, "name");
        assert!(attrs.is_empty());

        let chr = "name: {\"x\":7}";
        let (name, attrs) = CharTree::parse_name(chr.to_string());
        assert_eq!(&name, "name");
        assert_eq!(attrs.get("x"), Some(sj::json!(7)).as_ref());
    }
}
