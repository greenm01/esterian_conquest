//! Stardate / week assignment helpers for the maintenance engine.
//!
//! The classic EC engine uses a 1..52 internal weekly timeline per maintenance
//! year.  Each event is stamped with a week-of-year tick derived from:
//!
//! 1. A **base week** that reflects when the mission/event nominally occurs
//!    within the maintenance year (travel time, standing-mission vs. arrival).
//! 2. A **timing-code offset** recovered from `1000:a26e` in the original
//!    binary.  The offset shifts the base week forward to produce the
//!    player-visible `Stardate: <week>/<year>` header.
//!
//! Both helpers are used by [`super::canonicalize`] after event assembly.

use super::Mission;

// ---------------------------------------------------------------------------
// Timing-code offset table
// Recovered from ec-timing-spec.md (lines 407–409 of the 2026-03 revision).
// ---------------------------------------------------------------------------

/// Timing codes corresponding to mission/event families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingCode {
    /// Fleet combat / sensor contact family: +2 weeks.
    FleetCombat,
    /// Reserved / unfed slot: +7 weeks (no active writer in preserved image).
    Reserved2,
    /// Scout / view-world / IPBM family: +21 weeks.
    ScoutView,
    /// Standing mission family (orbit, guard, patrol, rendezvous): +0 weeks.
    Standing4,
    Standing5,
    Standing6,
    /// IPBM summary family: +0 weeks (bound to kind-3 dispatch).
    IpbmSummary7,
    /// High-offset consumer-side case: +30 weeks (no active writer).
    HighOffset8,
}

/// Return the week offset for a given timing code.
///
/// Values match the recovered `1000:a26e` offset table.
pub fn timing_code_offset(code: TimingCode) -> u8 {
    match code {
        TimingCode::FleetCombat => 2,
        TimingCode::Reserved2 => 7,
        TimingCode::ScoutView => 21,
        TimingCode::Standing4
        | TimingCode::Standing5
        | TimingCode::Standing6
        | TimingCode::IpbmSummary7 => 0,
        TimingCode::HighOffset8 => 30,
    }
}

/// Apply a timing-code offset to a base week, clamping to [1, 52].
pub fn apply_timing_offset(base_week: u8, code: TimingCode) -> u8 {
    let offset = timing_code_offset(code) as i32;
    let raw = base_week as i32 + offset;
    raw.clamp(1, 52) as u8
}

// ---------------------------------------------------------------------------
// Mission-family → timing code mapping
// ---------------------------------------------------------------------------

/// Return the timing code that governs when a mission event is stamped.
///
/// Derived from the mission-family mapping described in the Phase-1 plan:
/// - Fleet combat / contact → `FleetCombat` (Code 1, +2)
/// - Scout / view-world     → `ScoutView`   (Code 3, +21)
/// - Standing missions      → `Standing4`   (Code 4, +0)
pub fn mission_timing_code(mission: Mission) -> TimingCode {
    match mission {
        // Scout / view families use Code 3 (+21).
        Mission::ScoutSector | Mission::ScoutSolarSystem | Mission::ViewWorld => {
            TimingCode::ScoutView
        }
        // Standing-orbit and guarding families use Code 4 (+0).
        Mission::PatrolSector
        | Mission::GuardStarbase
        | Mission::GuardBlockadeWorld
        | Mission::RendezvousSector => TimingCode::Standing4,
        // All other fleet missions (movement, combat, colonize, etc.) use Code 1 (+2).
        _ => TimingCode::FleetCombat,
    }
}

// ---------------------------------------------------------------------------
// Base week derivation
// ---------------------------------------------------------------------------

/// Derive the base week from mission and travel characteristics.
///
/// Corpus anchors (from ec-timing-spec.md Strongest Behavioral Evidence):
/// - 1-year travel at speed ≥3 → base ≈ 30  (e.g. 1st Fleet colonize → week 32 = 30+2)
/// - Standing / orbit mission  → base = 1
/// - Guard-starbase arrival    → base = 1 in arrival year
///
/// For multi-year travel the fleet arrives at the start of its arrival year,
/// so the base week stays near 1.
pub fn event_base_week(mission: Mission, travel_time_years: u8, fleet_speed: u8) -> u8 {
    // Standing missions don't travel; they start at round begin.
    let is_standing = matches!(
        mission,
        Mission::PatrolSector
            | Mission::GuardStarbase
            | Mission::GuardBlockadeWorld
            | Mission::RendezvousSector
    );
    if is_standing {
        return 1;
    }

    // If no travel happened (fleet already at destination), start of year.
    if travel_time_years == 0 || fleet_speed == 0 {
        return 1;
    }

    if travel_time_years == 1 {
        // 1-year travel: corpus anchor → base ≈ 30.
        // Exact week depends on speed; for speed 3 the empirical base is ~30.
        // Scale linearly: speed 1 → ~10, speed 3 → ~30, speed 6 → ~52 (cap).
        let raw = (fleet_speed as u32 * 10).min(50) as u8;
        raw.max(1)
    } else {
        // Multi-year travel: fleet arrives near the beginning of its arrival year.
        // Use a small week to reflect early-year arrival.
        // 2-year: base ≈ 4; 3-year+: base = 1.
        if travel_time_years == 2 { 4 } else { 1 }
    }
}

