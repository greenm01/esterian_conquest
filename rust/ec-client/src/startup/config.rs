use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupArtConfig {
    pub bbs_art_path: PathBuf,
    pub ec_game_art_path: PathBuf,
}

#[derive(Debug)]
pub enum StartupConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for StartupConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StartupConfigError {}

impl From<std::io::Error> for StartupConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl StartupArtConfig {
    pub fn parse_kdl_str(input: &str, base_dir: &Path) -> Result<Self, StartupConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| StartupConfigError::Parse(format!("invalid KDL: {err}")))?;
        let bbs_node = document
            .get("bbs_art")
            .ok_or_else(|| StartupConfigError::Parse("missing bbs_art node".to_string()))?;
        let ec_node = document
            .get("ec_game_art")
            .ok_or_else(|| StartupConfigError::Parse("missing ec_game_art node".to_string()))?;

        Ok(Self {
            bbs_art_path: resolve_path(base_dir, prop_string(bbs_node, "path")?),
            ec_game_art_path: resolve_path(base_dir, prop_string(ec_node, "path")?),
        })
    }

    pub fn load_kdl(path: &Path) -> Result<Self, StartupConfigError> {
        let text = fs::read_to_string(path)?;
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
        Self::parse_kdl_str(&text, base_dir)
    }
}

fn resolve_path(base_dir: &Path, raw: String) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, StartupConfigError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StartupConfigError::Parse(format!("missing string property: {name}")))
}
