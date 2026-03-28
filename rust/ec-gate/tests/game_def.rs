//! Regression tests for 30500 GameDefinition event construction (step 9).

use ec_data::{HostedSeat, HostedSeatStatus};
use ec_gate::serve::catalog::HostedGame;
use ec_gate::serve::game_def::{build_game_def_tags, sha256_hex};

#[test]
fn sha256_hex_known_value() {
    let hash = sha256_hex("");
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_hex_is_64_chars() {
    assert_eq!(sha256_hex("velvet-mountain").len(), 64);
}

#[test]
fn sha256_hex_is_lowercase_hex() {
    let hash = sha256_hex("copper-sunrise");
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
    );
}

#[test]
fn sha256_hex_same_input_same_output() {
    assert_eq!(sha256_hex("amber-cascade"), sha256_hex("amber-cascade"));
}

#[test]
fn sha256_hex_different_input_different_output() {
    assert_ne!(sha256_hex("velvet-mountain"), sha256_hex("copper-sunrise"));
}

fn make_game() -> HostedGame {
    HostedGame {
        game_id: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        seats: vec![
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Claimed,
                player_npub: Some("npub1aaa000".to_string()),
            },
            HostedSeat {
                player_record_index_1_based: 2,
                invite_code: "copper-sunrise".to_string(),
                status: HostedSeatStatus::Pending,
                player_npub: None,
            },
        ],
    }
}

fn tags_as_strings(game: &HostedGame) -> Vec<Vec<String>> {
    build_game_def_tags(game)
        .unwrap()
        .iter()
        .map(|tag| tag.clone().to_vec())
        .collect()
}

#[test]
fn game_def_tags_starts_with_d_tag() {
    let tags = tags_as_strings(&make_game());
    assert_eq!(tags[0][0], "d");
    assert_eq!(tags[0][1], "friday-night");
}

#[test]
fn game_def_tags_has_name_tag() {
    let tags = tags_as_strings(&make_game());
    let name_tag = tags.iter().find(|tag| tag[0] == "name").unwrap();
    assert_eq!(name_tag[1], "Friday Night EC");
}

#[test]
fn game_def_tags_has_status_active() {
    let tags = tags_as_strings(&make_game());
    let status_tag = tags.iter().find(|tag| tag[0] == "status").unwrap();
    assert_eq!(status_tag[1], "active");
}

#[test]
fn game_def_tags_has_players_count() {
    let tags = tags_as_strings(&make_game());
    let players_tag = tags.iter().find(|tag| tag[0] == "players").unwrap();
    assert_eq!(players_tag[1], "2");
}

#[test]
fn game_def_tags_slot_count_matches_seats() {
    let tags = tags_as_strings(&make_game());
    assert_eq!(tags.iter().filter(|tag| tag[0] == "slot").count(), 2);
}

#[test]
fn game_def_tags_claimed_slot_has_npub() {
    let tags = tags_as_strings(&make_game());
    let slot = tags
        .iter()
        .find(|tag| tag[0] == "slot" && tag[1] == "1")
        .unwrap();
    assert_eq!(slot[3], "npub1aaa000");
    assert_eq!(slot[4], "claimed");
}

#[test]
fn game_def_tags_pending_slot_has_empty_npub() {
    let tags = tags_as_strings(&make_game());
    let slot = tags
        .iter()
        .find(|tag| tag[0] == "slot" && tag[1] == "2")
        .unwrap();
    assert_eq!(slot[3], "");
    assert_eq!(slot[4], "pending");
}

#[test]
fn game_def_tags_slot_code_is_sha256_hash() {
    let tags = tags_as_strings(&make_game());
    let slot = tags
        .iter()
        .find(|tag| tag[0] == "slot" && tag[1] == "1")
        .unwrap();
    assert_eq!(slot[2], sha256_hex("velvet-mountain"));
}

#[test]
fn game_def_tags_invite_code_normalized_before_hash() {
    let mut game = make_game();
    game.seats[0].invite_code = "VELVET-MOUNTAIN".to_string();
    let tags = tags_as_strings(&game);
    let slot = tags
        .iter()
        .find(|tag| tag[0] == "slot" && tag[1] == "1")
        .unwrap();
    assert_eq!(slot[2], sha256_hex("velvet-mountain"));
}

#[test]
fn game_def_tags_empty_game_has_no_slots() {
    let game = HostedGame {
        game_id: "empty-game".to_string(),
        game_name: "Empty".to_string(),
        seats: vec![],
    };
    let tags = tags_as_strings(&game);
    assert_eq!(tags.iter().filter(|tag| tag[0] == "slot").count(), 0);
    let players_tag = tags.iter().find(|tag| tag[0] == "players").unwrap();
    assert_eq!(players_tag[1], "0");
}
