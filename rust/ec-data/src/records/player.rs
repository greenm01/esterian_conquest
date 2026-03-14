use crate::support::{ParseError, copy_array, trim_ascii_field};
use crate::PLAYER_RECORD_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomaticRelation {
    Neutral,
    Enemy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub raw: [u8; PLAYER_RECORD_SIZE],
}

impl PlayerRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; PLAYER_RECORD_SIZE],
        }
    }

    pub fn occupied_flag(&self) -> u8 {
        self.raw[0]
    }

    pub fn owner_mode_raw(&self) -> u8 {
        self.raw[0]
    }

    pub fn handle_bytes(&self) -> &[u8] {
        &self.raw[1..0x1A]
    }

    pub fn empire_name_bytes(&self) -> &[u8] {
        &self.raw[0x1C..=0x2E]
    }

    pub fn assigned_player_flag_raw(&self) -> u8 {
        self.raw[0]
    }

    pub fn legacy_status_name_max_len_raw(&self) -> u8 {
        self.raw[26]
    }

    pub fn legacy_status_name_len_raw(&self) -> u8 {
        self.raw[27]
    }

    pub fn legacy_status_name_summary(&self) -> String {
        let len = self.legacy_status_name_len_raw() as usize;
        let end = (28 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[28..end])
    }

    pub fn assigned_player_handle_summary(&self) -> String {
        trim_ascii_field(self.handle_bytes())
    }

    pub fn controlled_empire_name_len_raw(&self) -> u8 {
        self.raw[27]
    }

    pub fn controlled_empire_name_summary(&self) -> String {
        let len = self.controlled_empire_name_len_raw() as usize;
        let end = (28 + len).min(self.raw.len());
        trim_ascii_field(&self.raw[28..end])
    }

    pub fn ownership_summary(&self) -> String {
        let legacy = self.legacy_status_name_summary();
        let handle = self.assigned_player_handle_summary();
        let empire = self.controlled_empire_name_summary();

        if self.owner_mode_raw() == 0xff {
            format!("rogue label='{legacy}'")
        } else if legacy.starts_with("In Civil Disorder") || legacy == "Unowned" {
            format!("unowned label='{legacy}'")
        } else if self.assigned_player_flag_raw() != 0 || !handle.is_empty() || !empire.is_empty() {
            format!("player handle='{}' empire='{}'", handle, empire)
        } else {
            format!("unowned label='{legacy}'")
        }
    }

    pub fn tax_rate(&self) -> u8 {
        self.raw[0x51]
    }

    pub fn set_tax_rate_raw(&mut self, value: u8) {
        self.raw[0x51] = value;
    }

    pub fn starbase_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x44], self.raw[0x45]])
    }

    pub fn set_starbase_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x44] = lo;
        self.raw[0x45] = hi;
    }

    pub fn fleet_chain_head_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x40], self.raw[0x41]])
    }

    pub fn ipbm_count_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x48], self.raw[0x49]])
    }

    pub fn set_ipbm_count_raw(&mut self, value: u16) {
        let [lo, hi] = value.to_le_bytes();
        self.raw[0x48] = lo;
        self.raw[0x49] = hi;
    }

    /// Accumulated production points available to spend this round.
    /// Confirmed from original/v1.5 player 1 state (value=3022, tax=65).
    /// Previously misnamed `last_run_year`.
    pub fn stored_production_pts_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x4E], self.raw[0x4F]])
    }

    /// Unknown u32 at 0x52. Observed as 100 in both the canonical baseline
    /// and original/v1.5 regardless of player state. Likely a percentage cap
    /// or production efficiency constant, not a treasury. Do not treat as
    /// confirmed until further RE.
    pub fn unknown_0x52_raw(&self) -> u32 {
        u32::from_le_bytes([
            self.raw[0x52],
            self.raw[0x53],
            self.raw[0x54],
            self.raw[0x55],
        ])
    }

    /// Autopilot flag. 1 = autopilot on (computer manages empire, mostly
    /// builds planetary defenses). 0 = human player submitting orders.
    /// Confirmed by black-box experiment: clearing this byte eliminates
    /// all autopilot-driven army/battery growth in ECMAINT.
    pub fn autopilot_flag(&self) -> u8 {
        self.raw[0x6D]
    }

    pub fn set_autopilot_flag(&mut self, value: u8) {
        self.raw[0x6D] = value;
    }

    /// Set the occupied/present flag at offset 0x00.
    /// This indicates whether a player slot is active (1) or unjoined (0).
    pub fn set_occupied_flag(&mut self, value: u8) {
        self.raw[0] = value;
    }

    /// Set the owner empire byte at offset 0x00.
    /// Same as occupied_flag for player records.
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0] = value;
    }

    /// Stored diplomatic relation toward another empire, if the raw bytes have
    /// been mapped. This remains unresolved in the classic `PLAYER.DAT`
    /// layout, so callers must handle `None` and fall back to documented
    /// hostility rules.
    pub fn diplomatic_relation_toward(
        &self,
        _other_empire_raw: u8,
    ) -> Option<DiplomaticRelation> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerDat {
    pub records: Vec<PlayerRecord>,
}

impl PlayerDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % PLAYER_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "PLAYER.DAT",
                record_size: PLAYER_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(PLAYER_RECORD_SIZE)
                .map(|chunk| PlayerRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}
