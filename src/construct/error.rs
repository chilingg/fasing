use serde::{Deserialize, Serialize};
use std::{error, fmt};

#[derive(Debug, Serialize, Deserialize)]
pub enum CstError {
    Empty(String),
    AxisTransform {
        axis: crate::base::Axis,
        length: f32,
        base_len: usize,
    },
    Surround {
        tp: char,
        comp: String,
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
                "The minimum length {} greater than {:.3} in {:?}!",
                base_len, length, axis
            ),
            Self::Surround { tp, comp } => {
                write!(f, "Components `{}` cannot be surrounded by {}", comp, tp)
            }
        }
    }
}

impl error::Error for CstError {}
