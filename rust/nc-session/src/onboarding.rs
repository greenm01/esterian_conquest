//! Frontend-neutral onboarding helpers shared by player clients.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirstTimeOnboardingMode {
    #[default]
    Generic,
    BbsReserved,
}

pub fn first_time_onboarding_mode(has_bbs_reserved_seat: bool) -> FirstTimeOnboardingMode {
    if has_bbs_reserved_seat {
        FirstTimeOnboardingMode::BbsReserved
    } else {
        FirstTimeOnboardingMode::Generic
    }
}
