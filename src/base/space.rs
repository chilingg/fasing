use super::axis::*;

use euclid::*;
use serde::{Deserialize, Serialize};

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

impl<T, U> ValueHV<T> for euclid::Point2D<T, U> {
    fn hv_get(&self, axis: Axis) -> &T {
        match axis {
            Axis::Horizontal => &self.x,
            Axis::Vertical => &self.y,
        }
    }

    fn hv_get_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.x,
            Axis::Vertical => &mut self.y,
        }
    }
}

impl<T, U> ValueHV<T> for euclid::Vector2D<T, U> {
    fn hv_get(&self, axis: Axis) -> &T {
        match axis {
            Axis::Horizontal => &self.x,
            Axis::Vertical => &self.y,
        }
    }

    fn hv_get_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.x,
            Axis::Vertical => &mut self.y,
        }
    }
}

impl<T, U> ValueHV<T> for euclid::Size2D<T, U> {
    fn hv_get(&self, axis: Axis) -> &T {
        match axis {
            Axis::Horizontal => &self.width,
            Axis::Vertical => &self.height,
        }
    }

    fn hv_get_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.width,
            Axis::Vertical => &mut self.height,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct PosWeigth {
    pub from: usize,
    pub pos: usize,
    pub to: usize,
}

impl Default for PosWeigth {
    fn default() -> Self {
        Self {
            from: 1,
            pos: 0,
            to: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyPoint<T, U> {
    pub pos: Point2D<T, U>,
    pub weight: PosWeigth,
}

pub fn key_pos(x: usize, y: usize) -> KeyPoint<usize, IndexSpace> {
    KeyPoint {
        pos: IndexPoint::new(x, y),
        weight: Default::default(),
    }
}

impl<T, U> KeyPoint<T, U> {
    pub fn new(pos: Point2D<T, U>) -> Self {
        Self {
            pos,
            weight: Default::default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct KeyPath<T, U> {
    pub kpoints: Vec<KeyPoint<T, U>>,
    pub hide: bool,
}

impl<T, U, P: IntoIterator<Item = KeyPoint<T, U>>> From<P> for KeyPath<T, U> {
    fn from(value: P) -> Self {
        KeyPath {
            kpoints: value.into_iter().collect(),
            hide: false,
        }
    }
}

pub type IdxKeyPath = KeyPath<usize, IndexSpace>;
pub type WorkKeyPath = KeyPath<f32, WorkSpace>;
