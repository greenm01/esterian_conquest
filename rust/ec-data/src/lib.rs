pub const PLAYER_RECORD_SIZE: usize = 110;

pub const PLANET_RECORD_SIZE: usize = 97;

pub const FLEET_RECORD_SIZE: usize = 54;
pub const INITIALIZED_FLEET_RECORD_COUNT: usize = 16;
pub const INITIALIZED_FLEETS_DAT_SIZE: usize = FLEET_RECORD_SIZE * INITIALIZED_FLEET_RECORD_COUNT;
pub const BASE_RECORD_SIZE: usize = 35;
pub const IPBM_RECORD_SIZE: usize = 32;

pub const SETUP_DAT_SIZE: usize = 522;
pub const CONQUEST_DAT_SIZE: usize = 2085;
pub const DATABASE_RECORD_SIZE: usize = 100;
pub const MAINTENANCE_DAY_ENABLED_CODES: [u8; 7] = [0x01, 0x01, 0xCA, 0x01, 0x0A, 0x01, 0x26];

mod builder;
mod config;
mod directory;
mod economy;
mod intel;
mod mapgen;
mod movement_geometry;
mod pathfinding;
mod player_mail;
mod records;
mod report_blocks;
mod rng;
mod starmap;
mod storage;
mod support;
mod turns;
pub mod maintenance_types;

pub use builder::{
    CanonicalFourPlayerSetup, FleetOrderSpec, GameStateBuilder, GuardStarbaseSpec, PlanetBuildSpec,
};
pub use config::{
    DiplomacyConfig, DiplomacyDirective, SetupConfig, SetupConfigError, SetupMode,
    SetupOptionsConfig,
};
pub use directory::{
    AutoCommissionSummary, CampaignOutcome, CampaignOutlook, CampaignState, CommissionResult,
    CoreGameData, CurrentKnownComplianceStatus, CurrentKnownGuardStarbaseLinkageSummary,
    CurrentKnownKeyWordSummary, EmpireEconomySummary, EmpirePlanetEconomyRow,
    EmpireProductionRankingRow, EmpireProductionRankingSort, EmpireUnitSummary, FleetDetachResult,
    FleetDetachSelection, FleetOrderValidationError, FleetPlayerInputValidationError,
    FleetTransferResult, GameDirectoryError, GameStateMutationError,
    PlanetPlayerInputValidationError, PlayerDiplomacyValidationError,
};
pub use economy::{
    build_capacity, yearly_growth_delta, yearly_high_tax_penalty, yearly_tax_revenue,
};
pub use intel::merge_player_intel_from_runtime;
pub use maintenance_types::{
    AssaultReportEvent, BombardEvent, CampaignOutcomeEvent, CampaignOutlookEvent,
    CivilDisorderEvent, ColonizationResolvedEvent, ContactReportSource, DiplomacyOverride,
    DiplomaticEscalationEvent, EncounterDispositionEvent, EncounterDispositionReason,
    FleetBattleEvent, FleetDefectionEvent, FleetDestroyedEvent, FleetMergeEvent,
    InvalidPlayerStateEvent, JoinMissionHostEvent, MaintenanceEvents, Mission, MissionEvent,
    MissionOutcome, MissionRetargetEvent, PlanetIntelEvent, PlanetIntelSource,
    PlanetOwnershipChangeEvent, SalvageFailureReason, SalvageResolvedEvent, ScoutContactEvent,
    ShipLosses, StarbaseDestroyedEvent,
};
pub use mapgen::{
    GeneratedMap, GeneratedWorld, build_seeded_initialized_game, build_seeded_new_game,
    generate_map, map_size_for_player_count,
};
#[doc(hidden)]
pub use movement_geometry::{
    advance_exact_position, decode_exact_position, reset_motion_state_for_new_orders,
    rounded_coords_from_exact, store_exact_position, visible_hazard_intel_is_empty,
};
pub use pathfinding::{
    FleetEtaEstimate, PlannedRoute, RouteStep, VisibleHazardIntel, estimate_fleet_eta,
    estimate_fleet_eta_to_destination, next_path_step, plan_route, plan_route_with_intel,
    visible_hazard_intel_from_snapshots,
};
pub use player_mail::{
    QueuedPlayerMail, append_mail_queue, clear_mail_queue, load_mail_queue, save_mail_queue,
};
pub use records::base::{BaseDat, BaseRecord};
pub use records::conquest::ConquestDat;
pub use records::fleet::{FleetDat, FleetRecord, Order};
pub use records::ipbm::{IpbmDat, IpbmRecord};
pub use records::planet::{PlanetDat, PlanetRecord, ProductionItemKind, STARDOCK_SLOT_COUNT};
pub use records::player::{DiplomaticRelation, PlayerDat, PlayerRecord};
pub use records::setup::SetupDat;
pub use report_blocks::ReportBlockRow;
pub use rng::{
    GameRng, RNG_TAG_COMBAT, RNG_TAG_MAPGEN, derive_campaign_seed_from_runtime,
    generate_campaign_seed, mix_seed,
};
pub use starmap::{
    PlayerStarmapProjection, PlayerStarmapWorld, build_player_starmap_projection_from_snapshots,
};
pub use storage::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, DEFAULT_CAMPAIGN_DB_NAME, IntelTier,
    PlanetIntelSnapshot,
};
pub use support::{ParseError, decode_real48, encode_real48};
pub use turns::{
    FleetTurnAction, FleetTurnBlock, MAX_MESSAGE_BODY_CHARS, MAX_MESSAGE_SUBJECT_CHARS,
    PlanetTurnAction, PlanetTurnBlock, TurnDiplomacyDirective, TurnMessage, TurnSubmission,
    TurnSubmissionError, TurnSubmissionReport,
};
