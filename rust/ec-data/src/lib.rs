pub const PLAYER_RECORD_SIZE: usize = 88;
pub const PLAYER_RECORD_COUNT: usize = 5;
pub const PLAYER_DAT_SIZE: usize = PLAYER_RECORD_SIZE * PLAYER_RECORD_COUNT;

pub const PLANET_RECORD_SIZE: usize = 97;
pub const PLANET_RECORD_COUNT: usize = 20;
pub const PLANETS_DAT_SIZE: usize = PLANET_RECORD_SIZE * PLANET_RECORD_COUNT;

pub const FLEET_RECORD_SIZE: usize = 54;
pub const INITIALIZED_FLEET_RECORD_COUNT: usize = 16;
pub const INITIALIZED_FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * INITIALIZED_FLEET_RECORD_COUNT;
pub const BASE_RECORD_SIZE: usize = 35;
pub const IPBM_RECORD_SIZE: usize = 32;

pub const SETUP_DAT_SIZE: usize = 522;
pub const CONQUEST_DAT_SIZE: usize = 2085;
pub const MAINTENANCE_DAY_ENABLED_CODES: [u8; 7] = [0x01, 0x01, 0xCA, 0x01, 0x0A, 0x01, 0x26];
mod records;
mod support;

pub use records::base::{BaseDat, BaseRecord};
pub use records::conquest::ConquestDat;
pub use records::fleet::{FleetDat, FleetRecord, FleetStandingOrderKind};
pub use records::ipbm::{IpbmDat, IpbmRecord};
pub use records::planet::{PlanetDat, PlanetRecord};
pub use records::player::{PlayerDat, PlayerRecord};
pub use records::setup::SetupDat;
pub use support::ParseError;
