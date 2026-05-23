use super::axis::*;

use euclid::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexSpace;
pub type IndexPoint = Point2D<usize, IndexSpace>;
pub type IndexSize = Size2D<usize, IndexSpace>;
pub type IndexBox = Box2D<usize, IndexSpace>;

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

#[derive(Clone, Debug)]
pub struct KeyPoint<T, U> {
    pub pos: Point2D<T, U>,
    pub labels: Vec<String>,
}

impl<T: serde::Serialize, U> Serialize for KeyPoint<T, U> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut len = 1;
        if !self.labels.is_empty() {
            len += 1;
        }

        let mut state = serializer.serialize_struct("KeyPoint", len)?;
        state.serialize_field("pos", &self.pos)?;

        let key = "labels";
        if self.labels.is_empty() {
            state.skip_field(key)?;
        } else {
            state.serialize_field(key, &self.labels)?;
        }

        state.end()
    }
}

impl<'de, T, U> Deserialize<'de> for KeyPoint<T, U>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde_json as sj;

        let mut obj: sj::value::Map<_, _> = Deserialize::deserialize(deserializer)?;

        let key = "pos";
        let pos = obj
            .remove(key)
            .ok_or(serde::de::Error::missing_field(key))
            .and_then(|value| Deserialize::deserialize(value))
            .map_err(|e| serde::de::Error::custom(e))?;

        let key = "labels";
        let labels = obj
            .remove(key)
            .map(|val| sj::from_value::<Vec<String>>(val))
            .unwrap_or(Ok(vec![]))
            .map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self { pos, labels })
    }
}

pub fn key_pos(x: usize, y: usize) -> KeyPoint<usize, IndexSpace> {
    KeyPoint {
        pos: IndexPoint::new(x, y),
        labels: Default::default(),
    }
}

impl<T, U> KeyPoint<T, U> {
    pub fn new(pos: Point2D<T, U>) -> Self {
        Self {
            pos,
            labels: Default::default(),
        }
    }

    pub fn is_mark(&self) -> bool {
        self.labels.iter().any(|l| l == "mark")
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
pub type IdxKeyPoint = KeyPoint<usize, IndexSpace>;
pub type WorkKeyPoint = KeyPoint<f32, WorkSpace>;
// pub type WorkKeyPath = KeyPath<f32, WorkSpace>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_keypoint() {
        let mut json = serde_json::json!({"pos": [5,4]});
        let mut kp: IdxKeyPoint = serde_json::from_value(json).unwrap();
        assert!(kp.labels.is_empty());
        kp.labels.push("tip".to_string());

        json = serde_json::to_value(kp).unwrap();
        let labels: Vec<&str> = json
            .get("labels")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|val| val.as_str().unwrap())
            .collect();
        assert_eq!(labels, vec!["tip"]);

        kp = serde_json::from_value(json).unwrap();
        assert_eq!(kp.labels, vec!["tip"]);
    }
}
