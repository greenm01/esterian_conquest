use crate::support::{copy_array, expect_size, ParseError};
use crate::{PLANET_RECORD_COUNT, PLANET_RECORD_SIZE, PLANETS_DAT_SIZE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetRecord {
    pub raw: [u8; PLANET_RECORD_SIZE],
}

impl PlanetRecord {
    pub fn header_bytes(&self) -> &[u8] {
        &self.raw[..3]
    }

    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0], self.raw[1]]
    }

    pub fn header_value_raw(&self) -> u8 {
        self.raw[2]
    }

    pub fn string_len(&self) -> u8 {
        self.raw[0x0F]
    }

    pub fn status_or_name_bytes(&self) -> &[u8] {
        &self.raw[0x10..=0x1C]
    }

    pub fn potential_production_raw(&self) -> [u8; 2] {
        [self.raw[0x02], self.raw[0x03]]
    }
    pub fn set_potential_production_raw(&mut self, value: [u8; 2]) {
        self.raw[0x02] = value[0];
        self.raw[0x03] = value[1];
    }

    pub fn factories_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x04..0x0A])
    }
    pub fn set_factories_raw(&mut self, value: [u8; 6]) {
        self.raw[0x04..0x0A].copy_from_slice(&value);
    }

    pub fn stored_goods_raw(&self) -> u32 {
        u32::from_le_bytes(copy_array(&self.raw[0x0A..0x0E]))
    }
    pub fn set_stored_goods_raw(&mut self, value: u32) {
        self.raw[0x0A..0x0E].copy_from_slice(&value.to_le_bytes());
    }

    pub fn planet_tax_rate_raw(&self) -> u8 {
        self.raw[0x0E]
    }
    pub fn set_planet_tax_rate_raw(&mut self, value: u8) {
        self.raw[0x0E] = value;
    }

    pub fn set_status_or_name_summary_raw(&mut self, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(13);
        self.raw[0x0F] = len as u8;
        self.raw[0x10..0x1D].fill(0);
        self.raw[0x10..0x10 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn set_status_or_name_prefix_raw(&mut self, value: &str) {
        let bytes = value.as_bytes();
        let len = bytes.len().min(13);
        self.raw[0x0F] = len as u8;
        self.raw[0x10..0x10 + len].copy_from_slice(&bytes[..len]);
    }

    pub fn build_count_raw(&self, slot: usize) -> u8 {
        self.raw[0x24 + slot]
    }

    pub fn build_kind_raw(&self, slot: usize) -> u8 {
        self.raw[0x2E + slot]
    }

    pub fn set_build_count_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x24 + slot] = value;
    }

    pub fn set_build_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x2E + slot] = value;
    }

    pub fn stardock_count_raw(&self, slot: usize) -> u16 {
        u16::from_le_bytes([self.raw[0x38 + slot * 2], self.raw[0x38 + slot * 2 + 1]])
    }

    pub fn stardock_kind_raw(&self, slot: usize) -> u8 {
        self.raw[0x4C + slot]
    }

    pub fn set_stardock_count_raw(&mut self, slot: usize, value: u16) {
        self.raw[0x38 + slot * 2..0x38 + slot * 2 + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_stardock_kind_raw(&mut self, slot: usize, value: u8) {
        self.raw[0x4C + slot] = value;
    }

    pub fn population_raw(&self) -> [u8; 6] {
        copy_array(&self.raw[0x52..0x58])
    }
    pub fn set_population_raw(&mut self, value: [u8; 6]) {
        self.raw[0x52..0x58].copy_from_slice(&value);
    }

    pub fn owner_empire_slot_raw(&self) -> u8 {
        self.raw[0x5D]
    }
    pub fn set_owner_empire_slot_raw(&mut self, value: u8) {
        self.raw[0x5D] = value;
    }

    pub fn army_count_raw(&self) -> u8 {
        self.raw[0x58]
    }

    pub fn likely_army_count_raw(&self) -> u8 {
        self.raw[0x5A]
    }
    pub fn set_likely_army_count_raw(&mut self, value: u8) {
        self.raw[0x5A] = value;
    }

    pub fn ground_batteries_raw(&self) -> u8 {
        self.raw[0x5A]
    }

    pub fn developed_value_raw(&self) -> u8 {
        self.raw[0x58]
    }
    pub fn set_developed_value_raw(&mut self, value: u8) {
        self.raw[0x58] = value;
    }

    pub fn ownership_status_raw(&self) -> u8 {
        self.raw[0x5C]
    }
    pub fn set_ownership_status_raw(&mut self, value: u8) {
        self.raw[0x5C] = value;
    }

    pub fn status_or_name_summary(&self) -> String {
        let len = self.string_len() as usize;
        let text = &self.status_or_name_bytes()[..len.min(self.status_or_name_bytes().len())];
        String::from_utf8_lossy(text)
            .trim_matches(char::from(0))
            .trim()
            .to_string()
    }

    pub fn is_named_homeworld_seed(&self) -> bool {
        self.status_or_name_summary() == "Not Named Yet"
    }

    pub fn derived_summary(&self) -> String {
        let [x, y] = self.coords_raw();
        let text = self.status_or_name_summary();
        let mut parts = vec![format!("({},{}): {}", x, y, text)];
        if self.is_named_homeworld_seed() {
            parts.push("likely_homeworld_seed".to_string());
        }
        if self.build_count_raw(0) != 0 || self.build_kind_raw(0) != 0 {
            parts.push(format!(
                "build_raw={:02x}/{:02x}",
                self.build_count_raw(0),
                self.build_kind_raw(0)
            ));
        }
        if self.owner_empire_slot_raw() != 0 {
            parts.push(format!(
                "owner_slot={} owner_status={:02x}",
                self.owner_empire_slot_raw(),
                self.ownership_status_raw()
            ));
        }
        if self.likely_army_count_raw() != 0 {
            parts.push(format!("likely_armies={}", self.likely_army_count_raw()));
        }
        if self.developed_value_raw() != 0 {
            parts.push(format!("dev58={}", self.developed_value_raw()));
        }
        parts.join(" | ")
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
