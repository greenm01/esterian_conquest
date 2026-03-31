use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ec_data::{CampaignStore, ClaimHostedSeatError, HostedSeat, HostedSeatStatus};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn pending(player: usize, code: &str) -> HostedSeat {
    HostedSeat {
        player_record_index_1_based: player,
        invite_code: code.to_string(),
        status: HostedSeatStatus::Pending,
        player_npub: None,
    }
}

#[test]
fn hosted_seats_round_trip_and_claim() {
    let dir = unique_temp_dir("ec-data-hosted-seats");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .initialize_hosted_seats_if_empty(&[
            pending(1, "velvet-mountain"),
            pending(2, "copper-sunrise"),
        ])
        .expect("initialize hosted seats");

    let seats = store.hosted_seats().expect("load seats");
    assert_eq!(seats.len(), 2);
    assert_eq!(seats[0].invite_code, "velvet-mountain");

    let claimed = store
        .claim_hosted_seat("velvet-mountain", "npub1player000")
        .expect("claim seat");
    assert_eq!(claimed.status, HostedSeatStatus::Claimed);
    assert_eq!(claimed.player_npub.as_deref(), Some("npub1player000"));

    let reconnect = store
        .claim_hosted_seat("velvet-mountain", "npub1player000")
        .expect("same player reconnect");
    assert_eq!(reconnect.player_record_index_1_based, 1);

    let wrong_player = store.claim_hosted_seat("velvet-mountain", "npub1other000");
    assert!(matches!(
        wrong_player,
        Err(ClaimHostedSeatError::CodeClaimed)
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reissue_hosted_seat_clears_claim() {
    let dir = unique_temp_dir("ec-data-hosted-reissue");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Claimed,
                player_npub: Some("npub1player000".to_string()),
            },
            pending(2, "copper-sunrise"),
        ])
        .expect("seed hosted seats");

    let reissued = store
        .reissue_hosted_seat(1, "amber-river")
        .expect("reissue seat")
        .expect("seat should exist");
    assert_eq!(reissued.invite_code, "amber-river");
    assert_eq!(reissued.status, HostedSeatStatus::Pending);
    assert!(reissued.player_npub.is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn claim_hosted_seat_for_player_claims_pending_seat_without_invite_lookup() {
    let dir = unique_temp_dir("ec-data-hosted-claim-by-player");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&[pending(1, "velvet-mountain"), pending(2, "copper-sunrise")])
        .expect("seed hosted seats");

    let claimed = store
        .claim_hosted_seat_for_player(2, "npub1player000")
        .expect("claim by player")
        .expect("seat should exist");
    assert_eq!(claimed.player_record_index_1_based, 2);
    assert_eq!(claimed.status, HostedSeatStatus::Claimed);
    assert_eq!(claimed.player_npub.as_deref(), Some("npub1player000"));

    let reconnect = store
        .claim_hosted_seat_for_player(2, "npub1player000")
        .expect("same player reconnect")
        .expect("seat should still exist");
    assert_eq!(reconnect.status, HostedSeatStatus::Claimed);

    let wrong_player = store
        .claim_hosted_seat_for_player(2, "npub1other000")
        .expect_err("different npub should fail");
    assert!(format!("{wrong_player}").contains("already claimed"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn claim_hosted_seat_rejects_same_identity_on_second_seat() {
    let dir = unique_temp_dir("ec-data-hosted-duplicate-npub");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Claimed,
                player_npub: Some("npub1player000".to_string()),
            },
            pending(2, "copper-sunrise"),
        ])
        .expect("seed hosted seats");

    let duplicate = store.claim_hosted_seat("copper-sunrise", "npub1player000");
    assert!(matches!(
        duplicate,
        Err(ClaimHostedSeatError::IdentityAlreadyClaimedDifferentSeat {
            player_record_index_1_based: 1
        })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn claim_hosted_seat_for_player_rejects_same_identity_on_second_seat() {
    let dir = unique_temp_dir("ec-data-hosted-duplicate-npub-player");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&[
            HostedSeat {
                player_record_index_1_based: 1,
                invite_code: "velvet-mountain".to_string(),
                status: HostedSeatStatus::Claimed,
                player_npub: Some("npub1player000".to_string()),
            },
            pending(2, "copper-sunrise"),
        ])
        .expect("seed hosted seats");

    let duplicate = store
        .claim_hosted_seat_for_player(2, "npub1player000")
        .expect_err("same identity should not claim a second seat");
    assert!(format!("{duplicate}").contains("already claimed hosted seat 1"));

    let _ = fs::remove_dir_all(&dir);
}
