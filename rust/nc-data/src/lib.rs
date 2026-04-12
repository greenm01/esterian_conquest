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

mod bbs_config;
mod builder;
mod config;
mod directory;
mod economy;
#[doc(hidden)]
pub mod fleet_motion_state;
pub mod hosted;
mod intel;
pub mod maintenance_types;
mod map_dimensions;
mod map_export;
mod planet_summary;
mod player_activity;
mod player_lifecycle;
mod player_mail;
mod player_war_stats;
mod records;
mod report_blocks;
mod rng;
mod runtime_reports;
mod starmap;
mod storage;
mod support;
mod turns;

pub use bbs_config::{BbsGameConfig, BbsGameConfigError, SeatReservation};
pub use builder::{FleetOrderSpec, GameStateBuilder, GuardStarbaseSpec, PlanetBuildSpec};
pub use config::{DiplomacyConfig, DiplomacyDirective, SetupConfigError};
pub use directory::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionReport,
    AutoCommissionStarbaseEntry, CampaignOutcome, CampaignOutlook, CampaignState,
    CommissionFleetDraft, CommissionResult, CoreGameData, CurrentKnownComplianceStatus,
    CurrentKnownGuardStarbaseLinkageSummary, CurrentKnownKeyWordSummary, EmpireEconomySummary,
    EmpirePlanetEconomyRow, EmpireProductionRankingRow, EmpireProductionRankingSort,
    EmpireUnitSummary, FleetDetachResult, FleetDetachSelection, FleetOrderValidationError,
    FleetPlayerInputValidationError, FleetTransferResult, GameDirectoryError,
    GameStateMutationError, PlanetPlayerInputValidationError, PlayerDiplomacyValidationError,
};
pub use economy::{
    build_capacity, starbase_growth_bonus_percent, yearly_growth_delta, yearly_high_tax_penalty,
    yearly_tax_revenue,
};
pub use intel::{
    active_starbase_count_at, build_runtime_planet_intel_snapshot,
    latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};
pub use maintenance_types::{
    AssaultReportEvent, BombardEvent, CampaignOutcomeEvent, CampaignOutlookEvent,
    CivilDisorderEvent, ColonizationResolvedEvent, ContactReportSource, DiplomacyOverride,
    DiplomaticEscalationEvent, EmpireEliminationCause, EmpireEliminationEvent,
    EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent, FleetDefectionEvent,
    FleetDestroyedEvent, FleetMergeEvent, GameVictoryNoticeEvent, InvalidPlayerStateEvent,
    JoinMissionHostEvent, MaintenanceEvents, Mission, MissionAbortReason, MissionEvent,
    MissionOutcome, MissionRetargetEvent, PlanetIntelEvent, PlanetIntelSource,
    PlanetOwnershipChangeEvent, SalvageFailureReason, SalvageResolvedEvent, ScoutContactEvent,
    ShipLosses, StarbaseDestroyedEvent,
};
pub use map_dimensions::map_size_for_player_count;
pub use map_export::{
    build_player_map_export_data, PlayerMapExportData, PlayerMapExportFile, STARMAP_CSV_FILE_NAME,
    STARMAP_DETAILS_CSV_FILE_NAME, STARMAP_TEXT_FILE_NAME,
};
pub use planet_summary::{
    build_queue_unit_counts, compact_unit_code, format_build_queue_summary,
    format_owned_orbit_summary, format_stardock_summary, format_unit_counts,
    ordered_unit_count_entries, owned_orbit_presence, owned_planet_status, stardock_unit_counts,
    CompactUnitSummaryStyle, OrbitPresenceSummary, OwnedPlanetStatus,
};
pub use player_activity::{
    apply_inactivity_autopilot_policy, clear_inactivity_autopilot_pending,
    default_player_activity_states, record_interactive_participation,
    record_submitted_turn_participation, DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS,
};
pub use player_lifecycle::{
    default_player_lifecycle_states, empire_has_recovery_path, player_access_mode,
    player_public_status, PlayerAccessMode, PublicEmpireStatus,
};
pub use player_mail::{
    append_mail_queue, clear_mail_queue, load_mail_queue,
    queued_message_count_for_sender_recipient_year, save_mail_queue, validate_queue_message_limit,
    QueuedPlayerMail, MAX_QUEUED_MESSAGES_PER_RECIPIENT_PER_YEAR,
};
pub use player_war_stats::{
    apply_maintenance_events_to_player_war_stats, default_player_war_stats_states,
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
    derive_campaign_seed_from_runtime, generate_campaign_seed, mix_seed, GameRng, RNG_TAG_COMBAT,
    RNG_TAG_MAPGEN,
};
pub use runtime_reports::{
    has_visible_runtime_messages, has_visible_runtime_reports, runtime_inbox_items,
    runtime_inbox_preview_lines, wrap_review_text_preserving_spacing, InboxItem, InboxItemSource,
    InboxItemType, ReportSummaryBucket, ReportsPreview, ReviewBlock,
};
pub use starmap::{
    build_player_starmap_projection_from_snapshots, PlayerStarmapProjection, PlayerStarmapWorld,
};
pub use storage::{
    CampaignRuntimeState, CampaignSettings, CampaignStore, CampaignStoreError, IntelTier,
    PlanetIntelSnapshot, PlayerActivityState, PlayerLifecycleState, PlayerWarStatsState,
    TerminalOutcome, WinnerState, DEFAULT_CAMPAIGN_DB_NAME, DEFAULT_CAMPAIGN_THEME_KEY,
    DEFAULT_MAINTENANCE_INTERVAL_MINUTES,
};
pub use support::{decode_real48, encode_real48, ParseError};
pub use turns::{
    FleetTurnAction, FleetTurnBlock, PlanetTurnAction, PlanetTurnBlock, TurnDiplomacyDirective,
    TurnMessage, TurnSubmission, TurnSubmissionError, TurnSubmissionReport, MAX_MESSAGE_BODY_CHARS,
    MAX_MESSAGE_SUBJECT_CHARS,
};
