use std::fs;
use std::path::Path;
use std::process::Command;

use ec_data::{
    CoreGameData, DatabaseDat, DiplomacyConfig, DiplomacyOverride, MaintenanceEvents,
    VisibleHazardIntel, DiplomaticRelation, run_maintenance_turn, run_maintenance_turn_with_context,
    run_maintenance_turn_with_visible_hazards, visible_hazard_intel_from_database,
};

use crate::commands::reports::{
    regenerate_database_dat, regenerate_messages_dat, regenerate_results_dat,
};

/// Run Rust maintenance on a game directory for specified number of turns
pub fn run_rust_maintenance(dir: &Path, turns: u16) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Running Rust maintenance on: {} ({} turn{})",
        dir.display(),
        turns,
        if turns == 1 { "" } else { "s" }
    );

    // Load the game state
    let mut game_data = CoreGameData::load(dir)?;
    let start_year = game_data.conquest.game_year();
    let mut database = load_database_dat_if_present(dir)?;
    let mut diplomacy_overrides = load_diplomacy_overrides_if_present(dir, &game_data)?;
    absorb_persistable_diplomacy_overrides(&mut game_data, &mut diplomacy_overrides)?;

    // Save a snapshot of the pre-maint planets so we can inspect build queues later.
    // DATABASE.DAT regeneration needs to know which planets had active builds
    // in order to clear the 0x1e field in the corresponding orbit records.
    let pre_maint_planets = game_data.planets.clone();

    // Run maintenance logic for specified turns, accumulating events across all turns.
    let mut all_events = MaintenanceEvents::default();
    for turn in 1..=turns {
        let visible_hazards = database
            .as_ref()
            .map(|db| visible_hazards_from_database(&game_data, db))
            .unwrap_or_default();
        let events = if visible_hazards.is_empty() && diplomacy_overrides.is_empty() {
            run_maintenance_turn(&mut game_data)?
        } else if diplomacy_overrides.is_empty() {
            run_maintenance_turn_with_visible_hazards(&mut game_data, &visible_hazards)?
        } else {
            run_maintenance_turn_with_context(
                &mut game_data,
                &visible_hazards,
                &diplomacy_overrides,
            )?
        };
        all_events.bombard_events.extend(events.bombard_events);
        all_events
            .planet_intel_events
            .extend(events.planet_intel_events);
        all_events
            .ownership_change_events
            .extend(events.ownership_change_events);
        all_events
            .fleet_battle_events
            .extend(events.fleet_battle_events);
        all_events
            .fleet_destroyed_events
            .extend(events.fleet_destroyed_events);
        all_events
            .starbase_destroyed_events
            .extend(events.starbase_destroyed_events);
        all_events
            .assault_report_events
            .extend(events.assault_report_events);
        all_events
            .scout_contact_events
            .extend(events.scout_contact_events);
        all_events
            .fleet_merge_events
            .extend(events.fleet_merge_events);
        all_events.join_host_events.extend(events.join_host_events);
        all_events
            .colonization_events
            .extend(events.colonization_events);
        all_events.mission_events.extend(events.mission_events);
        all_events
            .diplomatic_escalation_events
            .extend(events.diplomatic_escalation_events);
        all_events
            .civil_disorder_events
            .extend(events.civil_disorder_events);
        all_events
            .campaign_outlook_events
            .extend(events.campaign_outlook_events);
        all_events
            .campaign_outcome_events
            .extend(events.campaign_outcome_events);
        all_events
            .fleet_defection_events
            .extend(events.fleet_defection_events);

        apply_diplomatic_escalations(&mut game_data, &mut diplomacy_overrides, &all_events)?;

        if database.is_some() {
            regenerate_database_dat(dir, &game_data, &pre_maint_planets, &all_events)?;
            database = load_database_dat_if_present(dir)?;
        }

        println!("  Turn {}: year {}", turn, game_data.conquest.game_year());
    }

    println!(
        "  Year advanced: {} -> {}",
        start_year,
        game_data.conquest.game_year()
    );

    // Save the modified state
    game_data.save(dir)?;
    save_diplomacy_overrides_if_needed(dir, game_data.conquest.player_count(), &diplomacy_overrides)?;

    // Regenerate DATABASE.DAT from PLANETS.DAT
    regenerate_database_dat(dir, &game_data, &pre_maint_planets, &all_events)?;
    regenerate_results_dat(dir, &game_data, &all_events)?;
    regenerate_messages_dat(dir, &game_data, &all_events)?;

    println!("Rust maintenance complete.");
    Ok(())
}

