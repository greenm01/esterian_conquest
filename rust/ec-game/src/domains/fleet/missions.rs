use ec_data::FleetRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetMissionRequirement {
    Any,
    CombatShips,
    CombatAndLoadedTransports,
    LoadedTransports,
    AtLeastOneScout,
    AtLeastOneEtac,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetMissionOption {
    pub code: u8,
    pub mission: &'static str,
    pub requirements: &'static str,
    pub requirement: FleetMissionRequirement,
}

pub const FLEET_MISSION_OPTIONS: [FleetMissionOption; 16] = [
    FleetMissionOption {
        code: 0,
        mission: "None (hold position)",
        requirements: "Any ships",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 1,
        mission: "Move Fleet (only)",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 2,
        mission: "Seek Home",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 3,
        mission: "Patrol a Sector",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 4,
        mission: "Guard a Starbase",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 5,
        mission: "Guard/Blockade a World",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 6,
        mission: "Bombard a World",
        requirements: "Combat ships",
        requirement: FleetMissionRequirement::CombatShips,
    },
    FleetMissionOption {
        code: 7,
        mission: "Invade a World",
        requirements: "Combat + loaded transports",
        requirement: FleetMissionRequirement::CombatAndLoadedTransports,
    },
    FleetMissionOption {
        code: 8,
        mission: "Blitz a World",
        requirements: "Loaded transports (combat recommended)",
        requirement: FleetMissionRequirement::LoadedTransports,
    },
    FleetMissionOption {
        code: 9,
        mission: "View a World",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 10,
        mission: "Scout a Sector",
        requirements: "At least one scout",
        requirement: FleetMissionRequirement::AtLeastOneScout,
    },
    FleetMissionOption {
        code: 11,
        mission: "Scout a Solar System",
        requirements: "At least one scout",
        requirement: FleetMissionRequirement::AtLeastOneScout,
    },
    FleetMissionOption {
        code: 12,
        mission: "Colonize a World",
        requirements: "At least one ETAC",
        requirement: FleetMissionRequirement::AtLeastOneEtac,
    },
    FleetMissionOption {
        code: 13,
        mission: "Join another fleet",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 14,
        mission: "Rendezvous at Sector",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
    FleetMissionOption {
        code: 15,
        mission: "Salvage",
        requirements: "Any",
        requirement: FleetMissionRequirement::Any,
    },
];

pub fn fleet_record_supports_requirement(
    fleet: &FleetRecord,
    requirement: FleetMissionRequirement,
) -> bool {
    let has_combat =
        fleet.battleship_count() > 0 || fleet.cruiser_count() > 0 || fleet.destroyer_count() > 0;
    let has_loaded_troops = fleet.army_count() > 0;
    let has_scout = fleet.scout_count() > 0;
    let has_etac = fleet.etac_count() > 0;

    match requirement {
        FleetMissionRequirement::Any => true,
        FleetMissionRequirement::CombatShips => has_combat,
        FleetMissionRequirement::CombatAndLoadedTransports => has_combat && has_loaded_troops,
        FleetMissionRequirement::LoadedTransports => has_loaded_troops,
        FleetMissionRequirement::AtLeastOneScout => has_scout,
        FleetMissionRequirement::AtLeastOneEtac => has_etac,
    }
}

pub fn fleet_record_supports_mission_code(fleet: &FleetRecord, order_code: u8) -> bool {
    FLEET_MISSION_OPTIONS
        .iter()
        .find(|option| option.code == order_code)
        .map(|option| fleet_record_supports_requirement(fleet, option.requirement))
        .unwrap_or(false)
}
