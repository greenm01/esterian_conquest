use std::fs;
use std::path::Path;
use std::process::Command;

use ec_data::{run_maintenance_turn, CoreGameData, DatabaseDat, PlanetDat};

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

    // Save a snapshot of the pre-maint planets so we can inspect build queues later.
    // DATABASE.DAT regeneration needs to know which planets had active builds
    // in order to clear the 0x1e field in the corresponding orbit records.
    let pre_maint_planets = game_data.planets.clone();

    // Run maintenance logic for specified turns
    for turn in 1..=turns {
        run_maintenance_turn(&mut game_data)?;
        println!("  Turn {}: year {}", turn, game_data.conquest.game_year());
    }

    println!(
        "  Year advanced: {} -> {}",
        start_year,
        game_data.conquest.game_year()
    );

    // Save the modified state
    game_data.save(dir)?;

    // Regenerate DATABASE.DAT from PLANETS.DAT
    regenerate_database_dat(dir, &game_data, &pre_maint_planets)?;

    println!("Rust maintenance complete.");
    Ok(())
}

/// Regenerate DATABASE.DAT from current PLANETS.DAT and CONQUEST.DAT year.
///
/// `pre_maint_planets` is the planet state before maintenance ran, used to detect
/// which planets had active build queues (which affects certain DATABASE fields).
fn regenerate_database_dat(
    dir: &Path,
    game_data: &CoreGameData,
    pre_maint_planets: &PlanetDat,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load existing DATABASE.DAT as template
    let template_path = dir.join("DATABASE.DAT");
    let template = if template_path.exists() {
        let bytes = fs::read(&template_path)?;
        DatabaseDat::parse(&bytes).ok()
    } else {
        None
    };

    // Extract planet names from PLANETS.DAT and normalize them
    // ECMAINT normalizes certain names to "UNKNOWN"
    let planet_names: Vec<String> = game_data
        .planets
        .records
        .iter()
        .map(|p| {
            let name = p.planet_name();
            // Normalize names like ECMAINT does
            if name.eq_ignore_ascii_case("unowned") || name.eq_ignore_ascii_case("not named yet") {
                "UNKNOWN".to_string()
            } else {
                name
            }
        })
        .collect();

    // Get current game year
    let year = game_data.conquest.game_year();
    // Discovery uses the previous year (pre-maintenance state)
    let discovery_year = year - 1;

    // Generate new DATABASE.DAT
    let mut new_database =
        DatabaseDat::generate_from_planets_and_year(&planet_names, year, template.as_ref());

    // Handle planet discovery for newly visible planets.
    //
    // DATABASE layout: record = player * 20 + planet (player is 0-indexed)
    //
    // Two classes of records get updated each turn:
    //
    // 1. Homeworld orbit records: template has raw[0x15]=0x01..0x04 (empire scan index)
    //    and name_len=0.  ECMAINT stamps "Not Named Yet" + year.
    //    These correspond to each empire's orbital scan of homeworld planets.
    //
    // 2. Owned-planet UNKNOWN records: player owns the planet but their DATABASE record
    //    has scan_marker=0xff (UNKNOWN). ECMAINT populates it with the real planet name,
    //    year, and full planet intel (pot_prod, armies, batteries).
    //    This covers both:
    //      a. Newly colonized planets (name="Not Named Yet") — raw[0x1e]=0x00, raw[0x15]=0x01
    //      b. Pre-existing owned planets whose record was never populated — raw[0x1e]=0x40+owner_slot
    //    The distinction is whether the planet name is "Not Named Yet" or a real name.
    //    Confirmed from econ scenario:
    //      - Record 33 (player 1 "foo", planet 13 "TargetPrime"): pre UNKNOWN, oracle populated.
    //      - raw[0x15]=0x02 (owner_slot), raw[0x1e]=0x42 (0x40+2), name="TargetPrime".
    //      - raw[0x1c]=pot_prod, raw[0x1d]=pot_prod, raw[0x23]=armies, raw[0x25]=batteries.
    if let Some(ref template_db) = template {
        let year_bytes = discovery_year.to_le_bytes();

        for player in 0..4usize {
            for planet in 0..20usize {
                let record_idx = player * 20 + planet;
                let template_record = &template_db.records[record_idx];
                let scan_marker = template_record.raw[0x15];

                // Case 1: orbit record — scan marker is 0x01..0x04 and no name yet
                let is_orbit_record =
                    scan_marker >= 0x01 && scan_marker <= 0x04 && template_record.raw[0x00] == 0;

                // Case 2: owned-planet UNKNOWN record — player owns this planet but
                // their DATABASE record is UNKNOWN (scan_marker=0xff).
                let planet_owner = if planet < game_data.planets.records.len() {
                    game_data.planets.records[planet].owner_empire_slot_raw() as usize
                } else {
                    0
                };
                let is_owned_unknown = scan_marker == 0xff && planet_owner == player + 1;

                if is_orbit_record {
                    // Homeworld orbit record: stamp name + year, preserve scan_marker
                    new_database.records[record_idx].set_planet_name("Not Named Yet");
                    // Keep original scan_marker (0x01..0x04) — do NOT overwrite with 0x01
                    new_database.records[record_idx].raw[0x16] = year_bytes[0];
                    new_database.records[record_idx].raw[0x17] = year_bytes[1];
                    new_database.records[record_idx].raw[0x18] = year_bytes[0];
                    new_database.records[record_idx].raw[0x19] = year_bytes[1];
                    new_database.records[record_idx].raw[0x27] = year_bytes[0];
                    new_database.records[record_idx].raw[0x28] = year_bytes[1];

                    // If the planet had an active build queue in pre-maint state,
                    // clear raw[0x1e] in the database record.
                    // Confirmed from build-scenario fixture: planet 14 had build=3,
                    // DATABASE record 14 0x1e=0x23 → 0x00 after maintenance.
                    // In fleet scenario (no build queue), 0x1e is preserved.
                    if planet < pre_maint_planets.records.len() {
                        let had_build_queue = (0..10).any(|slot| {
                            pre_maint_planets.records[planet].build_count_raw(slot) > 0
                        });
                        if had_build_queue {
                            new_database.records[record_idx].raw[0x1e] = 0x00;
                        }
                    }
                } else if is_owned_unknown {
                    // Owned-planet UNKNOWN record: populate with real planet name + intel.
                    let owner_slot = planet_owner as u8; // = player + 1
                    let planet_name = if planet < game_data.planets.records.len() {
                        game_data.planets.records[planet].planet_name()
                    } else {
                        String::new()
                    };
                    let is_new_colony = planet_name.eq_ignore_ascii_case("not named yet");

                    new_database.records[record_idx].set_planet_name(&planet_name);
                    // raw[0x15]: owner_slot for pre-existing planets; 0x01 for new colonies
                    new_database.records[record_idx].raw[0x15] =
                        if is_new_colony { 0x01 } else { owner_slot };
                    new_database.records[record_idx].raw[0x16] = year_bytes[0];
                    new_database.records[record_idx].raw[0x17] = year_bytes[1];
                    new_database.records[record_idx].raw[0x18] = year_bytes[0];
                    new_database.records[record_idx].raw[0x19] = year_bytes[1];
                    new_database.records[record_idx].raw[0x27] = year_bytes[0];
                    new_database.records[record_idx].raw[0x28] = year_bytes[1];

                    if planet < game_data.planets.records.len() {
                        let p = &game_data.planets.records[planet];
                        let pot_prod_lo = p.raw[0x02];
                        let armies = p.army_count_raw();
                        let batteries = p.ground_batteries_raw();

                        new_database.records[record_idx].raw[0x1c] = pot_prod_lo;
                        // raw[0x1d]: owner_slot for new colonies; pot_prod_lo for pre-existing
                        // Confirmed: fleet new colony (slot=1) -> 0x01; econ TargetPrime -> 0x64
                        new_database.records[record_idx].raw[0x1d] = if is_new_colony {
                            owner_slot
                        } else {
                            pot_prod_lo
                        };
                        // raw[0x1e]: 0x00 for new colonies; 0x40+owner_slot for pre-existing
                        // Confirmed: econ record 33 (TargetPrime, owner_slot=2) → 0x42=0x40+2
                        new_database.records[record_idx].raw[0x1e] = if is_new_colony {
                            0x00
                        } else {
                            0x40 + owner_slot
                        };
                        new_database.records[record_idx].raw[0x1f] = 0x00;
                        // 0x23: army count, 0x24: 0x00, 0x25: battery count, 0x26: 0x00
                        new_database.records[record_idx].raw[0x23] = armies;
                        new_database.records[record_idx].raw[0x24] = 0x00;
                        new_database.records[record_idx].raw[0x25] = batteries;
                        new_database.records[record_idx].raw[0x26] = 0x00;
                    }
                }
            }
        }
    }

    // Save the regenerated DATABASE.DAT
    fs::write(template_path, new_database.to_bytes())?;

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

    // Auto-detect turns from year difference if not specified
    let turns_to_run = match turns {
        Some(t) => t,
        None => {
            // Try to detect from fixture directory name or default to 1
            let dir_str = dir.to_string_lossy();
            if dir_str.contains("move") {
                // Move scenario: year 3000 -> 3003 = 3 turns
                println!("Auto-detected: Move scenario requires 3 turns");
                3
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
    compare_dat_files(&rust_dir, &oracle_dir)?;

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

/// Compare .DAT files between two directories
fn compare_dat_files(rust_dir: &Path, oracle_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

    let mut total_files = 0;
    let mut matching_files = 0;

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

            println!(
                "  ✗ {}: DIFFER ({} differing bytes out of {})",
                file,
                max_diffs,
                rust_data.len().max(oracle_data.len())
            );

            // Show first few diffs for FLEETS.DAT
            if *file == "FLEETS.DAT" && diff_count > 0 {
                println!("    First 5 differences:");
                let mut shown = 0;
                for (i, (a, b)) in rust_data.iter().zip(oracle_data.iter()).enumerate() {
                    if a != b && shown < 5 {
                        let fleet_idx = if i >= 2 { (i - 2) / 54 } else { 0 };
                        let field_off = if i >= 2 { (i - 2) % 54 } else { i };
                        println!("      Offset 0x{:04x} (fleet {}, field 0x{:02x}): 0x{:02x} -> 0x{:02x}",
                            i, fleet_idx, field_off, a, b);
                        shown += 1;
                    }
                }
            }
        }
    }

    println!();
    let percentage = if total_files > 0 {
        (matching_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "Parity: {}/{} files match ({:.1}%)",
        matching_files, total_files, percentage
    );

    Ok(())
}