/// Run original ECMAINT oracle on a directory
pub fn run_original_ecmaint(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running original ECMAINT oracle on: {}", dir.display());

    // Ensure ECMAINT.EXE is present — without it DOSBox opens an interactive window.
    let engine_path = dir.join("ECMAINT.EXE");
    if !engine_path.exists() {
        let source_engine = std::path::Path::new("/home/mag/dev/esterian_conquest")
            .join("original/v1.5/ECMAINT.EXE");
        if source_engine.exists() {
            fs::copy(&source_engine, &engine_path)?;
        } else {
            return Err("ECMAINT.EXE not found in dir or original/v1.5/".into());
        }
    }

    // Use the existing oracle harness via Python script.
    // Pass SDL_VIDEODRIVER=dummy and SDL_AUDIODRIVER=dummy explicitly so DOSBox
    // runs headless even when called from a GUI environment.
    let output = Command::new("python3")
        .arg("tools/ecmaint_oracle.py")
        .arg("run")
        .arg(dir)
        .current_dir("/home/mag/dev/esterian_conquest")
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ECMAINT oracle failed: {}", stderr).into());
    }

    println!("Original ECMAINT complete.");
    Ok(())
}

/// Compare Rust maintenance output vs original ECMAINT output
///
/// # Arguments
/// * `dir` - The game directory to compare
/// * `turns` - Number of turns to run (default 1, or auto-detect from year difference)
pub fn compare_maintenance(
    dir: &Path,
    turns: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Comparing maintenance implementations for: {}",
        dir.display()
    );
    println!();

    // Auto-detect turns from directory name if not specified.
    // Known tick counts per scenario family:
    //   move: 3 ticks   (3000 → 3003)
    //   bombard: 2 ticks (3000 → 3002)
    //   invade: 2 ticks  (3010 → 3012)
    //   econ: 2 ticks    (3010 → 3012)
    //   all others: 1 tick
    let turns_to_run = match turns {
        Some(t) => t,
        None => {
            let dir_str = dir.to_string_lossy();
            if dir_str.contains("move") {
                println!("Auto-detected: move scenario — 3 turns");
                3
            } else if dir_str.contains("bombard")
                || dir_str.contains("invade")
                || dir_str.contains("econ")
            {
                println!("Auto-detected: 2-tick scenario — 2 turns");
                2
            } else {
                1
            }
        }
    };

    // Create temporary directories for comparison
    let temp_dir = std::env::temp_dir().join("ecmaint-compare");
    let rust_dir = temp_dir.join("rust");
    let oracle_dir = temp_dir.join("oracle");

    // Clean up and recreate
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&rust_dir)?;
    fs::create_dir_all(&oracle_dir)?;

    // Copy original to both working directories
    copy_directory(dir, &rust_dir)?;
    copy_directory(dir, &oracle_dir)?;

    // Run Rust maintenance for N turns
    println!("=== Running Rust maintenance ({} turns) ===", turns_to_run);
    run_rust_maintenance(&rust_dir, turns_to_run)?;
    println!();

    // Run original ECMAINT N times
    println!("=== Running original ECMAINT ({} turns) ===", turns_to_run);
    for turn in 1..=turns_to_run {
        println!("  Turn {}...", turn);
        run_original_ecmaint(&oracle_dir)?;
    }
    println!();

    // Compare outputs
    println!("=== Comparison Results ===");
    compare_dat_files(dir, &rust_dir, &oracle_dir)?;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Copy directory contents recursively
