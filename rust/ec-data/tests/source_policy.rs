use std::fs;
use std::path::{Path, PathBuf};

fn source_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

fn is_allowed_raw_path(path: &Path) -> bool {
    let path = path.to_string_lossy();
    path.contains("/src/records/") || path.ends_with("/src/storage/snapshot_core.rs")
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
            (!is_comment && line.contains(".raw[")).then(|| line.to_string())
        })
        .collect()
}

#[test]
fn shared_runtime_code_avoids_direct_raw_offsets_outside_record_boundaries() {
    let mut files = Vec::new();
    source_files(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );

    let offenders = files
        .into_iter()
        .filter(|path| !is_allowed_raw_path(path))
        .filter_map(|path| {
            let matches = non_comment_raw_lines(&path);
            (!matches.is_empty()).then_some((path, matches))
        })
        .collect::<Vec<_>>();

    assert!(
        offenders.is_empty(),
        "ec-data runtime/shared code should use record accessors, not .raw offsets: {offenders:#?}"
    );
}
