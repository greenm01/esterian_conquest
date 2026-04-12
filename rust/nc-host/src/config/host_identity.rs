use nostr_sdk::{Keys, ToBech32};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostIdentity {
    pub npub: String,
    pub nsec: String,
}

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("failed to read identity file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("invalid key format: {0}")]
    InvalidKey(String),
    #[error("Nostr SDK error: {0}")]
    NostrError(String),
}

impl HostIdentity {
    pub fn load(path: &PathBuf) -> Result<Self, IdentityError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self, IdentityError> {
        let content = content.trim();

        if let Some(nsec_line) = content.lines().find(|l| l.starts_with("nsec1")) {
            let nsec = nsec_line.trim().to_string();
            let keys = Keys::parse(&nsec).map_err(|e| IdentityError::NostrError(e.to_string()))?;
            let npub = keys
                .public_key()
                .to_bech32()
                .map_err(|e| IdentityError::NostrError(e.to_string()))?;
            return Ok(HostIdentity { npub, nsec });
        }

        Err(IdentityError::InvalidKey("no nsec key found".to_string()))
    }
}

pub fn generate_identity() -> Result<HostIdentity, IdentityError> {
    let keys = Keys::generate();
    let npub = keys
        .public_key()
        .to_bech32()
        .map_err(|e| IdentityError::NostrError(e.to_string()))?;
    let nsec = keys
        .secret_key()
        .to_bech32()
        .map_err(|e| IdentityError::NostrError(e.to_string()))?;

    Ok(HostIdentity { npub, nsec })
}

pub fn save_identity(path: &PathBuf, identity: &HostIdentity) -> Result<(), IdentityError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = format!(
        "# nc-host identity\n{}\n{}\n",
        identity.npub, identity.nsec
    );
    std::fs::write(path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    Ok(())
}
