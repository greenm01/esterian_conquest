use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    BaseDat, ConquestDat, FleetDat, IpbmDat, ParseError, PlanetDat, PlayerDat, SetupDat,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreGameData {
    pub player: PlayerDat,
    pub planets: PlanetDat,
    pub fleets: FleetDat,
    pub bases: BaseDat,
    pub ipbm: IpbmDat,
    pub setup: SetupDat,
    pub conquest: ConquestDat,
}

#[derive(Debug)]
pub enum GameDirectoryError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
}

impl std::fmt::Display for GameDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Parse { path, source } => write!(f, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for GameDirectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

impl CoreGameData {
    pub fn load(dir: &Path) -> Result<Self, GameDirectoryError> {
        Ok(Self {
            player: load_parsed(dir, "PLAYER.DAT", PlayerDat::parse)?,
            planets: load_parsed(dir, "PLANETS.DAT", PlanetDat::parse)?,
            fleets: load_parsed(dir, "FLEETS.DAT", FleetDat::parse)?,
            bases: load_parsed(dir, "BASES.DAT", BaseDat::parse)?,
            ipbm: load_parsed(dir, "IPBM.DAT", IpbmDat::parse)?,
            setup: load_parsed(dir, "SETUP.DAT", SetupDat::parse)?,
            conquest: load_parsed(dir, "CONQUEST.DAT", ConquestDat::parse)?,
        })
    }

    pub fn save(&self, dir: &Path) -> Result<(), GameDirectoryError> {
        save_bytes(dir, "PLAYER.DAT", &self.player.to_bytes())?;
        save_bytes(dir, "PLANETS.DAT", &self.planets.to_bytes())?;
        save_bytes(dir, "FLEETS.DAT", &self.fleets.to_bytes())?;
        save_bytes(dir, "BASES.DAT", &self.bases.to_bytes())?;
        save_bytes(dir, "IPBM.DAT", &self.ipbm.to_bytes())?;
        save_bytes(dir, "SETUP.DAT", &self.setup.to_bytes())?;
        save_bytes(dir, "CONQUEST.DAT", &self.conquest.to_bytes())?;
        Ok(())
    }

    pub fn player1_starbase_count_current_known(&self) -> usize {
        self.player
            .records
            .first()
            .map(|record| record.starbase_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn player1_ipbm_count_current_known(&self) -> usize {
        self.player
            .records
            .first()
            .map(|record| record.ipbm_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn current_known_core_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected_bases = self.player1_starbase_count_current_known();
        let expected_ipbm = self.player1_ipbm_count_current_known();

        if self.bases.records.len() != expected_bases {
            errors.push(format!(
                "BASES.DAT record count expected {}, got {}",
                expected_bases,
                self.bases.records.len()
            ));
        }

        if self.ipbm.records.len() != expected_ipbm {
            errors.push(format!(
                "IPBM.DAT record count expected {}, got {}",
                expected_ipbm,
                self.ipbm.records.len()
            ));
        }

        errors
    }

    pub fn sync_player1_current_known_counts(&mut self) {
        if let Some(player1) = self.player.records.first_mut() {
            player1.set_starbase_count_raw(self.bases.records.len() as u16);
            player1.set_ipbm_count_raw(self.ipbm.records.len() as u16);
        }
    }
}

fn load_parsed<T>(
    dir: &Path,
    file_name: &'static str,
    parse: impl Fn(&[u8]) -> Result<T, ParseError>,
) -> Result<T, GameDirectoryError> {
    let path = dir.join(file_name);
    let bytes = fs::read(&path).map_err(|source| GameDirectoryError::Io {
        path: path.clone(),
        source,
    })?;
    parse(&bytes).map_err(|source| GameDirectoryError::Parse { path, source })
}

fn save_bytes(dir: &Path, file_name: &'static str, bytes: &[u8]) -> Result<(), GameDirectoryError> {
    let path = dir.join(file_name);
    fs::write(&path, bytes).map_err(|source| GameDirectoryError::Io { path, source })
}
