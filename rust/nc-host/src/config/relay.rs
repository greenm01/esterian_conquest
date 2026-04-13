use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayConfig {
    pub url: String,
}

#[derive(Debug, Error)]
pub enum RelayError {
    #[error("invalid relay URL: {0}")]
    InvalidUrl(String),
}

impl RelayConfig {
    pub fn validate(url: &str) -> Result<Self, RelayError> {
        let parsed = url::Url::parse(url)
            .map_err(|e: url::ParseError| RelayError::InvalidUrl(e.to_string()))?;

        match parsed.scheme() {
            "ws" | "wss" => {}
            _ => {
                return Err(RelayError::InvalidUrl(
                    "must be ws:// or wss://".to_string(),
                ));
            }
        }

        if parsed.host().is_none() {
            return Err(RelayError::InvalidUrl("must have a host".to_string()));
        }

        Ok(RelayConfig {
            url: url.to_string(),
        })
    }
}
