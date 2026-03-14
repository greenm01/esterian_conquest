use crate::support::{ParseError, copy_array, expect_size};
use crate::{CONQUEST_DAT_SIZE, MAINTENANCE_DAY_ENABLED_CODES};

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
    pub fn game_year(&self) -> u16 {
        u16::from_le_bytes([self.raw[0], self.raw[1]])
    }
    pub fn player_count(&self) -> u8 {
        self.raw[2]
    }
    pub fn player_config_word(&self) -> u16 {
        u16::from_le_bytes([self.raw[2], self.raw[3]])
    }
    pub fn maintenance_schedule_bytes(&self) -> [u8; 7] {
        self.raw[3..10]
            .try_into()
            .expect("maintenance schedule should be 7 bytes")
    }
    pub fn maintenance_schedule_enabled(&self) -> [bool; 7] {
        self.maintenance_schedule_bytes().map(|byte| byte != 0)
    }
    pub fn set_maintenance_schedule_enabled(&mut self, enabled: [bool; 7]) {
        for (idx, enabled) in enabled.into_iter().enumerate() {
            self.raw[3 + idx] = if enabled {
                MAINTENANCE_DAY_ENABLED_CODES[idx]
            } else {
                0
            };
        }
    }
    pub fn header_words(&self) -> Vec<u16> {
        self.control_header()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    /// Set the game year (offset 0x00..0x01, little-endian u16).
    pub fn set_game_year(&mut self, year: u16) {
        self.raw[0..2].copy_from_slice(&year.to_le_bytes());
    }

    /// Set the player count (offset 0x02).
    pub fn set_player_count(&mut self, count: u8) {
        self.raw[2] = count;
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}
