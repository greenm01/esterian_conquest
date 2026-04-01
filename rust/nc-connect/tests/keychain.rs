//! Regression tests for keychain crypto, types, and I/O (step 1).

use std::path::PathBuf;

use nc_connect::keychain::crypto::{
    KEY_LEN, NONCE_LEN, PBKDF2_ITERATIONS, SALT_LEN, decrypt_keychain, derive_key, encrypt_keychain,
};
use nc_connect::keychain::io::{
    format_iso8601, load_keychain_from, now_iso8601, parse_keychain_str, render_keychain,
    save_keychain_to,
};
use nc_connect::keychain::{Identity, IdentityType, Keychain};

// ---------------------------------------------------------------------------
// crypto: constants
// ---------------------------------------------------------------------------

#[test]
fn crypto_constants_are_correct() {
    assert_eq!(PBKDF2_ITERATIONS, 100_000);
    assert_eq!(SALT_LEN, 16);
    assert_eq!(NONCE_LEN, 12);
    assert_eq!(KEY_LEN, 32);
}

// ---------------------------------------------------------------------------
// crypto: derive_key
// ---------------------------------------------------------------------------

#[test]
fn derive_key_is_deterministic() {
    let salt = b"abcdefghijklmnop"; // 16 bytes
    let k1 = derive_key("hunter2", salt);
    let k2 = derive_key("hunter2", salt);
    assert_eq!(k1, k2);
}

#[test]
fn derive_key_differs_by_password() {
    let salt = b"abcdefghijklmnop";
    let k1 = derive_key("correct horse", salt);
    let k2 = derive_key("wrong password", salt);
    assert_ne!(k1, k2);
}

#[test]
fn derive_key_differs_by_salt() {
    let k1 = derive_key("password", b"salt1___salt1___"); // 16 bytes
    let k2 = derive_key("password", b"salt2___salt2___");
    assert_ne!(k1, k2);
}

#[test]
fn derive_key_output_length_is_32() {
    let key: [u8; KEY_LEN] = derive_key("x", b"0000000000000000");
    assert_eq!(key.len(), 32);
}

// ---------------------------------------------------------------------------
// crypto: encrypt / decrypt round-trip
// ---------------------------------------------------------------------------

