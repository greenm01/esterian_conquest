use crate::support::{copy_array, ParseError};
use crate::IPBM_RECORD_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpbmRecord {
    pub raw: [u8; IPBM_RECORD_SIZE],
}

impl IpbmRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; IPBM_RECORD_SIZE],
        }
    }

    pub fn primary_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x00], self.raw[0x01]])
    }
    pub fn set_primary_word_raw(&mut self, value: u16) {
        self.raw[0x00..0x02].copy_from_slice(&value.to_le_bytes());
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x02]
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x02] = value;
    }
    pub fn gate_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x03], self.raw[0x04]])
    }
    pub fn set_gate_word_raw(&mut self, value: u16) {
        self.raw[0x03..0x05].copy_from_slice(&value.to_le_bytes());
    }
    pub fn follow_on_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }
    pub fn set_follow_on_word_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }
    pub fn tuple_a_tag_raw(&self) -> u8 {
        self.raw[0x09]
    }
    pub fn set_tuple_a_tag_raw(&mut self, value: u8) {
        self.raw[0x09] = value;
    }
    pub fn tuple_b_tag_raw(&self) -> u8 {
        self.raw[0x0A]
    }
    pub fn set_tuple_b_tag_raw(&mut self, value: u8) {
        self.raw[0x0A] = value;
    }
    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0B..0x10])
    }
    pub fn set_tuple_a_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x0B..0x10].copy_from_slice(&value);
    }
    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x11..0x16])
    }
    pub fn set_tuple_b_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x11..0x16].copy_from_slice(&value);
    }
    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x17..0x1C])
    }
    pub fn set_tuple_c_payload_raw(&mut self, value: [u8; 5]) {
        self.raw[0x17..0x1C].copy_from_slice(&value);
    }
    pub fn trailing_control_raw(&self) -> [u8; 3] {
        copy_array(&self.raw[0x1D..0x20])
    }
    pub fn set_trailing_control_raw(&mut self, value: [u8; 3]) {
        self.raw[0x1D..0x20].copy_from_slice(&value);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpbmDat {
    pub records: Vec<IpbmRecord>,
}

impl IpbmDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % IPBM_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "IPBM.DAT",
                record_size: IPBM_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(IPBM_RECORD_SIZE)
                .map(|chunk| IpbmRecord {
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
