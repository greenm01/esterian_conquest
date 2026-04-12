use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayStatus {
    Unknown,
    Ok,
    Timeout,
    ConnectFailed,
    ProtocolError,
}

impl RelayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Ok => "ok",
            Self::Timeout => "timeout",
            Self::ConnectFailed => "connect-failed",
            Self::ProtocolError => "protocol-error",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "ok" => Some(Self::Ok),
            "timeout" => Some(Self::Timeout),
            "connect-failed" => Some(Self::ConnectFailed),
            "protocol-error" => Some(Self::ProtocolError),
            _ => None,
        }
    }
}

pub fn validate_relay_url(input: &str) -> Result<Option<String>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = Url::parse(trimmed)
        .map_err(|_| "relay URL must be a valid ws:// or wss:// URL".to_string())?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        _ => return Err("relay URL must start with ws:// or wss://".to_string()),
    }
    if parsed.host_str().is_none() {
        return Err("relay URL must include a host".to_string());
    }
    Ok(Some(trimmed.to_string()))
}