#[test]
fn encrypt_decrypt_roundtrip() {
    let plaintext = "hello, keychain";
    let password = "s3cr3t";
    let envelope = encrypt_keychain(plaintext, password).unwrap();
    let recovered = decrypt_keychain(&envelope, password).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn encrypt_produces_random_envelopes() {
    let plaintext = "same plaintext";
    let password = "same password";
    let e1 = encrypt_keychain(plaintext, password).unwrap();
    let e2 = encrypt_keychain(plaintext, password).unwrap();
    // Two encryptions of the same data must differ (different random salt+nonce).
    assert_ne!(e1, e2);
}

#[test]
fn decrypt_rejects_wrong_password() {
    let envelope = encrypt_keychain("secret data", "correct").unwrap();
    let result = decrypt_keychain(&envelope, "wrong");
    assert!(result.is_err(), "wrong password should fail decryption");
}

#[test]
fn decrypt_rejects_truncated_envelope() {
    let short = vec![0u8; SALT_LEN + NONCE_LEN - 1];
    let result = decrypt_keychain(&short, "any");
    assert!(result.is_err(), "truncated envelope should fail");
}

#[test]
fn decrypt_rejects_empty_envelope() {
    let result = decrypt_keychain(&[], "any");
    assert!(result.is_err());
}

#[test]
fn envelope_header_layout() {
    // The first SALT_LEN + NONCE_LEN bytes are the header; the rest is ciphertext+tag.
    let plaintext = "test";
    let envelope = encrypt_keychain(plaintext, "pw").unwrap();
    // Minimum length: header (28) + at least 1 ciphertext byte + 16-byte tag.
    assert!(envelope.len() >= SALT_LEN + NONCE_LEN + 1 + 16);
}

// ---------------------------------------------------------------------------
// keychain types
// ---------------------------------------------------------------------------

#[test]
fn identity_type_roundtrip() {
    assert_eq!(IdentityType::Local.as_str(), "local");
    assert_eq!(IdentityType::Imported.as_str(), "imported");
    assert_eq!(IdentityType::parse("local").unwrap(), IdentityType::Local);
    assert_eq!(
        IdentityType::parse("imported").unwrap(),
        IdentityType::Imported
    );
}

#[test]
fn identity_type_parse_unknown_is_err() {
    assert!(IdentityType::parse("robot").is_err());
}

#[test]
fn keychain_empty() {
    let w = Keychain::empty();
    assert_eq!(w.active, 0);
    assert!(w.identities.is_empty());
    assert!(w.active_identity().is_none());
}

#[test]
fn keychain_active_identity() {
    let mut w = Keychain::empty();
    w.identities.push(Identity {
        nsec: "nsec1aaa".to_string(),
        identity_type: IdentityType::Local,
        created: "2026-01-01T00:00:00Z".to_string(),
        alias: None,
    });
    w.identities.push(Identity {
        nsec: "nsec1bbb".to_string(),
        identity_type: IdentityType::Imported,
        created: "2026-01-02T00:00:00Z".to_string(),
        alias: None,
    });
    w.active = 1;
    let ai = w.active_identity().unwrap();
    assert_eq!(ai.nsec, "nsec1bbb");
}

#[test]
fn keychain_active_identity_out_of_bounds_returns_none() {
    let mut w = Keychain::empty();
    w.active = 5; // no identities at all
    assert!(w.active_identity().is_none());
}

// ---------------------------------------------------------------------------
// io: parse_keychain_str / render_keychain round-trip
// ---------------------------------------------------------------------------

const SAMPLE_KDL: &str = r#"keychain active="1"
identity nsec="nsec1aaa" type="local" created="2026-03-01T00:00:00Z"
identity nsec="nsec1bbb" type="imported" created="2026-03-02T00:00:00Z"
"#;

#[test]
fn parse_keychain_str_basic() {
    let w = parse_keychain_str(SAMPLE_KDL).unwrap();
    assert_eq!(w.active, 1);
    assert_eq!(w.identities.len(), 2);
    assert_eq!(w.identities[0].nsec, "nsec1aaa");
    assert_eq!(w.identities[0].identity_type, IdentityType::Local);
    assert_eq!(w.identities[0].created, "2026-03-01T00:00:00Z");
    assert_eq!(w.identities[1].nsec, "nsec1bbb");
    assert_eq!(w.identities[1].identity_type, IdentityType::Imported);
}

#[test]
fn render_keychain_basic() {
    let w = parse_keychain_str(SAMPLE_KDL).unwrap();
    let rendered = render_keychain(&w);
    // Re-parse the rendered string and verify round-trip.
    let w2 = parse_keychain_str(&rendered).unwrap();
    assert_eq!(w2.active, 1);
    assert_eq!(w2.identities.len(), 2);
    assert_eq!(w2.identities[0].nsec, "nsec1aaa");
    assert_eq!(w2.identities[1].identity_type, IdentityType::Imported);
}

#[test]
fn parse_empty_keychain() {
    let kdl = "keychain active=\"0\"\n";
    let w = parse_keychain_str(kdl).unwrap();
    assert_eq!(w.active, 0);
    assert!(w.identities.is_empty());
}

#[test]
fn parse_keychain_missing_keychain_node_is_err() {
    let kdl = "identity nsec=\"nsec1x\" type=\"local\" created=\"2026-01-01T00:00:00Z\"\n";
    assert!(parse_keychain_str(kdl).is_err());
}

#[test]
fn parse_keychain_bad_active_is_err() {
    let kdl = "keychain active=\"not-a-number\"\n";
    assert!(parse_keychain_str(kdl).is_err());
}

#[test]
fn parse_keychain_missing_nsec_is_err() {
    let kdl = "keychain active=\"0\"\nidentity type=\"local\" created=\"2026-01-01T00:00:00Z\"\n";
    assert!(parse_keychain_str(kdl).is_err());
}

#[test]
fn render_keychain_escapes_quotes() {
    // A keychain with a (contrived) nsec containing a double-quote should not break KDL.
    let mut w = Keychain::empty();
    w.identities.push(Identity {
        nsec: "nsec1\"quoted\"".to_string(),
        identity_type: IdentityType::Local,
        created: "2026-01-01T00:00:00Z".to_string(),
        alias: None,
    });
    let rendered = render_keychain(&w);
    // Must round-trip without a parse error.
    let w2 = parse_keychain_str(&rendered).unwrap();
    assert_eq!(w2.identities[0].nsec, "nsec1\"quoted\"");
}

// ---------------------------------------------------------------------------
// io: save_keychain_to / load_keychain_from round-trip
// ---------------------------------------------------------------------------

fn tmp_keychain_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("nc_connect_test_keychain_{name}.bin"));
    p
}

