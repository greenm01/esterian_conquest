use ec_nostr::hosted::{invite_address_from_relay, parse_game_definition};
use nostr_sdk::{EventBuilder, Keys, Kind, Tag};

fn build_game_definition_event() -> nostr_sdk::Event {
    let keys = Keys::generate();
    let tags = vec![
        Tag::parse(["d", "beta-2"]).unwrap(),
        Tag::parse(["name", "Beta 2"]).unwrap(),
        Tag::parse(["status", "active"]).unwrap(),
        Tag::parse(["ssh-host", "play.example.com"]).unwrap(),
        Tag::parse(["ssh-port", "22"]).unwrap(),
        Tag::parse(["slot", "2", "abc123", "", "pending"]).unwrap(),
        Tag::parse(["slot", "3", "def456", "0123deadbeef", "claimed"]).unwrap(),
    ];
    EventBuilder::new(Kind::Custom(30500), "")
        .tags(tags)
        .sign_with_keys(&keys)
        .unwrap()
}

#[test]
fn invite_address_uses_relay_host() {
    let invite = invite_address_from_relay("amber-river", "wss://relay.example.com:7447").unwrap();
    assert_eq!(invite, "amber-river@relay.example.com:7447");
}

#[test]
fn parse_game_definition_extracts_slots_and_ssh_target() {
    let event = build_game_definition_event();

    let game = parse_game_definition(&event).expect("parse game definition");

    assert_eq!(game.game_id, "beta-2");
    assert_eq!(game.game_name, "Beta 2");
    assert_eq!(game.ssh_host, "play.example.com");
    assert_eq!(game.ssh_port, 22);
    assert_eq!(game.slots.len(), 2);
    assert_eq!(game.slots[0].seat, 2);
    assert_eq!(game.slots[0].invite_code_hash, "abc123");
    assert_eq!(game.slots[0].player_npub, None);
    assert_eq!(game.slots[0].status, "pending");
    assert_eq!(game.slots[1].player_npub.as_deref(), Some("0123deadbeef"));
}
