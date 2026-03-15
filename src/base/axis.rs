use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug, PartialOrd, Ord)]
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

    pub fn hv() -> DataHV<Self> {
        DataHV {
            h: Axis::Horizontal,
            v: Axis::Vertical,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Horizontal => "h",
            Self::Vertical => "v",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum Side {
    Front,
    Back,
}

impl Side {
    pub fn inverse(&self) -> Self {
        match self {
            Self::Front => Self::Back,
            Self::Back => Self::Front,
        }
    }

    pub fn fb() -> [Side; 2] {
        [Side::Front, Side::Back]
    }

    pub fn n(&self) -> usize {
        match self {
            Self::Front => 0,
            Self::Back => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub enum Section {
    Start,
    Middle,
    End,
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

impl<T> DataHV<T> {
    pub fn vh(&mut self) -> &mut Self {
        std::mem::swap(&mut self.h, &mut self.v);
        self
    }

    pub fn to_vh(self) -> Self {
        Self {
            h: self.v,
            v: self.h,
        }
    }

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

    pub fn as_mut(&mut self) -> DataHV<&mut T> {
        DataHV {
            h: &mut self.h,
            v: &mut self.v,
        }
    }

    pub fn to_array(self) -> [T; 2] {
        [self.h, self.v]
    }
}

impl<T> IntoIterator for DataHV<T> {
    type Item = T;
    type IntoIter = std::array::IntoIter<Self::Item, 2>;

    fn into_iter(self) -> Self::IntoIter {
        [self.h, self.v].into_iter()
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

    fn hv_axis_iter(&self) -> std::array::IntoIter<(Axis, &T), 2> {
        [
            (Axis::Horizontal, self.hv_get(Axis::Horizontal)),
            (Axis::Vertical, self.hv_get(Axis::Vertical)),
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
