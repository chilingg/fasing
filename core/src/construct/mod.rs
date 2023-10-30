mod data;
pub use data::*;

pub mod space;

mod types;
pub use types::*;

pub type Components = std::collections::BTreeMap<String, super::component::struc::StrucProto>;

#[derive(Debug)]
pub enum Error {
    Empty(String),
    AxisTransform {
        axis: super::axis::Axis,
        length: f32,
        bases: Vec<f32>,
    },
    Surround {
        place: crate::axis::DataHV<crate::axis::Place>,
        comp: String,
    },
}

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Self::Empty(name) => format!("\"{}\" is empty!", name),
            Self::AxisTransform {
                axis,
                length,
                bases,
            } => format!(
                "The minimum length {:?} greater than {} in {:?}!",
                bases, length, axis
            ),
            Self::Surround { place, comp } => {
                format!("Components `{}` cannot be surrounded by {:?}", comp, place)
            }
        }
    }
}
