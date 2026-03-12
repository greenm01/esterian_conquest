use crate::support::{copy_array, expect_size, ParseError};
use crate::SETUP_DAT_SIZE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupDat {
    pub raw: [u8; SETUP_DAT_SIZE],
}

impl SetupDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, SETUP_DAT_SIZE, "SETUP.DAT")?;
        Ok(Self { raw: copy_array(data) })
    }

    pub fn version_tag(&self) -> &[u8] { &self.raw[..5] }
    pub fn option_prefix(&self) -> &[u8] { &self.raw[5..13] }
    pub fn com_irq_raw(&self, com_index: usize) -> Option<u8> { (com_index < 4).then(|| self.raw[5 + com_index]) }
    pub fn set_com_irq_raw(&mut self, com_index: usize, irq: u8) -> bool {
        if com_index < 4 { self.raw[5 + com_index] = irq; true } else { false }
    }
    pub fn com_hardware_flow_control_enabled(&self, com_index: usize) -> Option<bool> {
        (com_index < 4).then(|| self.raw[9 + com_index] != 0)
    }
    pub fn set_com_hardware_flow_control_enabled(&mut self, com_index: usize, enabled: bool) -> bool {
        if com_index < 4 { self.raw[9 + com_index] = u8::from(enabled); true } else { false }
    }
    pub fn snoop_enabled(&self) -> bool { self.raw[512] != 0 }
    pub fn set_snoop_enabled(&mut self, enabled: bool) { self.raw[512] = u8::from(enabled); }
    pub fn max_time_between_keys_minutes_raw(&self) -> u8 { self.raw[513] }
    pub fn set_max_time_between_keys_minutes_raw(&mut self, minutes: u8) { self.raw[513] = minutes; }
    pub fn remote_timeout_enabled(&self) -> bool { self.raw[515] != 0 }
    pub fn set_remote_timeout_enabled(&mut self, enabled: bool) { self.raw[515] = u8::from(enabled); }
    pub fn local_timeout_enabled(&self) -> bool { self.raw[516] != 0 }
    pub fn set_local_timeout_enabled(&mut self, enabled: bool) { self.raw[516] = u8::from(enabled); }
    pub fn minimum_time_granted_minutes_raw(&self) -> u8 { self.raw[517] }
    pub fn set_minimum_time_granted_minutes_raw(&mut self, minutes: u8) { self.raw[517] = minutes; }
    pub fn purge_after_turns_raw(&self) -> u8 { self.raw[518] }
    pub fn set_purge_after_turns_raw(&mut self, turns: u8) { self.raw[518] = turns; }
    pub fn autopilot_inactive_turns_raw(&self) -> u8 { self.raw[520] }
    pub fn set_autopilot_inactive_turns_raw(&mut self, turns: u8) { self.raw[520] = turns; }
    pub fn to_bytes(&self) -> Vec<u8> { self.raw.to_vec() }
}
