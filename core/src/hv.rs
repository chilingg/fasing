use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy)]
pub enum Place {
    Start,
    End,
}

impl Place {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::End => Self::Start,
        }
    }
}

#[derive(Serialize, Deserialize, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
}

impl Axis {
    pub fn inverse(&self) -> Self {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn list() -> std::array::IntoIter<Self, 2> {
        static LIST: [Axis; 2] = [Axis::Horizontal, Axis::Vertical];
        LIST.into_iter()
    }

    pub fn hv_data() -> DataHV<Self> {
        DataHV {
            h: Axis::Horizontal,
            v: Axis::Vertical,
        }
    }
}

pub trait ValueHV<T> {
    fn hv_get(&self, axis: Axis) -> &T;
    fn hv_get_mut(&mut self, axis: Axis) -> &mut T;

    fn hv_iter(&self) -> std::array::IntoIter<&T, 2> {
        [self.hv_get(Axis::Horizontal), self.hv_get(Axis::Vertical)].into_iter()
    }

    fn hv_axis_iter(&self) -> std::array::IntoIter<(&T, Axis), 2> {
        [
            (self.hv_get(Axis::Horizontal), Axis::Horizontal),
            (self.hv_get(Axis::Vertical), Axis::Vertical),
        ]
        .into_iter()
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

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataHV<T> {
    pub h: T,
    pub v: T,
}

impl<T> DataHV<T> {
    pub fn new(h: T, v: T) -> Self {
        Self { h, v }
    }

    pub fn splat(val: T) -> Self
    where
        T: Clone,
    {
        Self {
            h: val.clone(),
            v: val,
        }
    }

    pub fn map<T2, F>(&self, f: F) -> DataHV<T2>
    where
        F: Fn(&T) -> T2,
    {
        DataHV {
            h: f(&self.h),
            v: f(&self.v),
        }
    }

    pub fn into_map<T2, F>(self, f: F) -> DataHV<T2>
    where
        F: Fn(T) -> T2,
    {
        DataHV {
            h: f(self.h),
            v: f(self.v),
        }
    }

    pub fn into_iter(self) -> std::array::IntoIter<T, 2> {
        [self.h, self.v].into_iter()
    }

    pub fn zip<'a, T2>(&'a self, other: &'a DataHV<T2>) -> DataHV<(&'a T, &'a T2)> {
        DataHV {
            h: (&self.h, &other.h),
            v: (&self.v, &other.v),
        }
    }

    pub fn into_zip<T2>(self, other: DataHV<T2>) -> DataHV<(T, T2)> {
        DataHV {
            h: (self.h, other.h),
            v: (self.v, other.v),
        }
    }
}

impl<T> ValueHV<T> for DataHV<T> {
    fn hv_get(&self, axis: Axis) -> &T {
        match axis {
            Axis::Horizontal => &self.h,
            Axis::Vertical => &self.v,
        }
    }

    fn hv_get_mut(&mut self, axis: Axis) -> &mut T {
        match axis {
            Axis::Horizontal => &mut self.h,
            Axis::Vertical => &mut self.v,
        }
    }
}

impl<T> From<(T, T)> for DataHV<T> {
    fn from(value: (T, T)) -> Self {
        Self {
            h: value.0,
            v: value.1,
        }
    }
}

impl<T: Copy> Copy for DataHV<T> {}

impl<T: Clone> DataHV<T> {
    pub fn vh(&self) -> Self {
        Self {
            h: self.v.clone(),
            v: self.h.clone(),
        }
    }
}
