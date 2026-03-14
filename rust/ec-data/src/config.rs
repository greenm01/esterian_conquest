use std::fmt;
use std::fs;
use std::path::Path;

use crate::{CoreGameData, DiplomaticRelation, build_seeded_new_game};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupMode {
    CanonicalFourPlayer,
    BuilderCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOptionsConfig {
    pub snoop: bool,
    pub local_timeout: bool,
    pub remote_timeout: bool,
    pub max_key_gap_minutes: u8,
    pub minimum_time_minutes: u8,
    pub purge_after_turns: u8,
    pub autopilot_after_turns: u8,
}

impl Default for SetupOptionsConfig {
    fn default() -> Self {
        Self {
            snoop: true,
            local_timeout: false,
            remote_timeout: true,
            max_key_gap_minutes: 10,
            minimum_time_minutes: 0,
            purge_after_turns: 0,
            autopilot_after_turns: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortSetupConfig {
    pub com_irq: [u8; 4],
    pub hardware_flow_control: [bool; 4],
}

impl Default for PortSetupConfig {
    fn default() -> Self {
        Self {
            com_irq: [4, 3, 4, 3],
            hardware_flow_control: [true; 4],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupConfig {
    pub player_count: u8,
    pub year: u16,
    pub setup_mode: SetupMode,
    pub seed: Option<u64>,
    pub setup_options: SetupOptionsConfig,
    pub port_setup: PortSetupConfig,
    pub maintenance_days: [bool; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomacyDirective {
    pub from_empire_raw: u8,
    pub to_empire_raw: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiplomacyConfig {
    pub directives: Vec<DiplomacyDirective>,
}

#[derive(Debug)]
pub enum SetupConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl fmt::Display for SetupConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "{source}"),
            Self::Parse(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SetupConfigError {}

impl From<std::io::Error> for SetupConfigError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl SetupConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, SetupConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| SetupConfigError::Parse(format!("invalid KDL: {err}")))?;

        let game = document
            .get("game")
            .ok_or_else(|| SetupConfigError::Parse("missing game node".to_string()))?;
        let player_count = prop_u8(game, "player_count")?;
        let year = prop_u16(game, "year")?;
        let setup_mode = match prop_string(game, "setup_mode")?.as_str() {
            "canonical-four-player" => SetupMode::CanonicalFourPlayer,
            "builder-compatible" => SetupMode::BuilderCompatible,
            other => {
                return Err(SetupConfigError::Parse(format!(
                    "unknown setup_mode: {other}"
                )));
            }
        };
        let seed = opt_prop_u64(game, "seed")?;

        let setup_options = if let Some(node) = document.get("setup_options") {
            SetupOptionsConfig {
                snoop: prop_bool(node, "snoop")?,
                local_timeout: prop_bool(node, "local_timeout")?,
                remote_timeout: prop_bool(node, "remote_timeout")?,
                max_key_gap_minutes: prop_u8(node, "max_key_gap_minutes")?,
                minimum_time_minutes: prop_u8(node, "minimum_time_minutes")?,
                purge_after_turns: prop_u8(node, "purge_after_turns")?,
                autopilot_after_turns: prop_u8(node, "autopilot_after_turns")?,
            }
        } else {
            SetupOptionsConfig::default()
        };

        let mut port_setup = PortSetupConfig::default();
        if let Some(node) = document.get("port_setup") {
            let children = node.children().ok_or_else(|| {
                SetupConfigError::Parse("port_setup must have children".to_string())
            })?;
            let mut seen_ports = [false; 4];
            for port_node in children.nodes() {
                if port_node.name().value() != "com" {
                    continue;
                }
                let port = prop_string(port_node, "port")?;
                let idx = com_index(&port)
                    .ok_or_else(|| SetupConfigError::Parse(format!("unknown COM port: {port}")))?;
                if seen_ports[idx] {
                    return Err(SetupConfigError::Parse(format!(
                        "duplicate COM port entry: {port}"
                    )));
                }
                port_setup.com_irq[idx] = prop_u8(port_node, "irq")?;
                port_setup.hardware_flow_control[idx] =
                    prop_bool(port_node, "hardware_flow_control")?;
                seen_ports[idx] = true;
            }
        }

        let mut maintenance_days = [false; 7];
        if let Some(node) = document.get("maintenance_days") {
            let children = node.children().ok_or_else(|| {
                SetupConfigError::Parse("maintenance_days must have children".to_string())
            })?;
            for day_node in children.nodes() {
                if day_node.name().value() != "day" {
                    continue;
                }
                let day = day_node
                    .get(0)
                    .and_then(|value| value.as_string())
                    .ok_or_else(|| {
                        SetupConfigError::Parse(
                            "maintenance_days day nodes must use a string value".to_string(),
                        )
                    })?;
                maintenance_days[weekday_index(day).ok_or_else(|| {
                    SetupConfigError::Parse(format!("unknown maintenance day: {day}"))
                })?] = true;
            }
        } else {
            maintenance_days = [true; 7];
        }

        Ok(Self {
            player_count,
            year,
            setup_mode,
            seed,
            setup_options,
            port_setup,
            maintenance_days,
        }
        .validate()?)
    }

    pub fn load_kdl(path: &Path) -> Result<Self, SetupConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn with_player_count_override(
        mut self,
        player_count: u8,
    ) -> Result<Self, SetupConfigError> {
        if !(1..=25).contains(&player_count) {
            return Err(SetupConfigError::Parse(format!(
                "player_count must be 1-25, got {player_count}"
            )));
        }

        self.player_count = player_count;

        if matches!(self.setup_mode, SetupMode::CanonicalFourPlayer) && player_count != 4 {
            self.setup_mode = SetupMode::BuilderCompatible;
        }

        self.validate()
    }

    pub fn build_game_data(&self, runtime_seed: u64) -> Result<CoreGameData, SetupConfigError> {
        let seed = self.seed.unwrap_or(runtime_seed);
        let mut data = match self.setup_mode {
            SetupMode::CanonicalFourPlayer | SetupMode::BuilderCompatible => {
                build_seeded_new_game(self.player_count, self.year, seed)
                    .map_err(|err| SetupConfigError::Parse(err.to_string()))?
            }
        };

        data.setup.set_snoop_enabled(self.setup_options.snoop);
        data.setup
            .set_local_timeout_enabled(self.setup_options.local_timeout);
        data.setup
            .set_remote_timeout_enabled(self.setup_options.remote_timeout);
        data.setup
            .set_max_time_between_keys_minutes_raw(self.setup_options.max_key_gap_minutes);
        data.setup
            .set_minimum_time_granted_minutes_raw(self.setup_options.minimum_time_minutes);
        data.setup
            .set_purge_after_turns_raw(self.setup_options.purge_after_turns);
        data.setup
            .set_autopilot_inactive_turns_raw(self.setup_options.autopilot_after_turns);
        for idx in 0..4 {
            data.setup
                .set_com_irq_raw(idx, self.port_setup.com_irq[idx]);
            data.setup.set_com_hardware_flow_control_enabled(
                idx,
                self.port_setup.hardware_flow_control[idx],
            );
        }
        data.conquest
            .set_maintenance_schedule_enabled(self.maintenance_days);
        data.conquest.set_game_year(self.year);
        data.conquest.set_player_count(self.player_count);
        Ok(data)
    }

    pub fn validate(self) -> Result<Self, SetupConfigError> {
        if !(1..=25).contains(&self.player_count) {
            return Err(SetupConfigError::Parse(format!(
                "player_count must be in 1..=25, got {}",
                self.player_count
            )));
        }
        if !(3000..=3100).contains(&self.year) {
            return Err(SetupConfigError::Parse(format!(
                "year must be in 3000..=3100, got {}",
                self.year
            )));
        }
        if matches!(self.setup_mode, SetupMode::CanonicalFourPlayer) && self.player_count != 4 {
            return Err(SetupConfigError::Parse(
                "canonical-four-player setup_mode requires player_count=4".to_string(),
            ));
        }

        if self.setup_options.max_key_gap_minutes > 120 {
            return Err(SetupConfigError::Parse(format!(
                "max_key_gap_minutes must be <= 120, got {}",
                self.setup_options.max_key_gap_minutes
            )));
        }
        if self.setup_options.minimum_time_minutes > 120 {
            return Err(SetupConfigError::Parse(format!(
                "minimum_time_minutes must be <= 120, got {}",
                self.setup_options.minimum_time_minutes
            )));
        }
        if self.setup_options.purge_after_turns > 100 {
            return Err(SetupConfigError::Parse(format!(
                "purge_after_turns must be <= 100, got {}",
                self.setup_options.purge_after_turns
            )));
        }
        if self.setup_options.autopilot_after_turns > 100 {
            return Err(SetupConfigError::Parse(format!(
                "autopilot_after_turns must be <= 100, got {}",
                self.setup_options.autopilot_after_turns
            )));
        }
        for irq in self.port_setup.com_irq {
            if irq > 7 {
                return Err(SetupConfigError::Parse(format!(
                    "COM IRQ values must be in 0..=7, got {}",
                    irq
                )));
            }
        }

        Ok(self)
    }
}

impl DiplomacyConfig {
    pub fn parse_kdl_str(input: &str) -> Result<Self, SetupConfigError> {
        let document: kdl::KdlDocument = input
            .parse()
            .map_err(|err| SetupConfigError::Parse(format!("invalid KDL: {err}")))?;

        let mut directives = Vec::new();
        for node in document.nodes() {
            if node.name().value() != "relation" {
                continue;
            }

            let relation = match prop_string(node, "status")?.as_str() {
                "enemy" => DiplomaticRelation::Enemy,
                "neutral" => DiplomaticRelation::Neutral,
                other => {
                    return Err(SetupConfigError::Parse(format!(
                        "unknown diplomacy status: {other}"
                    )));
                }
            };

            directives.push(DiplomacyDirective {
                from_empire_raw: prop_u8(node, "from")?,
                to_empire_raw: prop_u8(node, "to")?,
                relation,
            });
        }

        Ok(Self { directives })
    }

    pub fn load_kdl(path: &Path) -> Result<Self, SetupConfigError> {
        let text = fs::read_to_string(path)?;
        Self::parse_kdl_str(&text)
    }

    pub fn to_kdl_string(&self) -> String {
        let mut text = String::new();
        for directive in &self.directives {
            let status = match directive.relation {
                DiplomaticRelation::Neutral => "neutral",
                DiplomaticRelation::Enemy => "enemy",
            };
            text.push_str(&format!(
                "relation from={} to={} status=\"{}\"\n",
                directive.from_empire_raw, directive.to_empire_raw, status
            ));
        }
        text
    }

    pub fn validate_for_player_count(self, player_count: u8) -> Result<Self, SetupConfigError> {
        let mut seen = std::collections::BTreeSet::new();
        for directive in &self.directives {
            if directive.from_empire_raw == 0 || directive.from_empire_raw > player_count {
                return Err(SetupConfigError::Parse(format!(
                    "diplomacy relation 'from' must be in 1..={player_count}, got {}",
                    directive.from_empire_raw
                )));
            }
            if directive.to_empire_raw == 0 || directive.to_empire_raw > player_count {
                return Err(SetupConfigError::Parse(format!(
                    "diplomacy relation 'to' must be in 1..={player_count}, got {}",
                    directive.to_empire_raw
                )));
            }
            if directive.from_empire_raw == directive.to_empire_raw {
                return Err(SetupConfigError::Parse(
                    "diplomacy relation cannot target the same empire".to_string(),
                ));
            }
            if !seen.insert((directive.from_empire_raw, directive.to_empire_raw)) {
                return Err(SetupConfigError::Parse(format!(
                    "duplicate diplomacy relation from empire {} to {}",
                    directive.from_empire_raw, directive.to_empire_raw
                )));
            }
        }

        Ok(self)
    }
}

fn prop_bool(node: &kdl::KdlNode, name: &str) -> Result<bool, SetupConfigError> {
    node.get(name)
        .and_then(|value| value.as_bool())
        .ok_or_else(|| SetupConfigError::Parse(format!("missing or invalid bool property: {name}")))
}

fn prop_u8(node: &kdl::KdlNode, name: &str) -> Result<u8, SetupConfigError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            SetupConfigError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u8::try_from(value)
        .map_err(|_| SetupConfigError::Parse(format!("property {name} out of u8 range: {value}")))
}

fn prop_u16(node: &kdl::KdlNode, name: &str) -> Result<u16, SetupConfigError> {
    let value = node
        .get(name)
        .and_then(|value| value.as_integer())
        .ok_or_else(|| {
            SetupConfigError::Parse(format!("missing or invalid integer property: {name}"))
        })?;
    u16::try_from(value)
        .map_err(|_| SetupConfigError::Parse(format!("property {name} out of u16 range: {value}")))
}

fn opt_prop_u64(node: &kdl::KdlNode, name: &str) -> Result<Option<u64>, SetupConfigError> {
    let Some(value) = node.get(name) else {
        return Ok(None);
    };
    let integer = value.as_integer().ok_or_else(|| {
        SetupConfigError::Parse(format!("missing or invalid integer property: {name}"))
    })?;
    let integer = u64::try_from(integer).map_err(|_| {
        SetupConfigError::Parse(format!("property {name} out of u64 range: {integer}"))
    })?;
    Ok(Some(integer))
}

fn prop_string(node: &kdl::KdlNode, name: &str) -> Result<String, SetupConfigError> {
    node.get(name)
        .and_then(|value| value.as_string())
        .map(str::to_string)
        .ok_or_else(|| {
            SetupConfigError::Parse(format!("missing or invalid string property: {name}"))
        })
}

fn weekday_index(day_name: &str) -> Option<usize> {
    match day_name.to_ascii_lowercase().as_str() {
        "sun" | "sunday" => Some(0),
        "mon" | "monday" => Some(1),
        "tue" | "tues" | "tuesday" => Some(2),
        "wed" | "wednesday" => Some(3),
        "thu" | "thur" | "thurs" | "thursday" => Some(4),
        "fri" | "friday" => Some(5),
        "sat" | "saturday" => Some(6),
        _ => None,
    }
}

fn com_index(port_name: &str) -> Option<usize> {
    match port_name.to_ascii_lowercase().as_str() {
        "com1" | "1" => Some(0),
        "com2" | "2" => Some(1),
        "com3" | "3" => Some(2),
        "com4" | "4" => Some(3),
        _ => None,
    }
}
