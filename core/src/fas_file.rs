use std::collections::{ HashMap, HashSet };
use euclid::*;

use std::path::Path;

use super::construct::fasing_1_0;

use serde::{ Deserialize, Serialize };

#[derive(Default, Serialize, Deserialize)]
pub struct IndexSpace;
pub type IndexPoint = Point2D<usize, IndexSpace>;
pub type IndexSize = Size2D<usize, IndexSpace>;

#[derive(Default, PartialEq, Debug, Clone, Copy)]
pub struct WorkSpace;
pub type WorkPoint = Point2D<f32, WorkSpace>;
pub type WorkSize = Size2D<f32, WorkSpace>;
pub type WorkVec = Vector2D<f32, WorkSpace>;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyPoint<T: Clone + Copy, U> {
    Line(Point2D<T, U>),
    Curve(Point2D<T, U>),
}

pub type KeyIndexPoint = KeyPoint<usize, IndexSpace>;
pub type KeyFloatPoint = KeyPoint<f32, WorkSpace>;

impl KeyIndexPoint {
    pub fn to_work_space(&self) -> KeyFloatPoint {
        let p = self.point().cast().cast_unit();
        match self {
            KeyPoint::Line(_) => KeyPoint::Line(p),
            _ => unreachable!()
        }
    }
}

impl<T: Clone + Copy, U> KeyPoint<T, U> {
    pub fn point(&self) -> Point2D<T, U> {
        match self {
            Self::Line(p) => *p,
            _ => unreachable!()
        }
    }

