use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const HOSTED_ONBOARDING_INVARIANT_EXIT_CODE: i32 = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostedOnboardingInvariantError {
    screen_name: &'static str,
}

impl HostedOnboardingInvariantError {
    pub const fn new(screen_name: &'static str) -> Self {
        Self { screen_name }
    }
}

impl Display for HostedOnboardingInvariantError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hosted join invariant failed: this hosted player session reached {screen} instead of the empire-naming flow.",
            screen = self.screen_name
        )
    }
}

impl Error for HostedOnboardingInvariantError {}

pub fn exit_code_for(err: &(dyn Error + 'static)) -> Option<i32> {
    err.is::<HostedOnboardingInvariantError>()
        .then_some(HOSTED_ONBOARDING_INVARIANT_EXIT_CODE)
}
