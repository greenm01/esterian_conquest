//! Frontend-neutral onboarding helpers shared by player clients.

use nc_data::CoreGameData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirstTimeOnboardingMode {
    #[default]
    Generic,
    BbsReserved,
    HostedInvite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedFirstTimeStatus {
    NeedsEmpireName,
    NoPendingSeat,
}

pub fn first_time_onboarding_mode(
    has_hosted_invite_session: bool,
    has_bbs_reserved_seat: bool,
) -> FirstTimeOnboardingMode {
    if has_hosted_invite_session {
        FirstTimeOnboardingMode::HostedInvite
    } else if has_bbs_reserved_seat {
        FirstTimeOnboardingMode::BbsReserved
    } else {
        FirstTimeOnboardingMode::Generic
    }
}

pub fn hosted_first_time_status(game_data: &CoreGameData) -> HostedFirstTimeStatus {
    if game_data
        .player
        .records
        .iter()
        .any(|player| player.occupied_flag() == 0)
    {
        HostedFirstTimeStatus::NeedsEmpireName
    } else {
        HostedFirstTimeStatus::NoPendingSeat
    }
}
