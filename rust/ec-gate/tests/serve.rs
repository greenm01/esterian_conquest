//! Regression tests for 30501 SessionRequest event parsing.

use std::time::{SystemTime, UNIX_EPOCH};

use nostr_sdk::{EventBuilder, Keys, Kind, Tag, TagKind, Timestamp};

use ec_gate::serve::request::{MAX_EVENT_AGE_SECS, ParseError, parse_session_request};

// --- helpers ---

fn now_ts() -> Timestamp {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Timestamp::from(secs)
}

fn past_ts(offset_secs: u64) -> Timestamp {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .saturating_sub(offset_secs);
    Timestamp::from(secs)
}

/// Build a minimal valid 30501 event signed by `keys`.
fn build_request(
    keys: &Keys,
    nonce: &str,
    ssh_pubkey: &str,
    invite_code: &str,
    game_id: Option<&str>,
    gate_keys: &Keys,
) -> nostr_sdk::Event {
    let mut tags = vec![
        Tag::parse(["d", nonce]).unwrap(),
        Tag::parse(["p", &gate_keys.public_key().to_hex()]).unwrap(),
        Tag::parse(["ssh-pubkey", ssh_pubkey]).unwrap(),
    ];
    if let Some(gid) = game_id {
        tags.push(Tag::parse(["game-id", gid]).unwrap());
    }

    EventBuilder::new(Kind::Custom(30501), invite_code)
        .tags(tags)
        .custom_created_at(now_ts())
        .sign_with_keys(keys)
        .expect("sign failed")
}

const SSH_PUBKEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIBGk6o5SBhAFJYjaSsajjbqkqFbIV4GV3iBLnlHnSvfG test";

// --- valid event tests ---

#[test]
fn parse_valid_request_with_invite_code() {
    let player = Keys::generate();
    let gate = Keys::generate();
    let event = build_request(
        &player,
        "abc123nonce",
        SSH_PUBKEY,
        "velvet-azure",
        None,
        &gate,
    );

    let req = parse_session_request(&event).expect("should parse");
    assert_eq!(req.nonce, "abc123nonce");
    assert_eq!(req.player_pubkey, player.public_key().to_hex());
    assert_eq!(req.ssh_pubkey, SSH_PUBKEY);
    assert_eq!(req.invite_code.as_deref(), Some("velvet-azure"));
    assert!(req.game_id.is_none());
}

#[test]
fn parse_valid_request_without_invite_code() {
    let player = Keys::generate();
    let gate = Keys::generate();
    let event = build_request(&player, "nonce456", SSH_PUBKEY, "", None, &gate);

    let req = parse_session_request(&event).expect("should parse");
    assert!(req.invite_code.is_none(), "empty content → no invite code");
}

#[test]
fn parse_valid_request_with_game_id() {
    let player = Keys::generate();
    let gate = Keys::generate();
    let event = build_request(
        &player,
        "nonce789",
        SSH_PUBKEY,
        "",
        Some("friday-night"),
        &gate,
    );

    let req = parse_session_request(&event).expect("should parse");
    assert_eq!(req.game_id.as_deref(), Some("friday-night"));
}

#[test]
fn parse_valid_request_invite_code_whitespace_trimmed() {
    let player = Keys::generate();
    let gate = Keys::generate();
    let event = build_request(&player, "nonceW", SSH_PUBKEY, "  abbey-zoom  ", None, &gate);

    let req = parse_session_request(&event).expect("should parse");
    // Whitespace-only invite codes are trimmed; non-empty result expected.
    assert_eq!(req.invite_code.as_deref(), Some("abbey-zoom"));
}

// --- error cases ---

#[test]
fn parse_wrong_kind_is_error() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::TextNote, "hello")
        .tags([Tag::parse(["d", "nonce"]).unwrap()])
        .custom_created_at(now_ts())
        .sign_with_keys(&keys)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::WrongKind(k)) => assert_eq!(k, 1),
        other => panic!("expected WrongKind, got {other:?}"),
    }
}

#[test]
fn parse_stale_event_is_error() {
    let player = Keys::generate();
    let gate = Keys::generate();

    // Build a well-formed event but stamp it as old.
    let stale_age = MAX_EVENT_AGE_SECS + 10;
    let tags = vec![
        Tag::parse(["d", "nonce-stale"]).unwrap(),
        Tag::parse(["p", &gate.public_key().to_hex()]).unwrap(),
        Tag::parse(["ssh-pubkey", SSH_PUBKEY]).unwrap(),
    ];
    let event = EventBuilder::new(Kind::Custom(30501), "")
        .tags(tags)
        .custom_created_at(past_ts(stale_age))
        .sign_with_keys(&player)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::Stale) => {}
        other => panic!("expected Stale, got {other:?}"),
    }
}

#[test]
fn parse_missing_nonce_is_error() {
    let keys = Keys::generate();
    // No `d` tag.
    let event = EventBuilder::new(Kind::Custom(30501), "")
        .tags([Tag::parse(["ssh-pubkey", SSH_PUBKEY]).unwrap()])
        .custom_created_at(now_ts())
        .sign_with_keys(&keys)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::MissingNonce) => {}
        other => panic!("expected MissingNonce, got {other:?}"),
    }
}

#[test]
fn parse_empty_nonce_is_error() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30501), "")
        .tags([
            Tag::parse(["d", ""]).unwrap(),
            Tag::parse(["ssh-pubkey", SSH_PUBKEY]).unwrap(),
        ])
        .custom_created_at(now_ts())
        .sign_with_keys(&keys)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::MissingNonce) => {}
        other => panic!("expected MissingNonce, got {other:?}"),
    }
}

#[test]
fn parse_missing_ssh_pubkey_is_error() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30501), "")
        .tags([Tag::parse(["d", "some-nonce"]).unwrap()])
        .custom_created_at(now_ts())
        .sign_with_keys(&keys)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::MissingSshPubkey) => {}
        other => panic!("expected MissingSshPubkey, got {other:?}"),
    }
}

#[test]
fn parse_empty_ssh_pubkey_is_error() {
    let keys = Keys::generate();
    let event = EventBuilder::new(Kind::Custom(30501), "")
        .tags([
            Tag::parse(["d", "some-nonce"]).unwrap(),
            Tag::parse(["ssh-pubkey", ""]).unwrap(),
        ])
        .custom_created_at(now_ts())
        .sign_with_keys(&keys)
        .unwrap();

    match parse_session_request(&event) {
        Err(ParseError::MissingSshPubkey) => {}
        other => panic!("expected MissingSshPubkey, got {other:?}"),
    }
}

// --- TagKind coverage ---

#[test]
fn tag_kind_as_str_matches_expected_names() {
    // Confirm that custom tag names round-trip correctly through TagKind::as_str.
    // This guards against API changes breaking the tag lookup in parse_session_request.
    let ssh_tag = Tag::parse(["ssh-pubkey", "val"]).unwrap();
    assert_eq!(ssh_tag.kind().as_str(), "ssh-pubkey");

    let gid_tag = Tag::parse(["game-id", "friday-night"]).unwrap();
    assert_eq!(gid_tag.kind().as_str(), "game-id");

    let d_tag = Tag::parse(["d", "nonce"]).unwrap();
    assert_eq!(d_tag.kind().as_str(), "d");

    let _ = TagKind::d(); // just ensures the symbol is in scope
}
