use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, CoreGameData,
    DEFAULT_CAMPAIGN_DB_NAME, PlanetIntelSnapshot, QueuedPlayerMail, ReportBlockRow,
    TurnSubmission, TurnSubmissionError, TurnSubmissionReport, derive_campaign_seed_from_runtime,
    load_mail_queue, merge_player_intel_from_runtime,
};

pub(super) fn submit_turn_kdl_file(
    dir: &Path,
    player_record_index_1_based: usize,
    file: &Path,
    check_only: bool,
) -> Result<TurnSubmissionReport, TurnSubmissionError> {
    let submission = TurnSubmission::load_kdl(file)?;
    if submission.player_record_index_1_based != player_record_index_1_based {
        return Err(TurnSubmissionError::Validation(format!(
            "submit-turn player mismatch: CLI requested player {}, file declares player {}",
            player_record_index_1_based, submission.player_record_index_1_based
        )));
    }

    if check_only {
        let (mut game_data, mut queued_mail) = load_preview_state(dir)?;
        return submission.apply_to(&mut game_data, &mut queued_mail);
    }

    let store = CampaignStore::open_default_in_dir(dir)?;
    let mut state = load_or_seed_runtime_state(dir, &store)?;
    let report = submission.apply_to(&mut state.game_data, &mut state.queued_mail)?;
    let planet_intel_by_viewer = load_runtime_intel_by_viewer(&store, &state.game_data)?;
    store.save_runtime_state_structured_with_intel(
        &state.game_data,
        &state.planet_scorch_orders,
        &state.report_block_rows,
        &state.queued_mail,
        &planet_intel_by_viewer,
    )?;
    Ok(report)
}

fn load_preview_state(
    dir: &Path,
) -> Result<(CoreGameData, Vec<QueuedPlayerMail>), TurnSubmissionError> {
    let store_path = dir.join(DEFAULT_CAMPAIGN_DB_NAME);
    if store_path.exists() {
        let store = CampaignStore::open(store_path)?;
        if let Some(state) = store.load_latest_runtime_state()? {
            return Ok((state.game_data, state.queued_mail));
        }
    }

    Ok((
        CoreGameData::load(dir)?,
        load_mail_queue(dir)
            .map_err(|source| TurnSubmissionError::Validation(source.to_string()))?,
    ))
}

fn load_or_seed_runtime_state(
    dir: &Path,
    store: &CampaignStore,
) -> Result<CampaignRuntimeState, TurnSubmissionError> {
    if let Some(state) = store.load_latest_runtime_state()? {
        return Ok(state);
    }

    let game_data = CoreGameData::load(dir)?;
    let report_block_rows = Vec::<ReportBlockRow>::new();
    let queued_mail = load_mail_queue(dir)
        .map_err(|source| TurnSubmissionError::Validation(source.to_string()))?;
    let campaign_seed =
        derive_campaign_seed_from_runtime(&game_data, &report_block_rows, &queued_mail);
    let planet_intel_by_viewer = load_runtime_intel_by_viewer(store, &game_data)?;

    store.save_runtime_state_structured_with_intel_and_seed(
        &game_data,
        &BTreeSet::new(),
        &report_block_rows,
        &queued_mail,
        &planet_intel_by_viewer,
        Some(campaign_seed),
    )?;

    store.load_latest_runtime_state()?.ok_or_else(|| {
        TurnSubmissionError::Validation("campaign store has no snapshots".to_string())
    })
}

fn load_runtime_intel_by_viewer(
    campaign_store: &CampaignStore,
    game_data: &CoreGameData,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, CampaignStoreError> {
    (1..=game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            let previous = campaign_store
                .latest_planet_intel_for_viewer(viewer_empire_id)?
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<BTreeMap<_, _>>();
            Ok(merge_player_intel_from_runtime(
                game_data,
                viewer_empire_id,
                game_data.conquest.game_year(),
                Some(&previous),
                None,
            ))
        })
        .collect()
}
