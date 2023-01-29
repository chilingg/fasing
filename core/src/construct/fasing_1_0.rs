use super::char_construct::*;

pub fn generate() -> Table {
    const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));

    super::table_from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
}
