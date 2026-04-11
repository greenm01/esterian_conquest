use crate::{
    CoreGameData, PlayerActivityState, PlayerLifecycleState, TerminalOutcome, WinnerState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicEmpireStatus {
    Active,
    Mia,
    Defeated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAccessMode {
    Normal,
    ReviewOnly,
    SurveyOnly,
    LockedOut,
}

pub fn empire_has_recovery_path(game_data: &CoreGameData, empire_raw: u8) -> bool {
    let unowned_planet_exists = game_data
        .planets
        .records
        .iter()
        .any(|planet| planet.owner_empire_slot_raw() == 0);

    game_data.fleets.records.iter().any(|fleet| {
        if fleet.owner_empire_raw() != empire_raw {
            return false;
        }
        (fleet.troop_transport_count() > 0 && fleet.army_count() > 0)
            || (unowned_planet_exists && fleet.etac_count() > 0)
    })
}

pub fn default_player_lifecycle_states(player_count: u8) -> Vec<PlayerLifecycleState> {
    (1..=player_count as usize)
        .map(PlayerLifecycleState::for_player)
        .collect()
}

pub fn player_public_status(
    game_data: &CoreGameData,
    player_record_index_1_based: usize,
    player_activity_states: &[PlayerActivityState],
    player_lifecycle_states: &[PlayerLifecycleState],
) -> PublicEmpireStatus {
    let lifecycle = player_lifecycle_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(player_record_index_1_based));
    if matches!(
        lifecycle.terminal_outcome,
        TerminalOutcome::Defeated | TerminalOutcome::LostGame
    ) {
        return PublicEmpireStatus::Defeated;
    }

    let activity = player_activity_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or(PlayerActivityState {
            player_record_index_1_based,
            last_participation_year: 0,
            inactivity_autopilot_pending_clear: false,
        });
    let autopilot_active = game_data
        .player
        .records
        .get(player_record_index_1_based.saturating_sub(1))
        .map(|player| player.autopilot_flag() != 0)
        .unwrap_or(false);
    if autopilot_active && activity.inactivity_autopilot_pending_clear {
        PublicEmpireStatus::Mia
    } else {
        PublicEmpireStatus::Active
    }
}

pub fn player_access_mode(
    player_record_index_1_based: usize,
    player_lifecycle_states: &[PlayerLifecycleState],
    winner_state: WinnerState,
) -> PlayerAccessMode {
    let lifecycle = player_lifecycle_states
        .get(player_record_index_1_based.saturating_sub(1))
        .copied()
        .unwrap_or_else(|| PlayerLifecycleState::for_player(player_record_index_1_based));
    match lifecycle.terminal_outcome {
        TerminalOutcome::Winner => PlayerAccessMode::SurveyOnly,
        TerminalOutcome::Defeated | TerminalOutcome::LostGame => {
            if lifecycle.terminal_review_consumed {
                PlayerAccessMode::LockedOut
            } else {
                PlayerAccessMode::ReviewOnly
            }
        }
        TerminalOutcome::None => {
            if winner_state.winner_empire_raw.is_some() {
                PlayerAccessMode::LockedOut
            } else {
                PlayerAccessMode::Normal
            }
        }
    }
}
