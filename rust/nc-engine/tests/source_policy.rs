use std::fs;
use std::path::{Path, PathBuf};

fn engine_source_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            engine_source_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn non_comment_raw_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .expect("read source")
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let is_comment = trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with("*/");
            let uses_banned_raw_pattern = line.contains(".raw[")
                || line.contains(".raw_byte(")
                || line.contains(".raw_word(")
                || line.contains(".set_raw_byte(")
                || line.contains(".set_raw_word(")
                || line.contains(".clear_raw_byte_if_equal(");
            (!is_comment && uses_banned_raw_pattern).then(|| line.to_string())
        })
        .collect()
}

#[test]
fn engine_runtime_code_does_not_use_direct_raw_offsets() {
    let mut files = Vec::new();
    engine_source_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );

    let offenders = files
        .into_iter()
        .filter_map(|path| {
            let matches = non_comment_raw_lines(&path);
            (!matches.is_empty()).then_some((path, matches))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "nc-engine runtime should use record accessors, not generic raw access helpers: {offenders:#?}"
    );
}
