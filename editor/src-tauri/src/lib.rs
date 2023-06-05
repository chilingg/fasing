use directories_next::ProjectDirs;
use serde_json::json;
use std::{fs, path::PathBuf};

pub struct Context {
    path: PathBuf,
    data: serde_json::Map<String, serde_json::Value>,
}

impl Default for Context {
    fn default() -> Self {
        let proj_dirs = ProjectDirs::from("com", "fasing", "Fasing Editor")
            .expect("Error accessing Project directory!");
        fs::create_dir_all(proj_dirs.data_dir()).expect("Error creating project directory!");

        let path = proj_dirs.data_dir().join("context.json").to_path_buf();
        let content = match fs::read_to_string(&path) {
            Ok(str) => str,
            Err(e) => {
                match e.kind() {
                    std::io::ErrorKind::NotFound => {}
                    kind => println!("{:?}", kind),
                }
                String::from("{}")
            }
        };

        Self {
            path,
            data: serde_json::from_str::<serde_json::Value>(&content)
                .expect("Error reading context!")
                .as_object()
                .unwrap()
                .clone(),
        }
    }
}

impl Context {
    fn recursion(
        source: &serde_json::Map<String, serde_json::Value>,
        index: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        match &index[0] {
            serde_json::Value::String(key) => match source.get(key) {
                Some(value) => match index.len() {
                    1 => Some(value.clone()),
                    _ if value.is_object() => {
                        Self::recursion(value.as_object().unwrap(), &index[1..])
                    }
                    _ => None,
                },
                None => None,
            },
            _ => None,
        }
    }

    fn recursion_set<'a>(
        source: &'a mut serde_json::Map<String, serde_json::Value>,
        index: &[serde_json::Value],
        value: serde_json::Value,
    ) -> bool {
        match &index[0] {
            serde_json::Value::String(key) => {
                let next = source.entry(key).or_insert(json!({}));
                match index.len() {
                    1 => {
                        *next = value;
                        true
                    }
                    _ if next.is_object() => {
                        Self::recursion_set(next.as_object_mut().unwrap(), &index[1..], value)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn get(&self, index: serde_json::Value) -> Option<serde_json::Value> {
        match &index {
            serde_json::Value::Array(array) => {
                if array.is_empty() {
                    None
                } else {
                    Self::recursion(&self.data, array)
                }
            }
            serde_json::Value::String(key) => self.data.get(key).cloned(),
            _ => None,
        }
    }

    pub fn set(&mut self, index: serde_json::Value, value: serde_json::Value) -> bool {
        match &index {
            serde_json::Value::Array(array) => {
                if array.is_empty() {
                    false
                } else {
                    Self::recursion_set(&mut self.data, array, value)
                }
            }
            serde_json::Value::String(key) => {
                self.data.insert(key.clone(), value);
                true
            }
            _ => false,
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        fs::write(
            &self.path,
            serde_json::to_string_pretty(&self.data).expect("Error pasing context!"),
        )
    }
}
