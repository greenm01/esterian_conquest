use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn client_renders_main_menu_shell_from_fixture() {
    let fixture_dir = repo_root().join("fixtures/ecutil-init/v1.5");
    let output = Command::new(env!("CARGO_BIN_EXE_ec-client"))
        .args([
            "--dir",
            fixture_dir.to_str().expect("fixture path should be utf-8"),
            "--player",
            "1",
        ])
        .output()
        .expect("ec-client should run");

    assert!(
        output.status.success(),
        "ec-client failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("ESTERIAN CONQUEST"));
    assert!(stdout.contains("MAIN MENU COMMANDS"));
    assert!(stdout.contains("GENERAL COMMAND MENU"));
    assert!(stdout.contains("Brief Empire Report"));
}
