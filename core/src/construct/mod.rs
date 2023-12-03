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
        base_length: usize,
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
                base_length: base_total,
            } => format!(
                "The minimum length {} greater than {} in {:?}!",
                base_total, length, axis
            ),
            Self::Surround { place, comp } => {
                format!("Components `{}` cannot be surrounded by {:?}", comp, place)
            }
        }
    }
}
