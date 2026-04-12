use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedGameCatalogEntry {
    pub game_id: String,
    pub dir: PathBuf,
}
