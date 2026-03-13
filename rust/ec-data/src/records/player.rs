use crate::support::{copy_array, expect_size, trim_ascii_field, ParseError};
use crate::{PLAYER_DAT_SIZE, PLAYER_RECORD_COUNT, PLAYER_RECORD_SIZE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRecord {
    pub raw: [u8; PLAYER_RECORD_SIZE],
}

impl PlayerRecord {
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
