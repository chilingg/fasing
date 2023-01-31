pub mod char_construct {
    #[derive(Debug, PartialEq, Eq)]
    pub enum Format {
        Single,
        SurroundFromAbove,      // ⿵
        AboveToBelow,           // ⿱
        AboveToMiddleAndBelow,  // ⿳
        SurroundFromBelow,      // ⿶
        FullSurround,           // ⿴
        SurroundFromUpperRight, // ⿹
        SurroundFromLeft,       // ⿷
        SurroundFromUpperLeft,  // ⿸
        SurroundFromLowerLeft,  // ⿺
        LeftToMiddleAndRight,   // ⿲
        LeftToRight,            // ⿰
    }

    impl Format {
        pub fn from_symbol(name: &str) -> Self {
            match name {
                "" => Format::Single,
                "⿵" => Format::SurroundFromAbove,
                "⿱" => Format::AboveToBelow,
                "⿳" => Format::AboveToMiddleAndBelow,
                "⿶" => Format::SurroundFromBelow,
                "⿴" => Format::FullSurround,
                "⿹" => Format::SurroundFromUpperRight,
                "⿷" => Format::SurroundFromLeft,
                "⿸" => Format::SurroundFromUpperLeft,
                "⿺" => Format::SurroundFromLowerLeft,
                "⿲" => Format::LeftToMiddleAndRight,
                "⿰" => Format::LeftToRight,
                _ => panic!("Unkonw format `{}`", name),
            }
        }

        pub fn to_symbol(&self) -> &'static str {
            match self {
                Format::Single => "",
                Format::SurroundFromAbove => "⿵",
                Format::AboveToBelow => "⿱",
                Format::AboveToMiddleAndBelow => "⿳",
                Format::SurroundFromBelow => "⿶",
                Format::FullSurround => "⿴",
                Format::SurroundFromUpperRight => "⿹",
                Format::SurroundFromLeft => "⿷",
                Format::SurroundFromUpperLeft => "⿸",
                Format::SurroundFromLowerLeft => "⿺",
                Format::LeftToMiddleAndRight => "⿲",
                Format::LeftToRight => "⿰",
            }
        }
    }

    #[derive(Debug)]
    pub enum Component {
        Char(String),
        Complex(Attrs),
    }

    #[derive(Debug)]
    pub struct Attrs {
        pub format: Format,
        pub components: Vec<Component>
    }

    impl std::fmt::Display for Attrs {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}{}", self.format.to_symbol(), self.components.iter().map(|comp| {
                match comp {
                    Component::Char(s) => s.clone(),
                    Component::Complex(attr) => format!("{}", attr),
                }
            }).collect::<String>())
        }
    }

    pub type Table = std::collections::HashMap<char, Attrs>;
}

pub mod prelude {
    pub use super::char_construct::*;
}

pub use prelude::*;
extern crate serde_json as sj;

fn table_from_json_array(obj: sj::Value) -> Table {
    fn attr_from_json_array(array: &Vec<sj::Value>) -> Attrs {
        let format = Format::from_symbol(array[0].as_str().unwrap());
        let components = array[1].as_array().unwrap().iter().fold(vec![], |mut comps, v| {
            match v {
                sj::Value::String(c) => comps.push(Component::Char(c.clone())),
                sj::Value::Array(array) => comps.push(Component::Complex(attr_from_json_array(array))),
                _ => panic!("Unknow data: {}", v.to_string())
            }
            comps
        });

        Attrs {
            format,
            components
        }
    }
    
    let obj = obj.as_object().unwrap();
    let table = Table::with_capacity(obj.len());

    obj.into_iter().fold(table, |mut table, (chr, attr)| {
        if let Some(a) = table.insert(chr.chars().next().unwrap(), attr_from_json_array(attr.as_array().unwrap())) {
            eprintln!("Duplicate character `{}`:\n{}\n{:?}", chr, attr, a);
        }
        table
    })
}

pub mod fasing_1_0;