use super::char_construct::*;
use crate::fas_file::FasFile;

pub fn generate_table() -> Table {
    const TABLE_STRING: &str = include_str!(concat!(env!("OUT_DIR"), "/fasing_1_0.json"));

    super::table_from_json_array(serde_json::from_str(TABLE_STRING).unwrap())
}

pub fn generate_fas_file() -> FasFile {
    serde_json::from_str(include_str!("../../tmp/fasing_1_0.fas")).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completeness() {
        let table = generate_table();
        let requests = all_requirements(&table);

        let mut misses = std::collections::HashSet::new();

        requests.into_iter().for_each(|name| {
            let mut chars = name.chars();
            let chr = chars.next().unwrap();
            if chars.next().is_none() && !table.contains_key(&chr) {
                misses.insert(chr);
            }
        });

        assert_eq!(misses, std::collections::HashSet::new());
    }
}