use euclid::*;
use num_traits::cast::NumCast;
use serde::{Deserialize, Serialize};

use crate::axis::Axis;

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexSpace;
pub type IndexPoint = Point2D<usize, IndexSpace>;
pub type IndexSize = Size2D<usize, IndexSpace>;

#[derive(Default, Serialize, Deserialize, PartialEq, Debug, Clone, Copy)]
pub struct WorkSpace;
pub type WorkPoint = Point2D<f32, WorkSpace>;
pub type WorkSize = Size2D<f32, WorkSpace>;
pub type WorkVec = Vector2D<f32, WorkSpace>;
pub type WorkRect = Rect<f32, WorkSpace>;
pub type WorkBox = Box2D<f32, WorkSpace>;

pub trait BoxExpand<T, U> {
    fn contains_include(&self, p: Point2D<T, U>) -> bool;
}

impl<T, U> BoxExpand<T, U> for Box2D<T, U>
where
    T: PartialOrd,
{
    fn contains_include(&self, p: Point2D<T, U>) -> bool {
        self.min.x <= p.x && p.x <= self.max.x && self.min.y <= p.y && p.y <= self.max.y
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum KeyPointType {
    Line,
    Horizontal,
    Vertical,
    Mark,
    Hide,
}

impl KeyPointType {
    pub fn is_unreal(&self, axis: Axis) -> bool {
        match self {
            Self::Mark => true,
            Self::Horizontal if axis == Axis::Vertical => true,
            Self::Vertical if axis == Axis::Horizontal => true,
            _ => false,
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            Self::Line => 'L',
            Self::Horizontal => 'H',
            Self::Vertical => 'V',
            Self::Mark => 'M',
            Self::Hide => 'N',
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyPoint<T: Clone + Copy, U> {
    pub p_type: KeyPointType,
    pub point: Point2D<T, U>,
}

impl<T: Clone + Copy, U> KeyPoint<T, U> {
    pub fn new(point: Point2D<T, U>, p_type: KeyPointType) -> Self {
        Self { point, p_type }
    }

    pub fn new_line_point(point: Point2D<T, U>) -> Self {
        Self {
            point,
            p_type: KeyPointType::Line,
        }
    }
}

impl<T: Clone + Copy + NumCast, U> KeyPoint<T, U> {
    pub fn cast<NewT, NewU>(&self) -> KeyPoint<NewT, NewU>
    where
        NewT: Clone + Copy + NumCast,
    {
        KeyPoint {
            p_type: self.p_type,
            point: self.point.cast().cast_unit(),
        }
    }
}

pub type KeyIndexPoint = KeyPoint<usize, IndexSpace>;
pub type KeyFloatPoint<U> = KeyPoint<f32, U>;

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct KeyPath<T: Clone + Copy, U> {
    pub points: Vec<KeyPoint<T, U>>,
}

impl<T: Clone + Copy, U> KeyPath<T, U> {
    pub fn new(points: Vec<KeyPoint<T, U>>) -> Self {
        Self { points }
    }

    pub fn hide(&mut self) {
        self.points
            .iter_mut()
            .for_each(|p| p.p_type = KeyPointType::Hide);
    }
}

impl<T: Clone + Copy + NumCast, U: Clone + Copy> KeyPath<T, U> {
    pub fn cast<NewT, NewU>(&self) -> KeyPath<NewT, NewU>
    where
        NewT: Clone + Copy + NumCast,
    {
        KeyPath {
            points: self.points.iter().map(|p| p.cast()).collect(),
        }
    }
}

pub type KeyIndexPath = KeyPath<usize, IndexSpace>;
pub type KeyFloatPath<U> = KeyPath<f32, U>;

impl KeyFloatPath<WorkSpace> {
    pub fn from_lines<I>(path: I) -> Self
    where
        I: IntoIterator<Item = WorkPoint>,
    {
        Self {
            points: path
                .into_iter()
                .map(|p| KeyFloatPoint::new(p.cast(), KeyPointType::Line))
                .collect(),
        }
    }
}
