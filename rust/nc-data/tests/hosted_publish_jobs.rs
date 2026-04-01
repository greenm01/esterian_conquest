use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{
    CampaignStore, HostedPublishJobKind, HostedPublishJobStatus, HostedSeat, HostedSeatStatus,
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

#[test]
fn hosted_claim_enqueues_one_pending_map_push_job() {
    let dir = unique_temp_dir("nc-data-hosted-publish-job");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    let game_data = build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    store
        .replace_hosted_seats(&[pending(1, "velvet-mountain")])
        .expect("seed hosted seat");

    store
        .save_runtime_state_structured_and_claim_hosted_seat(
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
            1,
            "playerhex0001",
        )
        .expect("save and claim");

    let jobs = store
        .pending_hosted_publish_jobs()
        .expect("load publish jobs");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].kind, HostedPublishJobKind::MapPackOnFirstClaim);
    assert_eq!(jobs[0].status, HostedPublishJobStatus::Pending);
    assert_eq!(jobs[0].player_record_index_1_based, 1);
    assert_eq!(jobs[0].player_npub, "playerhex0001");
    assert!(jobs[0].published_at_unix_seconds.is_none());

    store
        .save_runtime_state_structured_and_claim_hosted_seat(
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
            1,
            "playerhex0001",
        )
        .expect("repeat save should not enqueue duplicate job");
    let jobs = store
        .pending_hosted_publish_jobs()
        .expect("reload publish jobs");
    assert_eq!(jobs.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reissued_seat_claim_enqueues_fresh_map_push_job() {
    let dir = unique_temp_dir("nc-data-hosted-publish-job-reissue");
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    let game_data = build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    store
        .replace_hosted_seats(&[pending(1, "velvet-mountain")])
        .expect("seed hosted seat");

    store
        .save_runtime_state_structured_and_claim_hosted_seat(
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
            1,
            "playerhex0001",
        )
        .expect("first claim");
    let first_job = store
        .pending_hosted_publish_jobs()
        .expect("load first publish job")
        .remove(0);
    store
        .mark_hosted_publish_job_published(first_job.id, 12345)
        .expect("mark first job published");

    store
        .reissue_hosted_seat(1, "amber-river")
        .expect("reissue seat")
        .expect("seat exists");
    store
        .save_runtime_state_structured_and_claim_hosted_seat(
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
            1,
            "playerhex0001",
        )
        .expect("claim reissued seat");

    let jobs = store.hosted_publish_jobs().expect("load all publish jobs");
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].status, HostedPublishJobStatus::Published);
    assert_eq!(jobs[1].status, HostedPublishJobStatus::Pending);
    assert!(jobs[1].id > jobs[0].id);

    let _ = fs::remove_dir_all(&dir);
}
