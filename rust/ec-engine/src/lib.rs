pub mod maint;
pub mod navigation;
pub mod setup;

pub use ec_data::{
    build_capacity, map_size_for_player_count, yearly_growth_delta, yearly_high_tax_penalty,
    yearly_tax_revenue, AssaultReportEvent, BaseRecord, BombardEvent, CampaignOutcome,
    CampaignOutcomeEvent, CampaignOutlook, CampaignOutlookEvent, CampaignState, CivilDisorderEvent,
    ColonizationResolvedEvent, CommissionResult, ConquestDat, ContactReportSource, CoreGameData,
    DiplomacyConfig, DiplomacyDirective, DiplomacyOverride, DiplomaticEscalationEvent,
    DiplomaticRelation, EmpireEconomySummary, EmpirePlanetEconomyRow, EmpireProductionRankingRow,
    EmpireProductionRankingSort, EncounterDispositionEvent, EncounterDispositionReason,
    FleetBattleEvent, FleetDefectionEvent, FleetDestroyedEvent, FleetMergeEvent, FleetOrderSpec,
    FleetOrderValidationError, FleetPlayerInputValidationError, GameRng, GameStateBuilder,
    GuardStarbaseSpec, InvalidPlayerStateEvent, JoinMissionHostEvent, MaintenanceEvents, Mission,
    MissionEvent, MissionOutcome, MissionRetargetEvent, Order, PlanetBuildSpec, PlanetIntelEvent,
    PlanetIntelSource, PlanetOwnershipChangeEvent, PlanetPlayerInputValidationError, PlanetRecord,
    PlayerDiplomacyValidationError, ProductionItemKind, SalvageFailureReason, SalvageResolvedEvent,
    ScoutContactEvent, SetupConfig, SetupConfigError, SetupOptionsConfig, ShipLosses,
    StarbaseDestroyedEvent, CONQUEST_DAT_SIZE, RNG_TAG_COMBAT, STARDOCK_SLOT_COUNT,
};
pub use maint::{
    process_autopilot_ai, run_maintenance_turn, run_maintenance_turn_with_context,
    run_maintenance_turn_with_context_and_seed, run_maintenance_turn_with_seed,
    run_maintenance_turn_with_visible_hazards, run_maintenance_turn_with_visible_hazards_and_seed,
    run_maintenance_turns,
};
pub use navigation::{
    estimate_fleet_eta, estimate_fleet_eta_to_destination, next_path_step, plan_route,
    plan_route_with_intel, visible_hazard_intel_from_snapshots, FleetEtaEstimate, PlannedRoute,
    RouteStep, VisibleHazardIntel,
};
pub use setup::{
    build_game_data_from_setup_config, build_seeded_initialized_game, build_seeded_new_game,
    generate_map, GeneratedMap, GeneratedWorld, MapMetrics,
};
