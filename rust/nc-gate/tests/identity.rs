//! Regression tests for daemon identity KDL parsing, rendering, and file I/O.

use std::fs;

use nostr_sdk::{Keys, ToBech32};

use nc_gate::format_iso8601;
use nc_gate::identity::io::{load_identity, parse_identity_str, render_identity, save_identity};

// --- KDL round-trip ---

#[test]
fn round_trip_render_and_parse() {
    let keys = Keys::generate();
    let created = "2026-03-26T12:00:00Z";
    let kdl = render_identity(&keys, created).expect("render failed");
    let loaded = parse_identity_str(&kdl).expect("parse failed");

    assert_eq!(
        loaded.keys.public_key().to_bech32().unwrap(),
        keys.public_key().to_bech32().unwrap(),
        "public keys must match after round-trip"
    );
    assert_eq!(loaded.created, created);
}

#[test]
fn parse_canonical_kdl() {
    // Minimal well-formed identity.kdl
    let keys = Keys::generate();
    let nsec = keys.secret_key().to_bech32().unwrap();
    let kdl = format!("daemon nsec=\"{nsec}\" created=\"2026-01-01T00:00:00Z\"\n");
    let loaded = parse_identity_str(&kdl).expect("parse failed");
    assert_eq!(
        loaded.keys.public_key().to_bech32().unwrap(),
        keys.public_key().to_bech32().unwrap()
    );
    assert_eq!(loaded.created, "2026-01-01T00:00:00Z");
}

#[test]
fn parse_missing_daemon_node_is_error() {
    let result = parse_identity_str("other-node nsec=\"x\" created=\"y\"");
    assert!(result.is_err(), "missing daemon node should fail");
}

#[test]
fn parse_missing_nsec_is_error() {
    let result = parse_identity_str("daemon created=\"2026-01-01T00:00:00Z\"");
    assert!(result.is_err(), "missing nsec should fail");
}

#[test]
fn parse_invalid_nsec_is_error() {
    let result =
        parse_identity_str("daemon nsec=\"not-a-valid-key\" created=\"2026-01-01T00:00:00Z\"");
    assert!(result.is_err(), "invalid nsec should fail");
}

// --- File I/O ---

#[test]
fn save_and_load_identity_via_file() {
    let dir = std::env::temp_dir().join("nc-gate-identity-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("identity.kdl");

    let keys = Keys::generate();
    let created = "2026-03-26T00:00:00Z";
    save_identity(&path, &keys, created).expect("save failed");
    assert!(path.exists(), "identity.kdl should exist after save");

    let loaded = load_identity(&path).expect("load failed");
    assert_eq!(
        loaded.keys.public_key().to_bech32().unwrap(),
        keys.public_key().to_bech32().unwrap()
    );
    assert_eq!(loaded.created, created);

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn save_identity_creates_parent_directory() {
    let dir = std::env::temp_dir().join("nc-gate-identity-mkdir-test");
    // Ensure the directory does not exist before the test.
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("identity.kdl");

    let keys = Keys::generate();
    save_identity(&path, &keys, "2026-01-01T00:00:00Z").expect("save failed");
    assert!(path.exists());

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

#[test]
fn save_identity_is_atomic_no_tmp_leftover() {
    let dir = std::env::temp_dir().join("nc-gate-identity-atomic-test");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("identity.kdl");
    let tmp = dir.join("identity.kdl.tmp");

    save_identity(&path, &Keys::generate(), "2026-01-01T00:00:00Z").expect("save failed");
    assert!(path.exists());
    assert!(!tmp.exists(), ".tmp file must be gone after rename");

    fs::remove_file(&path).ok();
    fs::remove_dir(&dir).ok();
}

// --- ISO-8601 timestamp formatting ---

#[test]
fn format_iso8601_epoch() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
}

#[test]
fn format_iso8601_known_timestamp() {
    // 2026-03-26 12:00:00 UTC = 1774526400 seconds since epoch
    assert_eq!(format_iso8601(1774526400), "2026-03-26T12:00:00Z");
}

#[test]
fn format_iso8601_leap_day() {
    // 2024-02-29 00:00:00 UTC
    // date -d "2024-02-29 00:00:00 UTC" +%s  → 1709164800
    assert_eq!(format_iso8601(1709164800), "2024-02-29T00:00:00Z");
}

#[test]
fn format_iso8601_end_of_year() {
    // 2023-12-31 23:59:59 UTC = 1704067199
    assert_eq!(format_iso8601(1704067199), "2023-12-31T23:59:59Z");
}