    pub fn point_mut(&mut self) -> &mut Point2D<T, U> {
        match self {
            Self::Line(p) => p,
            _ => unreachable!()
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct KeyPath<T: Clone + Copy, U> {
    pub closed: bool,
    pub points: Vec<KeyPoint<T, U>>
}

impl<T: Clone + Copy, U> KeyPath<T, U> {
    fn new(points: Vec<KeyPoint<T, U>>, closed: bool) -> Self {
        Self { closed, points }
    }
}

pub type KeyIndexPath = KeyPath<usize, IndexSpace>;
pub type KeyFloatPath = KeyPath<f32, WorkSpace>;

impl KeyFloatPath {
    pub fn from_lines<I>(path: I, closed: bool) -> Self
    where
        I: IntoIterator<Item = WorkPoint>
    {
        Self { closed, points: path.into_iter().map(|p| KeyFloatPoint::Line(p)).collect() }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct Struc<T: Default + Clone + Copy, U> {
    pub key_paths: Vec<KeyPath<T, U>>,
    pub tags: HashSet<String>
}

pub type StrucProto = Struc<usize, IndexSpace>;
pub type StrucWokr = Struc<f32, WorkSpace>;

impl StrucWokr {
    pub fn from_prototype(proto: &StrucProto) -> Self {
        Self {
            key_paths: proto.key_paths.iter().map(|path| {
                KeyFloatPath {
                    points: path.points.iter().map(|p| p.to_work_space()).collect(),
                    closed: path.closed,
                }
            }).collect(),
            tags: proto.tags.clone(),
        }
    }

    pub fn add_lines<I: IntoIterator<Item = WorkPoint>>(&mut self, lines: I, closed: bool) {
        self.key_paths.push(KeyFloatPath::from_lines(lines, closed));
    }

    pub fn to_prototype(&self) -> StrucProto {
        StrucProto::from_work(self)
    }

    pub fn to_prototype_offset(&self, offset: f32) -> StrucProto {
        StrucProto::from_work_offset(self, offset)
    }

    pub fn transform(&mut self, scale: WorkVec, moved: WorkVec) {
        self.key_paths.iter_mut().for_each(|path| {
            path.points.iter_mut().for_each(|p|{
                let p = p.point_mut();
                p.x = p.x * scale.x + moved.x;
                p.y = p.y * scale.y + moved.y;
            })
        })
    }
}

impl StrucProto {
    const OFFSET: f32 = 0.01;

    pub fn from_work(struc: &StrucWokr) -> Self {
        Self::from_work_offset(struc, Self::OFFSET)
    }

    pub fn from_work_offset(struc: &StrucWokr, offset: f32) -> Self {
        let mut x_sort = vec![];
        let mut y_sort = vec![];

        struc.key_paths.iter().for_each(|path| { path.points.iter().for_each(|p| {
            let p = p.point();
            x_sort.push(p.x);
            y_sort.push(p.y);
        })});

        x_sort.sort_by(|a, b| a.partial_cmp(b).unwrap());
        y_sort.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let x_map = x_sort.iter().fold(vec![], |mut map: Vec<Vec<f32>>, &n| {
            if !map.is_empty() && (n - map.last().unwrap().last().unwrap()).abs() < offset {
                map.last_mut().unwrap().push(n);
            } else {
                map.push(vec![n]);
            }
            map
        });
        let y_map = y_sort.iter().fold(vec![], |mut map: Vec<Vec<f32>>, &n| {
            if !map.is_empty() && (n - map.last().unwrap().last().unwrap()).abs() < offset {
                map.last_mut().unwrap().push(n);
            } else {
                map.push(vec![n]);
            }
            map
        });

        let key_paths: Vec<KeyIndexPath> = struc.key_paths.iter().fold(vec![], |mut key_paths, f_path| {
            let path = f_path.points.iter().fold(vec![], |mut path, p| {
                let pos = p.point();
                let x = x_map.iter().enumerate().find_map(|(i, map)| {
                    map.iter().find(|&&n| n == pos.x).and(Some(i))
                }).unwrap();
                let y = y_map.iter().enumerate().find_map(|(i, map)| {
                    map.iter().find(|&&n| n == pos.y).and(Some(i))
                }).unwrap();
                path.push(match p {
                    KeyPoint::Line(_) => KeyIndexPoint::Line(IndexPoint::new(x, y)),
                    KeyPoint::Curve(_) => KeyIndexPoint::Curve(IndexPoint::new(x, y)),
                });
                path
            });
            key_paths.push(KeyIndexPath::new(path, f_path.closed));
            key_paths
        });

        StrucProto {
            key_paths,
            tags: struc.tags.clone()
        }
    }

    pub fn to_work(&self) -> StrucWokr {
        StrucWokr::from_prototype(self)
    }
}

impl<T: Default + Clone + Copy + Ord, U> Struc<T, U> {
    pub fn size(&self) -> Size2D<T, U> {
        self.key_paths.iter().fold(Size2D::default(), |size, path| {
            path.points.iter().fold(size, |size, kp| {
                let p = kp.point();
                Size2D::new(size.width.max(p.x), size.height.max(p.y))
            })
        })
    }
}

#[derive(Debug)]
pub enum Error {
    Deserialize(serde_json::Error),
    Io(std::io::Error)
}

#[derive(Serialize, Deserialize)]
pub struct FasFile {
    pub name: String,
    pub major_version: u32,
    pub minor_version: u32,
    pub components: HashMap<String, StrucProto>
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
    pub fn new_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // let t = ;
                match serde_json::from_str::<Self>(&content) {
                    Ok(obj) => Ok(obj),
                    Err(e) => Err(Error::Deserialize(e))
                }
            },
            Err(e) => Err(Error::Io(e))
        }
    }

    pub fn from_template_fasing_1_0() -> Self {
        fasing_1_0::generate_fas_file()
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<usize> {
        let texts = serde_json::to_string(self).unwrap();
        std::fs::write(path, &texts).and_then(|_| Ok(texts.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::construct;
    
    #[test]
    fn test_fas_file() {
        let mut test_file = FasFile::default();
        let table = construct::fasing_1_0::generate_table();

        let requis = construct::all_requirements(&table);
        requis.into_iter().for_each(|comp| { test_file.components.insert(comp, StrucProto::default()); });

        let mut key_points = StrucWokr::default();
        key_points.add_lines([WorkPoint::new(0.0, 1.0), WorkPoint::new(1.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(2.0, 0.0), WorkPoint::new(2.0, 2.0)], false);
        key_points.add_lines([WorkPoint::new(4.0, 1.0), WorkPoint::new(3.0, 2.0)], false);
        assert_eq!(key_points.key_paths[0].points[0], KeyFloatPoint::Line(WorkPoint::new(0.0, 1.0)));
        assert_eq!(key_points.key_paths[1].points[1], KeyFloatPoint::Line(WorkPoint::new(2.0, 2.0)));

        test_file.components.insert("⺌".to_string(), key_points.to_prototype());

        let tmp_dir = Path::new("tmp");
        if !tmp_dir.exists() {
            std::fs::create_dir(tmp_dir.clone()).unwrap();
        }
        std::fs::write(tmp_dir.join("fas_file.fas"), serde_json::to_string_pretty(&test_file).unwrap()).unwrap();
    }
}