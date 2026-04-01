use std::fs;
use std::path::Path;

use super::*;

impl CoreGameData {
    pub fn load(dir: &Path) -> Result<Self, GameDirectoryError> {
        Ok(Self {
            player: load_parsed(dir, "PLAYER.DAT", PlayerDat::parse)?,
            planets: load_parsed(dir, "PLANETS.DAT", PlanetDat::parse)?,
            fleets: load_parsed(dir, "FLEETS.DAT", FleetDat::parse)?,
            bases: load_parsed(dir, "BASES.DAT", BaseDat::parse)?,
            ipbm: load_optional_parsed(dir, "IPBM.DAT", IpbmDat::parse)?
                .unwrap_or_else(|| IpbmDat { records: vec![] }),
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

fn load_optional_parsed<T>(
    dir: &Path,
    file_name: &'static str,
    parse: impl Fn(&[u8]) -> Result<T, ParseError>,
) -> Result<Option<T>, GameDirectoryError> {
    let path = dir.join(file_name);
    let bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(GameDirectoryError::Io {
                path: path.clone(),
                source,
            });
        }
    };
    parse(&bytes)
        .map(Some)
        .map_err(|source| GameDirectoryError::Parse { path, source })
}

fn save_bytes(dir: &Path, file_name: &'static str, bytes: &[u8]) -> Result<(), GameDirectoryError> {
    let path = dir.join(file_name);
    fs::write(&path, bytes).map_err(|source| GameDirectoryError::Io { path, source })
}
