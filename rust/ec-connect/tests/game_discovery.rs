use ec_connect::connect::game_discovery::select_discovered_game_from_events;
use ec_connect::connect::resolve::ResolvedTarget;
use nostr_sdk::{Event, EventBuilder, Keys, Kind, Tag};
use sha2::{Digest, Sha256};

fn build_game_definition_event(ssh_host: &str, ssh_port: u16, invite_hash: &str) -> Event {
    let keys = Keys::generate();
    EventBuilder::new(Kind::Custom(30500), "")
        .tags([
            Tag::parse(["d", "friday-night"]).unwrap(),
            Tag::parse(["name", "Friday Night EC"]).unwrap(),
            Tag::parse(["status", "active"]).unwrap(),
            Tag::parse(["ssh-host", ssh_host]).unwrap(),
            Tag::parse(["ssh-port", &ssh_port.to_string()]).unwrap(),
            Tag::parse(["players", "4"]).unwrap(),
            Tag::parse(["slot", "1", invite_hash, "", "pending"]).unwrap(),
        ])
        .sign_with_keys(&keys)
        .unwrap()
}

fn target() -> ResolvedTarget {
    ResolvedTarget {
        server_host: "play.example.com".to_string(),
        server_port: 2222,
        relay_url: "wss://relay.example.com".to_string(),
        invite_code: Some("amber-river".to_string()),
        game_id: None,
        gate_npub: None,
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn discovery_matches_invite_hash_and_ssh_target() {
    let event = build_game_definition_event("play.example.com", 2222, &sha256_hex("amber-river"));

    let discovered = select_discovered_game_from_events([&event], &target(), "amber-river")
        .expect("discover game");

    assert_eq!(discovered.game_id, "friday-night");
    assert_eq!(discovered.game_name, "Friday Night EC");
    assert_eq!(discovered.ssh_host, "play.example.com");
    assert_eq!(discovered.ssh_port, 2222);
    assert_eq!(discovered.seat, 1);
    assert!(discovered.gate_npub.starts_with("npub1"));
}

#[test]
fn discovery_normalizes_invite_with_host_suffix() {
    let event = build_game_definition_event("play.example.com", 2222, &sha256_hex("amber-river"));

    let discovered = select_discovered_game_from_events(
        [&event],
        &target(),
        "amber-river@relay.example.com:7447",
    )
    .expect("discover game");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_falls_back_to_gate_override_message_when_no_match_exists() {
    let event = build_game_definition_event(
        "play.example.com",
        2222,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );

    let err = select_discovered_game_from_events([&event], &target(), "amber-river")
        .expect_err("no matching event");

    assert!(err.contains("--gate <npub>"));
}

#[test]
fn discovery_accepts_mismatched_host_when_unique() {
    let event = build_game_definition_event("other.example.com", 2222, &sha256_hex("amber-river"));

    let discovered = select_discovered_game_from_events([&event], &target(), "amber-river")
        .expect("should match despite mismatched host because hash is unique");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_disambiguates_by_host_and_port_when_multiple_hashes_match() {
    let first = build_game_definition_event("other.example.com", 2222, &sha256_hex("amber-river"));
    let second = build_game_definition_event("play.example.com", 2222, &sha256_hex("amber-river"));

    // Even though two games have the same hash, exactly one matches the target host/port.
    let discovered =
        select_discovered_game_from_events([&first, &second], &target(), "amber-river")
            .expect("should disambiguate using exact host/port match");

    assert_eq!(discovered.game_id, "friday-night");
}

#[test]
fn discovery_reports_ambiguous_matches() {
    let first = build_game_definition_event("play.example.com", 2222, &sha256_hex("amber-river"));
    let second = build_game_definition_event("play.example.com", 2222, &sha256_hex("amber-river"));

    let err = select_discovered_game_from_events([&first, &second], &target(), "amber-river")
        .expect_err("ambiguous match");

    assert!(err.contains("multiple hosted games matched"));
    assert!(err.contains("--gate <npub>"));
}
