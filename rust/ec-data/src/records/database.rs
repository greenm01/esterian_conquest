use crate::support::{copy_array, ParseError};

pub const DATABASE_RECORD_SIZE: usize = 100;
pub const DATABASE_RECORD_COUNT: usize = 80;
pub const DATABASE_DAT_SIZE: usize = DATABASE_RECORD_SIZE * DATABASE_RECORD_COUNT;

/// A single DATABASE.DAT record (100 bytes).
///
/// DATABASE.DAT contains 80 records of 100 bytes each (8000 bytes total).
/// Structure: 20 planets × 4 player intel slots = 80 records.
/// Each record caches planet display information and derived intel for one player's view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseRecord {
    pub raw: [u8; DATABASE_RECORD_SIZE],
}

impl DatabaseRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; DATABASE_RECORD_SIZE],
        }
    }

    pub fn from_raw(raw: [u8; DATABASE_RECORD_SIZE]) -> Self {
        Self { raw }
    }

    /// Get the raw bytes of the planet name field.
    /// Name is Pascal-style: length byte at offset 0x00, followed by characters at 0x01+.
    /// Maximum name length is 14 characters (fits in 0x00..0x0E).
    pub fn planet_name_bytes(&self) -> &[u8] {
        let len = self.raw[0x00] as usize;
        if len > 0 && len <= 14 {
            &self.raw[0x01..0x01 + len]
        } else {
            &self.raw[0x01..0x01] // Empty slice
        }
    }

    /// Set the planet name using Pascal-style encoding (length prefix at 0x00).
    pub fn set_planet_name(&mut self, name: &str) {
        // Clear the name area first (offsets 0x00 to 0x0E, 15 bytes total)
        self.raw[0x00..0x0F].fill(0);

        let bytes = name.as_bytes();
        let len = bytes.len().min(14); // Max 14 chars

        // Pascal-style: first byte at offset 0x00 is length
        self.raw[0x00] = len as u8;
        self.raw[0x01..0x01 + len].copy_from_slice(&bytes[..len]);
    }

    /// Get the raw name string area (15 bytes at offset 0x00, including length prefix).
    pub fn name_area_raw(&self) -> [u8; 15] {
        copy_array(&self.raw[0x00..0x0F])
    }

    /// Set the entire name area (15 bytes including length prefix at 0x00).
    pub fn set_name_area_raw(&mut self, area: [u8; 15]) {
        self.raw[0x00..0x0F].copy_from_slice(&area);
    }

    /// Get a word at a specific offset (little-endian u16).
    pub fn word_at(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.raw[offset], self.raw[offset + 1]])
    }

    /// Set a word at a specific offset (little-endian u16).
    pub fn set_word_at(&mut self, offset: usize, value: u16) {
        self.raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Get the embedded year word (observed at offset 0x08 in some records).
    /// This is the CONQUEST.DAT year embedded in homeworld planet records.
    pub fn year_word(&self) -> u16 {
        self.word_at(0x08)
    }

    /// Set the embedded year word.
    pub fn set_year_word(&mut self, year: u16) {
        self.set_word_at(0x08, year);
    }

    /// Copy all bytes from a source record.
    pub fn copy_from(&mut self, source: &DatabaseRecord) {
        self.raw.copy_from_slice(&source.raw);
    }
}

/// The complete DATABASE.DAT file (80 records, 8000 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseDat {
    pub records: [DatabaseRecord; DATABASE_RECORD_COUNT],
}

impl DatabaseDat {
    pub fn new_zeroed() -> Self {
        Self {
            records: std::array::from_fn(|_| DatabaseRecord::new_zeroed()),
        }
    }

    /// Parse DATABASE.DAT from bytes.
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() != DATABASE_DAT_SIZE {
            return Err(ParseError::WrongSize {
                file_type: "DATABASE.DAT",
                expected: DATABASE_DAT_SIZE,
                actual: data.len(),
            });
        }

        let records: Vec<DatabaseRecord> = data
            .chunks_exact(DATABASE_RECORD_SIZE)
            .map(|chunk| DatabaseRecord {
                raw: copy_array(chunk),
            })
            .collect();

        // Convert Vec to fixed-size array
        let records_array: [DatabaseRecord; DATABASE_RECORD_COUNT] =
            records
                .try_into()
                .map_err(|_| ParseError::WrongRecordMultiple {
                    file_type: "DATABASE.DAT",
                    record_size: DATABASE_RECORD_SIZE,
                    actual: data.len(),
                })?;

        Ok(Self {
            records: records_array,
        })
    }

    /// Serialize DATABASE.DAT to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }

    /// Get record index for a specific planet and player.
    /// Layout: player 0-3, each has slots for planets 0-19.
    /// Index = player * 20 + planet
    pub fn record_index(planet_index: usize, player_index: usize) -> usize {
        player_index * PLANET_RECORD_COUNT + planet_index
    }

    /// Get mutable reference to a specific planet/player record.
    pub fn record_mut(&mut self, planet_index: usize, player_index: usize) -> &mut DatabaseRecord {
        let idx = Self::record_index(planet_index, player_index);
        &mut self.records[idx]
    }

    /// Get reference to a specific planet/player record.
    pub fn record(&self, planet_index: usize, player_index: usize) -> &DatabaseRecord {
        let idx = Self::record_index(planet_index, player_index);
        &self.records[idx]
    }

    /// Generate DATABASE.DAT from PLANETS.DAT and CONQUEST.DAT year.
    ///
    /// This creates a valid DATABASE.DAT by:
    /// 1. Starting from a template (either zeroed or from an initialized fixture)
    /// 2. Copying planet names from PLANETS.DAT into each player's intel view
    /// 3. Embedding the CONQUEST.DAT year in appropriate locations
    pub fn generate_from_planets_and_year(
        planet_names: &[String],
        _game_year: u16,
        template: Option<&DatabaseDat>,
    ) -> Self {
        let mut result = if let Some(t) = template {
            t.clone()
        } else {
            // Create a default template with "UNKNOWN" names
            let mut default = Self::new_zeroed();
            for player in 0..4 {
                for planet in 0..20 {
                    let record = default.record_mut(planet, player);
                    record.set_planet_name("UNKNOWN");
                }
            }
            default
        };

        // Copy planet names from PLANETS.DAT
        for (planet_idx, name) in planet_names.iter().enumerate().take(20) {
            for player in 0..4 {
                let record = result.record_mut(planet_idx, player);
                record.set_planet_name(name);
            }
        }

        // TODO: Embed CONQUEST.DAT year in homeworld records
        // The year appears to be embedded at specific offsets in homeworld planet records
        // This requires knowing which planets are homeworlds

        result
    }
}

// Import constant from parent module
const PLANET_RECORD_COUNT: usize = 20;
