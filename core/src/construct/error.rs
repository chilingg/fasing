use serde::{Deserialize, Serialize};
use std::{error, fmt};

#[derive(Debug, Serialize, Deserialize)]
pub enum CstError {
    Empty(String),
    AxisTransform {
        axis: crate::axis::Axis,
        length: f32,
        base_len: usize,
    },
}

impl fmt::Display for CstError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(name) => write!(f, "`{name}` is empty!"),
            Self::AxisTransform {
                axis,
                length,
                base_len,
            } => write!(
                f,
                "The minimum length {} greater than {} in {:?}!",
                base_len, length, axis
            ),
        }
    }
}

impl error::Error for CstError {}
