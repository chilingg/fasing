#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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

    pub fn list() -> std::array::IntoIter<Self, 2> {
        static LIST: [Axis; 2] = [Axis::Horizontal, Axis::Vertical];
        LIST.into_iter()
    }
}

pub trait ValueHV<T> {
    fn hv_get(&self, axis: Axis) -> &T;
    fn hv_get_mut(&mut self, axis: Axis) -> &mut T;

    fn hv_iter(&self) -> std::array::IntoIter<&T, 2> {
        [self.hv_get(Axis::Horizontal), self.hv_get(Axis::Vertical)].into_iter()
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