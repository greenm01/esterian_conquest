use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("read_dir");
    for entry in entries {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn table_render_files_do_not_use_legacy_footer_or_hanger_helpers() {
    let domains_root = repo_root().join("rust/nc-game/src/domains");
    let mut files = Vec::new();
    collect_rs_files(&domains_root, &mut files);

    let table_markers = [
        "write_table_window",
        "write_stacked_table_window",
        "write_split_table",
    ];
    let forbidden = [
        "draw_table_command_bar_at(",
        "draw_table_command_bar_at_col(",
        "draw_table_command_prompt_at(",
        "draw_table_command_prompt_at_col(",
        "draw_general_message_after_command(",
        "draw_command_message_stack(",
        "draw_command_message_stack_after(",
    ];

    let mut violations = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).expect("read source");
        if !table_markers.iter().any(|marker| source.contains(marker)) {
            continue;
        }
        let hits = forbidden
            .iter()
            .filter(|pattern| source.contains(**pattern))
            .copied()
            .collect::<Vec<_>>();
        if !hits.is_empty() {
            violations.push(format!(
                "{} -> {}",
                path.strip_prefix(repo_root()).unwrap_or(&path).display(),
                hits.join(", ")
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "legacy table footer/hanger helpers found:\n{}",
        violations.join("\n")
    );
}
