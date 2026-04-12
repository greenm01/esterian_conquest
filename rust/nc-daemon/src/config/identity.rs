use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonIdentityConfig {
    pub secret_key_path: PathBuf,
}
