use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(root).expect("read_dir succeeds");
    for entry in entries {
        let entry = entry.expect("dir entry succeeds");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn non_menu_prompt_labels_do_not_use_menu_specific_command_names() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);

    let allowed = [
        "src/screen/mod.rs",
        "src/screen/layout.rs",
        "src/domains/startup/screens/main_menu.rs",
        "src/domains/startup/screens/general_menu.rs",
        "src/domains/startup/screens/first_time.rs",
        "src/domains/fleet/screens/fleet.rs",
        "src/domains/starbase/screens/starbase.rs",
        "src/domains/planet/screens/planet_menu.rs",
        "src/domains/planet/screens/planet_build.rs",
    ];
    let forbidden = [
        "\"MAIN COMMAND\",",
        "\"GENERAL COMMAND\",",
        "\"FLEET COMMAND\",",
        "\"STARBASE COMMAND\",",
        "\"PLANET COMMAND\",",
        "\"BUILD COMMAND\",",
        "\"FIRST TIME COMMAND\",",
        "\"COMMANDS\",",
    ];

    let mut offenders = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("strip prefix succeeds")
            .to_string_lossy()
            .replace('\\', "/");
        if allowed.contains(&rel.as_str()) {
            continue;
        }
        let body = fs::read_to_string(&path).expect("read_to_string succeeds");
        for token in forbidden {
            if body.contains(token) {
                offenders.push(format!("{rel}: {token}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "non-menu files still hardcode menu-specific prompt labels:\n{}",
        offenders.join("\n")
    );
}
