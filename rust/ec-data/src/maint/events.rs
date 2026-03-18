//! Event and type definitions for the maintenance engine.

use crate::{
    DiplomaticRelation, FleetOrderValidationError, FleetPlayerInputValidationError,
    PlanetPlayerInputValidationError, PlayerDiplomacyValidationError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShipLosses {
    pub destroyers: u32,
    pub cruisers: u32,
    pub battleships: u32,
    pub scouts: u32,
    pub transports: u32,
    pub etacs: u32,
}

/// A bombardment event: one fleet executed BombardWorld against one planet.
#[derive(Debug)]
pub struct BombardEvent {
    /// Planet index (into PLANETS.DAT records) that was bombarded.
    pub planet_idx: usize,
    /// Attacking fleet's owner_empire_raw (1-based player index).
    pub attacker_empire_raw: u8,
    /// Attacking fleet ID when one specific fleet can be named.
    pub attacker_fleet_id: Option<u8>,
    /// Defending empire that should receive the bombardment report, if any.
    pub defender_empire_raw: u8,
    /// Initial attacking fleet composition observed by both sides.
    pub attacker_initial: ShipLosses,
    /// Initial defender ground batteries.
    pub defender_batteries_initial: u8,
    /// Initial defender armies.
    pub defender_armies_initial: u8,
    /// Exact attacker fleet losses during the bombardment exchange.
    pub attacker_losses: ShipLosses,
    /// Observed defender ground battery losses.
    pub defender_battery_losses: u8,
    /// Observed defender army losses.
    pub defender_army_losses: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A ground-assault event for invade/blitz mission reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssaultReportEvent {
    /// Fleet mission kind that produced the assault.
    pub kind: Mission,
    /// Attacking fleet ID when one specific fleet can be named.
    pub attacker_fleet_id: Option<u8>,
    /// Planet index (into PLANETS.DAT records) that was attacked.
    pub planet_idx: usize,
    /// Acting empire that should receive the attacker-side report.
    pub attacker_empire_raw: u8,
    /// Defending empire that was attacked, if any.
    pub defender_empire_raw: u8,
    /// Initial attacking fleet composition observed by both sides.
    pub attacker_initial: ShipLosses,
    /// Initial defender ground batteries.
    pub defender_batteries_initial: u8,
    /// Initial defender armies.
    pub defender_armies_initial: u8,
    /// Exact attacker fleet losses during the orbital/landing exchange.
    pub attacker_ship_losses: ShipLosses,
    /// Attacker ground losses.
    pub attacker_army_losses: u32,
    /// Attacker troop losses suffered in destroyed transports during landing.
    pub transport_army_losses: u32,
    /// Defender battery losses.
    pub defender_battery_losses: u8,
    /// Defender army losses.
    pub defender_army_losses: u8,
    /// Final mission outcome.
    pub outcome: MissionOutcome,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A combat-triggered intel refresh for one player's DATABASE view of one planet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetIntelEvent {
    /// Planet index (into PLANETS.DAT records) whose intel should be refreshed.
    pub planet_idx: usize,
    /// Viewer empire (1-based player index) whose DATABASE record should be updated.
    pub viewer_empire_raw: u8,
}

/// A combat-triggered planet ownership change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetOwnershipChangeEvent {
    /// Planet index (into PLANETS.DAT records) that changed owner.
    pub planet_idx: usize,
    /// Empire that should receive the "we were captured" report.
    pub reporting_empire_raw: u8,
    /// Previous owner empire slot (1-based, or 0 if unowned).
    pub previous_owner_empire_raw: u8,
    /// New owner empire slot (1-based).
    pub new_owner_empire_raw: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A fleet battle resolved at one location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetBattlePerspective {
    Attacked,
    Intercepted,
}

/// A fleet battle resolved at one location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetBattleEvent {
    /// Empire that should receive this battle report.
    pub reporting_empire_raw: u8,
    /// Reporting fleet ID when one specific fleet can be named.
    pub reporting_fleet_id: Option<u8>,
    /// Reporting fleet mission context when one classic mission-family label applies.
    pub reporting_mission: Option<Mission>,
    /// Whether the report should read as "we were attacked" or "we intercepted".
    pub perspective: FleetBattlePerspective,
    /// Coordinates where the battle took place.
    pub coords: [u8; 2],
    /// Hostile empires this side encountered.
    pub enemy_empires_raw: Vec<u8>,
    /// Primary hostile fleet ID when one specific enemy fleet can be named.
    pub primary_enemy_fleet_id: Option<u8>,
    /// Whether the reporting empire held the field after the battle.
    pub held_field: bool,
    /// Initial composition of the reporting force.
    pub friendly_initial: ShipLosses,
    /// Exact losses suffered by the reporting empire.
    pub friendly_losses: ShipLosses,
    /// Initial observed hostile composition across opposing forces.
    pub enemy_initial: ShipLosses,
    /// Observed hostile losses across the opposing forces.
    pub enemy_losses: ShipLosses,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A fleet was completely destroyed and command lost all contact with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetDestroyedEvent {
    /// Empire that receives the destruction report.
    pub reporting_empire_raw: u8,
    /// Fleet id that was lost.
    pub fleet_id: u8,
    /// Coordinates of the loss.
    pub coords: [u8; 2],
    /// Whether the fleet was attacking/intercepting or was attacked.
    pub was_intercepting: bool,
    /// Initial composition of the lost fleet.
    pub friendly_initial: ShipLosses,
    /// Initial observed hostile composition.
    pub enemy_initial: ShipLosses,
    /// Observed hostile losses before contact was lost.
    pub enemy_losses: ShipLosses,
    /// Armies carried by the lost fleet.
    pub friendly_armies: u32,
    /// Hostile empire if a primary enemy can be named.
    pub primary_enemy_empire_raw: Option<u8>,
    /// Hostile fleet ID if one specific enemy fleet can be named.
    pub primary_enemy_fleet_id: Option<u8>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A starbase was completely destroyed and command lost all contact with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarbaseDestroyedEvent {
    /// Empire that receives the destruction report.
    pub reporting_empire_raw: u8,
    /// Starbase ID that was lost.
    pub starbase_id: u8,
    /// Coordinates of the loss.
    pub coords: [u8; 2],
    /// Initial observed hostile composition.
    pub enemy_initial: ShipLosses,
    /// Observed hostile losses before contact was lost.
    pub enemy_losses: ShipLosses,
    /// Hostile empire if a primary enemy can be named.
    pub primary_enemy_empire_raw: Option<u8>,
    /// Hostile fleet ID if one specific enemy fleet can be named.
    pub primary_enemy_fleet_id: Option<u8>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A scout-style hostile contact report resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactReportSource {
    FleetMission(Mission),
    Fleet(u8),
    Starbase(u8),
}

/// A scout-style hostile contact report resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoutContactEvent {
    /// Empire that owns the observing fleet.
    pub viewer_empire_raw: u8,
    /// Fleet or starbase source that made the contact.
    pub source: ContactReportSource,
    /// Fleet ID of the reporting fleet when the source is a fleet.
    pub reporting_fleet_id: Option<u8>,
    /// Coordinates where the contact occurred.
    pub coords: [u8; 2],
    /// Empire that was detected.
    pub target_empire_raw: u8,
    /// Target fleet ID when one specific hostile fleet can be named.
    pub target_fleet_id: Option<u8>,
    /// Aggregate "small vessel" count in the detected force.
    pub small_vessels: u32,
    /// Aggregate "medium vessel" count in the detected force.
    pub medium_vessels: u32,
    /// Aggregate "large vessel" count in the detected force.
    pub large_vessels: u32,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A friendly merge result for join/rendezvous style orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetMergeEvent {
    /// Fleet index in FLEETS.DAT that merged away.
    pub fleet_idx: usize,
    /// Empire that owned the merging fleet.
    pub owner_empire_raw: u8,
    /// Kind of merge-producing mission.
    pub kind: Mission,
    /// Host fleet ID that remained after the merge.
    pub host_fleet_id: u8,
    /// Fleet ID that was absorbed/merged away.
    pub absorbed_fleet_id: u8,
    /// Coordinates where the merge occurred.
    pub coords: [u8; 2],
    /// Whether this is the survivor-side "absorbing" report.
    pub survivor_side: bool,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A join mission whose host fleet changed or was lost during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMissionHostEvent {
    /// The intended host merged into another surviving fleet.
    Retargeted {
        /// Joining fleet index in FLEETS.DAT.
        fleet_idx: usize,
        /// Empire that owned the joining fleet.
        owner_empire_raw: u8,
        /// Previous host fleet ID.
        previous_host_fleet_id: u8,
        /// New surviving host fleet ID.
        new_host_fleet_id: u8,
        /// Current location of the joining fleet.
        coords: [u8; 2],
    },
    /// The intended host was destroyed and the joining fleet abandoned its mission.
    HostDestroyed {
        /// Joining fleet index in FLEETS.DAT.
        fleet_idx: usize,
        /// Empire that owned the joining fleet.
        owner_empire_raw: u8,
        /// Destroyed host fleet ID.
        destroyed_host_fleet_id: u8,
        /// Current location of the joining fleet.
        coords: [u8; 2],
    },
}

/// A fleet mission whose semantic target changed or disappeared during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRetargetEvent {
    Retargeted {
        fleet_idx: usize,
        owner_empire_raw: u8,
        mission: Mission,
        previous_target_coords: [u8; 2],
        new_target_coords: [u8; 2],
    },
    Abandoned {
        fleet_idx: usize,
        owner_empire_raw: u8,
        mission: Mission,
        previous_target_coords: [u8; 2],
        coords: [u8; 2],
    },
}

/// The generic outcome class for a mission report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionOutcome {
    Arrived,
    Succeeded,
    Failed,
    Aborted,
}

/// Mission kinds that currently participate in typed maintenance reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mission {
    MoveOnly,
    SeekHome,
    PatrolSector,
    ViewWorld,
    Salvage,
    GuardStarbase,
    GuardBlockadeWorld,
    JoinAnotherFleet,
    RendezvousSector,
    ColonizeWorld,
    BombardWorld,
    InvadeWorld,
    BlitzWorld,
    ScoutSector,
    ScoutSolarSystem,
}

/// A generic mission-resolution report event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionEvent {
    /// Fleet index in FLEETS.DAT that attempted the mission.
    pub fleet_idx: usize,
    /// Empire that owned the acting fleet (1-based player index).
    pub owner_empire_raw: u8,
    /// Mission kind.
    pub kind: Mission,
    /// Resolved outcome class.
    pub outcome: MissionOutcome,
    /// Target planet index when the mission is planet-directed.
    pub planet_idx: Option<usize>,
    /// Coordinates where the mission resolved, if known.
    pub location_coords: Option<[u8; 2]>,
    /// Original mission target coordinates, if known.
    pub target_coords: Option<[u8; 2]>,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterDispositionReason {
    RoeDeclined,
    RoeWithdrawal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterDispositionEvent {
    NoEngagement {
        fleet_idx: usize,
        owner_empire_raw: u8,
        mission: Option<Mission>,
        coords: [u8; 2],
        target_empire_raw: u8,
        target_fleet_id: Option<u8>,
        small_vessels: u32,
        medium_vessels: u32,
        large_vessels: u32,
        reason: EncounterDispositionReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    Retreated {
        fleet_idx: usize,
        owner_empire_raw: u8,
        mission: Option<Mission>,
        coords: [u8; 2],
        target_empire_raw: u8,
        target_fleet_id: Option<u8>,
        enemy_initial: ShipLosses,
        retreat_target_coords: [u8; 2],
        losses_sustained: ShipLosses,
        enemy_losses_inflicted: ShipLosses,
        reason: EncounterDispositionReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidPlayerStateEvent {
    FleetMission {
        fleet_idx: usize,
        owner_empire_raw: u8,
        order_code_raw: u8,
        coords: [u8; 2],
        reason: FleetOrderValidationError,
    },
    FleetInput {
        fleet_idx: usize,
        owner_empire_raw: u8,
        coords: [u8; 2],
        reason: FleetPlayerInputValidationError,
    },
    PlanetInput {
        planet_idx: usize,
        owner_empire_raw: u8,
        coords: [u8; 2],
        reason: PlanetPlayerInputValidationError,
    },
    PlayerTaxRate {
        player_idx: usize,
        owner_empire_raw: u8,
        tax_rate: u8,
    },
    DiplomacyInput {
        player_idx: usize,
        owner_empire_raw: u8,
        reason: PlayerDiplomacyValidationError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalvageFailureReason {
    NoPlanetAtTarget,
    PlanetNotOwned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalvageResolvedEvent {
    Succeeded {
        fleet_idx: usize,
        owner_empire_raw: u8,
        planet_idx: usize,
        coords: [u8; 2],
        recovered_points: u32,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    Failed {
        fleet_idx: usize,
        owner_empire_raw: u8,
        planet_idx: Option<usize>,
        coords: [u8; 2],
        reason: SalvageFailureReason,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomacyOverride {
    pub from_empire_raw: u8,
    pub to_empire_raw: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiplomaticEscalationEvent {
    pub left_empire_raw: u8,
    pub right_empire_raw: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivilDisorderEvent {
    pub reporting_empire_raw: u8,
    pub prior_label: String,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignOutlookEvent {
    pub empire_raw: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignOutcomeEvent {
    pub emperor_empire_raw: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetDefectionEvent {
    pub reporting_empire_raw: u8,
    pub fleet_id: u8,
    /// Week of year (1–52) when this event occurred; None until canonicalized.
    pub stardate_week: Option<u8>,
}

/// A colonization outcome resolved during maintenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColonizationResolvedEvent {
    /// The fleet established a new colony.
    Succeeded {
        /// Fleet index in FLEETS.DAT that completed the mission.
        fleet_idx: usize,
        /// Planet index (into PLANETS.DAT records) that was colonized.
        planet_idx: usize,
        /// Empire that established the colony (1-based player index).
        colonizer_empire_raw: u8,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
    /// The fleet reached the target world but it was already occupied.
    BlockedByOwner {
        /// Fleet index in FLEETS.DAT that attempted the mission.
        fleet_idx: usize,
        /// Planet index (into PLANETS.DAT records) that blocked colonization.
        planet_idx: usize,
        /// Empire that attempted the colony mission (1-based player index).
        colonizer_empire_raw: u8,
        /// Current owner of the world (1-based player index).
        owner_empire_raw: u8,
        /// Week of year (1–52) when this event occurred; None until canonicalized.
        stardate_week: Option<u8>,
    },
}

/// Events produced by a single maintenance turn, for use by callers
/// (e.g. DATABASE.DAT regeneration in the CLI layer).
#[derive(Debug, Default)]
pub struct MaintenanceEvents {
    /// Bombardment events: each describes one fleet-vs-planet bombardment.
    pub bombard_events: Vec<BombardEvent>,
    /// Planet intel refresh events generated by combat.
    pub planet_intel_events: Vec<PlanetIntelEvent>,
    /// Ownership changes caused by combat.
    pub ownership_change_events: Vec<PlanetOwnershipChangeEvent>,
    /// Fleet battle summaries for later reporting layers.
    pub fleet_battle_events: Vec<FleetBattleEvent>,
    /// Command-center reports for fleets that were totally destroyed.
    pub fleet_destroyed_events: Vec<FleetDestroyedEvent>,
    /// Command-center reports for starbases that were totally destroyed.
    pub starbase_destroyed_events: Vec<StarbaseDestroyedEvent>,
    /// Attacker-side invade/blitz reports with bilateral ground losses.
    pub assault_report_events: Vec<AssaultReportEvent>,
    /// Scout-style hostile contact reports.
    pub scout_contact_events: Vec<ScoutContactEvent>,
    /// Contact/retreat outcomes driven by ROE and hostile encounters.
    pub encounter_disposition_events: Vec<EncounterDispositionEvent>,
    /// Sanitization reports for invalid player-authored state.
    pub invalid_player_state_events: Vec<InvalidPlayerStateEvent>,
    /// Friendly merge reports for join/rendezvous outcomes.
    pub fleet_merge_events: Vec<FleetMergeEvent>,
    /// Join mission host retarget/destruction reports.
    pub join_host_events: Vec<JoinMissionHostEvent>,
    /// Semantic mission target refresh/abandon reports.
    pub mission_retarget_events: Vec<MissionRetargetEvent>,
    /// Successful colonization outcomes.
    pub colonization_events: Vec<ColonizationResolvedEvent>,
    /// Generic mission outcomes for report generation.
    pub mission_events: Vec<MissionEvent>,
    /// Salvage mission outcomes with recovered production detail.
    pub salvage_events: Vec<SalvageResolvedEvent>,
    /// Diplomatic escalations caused by hostile action during maint.
    pub diplomatic_escalation_events: Vec<DiplomaticEscalationEvent>,
    /// Empires that fell into civil disorder this turn.
    pub civil_disorder_events: Vec<CivilDisorderEvent>,
    /// A new sole remaining serious contender emerged this turn.
    pub campaign_outlook_events: Vec<CampaignOutlookEvent>,
    /// A stable sole contender is now recognized as emperor.
    pub campaign_outcome_events: Vec<CampaignOutcomeEvent>,
    /// A civil-disorder empire lost another fleet to defection.
    pub fleet_defection_events: Vec<FleetDefectionEvent>,
}
