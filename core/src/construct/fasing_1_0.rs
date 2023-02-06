use super::char_construct::*;
use crate::fas_file::FasFile;

pub fn generate_table() -> Table {
    const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));

    super::table_from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
}

pub fn generate_fas_file() -> FasFile {
    serde_json::from_str(include_str!("../../tmp/fasing_1_0.fas")).unwrap()
}