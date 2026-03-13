use std::fs;
use std::path::Path;
use std::process::Command;

use ec_data::{run_maintenance_turn, CoreGameData};

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

    println!("Rust maintenance complete.");
    Ok(())
}

/// Run original ECMAINT oracle on a directory
pub fn run_original_ecmaint(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running original ECMAINT oracle on: {}", dir.display());

    // Use the existing oracle harness via Python script
    let output = Command::new("python3")
        .arg("tools/ecmaint_oracle.py")
        .arg("run")
        .arg(dir)
        .current_dir("/home/mag/dev/esterian_conquest")
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
