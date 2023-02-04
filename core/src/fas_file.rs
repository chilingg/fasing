use std::collections::HashMap;
use euclid::*;

use super::construct::fasing_1_0;

use serde::{ Deserialize, Serialize };

pub struct OriginSpace;

pub type OriginPoint = Point2D<u32, OriginSpace>;
pub type OriginSize = Size2D<u32, OriginSpace>;

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum KeyPoint {
    Line(OriginPoint),
    Break,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Structure {
    pub key_points: Vec<KeyPoint>
}

impl Structure {
    pub fn size(&self) -> OriginSize {
        self.key_points.iter().fold(OriginSize::default(), |size, kp| {
            if let KeyPoint::Line(p) = kp {
                OriginSize::new(size.width.max(p.x), size.height.max(p.y))
            } else {
                size
            }
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub components: HashMap<String, Structure>
}

impl std::default::Default for FasFile {
    fn default() -> Self {
        Self {
            name: "untile".to_string(),
            major_version: 0,
            minor_version: 1,
            components: HashMap::new(),
        }
    }
}

impl FasFile {
    pub fn from_template_fasing_1_0() -> Self {
        fasing_1_0::generate_fas_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;
    use crate::construct;
    
    #[test]
    fn test_fas_file() {
        let mut test_file = FasFile::default();
        let table = construct::fasing_1_0::generate_table();

        let requis = construct::all_requirements(&table);
        requis.into_iter().for_each(|comp| { test_file.components.insert(comp, Structure::default()); });

        let mut key_points = Structure::default();
        key_points.key_points.extend([
            KeyPoint::Line(OriginPoint::new(0, 1)),
            KeyPoint::Line(OriginPoint::new(1, 2)),
            KeyPoint::Break,
            KeyPoint::Line(OriginPoint::new(2, 0)),
            KeyPoint::Line(OriginPoint::new(2, 2)),
            KeyPoint::Break,
            KeyPoint::Line(OriginPoint::new(4, 1)),
            KeyPoint::Line(OriginPoint::new(3, 2)),
            KeyPoint::Break,
        ]);
        test_file.components.insert("⺌".to_string(), key_points);

        let tmp_dir = Path::new("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir(tmp_dir.clone()).unwrap();
        }
        std::fs::write(tmp_dir.join("fas_file.fas"), serde_json::to_string_pretty(&test_file).unwrap()).unwrap();
    }
}