// ---------------------------------------------------------------------------
// Player-visible Stardate formatting
// ---------------------------------------------------------------------------

/// Format a standard fleet/planet/starbase report `Stardate` fragment.
///
/// Classic EC renders: `Stardate: <week>/<year>` (no zero-padding on week).
pub fn format_stardate(week: u8, year: u16) -> String {
    format!("Stardate: {week}/{year}")
}

/// Format the first-line header for a player report entry.
///
/// Classic EC places the `Stardate` right-justified on the same line as the
/// source clause, with intervening spaces to fill to the classic results-text
/// payload width (72
/// characters).
///
/// ```text
///  -> From your 1st Fleet, located in System(13,15)          Stardate: 32/3001
/// ```
///
/// `source_clause` should be the full "From …" phrase without a trailing
/// newline.  The function pads with spaces so the combined length reaches 72
/// characters (or inserts a single space if the clause is already too long).
pub fn format_report_first_line(source_clause: &str, week: u8, year: u16) -> String {
    const LINE_WIDTH: usize = 72;
    let stardate = format_stardate(week, year);
    let combined_min = source_clause.len() + 1 + stardate.len(); // at least one space
    if combined_min >= LINE_WIDTH {
        format!("{source_clause} {stardate}")
    } else {
        let padding = LINE_WIDTH - source_clause.len() - stardate.len();
        format!("{source_clause}{}{stardate}", " ".repeat(padding))
    }
}

/// Format the year-only rankings banner `Stardate`.
///
/// Classic EC uses `Stardate: YYYY A.D.` for the rankings header rather than
/// the `week/year` form used in per-report entries.
pub fn format_rankings_stardate(year: u16) -> String {
    format!("Stardate: {year} A.D.")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_code_offsets_match_recovered_table() {
        assert_eq!(timing_code_offset(TimingCode::FleetCombat), 2);
        assert_eq!(timing_code_offset(TimingCode::Reserved2), 7);
        assert_eq!(timing_code_offset(TimingCode::ScoutView), 21);
        assert_eq!(timing_code_offset(TimingCode::Standing4), 0);
        assert_eq!(timing_code_offset(TimingCode::Standing5), 0);
        assert_eq!(timing_code_offset(TimingCode::Standing6), 0);
        assert_eq!(timing_code_offset(TimingCode::IpbmSummary7), 0);
        assert_eq!(timing_code_offset(TimingCode::HighOffset8), 30);
    }

    #[test]
    fn apply_timing_offset_clamps_to_valid_range() {
        assert_eq!(apply_timing_offset(1, TimingCode::Standing4), 1);
        assert_eq!(apply_timing_offset(1, TimingCode::FleetCombat), 3);
        assert_eq!(apply_timing_offset(1, TimingCode::ScoutView), 22);
        assert_eq!(apply_timing_offset(40, TimingCode::ScoutView), 52); // 40+21=61 → clamped to 52
        assert_eq!(apply_timing_offset(52, TimingCode::FleetCombat), 52); // 52+2=54 → clamped
    }

    #[test]
    fn base_week_standing_mission_is_1() {
        assert_eq!(event_base_week(Mission::PatrolSector, 1, 3), 1);
        assert_eq!(event_base_week(Mission::GuardStarbase, 1, 3), 1);
        assert_eq!(event_base_week(Mission::GuardBlockadeWorld, 1, 3), 1);
        assert_eq!(event_base_week(Mission::RendezvousSector, 1, 3), 1);
    }

    #[test]
    fn base_week_no_travel_is_1() {
        assert_eq!(event_base_week(Mission::ColonizeWorld, 0, 3), 1);
        assert_eq!(event_base_week(Mission::MoveOnly, 0, 0), 1);
    }

    #[test]
    fn base_week_one_year_speed3_is_around_30() {
        let w = event_base_week(Mission::ColonizeWorld, 1, 3);
        assert!(w >= 25 && w <= 35, "expected ~30, got {w}");
    }

    #[test]
    fn format_stardate_no_padding() {
        assert_eq!(format_stardate(32, 3001), "Stardate: 32/3001");
        assert_eq!(format_stardate(1, 3010), "Stardate: 1/3010");
    }

    #[test]
    fn format_rankings_stardate_uses_ad_suffix() {
        assert_eq!(format_rankings_stardate(3001), "Stardate: 3001 A.D.");
    }

    #[test]
    fn format_report_first_line_right_justifies_stardate() {
        let source = "From your 1st Fleet, located in System(13,15)";
        let line = format_report_first_line(source, 32, 3001);
        assert!(line.ends_with("Stardate: 32/3001"));
        assert!(
            line.len() == 72 || line.contains("  "),
            "should pad to 72 chars"
        );
    }
}
