use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use kdl::KdlDocument;

fn main() {
    println!("cargo::rerun-if-changed=src/builtins/");

    let builtins_dir = Path::new("src/builtins");
    let mut themes: Vec<(String, String)> = Vec::new(); // (file_stem, display_name)

    if let Ok(entries) = fs::read_dir(builtins_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "kdl") {
                let stem = path
                    .file_stem()
                    .expect("file has stem")
                    .to_str()
                    .expect("valid UTF-8")
                    .to_string();

                let contents = fs::read_to_string(&path).expect("readable KDL");
                let display_name = extract_meta_name(&contents).unwrap_or_else(|| stem.clone());

                themes.push((stem, display_name));
            }
        }
    }

    themes.sort_by(|a, b| a.0.cmp(&b.0));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR set");
    let dest = Path::new(&out_dir).join("builtins_generated.rs");

    let mut code = String::new();

    // include_str! constants
    for (stem, _) in &themes {
        let const_name = stem.replace('-', "_").to_uppercase();
        writeln!(
            code,
            "const {const_name}_KDL: &str = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/builtins/{stem}.kdl\"));"
        ).expect("write to String");
    }

    // Static name registry: &[(&str, &str)] — (kebab-id, display-name)
    writeln!(
        code,
        "\n/// Auto-generated list of (kebab-id, display-name) for all builtin themes."
    )
    .expect("write to String");
    writeln!(code, "const GENERATED_BUILTIN_NAMES: &[(&str, &str)] = &[").expect("write to String");
    for (stem, display) in &themes {
        let id = stem.replace('_', "-");
        let escaped = display.replace('\\', "\\\\").replace('"', "\\\"");
        writeln!(code, "    (\"{id}\", \"{escaped}\"),").expect("write to String");
    }
    writeln!(code, "];").expect("write to String");

    writeln!(code, "\n/// Auto-generated builtin file names.").expect("write to String");
    writeln!(code, "const GENERATED_BUILTIN_FILE_NAMES: &[&str] = &[").expect("write to String");
    for (stem, _) in &themes {
        writeln!(code, "    \"{stem}.kdl\",").expect("write to String");
    }
    writeln!(code, "];").expect("write to String");

    // load_kdl match
    writeln!(code, "\n/// Auto-generated KDL lookup by kebab-case ID.").expect("write to String");
    writeln!(
        code,
        "fn generated_load_kdl(id: &str) -> Option<&'static str> {{\n    match id {{"
    )
    .expect("write to String");
    for (stem, _) in &themes {
        let id = stem.replace('_', "-");
        let const_name = stem.replace('-', "_").to_uppercase();
        writeln!(code, "        \"{id}\" => Some({const_name}_KDL),").expect("write to String");
    }
    writeln!(code, "        _ => None,").expect("write to String");
    writeln!(code, "    }}\n}}").expect("write to String");

    // Count
    writeln!(
        code,
        "\n/// Number of auto-discovered builtin themes.\nconst GENERATED_BUILTIN_COUNT: usize = {};",
        themes.len()
    ).expect("write to String");

    fs::write(dest, code).expect("write generated code");
}

/// Extract the `name` field from the `meta` node.
fn extract_meta_name(kdl_source: &str) -> Option<String> {
    let document: KdlDocument = kdl_source.parse().ok()?;
    let meta = document
        .nodes()
        .iter()
        .find(|node| node.name().value() == "meta")?;
    meta.get("name")
        .and_then(|value| value.as_string())
        .map(str::to_string)
}
