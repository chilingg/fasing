use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub fn inverse(&self) -> Self {
        match self {
            Axis::Horizontal => Axis::Vertical,
            Axis::Vertical => Axis::Horizontal,
        }
    }

    pub fn list() -> [Axis; 2] {
        [Axis::Horizontal, Axis::Vertical]
    }

    pub fn hv_data() -> DataHV<Self> {
        DataHV {
            h: Axis::Horizontal,
            v: Axis::Vertical,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum LineType {
    HLine,
    VLine,
    DLine,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum Place {
    Start,
    Mind,
    End,
}

impl Place {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Start => Self::End,
            Self::Mind => Self::Mind,
            Self::End => Self::Start,
        }
    }

    pub fn start_and_end() -> [Place; 2] {
        [Place::Start, Place::End]
    }
}

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct DataHV<T> {
    pub h: T,
    pub v: T,
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

    pub fn in_axis<F>(&self, f: F) -> Option<Axis>
    where
        F: Fn(&T) -> bool,
    {
        if f(&self.h) {
            Some(Axis::Horizontal)
        } else if f(&self.v) {
            Some(Axis::Vertical)
        } else {
            None
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

    pub fn into_map<T2, F>(self, mut f: F) -> DataHV<T2>
    where
        F: FnMut(T) -> T2,
    {
        DataHV {
            h: f(self.h),
            v: f(self.v),
        }
    }

    pub fn zip<'a, T2>(self, other: DataHV<T2>) -> DataHV<(T, T2)> {
        DataHV {
            h: (self.h, other.h),
            v: (self.v, other.v),
        }
    }

    pub fn as_ref(&self) -> DataHV<&T> {
        DataHV {
            h: &self.h,
            v: &self.v,
        }
    }

    pub fn into_iter(self) -> std::array::IntoIter<T, 2> {
        [self.h, self.v].into_iter()
    }

    pub fn to_array(self) -> [T; 2] {
        [self.h, self.v]
    }
}

impl<T, U> DataHV<(T, U)> {
    pub fn unzip(self) -> (DataHV<T>, DataHV<U>) {
        (
            DataHV {
                h: self.h.0,
                v: self.v.0,
            },
            DataHV {
                h: self.h.1,
                v: self.v.1,
            },
        )
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

    fn to_hv_data(&self) -> DataHV<T>
    where
        T: Clone,
    {
        DataHV {
            h: self.hv_get(Axis::Horizontal).clone(),
            v: self.hv_get(Axis::Vertical).clone(),
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
