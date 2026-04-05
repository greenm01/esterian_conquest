use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{
    CampaignStore, ClaimHostedSeatError, HostedSeat, HostedSeatStatus, QueuedPlayerMail,
    ReportBlockRow,
};
use nc_engine::build_seeded_new_game;

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

fn pending_seats(player_count: usize) -> Vec<HostedSeat> {
    (1..=player_count)
        .map(|player| pending(player, &format!("seat-{player}")))
        .collect()
}

#[test]
fn hosted_seats_round_trip_and_claim() {
    let dir = unique_temp_dir("nc-data-hosted-seats");
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
    let dir = unique_temp_dir("nc-data-hosted-reissue");
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
    let dir = unique_temp_dir("nc-data-hosted-claim-by-player");
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
    let dir = unique_temp_dir("nc-data-hosted-duplicate-npub");
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
    let dir = unique_temp_dir("nc-data-hosted-duplicate-npub-player");
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

#[test]
fn nuke_hosted_seat_resets_runtime_slice_and_side_data() {
    let dir = unique_temp_dir("nc-data-hosted-nuke");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&pending_seats(4))
        .expect("seed hosted seats");

    let seed = 1515u64;
    let mut game_data = build_seeded_new_game(4, 3000, seed).expect("build baseline");
    game_data
        .join_player(1, "Empire One")
        .expect("join player one");
    game_data
        .join_player(2, "Empire Two")
        .expect("join player two");
    game_data
        .rename_player_homeworld(1, "Forge")
        .expect("rename player one homeworld");
    game_data
        .rename_player_homeworld(2, "Anchor")
        .expect("rename player two homeworld");
    let queued_mail = vec![
        QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "alpha".to_string(),
            body: "to seat two".to_string(),
            recipient_deleted: false,
        },
        QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 1,
            year: 3000,
            subject: "beta".to_string(),
            body: "to seat one".to_string(),
            recipient_deleted: false,
        },
        QueuedPlayerMail {
            sender_empire_id: 2,
            recipient_empire_id: 3,
            year: 3000,
            subject: "gamma".to_string(),
            body: "keep this".to_string(),
            recipient_deleted: false,
        },
    ];
    let report_block_rows = vec![ReportBlockRow {
        viewer_empire_id: 0,
        block_index: 0,
        decoded_text: "report".to_string(),
        raw_bytes: None,
        recipient_deleted: false,
    }];
    store
        .save_runtime_state_structured_and_claim_hosted_seat(
            &game_data,
            &BTreeSet::new(),
            &report_block_rows,
            &queued_mail,
            1,
            "npub1oldplayer",
        )
        .expect("save claimed runtime");
    store
        .claim_hosted_seat_for_player(2, "npub1seat2")
        .expect("claim seat two")
        .expect("seat two should exist");
    store
        .set_player_theme_preference(1, "chrome")
        .expect("set seat one theme");
    store
        .set_player_theme_preference(2, "tokyo_night")
        .expect("set seat two theme");

    let baseline = build_seeded_new_game(4, 3000, seed).expect("build reset baseline");
    let reissued = store
        .reissue_hosted_seat_and_reset_runtime(1, "amber-river", &baseline, 100)
        .expect("nuke seat one")
        .expect("seat one should exist");
    assert_eq!(reissued.invite_code, "amber-river");
    assert_eq!(reissued.status, HostedSeatStatus::Pending);
    assert!(reissued.player_npub.is_none());

    let seats = store.hosted_seats().expect("reload hosted seats");
    assert_eq!(seats[0].status, HostedSeatStatus::Pending);
    assert_eq!(seats[0].invite_code, "amber-river");
    assert!(seats[0].player_npub.is_none());
    assert_eq!(seats[1].status, HostedSeatStatus::Claimed);
    assert_eq!(seats[1].player_npub.as_deref(), Some("npub1seat2"));

    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime should exist");
    assert_eq!(runtime.game_year, 3000);
    assert_eq!(runtime.game_data.player.records[0].owner_mode_raw(), 0);
    assert_eq!(runtime.game_data.player.records[1].owner_mode_raw(), 1);
    let player_one_homeworld =
        runtime.game_data.player.records[0].homeworld_planet_index_1_based_raw() as usize;
    let player_two_homeworld =
        runtime.game_data.player.records[1].homeworld_planet_index_1_based_raw() as usize;
    assert_eq!(
        runtime.game_data.planets.records[player_one_homeworld - 1].planet_name(),
        "Not Named Yet"
    );
    assert_eq!(
        runtime.game_data.planets.records[player_two_homeworld - 1].planet_name(),
        "Anchor"
    );
    assert!(runtime.report_block_rows.is_empty());
    assert_eq!(runtime.queued_mail.len(), 1);
    assert_eq!(runtime.queued_mail[0].sender_empire_id, 2);
    assert_eq!(runtime.queued_mail[0].recipient_empire_id, 3);
    assert_eq!(
        store
            .player_theme_preference(1)
            .expect("load seat one theme"),
        None
    );
    assert_eq!(
        store
            .player_theme_preference(2)
            .expect("load seat two theme"),
        Some("tokyo_night".to_string())
    );
    assert!(
        store
            .hosted_publish_jobs()
            .expect("load hosted publish jobs")
            .iter()
            .all(|job| job.player_record_index_1_based != 1)
    );

    let viewer_one_intel = store
        .latest_planet_intel_for_viewer(1)
        .expect("load viewer one intel");
    let homeworld_intel = viewer_one_intel
        .into_iter()
        .find(|row| row.planet_record_index_1_based == player_one_homeworld)
        .expect("viewer one homeworld intel should exist");
    assert_eq!(homeworld_intel.known_name.as_deref(), Some("Not Named Yet"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn nuke_hosted_seat_rejects_post_first_turn_runtime() {
    let dir = unique_temp_dir("nc-data-hosted-nuke-late");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&pending_seats(4))
        .expect("seed hosted seats");

    let seed = 1515u64;
    let game_data = build_seeded_new_game(4, 3001, seed).expect("build year 3001 state");
    store
        .save_runtime_state_structured(&game_data, &BTreeSet::new(), &[], &[])
        .expect("save runtime");
    let baseline = build_seeded_new_game(4, 3000, seed).expect("build reset baseline");

    let err = store
        .reissue_hosted_seat_and_reset_runtime(1, "amber-river", &baseline, 100)
        .expect_err("nuke should fail after first turn");
    assert!(err.to_string().contains("year 3000"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn nuke_hosted_seat_rejects_live_session_leases() {
    let dir = unique_temp_dir("nc-data-hosted-nuke-busy");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    store
        .replace_hosted_seats(&pending_seats(4))
        .expect("seed hosted seats");

    let seed = 1515u64;
    let game_data = build_seeded_new_game(4, 3000, seed).expect("build runtime");
    store
        .save_runtime_state_structured(&game_data, &BTreeSet::new(), &[], &[])
        .expect("save runtime");
    store
        .create_pending_session_lease("session-1", 1, "npub1seat1", 100, 60)
        .expect("create live session lease");
    let baseline = build_seeded_new_game(4, 3000, seed).expect("build reset baseline");

    let err = store
        .reissue_hosted_seat_and_reset_runtime(1, "amber-river", &baseline, 100)
        .expect_err("nuke should fail while a session is live");
    assert!(err.to_string().contains("live session lease"));

    let _ = fs::remove_dir_all(&dir);
}
