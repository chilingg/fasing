use crate::axis::*;

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

pub trait BoxExpand<U> {
    fn contains_include(&self, p: Point2D<f32, U>, offset: f32) -> bool;
}

impl<U> BoxExpand<U> for Box2D<f32, U> {
    fn contains_include(&self, p: Point2D<f32, U>, offset: f32) -> bool {
        self.min.x - offset <= p.x
            && p.x <= self.max.x + offset
            && self.min.y - offset <= p.y
            && p.y <= self.max.y + offset
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct KeyPath {
    pub points: Vec<IndexPoint>,
    pub hide: bool,
}

impl<T: IntoIterator<Item = IndexPoint>> From<T> for KeyPath {
    fn from(value: T) -> Self {
        KeyPath {
            points: value.into_iter().collect(),
            hide: false,
        }
    }
}

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
