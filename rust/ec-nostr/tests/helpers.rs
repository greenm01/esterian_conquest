use ec_nostr::hash::sha256_hex;
use ec_nostr::json::{escape_json_string, extract_str, extract_u32};
use ec_nostr::nonce::random_nonce_hex;
use ec_nostr::timing::MAX_EVENT_AGE_SECS;

// ── hash ─────────────────────────────────────────────────────────────────────

#[test]
fn sha256_hex_known_vector() {
    // SHA-256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let digest = sha256_hex(b"");
    assert_eq!(
        digest,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_hex_invite_code() {
    let digest = sha256_hex("velvet-mountain".as_bytes());
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── json::extract_str ─────────────────────────────────────────────────────────

#[test]
fn extract_str_simple() {
    let json = r#"{"error":"invalid_code","message":"No match"}"#;
    assert_eq!(extract_str(json, "error").unwrap(), "invalid_code");
    assert_eq!(extract_str(json, "message").unwrap(), "No match");
}

#[test]
fn extract_str_with_escapes() {
    let json = r#"{"msg":"hello \"world\"\nfoo"}"#;
    assert_eq!(extract_str(json, "msg").unwrap(), "hello \"world\"\nfoo");
}

#[test]
fn extract_str_missing_key_is_err() {
    let json = r#"{"a":"1"}"#;
    assert!(extract_str(json, "b").is_err());
}

#[test]
fn extract_str_whitespace_around_colon() {
    let json = r#"{"key" : "value"}"#;
    assert_eq!(extract_str(json, "key").unwrap(), "value");
}

// ── json::extract_u32 ────────────────────────────────────────────────────────

#[test]
fn extract_u32_simple() {
    let json = r#"{"seat":3,"port":2222}"#;
    assert_eq!(extract_u32(json, "seat"), Some(3));
    assert_eq!(extract_u32(json, "port"), Some(2222));
}

#[test]
fn extract_u32_missing_is_none() {
    assert_eq!(extract_u32(r#"{"a":1}"#, "b"), None);
}

// ── json::escape_json_string ─────────────────────────────────────────────────

#[test]
fn escape_json_string_backslash_and_quote() {
    assert_eq!(escape_json_string(r#"a\"b"#), r#"a\\\"b"#);
}

#[test]
fn escape_json_string_control_chars() {
    assert_eq!(escape_json_string("a\nb\tc"), r#"a\nb\tc"#);
}

#[test]
fn escape_json_string_plain() {
    assert_eq!(escape_json_string("hello"), "hello");
}

// ── nonce ────────────────────────────────────────────────────────────────────

#[test]
fn random_nonce_hex_is_64_lowercase_hex_chars() {
    let nonce = random_nonce_hex();
    assert_eq!(nonce.len(), 64);
    assert!(nonce.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn random_nonce_hex_is_unique() {
    let a = random_nonce_hex();
    let b = random_nonce_hex();
    assert_ne!(a, b);
}

// ── timing ───────────────────────────────────────────────────────────────────

#[test]
fn max_event_age_is_60() {
    assert_eq!(MAX_EVENT_AGE_SECS, 60);
}

// is_event_stale requires constructing a nostr Event which needs signing;
// covered indirectly by gate integration tests.
