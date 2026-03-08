use std::fmt;

pub const PLAYER_RECORD_SIZE: usize = 88;
pub const PLAYER_RECORD_COUNT: usize = 5;
pub const PLAYER_DAT_SIZE: usize = PLAYER_RECORD_SIZE * PLAYER_RECORD_COUNT;

pub const PLANET_RECORD_SIZE: usize = 97;
pub const PLANET_RECORD_COUNT: usize = 20;
pub const PLANETS_DAT_SIZE: usize = PLANET_RECORD_SIZE * PLANET_RECORD_COUNT;

pub const FLEET_RECORD_SIZE: usize = 54;
pub const FLEET_RECORD_COUNT: usize = 16;
pub const FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * FLEET_RECORD_COUNT;

pub const SETUP_DAT_SIZE: usize = 522;
pub const CONQUEST_DAT_SIZE: usize = 2085;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    WrongSize {
        file_type: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongSize {
                file_type,
                expected,
                actual,
            } => write!(
                f,
                "{file_type} had wrong size: expected {expected} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

fn expect_size(data: &[u8], expected: usize, file_type: &'static str) -> Result<(), ParseError> {
    if data.len() == expected {
        Ok(())
    } else {
        Err(ParseError::WrongSize {
            file_type,
            expected,
            actual: data.len(),
        })
    }
}

fn copy_array<const N: usize>(data: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(data);
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub raw: [u8; PLAYER_RECORD_SIZE],
}

impl PlayerRecord {
    pub fn occupied_flag(&self) -> u8 {
        self.raw[0]
    }

    pub fn handle_bytes(&self) -> &[u8] {
        &self.raw[1..=0x1A]
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        &self.raw[0x1C..=0x2E]
    }

    pub fn tax_rate(&self) -> u8 {
        self.raw[0x51]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDat {
    pub records: [PlayerRecord; PLAYER_RECORD_COUNT],
}

impl PlayerDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, PLAYER_DAT_SIZE, "PLAYER.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * PLAYER_RECORD_SIZE;
                let end = start + PLAYER_RECORD_SIZE;
                PlayerRecord {
                    raw: copy_array(&data[start..end]),
                }
            }),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetRecord {
    pub raw: [u8; PLANET_RECORD_SIZE],
}

impl PlanetRecord {
    pub fn header_bytes(&self) -> &[u8] {
        &self.raw[..3]
    }

    pub fn string_len(&self) -> u8 {
        self.raw[0x0F]
    }

    pub fn status_or_name_bytes(&self) -> &[u8] {
        &self.raw[0x10..=0x1C]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDat {
    pub records: [PlanetRecord; PLANET_RECORD_COUNT],
}

impl PlanetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, PLANETS_DAT_SIZE, "PLANETS.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * PLANET_RECORD_SIZE;
                let end = start + PLANET_RECORD_SIZE;
                PlanetRecord {
                    raw: copy_array(&data[start..end]),
                }
            }),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetRecord {
    pub raw: [u8; FLEET_RECORD_SIZE],
}

impl FleetRecord {
    pub fn mission_code(&self) -> u8 {
        self.raw[0x0A]
    }

    pub fn mission_param_bytes(&self) -> &[u8] {
        &self.raw[0x1F..=0x21]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetDat {
    pub records: [FleetRecord; FLEET_RECORD_COUNT],
}

impl FleetDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, FLEETS_DAT_SIZE, "FLEETS.DAT")?;
        Ok(Self {
            records: std::array::from_fn(|idx| {
                let start = idx * FLEET_RECORD_SIZE;
                let end = start + FLEET_RECORD_SIZE;
                FleetRecord {
                    raw: copy_array(&data[start..end]),
                }
            }),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDat {
    pub raw: [u8; SETUP_DAT_SIZE],
}

impl SetupDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, SETUP_DAT_SIZE, "SETUP.DAT")?;
        Ok(Self {
            raw: copy_array(data),
        })
    }

    pub fn version_tag(&self) -> &[u8] {
        &self.raw[..5]
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConquestDat {
    pub raw: [u8; CONQUEST_DAT_SIZE],
}

impl ConquestDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, CONQUEST_DAT_SIZE, "CONQUEST.DAT")?;
        Ok(Self {
            raw: copy_array(data),
        })
    }

    pub fn control_header(&self) -> &[u8] {
        &self.raw[..0x55]
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../original/v1.5")
            .join(name)
    }

    fn read_fixture(name: &str) -> Vec<u8> {
        fs::read(fixture_path(name)).expect("fixture should exist")
    }

    #[test]
    fn round_trip_player_dat() {
        let bytes = read_fixture("PLAYER.DAT");
        let parsed = PlayerDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
    }

    #[test]
    fn round_trip_planets_dat() {
        let bytes = read_fixture("PLANETS.DAT");
        let parsed = PlanetDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
    }

    #[test]
    fn round_trip_setup_dat() {
        let bytes = read_fixture("SETUP.DAT");
        let parsed = SetupDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.version_tag(), b"EC151");
    }

    #[test]
    fn round_trip_conquest_dat() {
        let bytes = read_fixture("CONQUEST.DAT");
        let parsed = ConquestDat::parse(&bytes).unwrap();
        assert_eq!(parsed.to_bytes(), bytes);
        assert_eq!(parsed.control_header().len(), 0x55);
    }

    #[test]
    fn player_tax_rate_matches_current_notes() {
        let bytes = read_fixture("PLAYER.DAT");
        let parsed = PlayerDat::parse(&bytes).unwrap();
        assert_eq!(parsed.records[0].tax_rate(), 65);
    }

    #[test]
    fn shipped_fleets_dat_is_not_the_initialized_layout() {
        let bytes = read_fixture("FLEETS.DAT");
        let err = FleetDat::parse(&bytes).unwrap_err();
        assert_eq!(
            err,
            ParseError::WrongSize {
                file_type: "FLEETS.DAT",
                expected: FLEETS_DAT_SIZE,
                actual: 702,
            }
        );
    }
}
