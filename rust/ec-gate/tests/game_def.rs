//! Regression tests for 30500 GameDefinition event construction (step 9).

use ec_gate::roster::{Roster, Seat, SeatStatus};
use ec_gate::serve::game_def::{build_game_def_tags, sha256_hex};

// ---------------------------------------------------------------------------
// sha256_hex helpers
// ---------------------------------------------------------------------------

#[test]
fn sha256_hex_known_value() {
    // SHA-256 of empty string is a well-known constant.
    let hash = sha256_hex("");
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_hex_is_64_chars() {
    let hash = sha256_hex("velvet-mountain");
    assert_eq!(hash.len(), 64);
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
    let a = sha256_hex("amber-cascade");
    let b = sha256_hex("amber-cascade");
    assert_eq!(a, b);
}

#[test]
fn sha256_hex_different_input_different_output() {
    let a = sha256_hex("velvet-mountain");
    let b = sha256_hex("copper-sunrise");
    assert_ne!(a, b);
}

// ---------------------------------------------------------------------------
// build_game_def_tags
// ---------------------------------------------------------------------------

fn make_roster() -> Roster {
    Roster {
        id: "friday-night".to_string(),
        name: "Friday Night EC".to_string(),
        seats: vec![
            Seat {
                player: 1,
                code: "velvet-mountain".to_string(),
                status: SeatStatus::Claimed,
                npub: Some("npub1aaa000".to_string()),
            },
            Seat {
                player: 2,
                code: "copper-sunrise".to_string(),
                status: SeatStatus::Pending,
                npub: None,
            },
        ],
    }
}

fn tags_as_strings(roster: &Roster) -> Vec<Vec<String>> {
    build_game_def_tags(roster)
        .unwrap()
        .iter()
        .map(|t| {
            // Collect tag values via to_vec which returns owned strings.
            t.clone().to_vec()
        })
        .collect()
}

#[test]
fn game_def_tags_starts_with_d_tag() {
    let tags = tags_as_strings(&make_roster());
    assert_eq!(tags[0][0], "d");
    assert_eq!(tags[0][1], "friday-night");
}

#[test]
fn game_def_tags_has_name_tag() {
    let tags = tags_as_strings(&make_roster());
    let name_tag = tags.iter().find(|t| t[0] == "name").unwrap();
    assert_eq!(name_tag[1], "Friday Night EC");
}

#[test]
fn game_def_tags_has_status_active() {
    let tags = tags_as_strings(&make_roster());
    let status_tag = tags.iter().find(|t| t[0] == "status").unwrap();
    assert_eq!(status_tag[1], "active");
}

#[test]
fn game_def_tags_has_players_count() {
    let tags = tags_as_strings(&make_roster());
    let players_tag = tags.iter().find(|t| t[0] == "players").unwrap();
    assert_eq!(players_tag[1], "2");
}

#[test]
fn game_def_tags_slot_count_matches_seats() {
    let tags = tags_as_strings(&make_roster());
    let slot_count = tags.iter().filter(|t| t[0] == "slot").count();
    assert_eq!(slot_count, 2);
}

#[test]
fn game_def_tags_claimed_slot_has_npub() {
    let tags = tags_as_strings(&make_roster());
    let slot1 = tags.iter().find(|t| t[0] == "slot" && t[1] == "1").unwrap();
    // [slot, index, code_hash, npub, status]
    assert_eq!(slot1[3], "npub1aaa000");
    assert_eq!(slot1[4], "claimed");
}

#[test]
fn game_def_tags_pending_slot_has_empty_npub() {
    let tags = tags_as_strings(&make_roster());
    let slot2 = tags.iter().find(|t| t[0] == "slot" && t[1] == "2").unwrap();
    assert_eq!(slot2[3], "");
    assert_eq!(slot2[4], "pending");
}

#[test]
fn game_def_tags_slot_code_is_sha256_hash() {
    let tags = tags_as_strings(&make_roster());
    let slot1 = tags.iter().find(|t| t[0] == "slot" && t[1] == "1").unwrap();
    let expected_hash = sha256_hex("velvet-mountain");
    assert_eq!(slot1[2], expected_hash);
}

#[test]
fn game_def_tags_invite_code_normalized_before_hash() {
    // Uppercase code should hash the same as lowercase.
    let mut roster = make_roster();
    roster.seats[0].code = "VELVET-MOUNTAIN".to_string();
    let tags = tags_as_strings(&roster);
    let slot1 = tags.iter().find(|t| t[0] == "slot" && t[1] == "1").unwrap();
    let expected_hash = sha256_hex("velvet-mountain");
    assert_eq!(slot1[2], expected_hash);
}

#[test]
fn game_def_tags_empty_roster_has_no_slots() {
    let roster = Roster {
        id: "empty-game".to_string(),
        name: "Empty".to_string(),
        seats: vec![],
    };
    let tags = tags_as_strings(&roster);
    let slot_count = tags.iter().filter(|t| t[0] == "slot").count();
    assert_eq!(slot_count, 0);
    let players_tag = tags.iter().find(|t| t[0] == "players").unwrap();
    assert_eq!(players_tag[1], "0");
}
