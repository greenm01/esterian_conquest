mod common;

use common::{cleanup_dir, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};

#[test]
fn compliance_report_summarizes_known_post_fixture_failures() {
    let stdout = run_ec_cli(&["compliance-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Compliance Report"));
    assert!(stdout.contains("FAIL guard-starbase-linkage:"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("Key words: player.starbase_count=0 player.ipbm_count=0"));
}

#[test]
fn compliance_report_summarizes_valid_parameterized_guard_starbase_directory() {
    let target = unique_temp_dir("ec-cli-compliance-report");

    run_ec_cli_in_dir(
        &["guard-starbase-init", target.to_str().unwrap(), "12", "9"],
        common::rust_workspace(),
    );

    let stdout = run_ec_cli_in_dir(
        &["compliance-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("OK   guard-starbase-linkage"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("fleet1.local_slot=1 fleet1.id=1"));
    assert!(stdout.contains("base1.summary=1 base1.id=1 base1.chain=1 coords=[12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn compliance_batch_report_summarizes_batch_directory_status() {
    let target = unique_temp_dir("ec-cli-compliance-batch");

    run_ec_cli_in_dir(
        &["guard-starbase-batch-init", target.to_str().unwrap(), "12:9", "14:7"],
        common::rust_workspace(),
    );

    let stdout = run_ec_cli_in_dir(
        &["compliance-batch-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Compliance Batch Report"));
    assert!(stdout.contains("x12-y09: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok"));
    assert!(stdout.contains("x14-y07: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok"));

    cleanup_dir(&target);
}

#[test]
fn validate_preserved_all_classifies_known_build_fixture() {
    let stdout = run_ec_cli(&[
        "validate-preserved",
        "fixtures/ecmaint-build-pre/v1.5",
        "all",
    ]);
    assert!(stdout.contains("OK   planet-build"));
    assert!(stdout.contains("FAIL fleet-order:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_classifies_known_fleet_fixture() {
    let stdout = run_ec_cli(&["validate", "fixtures/ecmaint-fleet-pre/v1.5", "all"]);
    assert!(stdout.contains("OK   fleet-order"));
    assert!(stdout.contains("FAIL planet-build:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_rejects_post_maint_fixture() {
    let stderr = common::run_ec_cli_failure_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("directory does not match any known accepted scenario"));
}

#[test]
fn validate_preserved_rejects_post_maint_fixture() {
    let stderr = common::run_ec_cli_failure_in_dir(
        &["validate-preserved", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stderr.contains("directory does not exactly match any preserved accepted scenario"));
}
