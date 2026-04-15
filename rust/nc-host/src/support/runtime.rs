use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nc_data::{
    CampaignRuntimeState, CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME,
    PlanetIntelSnapshot, QueuedPlayerMail, derive_campaign_seed_from_runtime, load_mail_queue,
    merge_player_intel_from_runtime,
};

pub fn current_runtime_year(game_dir: &Path) -> Result<u16, Box<dyn std::error::Error>> {
    let runtime_db_path = game_dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if runtime_db_path.exists() {
        let store = CampaignStore::open(&runtime_db_path)?;
        if let Some(runtime) = store.load_latest_runtime_state()? {
            return Ok(runtime.game_year);
        }
    }

    let game_data = CoreGameData::load(game_dir)?;
    Ok(game_data.conquest.game_year())
}

pub fn ensure_hosted_player_initialized(
    game_dir: &Path,
    player_seat: u32,
    handle: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let player_record_index_1_based =
        usize::try_from(player_seat).map_err(|_| format!("invalid player seat {}", player_seat))?;
    let store = CampaignStore::open_default_in_dir(game_dir)?;
    let runtime = load_or_seed_runtime_state(game_dir, &store)?;

    if !runtime
        .game_data
        .player_slot_is_open_for_first_join(player_record_index_1_based)
    {
        return Ok(());
    }

    let player_count = runtime.game_data.conquest.player_count();
    let mut game_data = runtime.game_data;
    let empire_name = game_data
        .player
        .records
        .get(player_record_index_1_based.saturating_sub(1))
        .map(|player| {
            let empire_name = player.controlled_empire_name_summary();
            if empire_name.is_empty() {
                format!("Seat {}", player_seat)
            } else {
                empire_name
            }
        })
        .unwrap_or_else(|| format!("Seat {}", player_seat));

    game_data.join_player(player_record_index_1_based, &empire_name)?;
    if let Some(handle) = handle {
        if let Some(player) = game_data
            .player
            .records
            .get_mut(player_record_index_1_based.saturating_sub(1))
        {
            player.set_assigned_player_handle_raw(handle);
        }
    }

    let planet_intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            let previous = store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<usize, PlanetIntelSnapshot>>();
            Ok(merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year(),
                Some(&previous),
                None,
            ))
        })
        .collect::<Result<Vec<_>, nc_data::CampaignStoreError>>()?;
    let player_activity_states = store.latest_player_activity_states(player_count)?;
    let player_lifecycle_states = store.latest_player_lifecycle_states(player_count)?;
    let player_war_stats = store.latest_player_war_stats(player_count)?;

    store.save_runtime_state_structured_with_intel_activity_lifecycle_and_war_stats(
        &game_data,
        &runtime.planet_scorch_orders,
        &runtime.report_block_rows,
        &runtime.queued_mail,
        &planet_intel_by_viewer,
        &player_activity_states,
        &player_lifecycle_states,
        &player_war_stats,
        runtime.winner_state,
    )?;

    Ok(())
}

fn load_or_seed_runtime_state(
    game_dir: &Path,
    store: &CampaignStore,
) -> Result<CampaignRuntimeState, Box<dyn std::error::Error>> {
    if let Some(runtime) = store.load_latest_runtime_state()? {
        return Ok(runtime);
    }

    let game_data = CoreGameData::load(game_dir)?;
    let queued_mail: Vec<QueuedPlayerMail> = load_mail_queue(game_dir).unwrap_or_default();
    let report_block_rows = Vec::new();
    let campaign_seed =
        derive_campaign_seed_from_runtime(&game_data, &report_block_rows, &queued_mail);
    let planet_intel_by_viewer = (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year(),
                None,
                None,
            )
        })
        .collect::<Vec<_>>();

    store.save_runtime_state_structured_with_intel_and_seed(
        &game_data,
        &BTreeSet::new(),
        &report_block_rows,
        &queued_mail,
        &planet_intel_by_viewer,
        Some(campaign_seed),
    )?;

    store
        .load_latest_runtime_state()?
        .ok_or_else(|| std::io::Error::other("campaign store has no runtime snapshots").into())
}
