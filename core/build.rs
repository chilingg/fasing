use std::env;
use std::path::Path;

use anyhow::Result;

fn process_data(
    value: &mut serde_json::Value,
    format_maps: &std::collections::HashMap<String, String>,
) {
    if value.is_object() {
        let attr = value.as_object_mut().unwrap();
        let mut array: Vec<serde_json::Value> = vec![];
        array.push(serde_json::Value::String(
            format_maps[attr["format"].as_str().unwrap()].clone(),
        ));
        array.push(attr.get("components").unwrap().clone());

        for comp in array[1].as_array_mut().unwrap() {
            if comp.is_object() {
                process_data(comp, format_maps);
            } else {
                let comp_str = comp.as_str().unwrap().to_owned();
                let mut chars = comp_str.chars();
                if let Some('>') = chars.nth(1) {
                    *comp = serde_json::Value::String(chars.next().unwrap().to_string());
                }
            }
        }
        *value = serde_json::Value::Array(array);
    }
}

fn main() -> Result<()> {
    let format_maps: std::collections::HashMap<String, String> = std::collections::HashMap::from([
        ("单体".to_string(), "".to_string()),
        ("上三包围".to_string(), "⿵".to_string()),
        ("上下".to_string(), "⿱".to_string()),
        ("上中下".to_string(), "⿱".to_string()),
        ("下三包围".to_string(), "⿶".to_string()),
        ("全包围".to_string(), "⿴".to_string()),
        ("右上包围".to_string(), "⿹".to_string()),
        ("左三包围".to_string(), "⿷".to_string()),
        ("左上包围".to_string(), "⿸".to_string()),
        ("左下包围".to_string(), "⿺".to_string()),
        ("左中右".to_string(), "⿰".to_string()),
        ("左右".to_string(), "⿰".to_string()),
    ]);

    let src_path =
        Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("../hanzi-jiegou/hanzi-jiegou.json");
    println!("cargo:rerun-if-changed={}", src_path.display());

    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("fasing_1_0.json");

    if src_path.exists() {
        let mut src_data: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(src_path)?)?;
        src_data
            .as_object_mut()
            .unwrap()
            .iter_mut()
            .for_each(|(_, attr)| {
                process_data(attr, &format_maps);
            });

        std::fs::write(dest_path, serde_json::to_string(&src_data).unwrap())?;
    }

    Ok(())
}
