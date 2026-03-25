use std::fmt;
use std::fs;
use std::path::PathBuf;

/// Default `config.kdl` content bundled into `ec-data`.
///
/// Callers (e.g. `ec-client`) bootstrap this into a new game directory when
/// `config.kdl` is absent.
pub const DEFAULT_GAME_CONFIG_KDL: &str = include_str!("../config/config.kdl");

// ─── Types ────────────────────────────────────────────────────────────────────

/// Sysop-facing runtime configuration parsed from `config.kdl`.
///
/// This is the authoritative source for operational settings.  On startup,
/// `ec-client` reads this file and applies any differing values into
/// `ecgame.db` so the engine and TUI always see the current config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConfig {
    /// Display name shown in the main menu header.
    pub game_name: String,

    /// Path to the theme KDL file.  Relative paths are resolved against the
    /// game directory.  `None` means use `theme.kdl` in the game directory.
    pub theme: Option<PathBuf>,

    /// Whether sysop snoop is enabled.
    pub snoop: bool,

    /// Session policy settings.
    pub session: SessionConfig,

    /// Inactivity policy settings.
    pub inactivity: InactivityConfig,
}

/// Session timeout and timing policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    /// Maximum minutes of inactivity before timeout kicks in.
    pub max_idle_minutes: u8,
    /// Minimum time (minutes) granted to a player per session.
    pub minimum_time_minutes: u8,
    /// Whether the timeout applies to local (non-remote) sessions.
    pub local_timeout: bool,
    /// Whether the timeout applies to remote sessions.
    pub remote_timeout: bool,
}

/// Inactivity thresholds (in turns) for automated responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InactivityConfig {
    /// Purge player after this many inactive turns (0 = disabled).
    pub purge_after_turns: u8,
    /// Put player on autopilot after this many inactive turns (0 = disabled).
    pub autopilot_after_turns: u8,
}

// ─── Defaults ─────────────────────────────────────────────────────────────────

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            game_name: "Esterian Conquest".to_string(),
            theme: None,
            snoop: true,
            session: SessionConfig::default(),
            inactivity: InactivityConfig::default(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_idle_minutes: 10,
            minimum_time_minutes: 0,
            local_timeout: false,
            remote_timeout: true,
        }
    }
}

impl Default for InactivityConfig {
    fn default() -> Self {
        Self {
            purge_after_turns: 0,
            autopilot_after_turns: 0,
        }
    }
}

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum GameConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for GameConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for GameConfigError {}

impl From<std::io::Error> for GameConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

impl GameConfig {
    /// Parse a `config.kdl` string.
    pub fn parse_kdl_str(input: &str) -> Result<Self, GameConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| GameConfigError::Parse(format!("invalid KDL: {err}")))?;

        let game_name = opt_node_string(&document, "game_name")
            .unwrap_or_else(|| "Esterian Conquest".to_string());

        let theme = opt_node_string(&document, "theme").map(PathBuf::from);

        let snoop = if let Some(node) = document.get("snoop") {
            node.get(0).and_then(|v| v.as_bool()).ok_or_else(|| {
                GameConfigError::Parse("snoop requires a bool argument".to_string())
            })?
        } else {
            GameConfig::default().snoop
        };

        let session = if let Some(node) = document.get("session") {
            let defaults = SessionConfig::default();
            SessionConfig {
                max_idle_minutes: opt_child_u8(node, "max_idle_minutes")?
                    .unwrap_or(defaults.max_idle_minutes),
                minimum_time_minutes: opt_child_u8(node, "minimum_time_minutes")?
                    .unwrap_or(defaults.minimum_time_minutes),
                local_timeout: opt_child_bool(node, "local_timeout")?
                    .unwrap_or(defaults.local_timeout),
                remote_timeout: opt_child_bool(node, "remote_timeout")?
                    .unwrap_or(defaults.remote_timeout),
            }
        } else {
            SessionConfig::default()
        };

        let inactivity = if let Some(node) = document.get("inactivity") {
            let defaults = InactivityConfig::default();
            InactivityConfig {
                purge_after_turns: opt_child_u8(node, "purge_after_turns")?
                    .unwrap_or(defaults.purge_after_turns),
                autopilot_after_turns: opt_child_u8(node, "autopilot_after_turns")?
                    .unwrap_or(defaults.autopilot_after_turns),
            }
        } else {
            InactivityConfig::default()
        };

        Ok(Self {
            game_name,
            theme,
            snoop,
            session,
            inactivity,
        }
        .validate()?)
    }

    /// Read and parse a `config.kdl` file from disk.
    pub fn load_kdl(path: &std::path::Path) -> Result<Self, GameConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    /// Validate field ranges.
    pub fn validate(self) -> Result<Self, GameConfigError> {
        if self.session.max_idle_minutes > 120 {
            return Err(GameConfigError::Parse(format!(
                "max_idle_minutes must be <= 120, got {}",
                self.session.max_idle_minutes
            )));
        }
        if self.session.minimum_time_minutes > 120 {
            return Err(GameConfigError::Parse(format!(
                "minimum_time_minutes must be <= 120, got {}",
                self.session.minimum_time_minutes
            )));
        }
        if self.inactivity.purge_after_turns > 100 {
            return Err(GameConfigError::Parse(format!(
                "purge_after_turns must be <= 100, got {}",
                self.inactivity.purge_after_turns
            )));
        }
        if self.inactivity.autopilot_after_turns > 100 {
            return Err(GameConfigError::Parse(format!(
                "autopilot_after_turns must be <= 100, got {}",
                self.inactivity.autopilot_after_turns
            )));
        }
        Ok(self)
    }
}

// ─── KDL helpers (local) ─────────────────────────────────────────────────────

/// Return the first positional string argument of a top-level node by name.
fn opt_node_string(document: &kdl::KdlDocument, name: &str) -> Option<String> {
    document.get(name)?.get(0)?.as_string().map(str::to_string)
}

/// Return the value of a child node's first positional argument as `u8`.
/// Returns `None` if the child node is absent; errors if present but invalid.
fn opt_child_u8(parent: &kdl::KdlNode, name: &str) -> Result<Option<u8>, GameConfigError> {
    let children = match parent.children() {
        Some(c) => c,
        None => return Ok(None),
    };
    let Some(node) = children.get(name) else {
        return Ok(None);
    };
    let value = node
        .get(0)
        .and_then(|v| v.as_integer())
        .ok_or_else(|| GameConfigError::Parse(format!("{name} requires an integer argument")))?;
    let byte = u8::try_from(value)
        .map_err(|_| GameConfigError::Parse(format!("{name} out of u8 range: {value}")))?;
    Ok(Some(byte))
}

/// Return the value of a child node's first positional argument as `bool`.
/// Returns `None` if the child node is absent; errors if present but invalid.
fn opt_child_bool(parent: &kdl::KdlNode, name: &str) -> Result<Option<bool>, GameConfigError> {
    let children = match parent.children() {
        Some(c) => c,
        None => return Ok(None),
    };
    let Some(node) = children.get(name) else {
        return Ok(None);
    };
    let value = node
        .get(0)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| GameConfigError::Parse(format!("{name} requires a bool argument")))?;
    Ok(Some(value))
}
