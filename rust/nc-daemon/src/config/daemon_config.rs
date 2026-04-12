use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub games_root: PathBuf,
    pub relay_url: String,
    pub identity_path: PathBuf,
    pub sysop_contact_npub: String,
}
