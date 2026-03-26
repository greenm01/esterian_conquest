use super::*;
use crate::fleet_motion_state::reset_motion_state_for_new_orders;
use crate::{
    BaseDat, BaseRecord, DiplomaticRelation, FleetRecord, IPBM_RECORD_SIZE, IpbmDat, IpbmRecord,
    Order, PlayerRecord, ProductionItemKind,
};

#[path = "mutators_commission.rs"]
mod commission;
#[path = "mutators_fleet.rs"]
mod fleet;
#[path = "mutators_planets.rs"]
mod planets;
#[path = "mutators_player.rs"]
mod player;
#[path = "mutators_starbase.rs"]
mod starbase;
#[path = "mutators_support.rs"]
mod support;
#[path = "mutators_transfers.rs"]
mod transfers;
