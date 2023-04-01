pub mod construct;
pub mod fas_file;
pub mod struc;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    Horizontal,
    Vertical,
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

    pub fn get(&self, axis: Axis) -> &T {
        match axis {
            Axis::Horizontal => &self.h,
            Axis::Vertical => &self.v,
        }
    }

    pub fn get_mut(&mut self, axis: Axis) -> &mut T {
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
