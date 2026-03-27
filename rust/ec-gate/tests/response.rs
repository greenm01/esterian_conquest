//! Regression tests for 30502/30503 response event construction (step 8).
//!
//! These tests exercise the pure logic: payload serialization, JSON escaping,
//! and the error payload format.  Publishing to a live relay is not tested here.

use ec_gate::serve::response::{SessionReadyPayload, session_error_payload};
use ec_gate::serve::routing::{GameEntry, RouteError};

// ---------------------------------------------------------------------------
// SessionReady payload serialization
// ---------------------------------------------------------------------------

#[test]
fn session_ready_payload_json_basic() {
    let payload = SessionReadyPayload {
        game_id: "friday-night",
        ssh_host: "play.example.com",
        ssh_port: 22,
        ssh_user: "ecgame",
        game_name: "Friday Night EC",
        seat: 2,
    };
    let json = payload.to_json();
    assert!(json.contains(r#""game_id":"friday-night""#));
    assert!(json.contains(r#""ssh_host":"play.example.com""#));
    assert!(json.contains(r#""ssh_port":22"#));
    assert!(json.contains(r#""ssh_user":"ecgame""#));
    assert!(json.contains(r#""game_name":"Friday Night EC""#));
    assert!(json.contains(r#""seat":2"#));
}

#[test]
fn session_ready_payload_is_valid_json_structure() {
    let payload = SessionReadyPayload {
        game_id: "test",
        ssh_host: "host",
        ssh_port: 2222,
        ssh_user: "mag",
        game_name: "Test",
        seat: 1,
    };
    let json = payload.to_json();
    // Must start and end with braces.
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
}

// ---------------------------------------------------------------------------
// SessionError payload serialization
// ---------------------------------------------------------------------------

#[test]
fn error_payload_invalid_code() {
    let payload = session_error_payload(&RouteError::InvalidCode);
    assert!(payload.contains(r#""error":"invalid_code""#));
    assert!(payload.contains(r#""message":"#));
}

#[test]
fn error_payload_code_claimed() {
    let payload = session_error_payload(&RouteError::CodeClaimed);
    assert!(payload.contains(r#""error":"code_claimed""#));
}

#[test]
fn error_payload_unknown_player() {
    let payload = session_error_payload(&RouteError::UnknownPlayer);
    assert!(payload.contains(r#""error":"unknown_player""#));
}

#[test]
fn error_payload_game_not_found() {
    let payload = session_error_payload(&RouteError::GameNotFound);
    assert!(payload.contains(r#""error":"game_not_found""#));
}

#[test]
fn error_payload_multiple_games_includes_game_list() {
    let games = vec![
        GameEntry {
            game_id: "friday-night".to_string(),
            game_name: "Friday Night EC".to_string(),
            player: 2,
        },
        GameEntry {
            game_id: "saturday-showdown".to_string(),
            game_name: "Saturday Showdown".to_string(),
            player: 5,
        },
    ];
    let payload = session_error_payload(&RouteError::MultipleGames(games));
    assert!(payload.contains(r#""error":"multiple_games""#));
    assert!(payload.contains(r#""friday-night""#));
    assert!(payload.contains(r#""saturday-showdown""#));
    assert!(payload.contains(r#""seat":2"#));
    assert!(payload.contains(r#""seat":5"#));
    assert!(payload.contains(r#""games":"#));
}

#[test]
fn error_payload_multiple_games_empty_list() {
    let payload = session_error_payload(&RouteError::MultipleGames(vec![]));
    assert!(payload.contains(r#""error":"multiple_games""#));
    assert!(payload.contains(r#""games":[]"#));
}

#[test]
fn error_payload_message_escapes_quotes() {
    // RouteError::GameNotFound produces "game not found"
    // This test verifies the escape path by constructing an error with a
    // message that would need escaping if it contained quotes. The current
    // Display impls use plain ASCII, so this mainly checks the json is valid.
    let payload = session_error_payload(&RouteError::InvalidCode);
    // No raw unescaped double-quotes inside string values.
    // The outermost braces and key/value quotes are expected; the string
    // *values* between :" and " should not contain unescaped quotes.
    let value_re = regex_free_check_no_unescaped_quotes_in_values(&payload);
    assert!(
        value_re,
        "JSON values should not contain unescaped double-quotes: {payload}"
    );
}

/// Naive check: after removing all `\"` sequences, no `"` appears inside a
/// string value (between a `:` and the closing `"`).  This is not a full JSON
/// parser but is sufficient for the simple fixed-template output we produce.
fn regex_free_check_no_unescaped_quotes_in_values(json: &str) -> bool {
    // Remove all escaped quotes first.
    let cleaned = json.replace("\\\"", "");
    // Now no value string should contain a lone `"` in the middle.
    // Count quotes: must be even (opening/closing pairs only).
    cleaned.chars().filter(|&c| c == '"').count() % 2 == 0
}

// ---------------------------------------------------------------------------
// NIP-44 round-trip (encrypt + decrypt)
// ---------------------------------------------------------------------------

#[test]
fn nip44_round_trip_session_ready_payload() {
    use nostr_sdk::Keys;
    use nostr_sdk::nips::nip44;
    use nostr_sdk::nips::nip44::Version;

    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();

    let payload = SessionReadyPayload {
        game_id: "friday-night",
        ssh_host: "play.example.com",
        ssh_port: 22,
        ssh_user: "ecgame",
        game_name: "Friday Night EC",
        seat: 3,
    };
    let plaintext = payload.to_json();

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &plaintext,
        Version::V2,
    )
    .expect("encrypt should succeed");

    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .expect("decrypt should succeed");

    assert_eq!(decrypted, plaintext);
}

#[test]
fn nip44_round_trip_session_error_payload() {
    use nostr_sdk::Keys;
    use nostr_sdk::nips::nip44;
    use nostr_sdk::nips::nip44::Version;

    let gate_keys = Keys::generate();
    let player_keys = Keys::generate();

    let plaintext = session_error_payload(&RouteError::InvalidCode);

    let encrypted = nip44::encrypt(
        gate_keys.secret_key(),
        &player_keys.public_key(),
        &plaintext,
        Version::V2,
    )
    .expect("encrypt should succeed");

    let decrypted = nip44::decrypt(
        player_keys.secret_key(),
        &gate_keys.public_key(),
        &encrypted,
    )
    .expect("decrypt should succeed");

    assert_eq!(decrypted, plaintext);
}
