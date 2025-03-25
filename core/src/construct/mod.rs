mod data;
pub use data::*;

pub mod space;

mod types;
pub use types::*;

mod error;
pub use error::*;

pub type Components = std::collections::BTreeMap<String, super::component::struc::StrucProto>;