#[test]
fn save_load_roundtrip_single_identity() {
    let path = tmp_keychain_path("single");
    // Clean up any leftover from a prior run.
    let _ = std::fs::remove_file(&path);

    let mut w = Keychain::empty();
    w.identities.push(Identity {
        nsec: "nsec1test".to_string(),
        identity_type: IdentityType::Local,
        created: "2026-03-26T00:00:00Z".to_string(),
        alias: None,
    });

    save_keychain_to(&w, "mypassword", &path).unwrap();
    let loaded = load_keychain_from("mypassword", &path).unwrap().unwrap();

    assert_eq!(loaded.active, 0);
    assert_eq!(loaded.identities.len(), 1);
    assert_eq!(loaded.identities[0].nsec, "nsec1test");
    assert_eq!(loaded.identities[0].identity_type, IdentityType::Local);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_keychain_from_missing_file_returns_none() {
    let path = tmp_keychain_path("missing_xyz_99999");
    let _ = std::fs::remove_file(&path);
    let result = load_keychain_from("pw", &path).unwrap();
    assert!(result.is_none());
}

#[test]
fn load_keychain_from_wrong_password_is_err() {
    let path = tmp_keychain_path("wrongpw");
    let _ = std::fs::remove_file(&path);

    let w = Keychain::empty();
    save_keychain_to(&w, "correct", &path).unwrap();

    let result = load_keychain_from("wrong", &path);
    assert!(result.is_err(), "wrong password should yield an error");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_load_preserves_active_and_multiple_identities() {
    let path = tmp_keychain_path("multi");
    let _ = std::fs::remove_file(&path);

    let mut w = Keychain::empty();
    w.identities.push(Identity {
        nsec: "nsec1first".to_string(),
        identity_type: IdentityType::Local,
        created: "2026-01-01T00:00:00Z".to_string(),
        alias: None,
    });
    w.identities.push(Identity {
        nsec: "nsec1second".to_string(),
        identity_type: IdentityType::Imported,
        created: "2026-01-02T12:30:00Z".to_string(),
        alias: None,
    });
    w.active = 1;

    save_keychain_to(&w, "pw", &path).unwrap();
    let loaded = load_keychain_from("pw", &path).unwrap().unwrap();

    assert_eq!(loaded.active, 1);
    assert_eq!(loaded.identities.len(), 2);
    assert_eq!(loaded.identities[1].nsec, "nsec1second");
    assert_eq!(loaded.identities[1].identity_type, IdentityType::Imported);

    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// io: format_iso8601 / now_iso8601
// ---------------------------------------------------------------------------

#[test]
fn format_iso8601_epoch() {
    assert_eq!(format_iso8601(0), "1970-01-01T00:00:00Z");
}

#[test]
fn format_iso8601_known_timestamps() {
    // Unix timestamp 1774742400 = 2026-03-29T00:00:00Z (verified via Python)
    assert_eq!(format_iso8601(1_774_742_400), "2026-03-29T00:00:00Z");
    // Unix timestamp 1711670400 = 2024-03-29T00:00:00Z (leap year, valid post-Feb-28)
    assert_eq!(format_iso8601(1_711_670_400), "2024-03-29T00:00:00Z");
}

#[test]
fn format_iso8601_leap_year_feb_29() {
    // 2000-02-29T00:00:00Z = 951782400
    assert_eq!(format_iso8601(951_782_400), "2000-02-29T00:00:00Z");
}

#[test]
fn format_iso8601_non_midnight() {
    // 2000-01-01T12:34:56Z = 946729200 + 12*3600 + 34*60 + 56 = 946729200 + 45296 = 946774496
    // Actually: 2000-01-01T00:00:00Z = 946684800; + 12*3600+34*60+56 = 45296 => 946730096
    assert_eq!(format_iso8601(946_684_800 + 45_296), "2000-01-01T12:34:56Z");
}

#[test]
fn now_iso8601_looks_like_iso8601() {
    let s = now_iso8601();
    // Rough structural check: YYYY-MM-DDTHH:MM:SSZ
    assert_eq!(s.len(), 20);
    assert!(s.ends_with('Z'));
    assert_eq!(&s[4..5], "-");
    assert_eq!(&s[7..8], "-");
    assert_eq!(&s[10..11], "T");
    assert_eq!(&s[13..14], ":");
    assert_eq!(&s[16..17], ":");
}
