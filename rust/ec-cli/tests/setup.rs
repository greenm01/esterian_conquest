mod common;

use common::{cleanup_dir, copy_fixture_dir, run_ec_cli, run_ec_cli_in_dir, unique_temp_dir};

#[test]
fn maintenance_days_set_rewrites_conquest_schedule() {
    let target = unique_temp_dir("ec-cli-maint");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &[
            "maintenance-days",
            target.to_str().unwrap(),
            "set",
            "sun",
            "tue",
            "thu",
            "sat",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("sun=yes mon=no tue=yes wed=no thu=yes fri=no sat=yes"));
    assert!(stdout.contains("Maintenance raw: [01, 00, ca, 00, 0a, 00, 26]"));

    cleanup_dir(&target);
}

#[test]
fn snoop_off_rewrites_setup_flag() {
    let target = unique_temp_dir("ec-cli-snoop");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap(), "off"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["snoop", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Snoop enabled: no"));

    cleanup_dir(&target);
}

#[test]
fn purge_after_rewrites_setup_raw_value() {
    let target = unique_temp_dir("ec-cli-purge");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    let stdout = run_ec_cli_in_dir(
        &["purge-after", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Purge after turns (raw): 1"));

    cleanup_dir(&target);
}

#[test]
fn setup_programs_prints_mapped_f4_values() {
    let stdout = run_ec_cli(&["setup-programs", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F4 Modify Program Options"));
    assert!(stdout.contains("A Purge messages & reports after: 0 turn(s)"));
    assert!(stdout.contains("B Autopilot any empires inactive for: 0 turn(s)"));
    assert!(stdout.contains("C Snoop Enabled: Yes"));
    assert!(stdout.contains("D Enable timeout for local users: No"));
    assert!(stdout.contains("E Enable timeout for remote users: Yes"));
    assert!(stdout.contains("F Maximum time between key strokes: 10 minute(s)"));
    assert!(stdout.contains("G Minimum time granted: 0 minute(s)"));
}

#[test]
fn remaining_f4_commands_rewrite_setup_fields() {
    let target = unique_temp_dir("ec-cli-f4");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["local-timeout", target.to_str().unwrap(), "on"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Local timeout enabled: yes"));

    let stdout = run_ec_cli_in_dir(
        &["remote-timeout", target.to_str().unwrap(), "off"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Remote timeout enabled: no"));

    let stdout = run_ec_cli_in_dir(
        &["max-key-gap", target.to_str().unwrap(), "15"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Maximum time between key strokes (minutes): 15"));

    let stdout = run_ec_cli_in_dir(
        &["minimum-time", target.to_str().unwrap(), "69"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Minimum time granted (minutes): 69"));

    let stdout = run_ec_cli_in_dir(
        &["autopilot-after", target.to_str().unwrap(), "3"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Autopilot inactive turns (raw): 3"));

    let stdout = run_ec_cli_in_dir(
        &["setup-programs", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("D Enable timeout for local users: Yes"));
    assert!(stdout.contains("E Enable timeout for remote users: No"));
    assert!(stdout.contains("F Maximum time between key strokes: 15 minute(s)"));
    assert!(stdout.contains("G Minimum time granted: 69 minute(s)"));
    assert!(stdout.contains("B Autopilot any empires inactive for: 3 turn(s)"));

    cleanup_dir(&target);
}

#[test]
fn port_setup_prints_known_f5_values() {
    let stdout = run_ec_cli(&["port-setup", "original/v1.5"]);
    assert!(stdout.contains("ECUTIL F5 Modem / Com Port Setup"));
    assert!(stdout.contains("COM 1 IRQ: 4"));
    assert!(stdout.contains("COM 2 IRQ: 3"));
    assert!(stdout.contains("COM 3 IRQ: 4"));
    assert!(stdout.contains("COM 4 IRQ: 3"));
    assert!(stdout.contains("COM 1 Hardware Flow Control: Yes"));
    assert!(stdout.contains("COM 4 Hardware Flow Control: Yes"));
}

#[test]
fn flow_control_off_rewrites_setup_flag() {
    let target = unique_temp_dir("ec-cli-flow");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["flow-control", target.to_str().unwrap(), "com1", "off"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("COM 1 Hardware Flow Control: No"));

    let stdout = run_ec_cli_in_dir(
        &["port-setup", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("COM 1 Hardware Flow Control: No"));
    assert!(stdout.contains("COM 2 Hardware Flow Control: Yes"));

    cleanup_dir(&target);
}

#[test]
fn com_irq_rewrites_setup_value() {
    let target = unique_temp_dir("ec-cli-irq");
    copy_fixture_dir("original/v1.5", &target);

    let stdout = run_ec_cli_in_dir(
        &["com-irq", target.to_str().unwrap(), "com1", "7"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("COM 1 IRQ: 7"));

    let stdout = run_ec_cli_in_dir(
        &["port-setup", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("COM 1 IRQ: 7"));
    assert!(stdout.contains("COM 2 IRQ: 3"));

    cleanup_dir(&target);
}
