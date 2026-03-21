use std::fmt;

use ec_data::{CampaignStoreError, GameStateMutationError};

#[derive(Debug)]
pub enum HarnessError {
    Io(std::io::Error),
    Parse(String),
    Validation(String),
    Mutation(GameStateMutationError),
    Store(CampaignStoreError),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) | Self::Validation(message) => write!(f, "{message}"),
            Self::Mutation(source) => write!(f, "{source}"),
            Self::Store(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for HarnessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Mutation(source) => Some(source),
            Self::Store(source) => Some(source),
            Self::Parse(_) | Self::Validation(_) => None,
        }
    }
}

impl From<std::io::Error> for HarnessError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<GameStateMutationError> for HarnessError {
    fn from(source: GameStateMutationError) -> Self {
        Self::Mutation(source)
    }
}

impl From<CampaignStoreError> for HarnessError {
    fn from(source: CampaignStoreError) -> Self {
        Self::Store(source)
    }
}