fn copy_directory(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default();

        // Skip .oracle directory and .SAV files
        let name_str = file_name.to_string_lossy();
        if name_str.starts_with('.') || name_str.ends_with(".SAV") {
            continue;
        }

        let dest_path = dst.join(file_name);

        if path.is_file() {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

fn load_database_dat_if_present(
    dir: &Path,
) -> Result<Option<DatabaseDat>, Box<dyn std::error::Error>> {
    let path = dir.join("DATABASE.DAT");
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    Ok(Some(DatabaseDat::parse(&bytes)?))
}

fn load_diplomacy_overrides_if_present(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<Vec<DiplomacyOverride>, Box<dyn std::error::Error>> {
    let path = dir.join("diplomacy.kdl");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let config = DiplomacyConfig::load_kdl(&path)?
        .validate_for_player_count(game_data.player.records.len() as u8)?;
    Ok(config
        .directives
        .into_iter()
        .map(|directive| DiplomacyOverride {
            from_empire_raw: directive.from_empire_raw,
            to_empire_raw: directive.to_empire_raw,
            relation: directive.relation,
        })
        .collect())
}

fn apply_diplomatic_escalations(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pairs = Vec::new();

    for event in &events.fleet_battle_events {
        for &enemy_empire_raw in &event.enemy_empires_raw {
            pairs.push((event.reporting_empire_raw, enemy_empire_raw));
        }
    }

    for event in &events.bombard_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for event in &events.assault_report_events {
        if event.defender_empire_raw != 0 {
            pairs.push((event.attacker_empire_raw, event.defender_empire_raw));
        }
    }

    for (left, right) in pairs {
        escalate_pair(game_data, diplomacy_overrides, left, right)?;
    }

    Ok(())
}

fn escalate_pair(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    left: u8,
    right: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if left == 0 || right == 0 || left == right {
        return Ok(());
    }
    apply_one_way_escalation(game_data, diplomacy_overrides, left, right)?;
    apply_one_way_escalation(game_data, diplomacy_overrides, right, left)?;
    Ok(())
}

fn apply_one_way_escalation(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    from: u8,
    to: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(
        game_data.stored_diplomatic_relation(from, to),
        Some(DiplomaticRelation::Enemy)
    ) {
        return Ok(());
    }

    if game_data.set_stored_diplomatic_relation(from, to, DiplomaticRelation::Enemy)? {
        return Ok(());
    }

    if let Some(existing) = diplomacy_overrides
        .iter_mut()
        .find(|directive| directive.from_empire_raw == from && directive.to_empire_raw == to)
    {
        existing.relation = DiplomaticRelation::Enemy;
        return Ok(());
    }

    diplomacy_overrides.push(DiplomacyOverride {
        from_empire_raw: from,
        to_empire_raw: to,
        relation: DiplomaticRelation::Enemy,
    });
    Ok(())
}

fn save_diplomacy_overrides_if_needed(
    dir: &Path,
    player_count: u8,
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("diplomacy.kdl");
    let directives = diplomacy_overrides
        .iter()
        .copied()
        .map(|directive| ec_data::DiplomacyDirective {
            from_empire_raw: directive.from_empire_raw,
            to_empire_raw: directive.to_empire_raw,
            relation: directive.relation,
        })
        .collect::<Vec<_>>();

    if directives.is_empty() {
        if path.exists() {
            fs::write(path, [])?;
        }
        return Ok(());
    }

    let config = DiplomacyConfig { directives }.validate_for_player_count(player_count)?;
    fs::write(path, config.to_kdl_string())?;
    Ok(())
}

fn absorb_persistable_diplomacy_overrides(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut remaining = Vec::with_capacity(diplomacy_overrides.len());
    for directive in diplomacy_overrides.drain(..) {
        if game_data.set_stored_diplomatic_relation(
            directive.from_empire_raw,
            directive.to_empire_raw,
            directive.relation,
        )? {
            continue;
        }
        remaining.push(directive);
    }
    *diplomacy_overrides = remaining;
    Ok(())
}

fn visible_hazards_from_database(
    game_data: &CoreGameData,
    database: &DatabaseDat,
) -> Vec<VisibleHazardIntel> {
    (1..=game_data.conquest.player_count())
        .map(|empire_raw| visible_hazard_intel_from_database(game_data, database, empire_raw))
        .collect()
}

/// Compare .DAT files between two directories
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComparePolicy {
    Strict,
    CanonicalCombat,
}

fn compare_policy_for_dir(dir: &Path) -> ComparePolicy {
    let dir_str = dir.to_string_lossy();
    if dir_str.contains("fleet-battle")
        || dir_str.contains("bombard")
        || dir_str.contains("invade")
        || dir_str.contains("econ")
    {
        ComparePolicy::CanonicalCombat
    } else {
        ComparePolicy::Strict
    }
}

fn is_structurally_accepted_diff(policy: ComparePolicy, file: &str) -> bool {
    match policy {
        ComparePolicy::Strict => false,
        ComparePolicy::CanonicalCombat => matches!(
            file,
            "FLEETS.DAT" | "PLANETS.DAT" | "DATABASE.DAT" | "MESSAGES.DAT" | "RESULTS.DAT"
        ),
    }
}

fn compare_dat_files(
    source_dir: &Path,
    rust_dir: &Path,
    oracle_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let dat_files = [
        "CONQUEST.DAT",
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "RESULTS.DAT",
        "DATABASE.DAT",
        "SETUP.DAT",
    ];

    let policy = compare_policy_for_dir(source_dir);
    let mut total_files = 0;
    let mut matching_files = 0;
    let mut accepted_files = 0;

    for file in &dat_files {
        let rust_path = rust_dir.join(file);
        let oracle_path = oracle_dir.join(file);

        if !rust_path.exists() || !oracle_path.exists() {
            continue;
        }

        total_files += 1;
        let rust_data = fs::read(&rust_path)?;
        let oracle_data = fs::read(&oracle_path)?;

        if rust_data == oracle_data {
            matching_files += 1;
            accepted_files += 1;
            println!("  ✓ {}: MATCH ({} bytes)", file, rust_data.len());
        } else {
            // Calculate diff stats
            let diff_count = rust_data
                .iter()
                .zip(oracle_data.iter())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .count();
            let max_diffs = rust_data.len().max(oracle_data.len())
                - rust_data.len().min(oracle_data.len())
                + diff_count;

            if is_structurally_accepted_diff(policy, file) {
                accepted_files += 1;
                println!(
                    "  ~ {}: STRUCTURAL DIFF ACCEPTED ({} differing bytes out of {})",
                    file,
                    max_diffs,
                    rust_data.len().max(oracle_data.len())
                );
            } else {
                println!(
                    "  ✗ {}: DIFFER ({} differing bytes out of {})",
                    file,
                    max_diffs,
                    rust_data.len().max(oracle_data.len())
                );
            }

            // Show first few diffs for FLEETS.DAT
            if *file == "FLEETS.DAT" && diff_count > 0 {
                println!("    First 5 differences:");
                let mut shown = 0;
                for (i, (a, b)) in rust_data.iter().zip(oracle_data.iter()).enumerate() {
                    if a != b && shown < 5 {
                        let fleet_idx = if i >= 2 { (i - 2) / 54 } else { 0 };
                        let field_off = if i >= 2 { (i - 2) % 54 } else { i };
                        println!(
                            "      Offset 0x{:04x} (fleet {}, field 0x{:02x}): 0x{:02x} -> 0x{:02x}",
                            i, fleet_idx, field_off, a, b
                        );
                        shown += 1;
                    }
                }
            }
        }
    }

    println!();
    let strict_percentage = if total_files > 0 {
        (matching_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "Strict parity: {}/{} files match ({:.1}%)",
        matching_files, total_files, strict_percentage
    );
    let accepted_percentage = if total_files > 0 {
        (accepted_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    match policy {
        ComparePolicy::Strict => {
            println!(
                "Acceptance: strict byte-exact comparison ({}/{} files accepted, {:.1}%)",
                accepted_files, total_files, accepted_percentage
            );
        }
        ComparePolicy::CanonicalCombat => {
            println!(
                "Acceptance: canonical combat structural comparison ({}/{} files accepted, {:.1}%)",
                accepted_files, total_files, accepted_percentage
            );
            println!(
                "  Structural diffs are accepted only in combat-driven files: FLEETS.DAT, PLANETS.DAT, DATABASE.DAT, MESSAGES.DAT, RESULTS.DAT"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ComparePolicy, compare_policy_for_dir, is_structurally_accepted_diff};

    #[test]
    fn compare_policy_uses_canonical_combat_for_combat_heavy_scenarios() {
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-fleet-battle-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-bombard-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-invade-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-econ-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
    }

    #[test]
    fn compare_policy_keeps_deterministic_scenarios_strict() {
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-move-pre/v1.5")),
            ComparePolicy::Strict
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-build-pre/v1.5")),
            ComparePolicy::Strict
        );
    }

    #[test]
    fn canonical_combat_acceptance_is_limited_to_combat_driven_files() {
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "FLEETS.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "PLANETS.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "DATABASE.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "MESSAGES.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "RESULTS.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "PLAYER.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "CONQUEST.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::Strict,
            "FLEETS.DAT"
        ));
    }
}
