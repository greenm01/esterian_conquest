use std::fs;
use std::path::Path;

use crate::support::paths::{
    init_fixture_dir, post_maint_fixture_dir, pre_maint_replay_context_fixture_dir,
};
use ec_data::{ConquestDat, DatabaseDat, PlanetDat};

pub(crate) const INIT_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "FLEETS.DAT",
    "IPBM.DAT",
    "MESSAGES.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "RESULTS.DAT",
    "SETUP.DAT",
    // Note: DATABASE.DAT is now generated, not copied
];

pub(crate) const CURRENT_KNOWN_CORE_FILES: &[&str] = &[
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
];

pub(crate) const PRE_MAINT_REPLAY_CONTEXT_FILES: &[&str] = &["CONQUEST.DAT", "DATABASE.DAT"];

const ORIGINAL_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "SETUP.DAT",
];

pub fn initialize_dir(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;

    let init_dir = init_fixture_dir();
    copy_named_files(&init_dir, target, INIT_FILES)?;

    // Generate DATABASE.DAT from PLANETS.DAT + CONQUEST.DAT
    generate_database_dat(target)?;

    // Ensure auxiliary files exist
    ensure_auxiliary_files(target)?;

    println!("Initialized game directory: {}", target.display());
    println!("  source snapshot: {}", source.display());
    println!("  init fixture set: {}", init_dir.display());
    println!("  overlaid files:");
    for name in INIT_FILES {
        println!("    {name}");
    }
    println!("  generated files:");
    println!("    DATABASE.DAT");

    Ok(())
}

pub fn copy_init_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(source, target, INIT_FILES)
}

pub fn copy_current_known_core_files(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(source, target, CURRENT_KNOWN_CORE_FILES)
}

pub fn copy_pre_maint_replay_context_files(
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(
        &pre_maint_replay_context_fixture_dir(),
        target,
        PRE_MAINT_REPLAY_CONTEXT_FILES,
    )
}

/// Ensure auxiliary files (MESSAGES.DAT, RESULTS.DAT) exist.
/// These files are not critical for gameplay but are expected by some tools.
pub fn ensure_auxiliary_files(target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for name in ["MESSAGES.DAT", "RESULTS.DAT"] {
        let path = target.join(name);
        if !path.exists() {
            fs::write(path, [])?;
        }
    }
    Ok(())
}

pub fn match_fixture_set(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let candidates = [
        (
            "original/v1.5",
            crate::support::paths::default_fixture_dir(),
            ORIGINAL_FILES,
        ),
        ("fixtures/ecutil-init/v1.5", init_fixture_dir(), INIT_FILES),
        (
            "fixtures/ecmaint-post/v1.5",
            post_maint_fixture_dir(),
            INIT_FILES,
        ),
    ];

    println!("Directory: {}", dir.display());
    let mut matched_any = false;
    let current_known_core = post_maint_fixture_dir();
    if dir_matches(dir, &current_known_core, CURRENT_KNOWN_CORE_FILES)? {
        println!("MATCH current-known-post-maint-baseline-core");
        matched_any = true;
    }

    for (label, candidate, files) in candidates {
        if dir_matches(dir, &candidate, files)? {
            println!("MATCH {label}");
            matched_any = true;
        }
    }
    if !matched_any {
        println!("MATCH none");
    }

    Ok(())
}

fn dir_matches(
    dir: &Path,
    candidate: &Path,
    files: &[&str],
) -> Result<bool, Box<dyn std::error::Error>> {
    for name in files {
        if fs::read(dir.join(name))? != fs::read(candidate.join(name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn copy_top_level_files(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        fs::copy(&path, target.join(file_name))?;
    }

    Ok(())
}

fn copy_named_files(
    source: &Path,
    target: &Path,
    names: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;

    for name in names {
        fs::copy(source.join(name), target.join(name))?;
    }

    Ok(())
}

/// Generate DATABASE.DAT from PLANETS.DAT and CONQUEST.DAT.
///
/// Reads the template from the init fixture, copies planet names from PLANETS.DAT,
/// and embeds the CONQUEST.DAT year in homeworld records.
pub fn generate_database_dat(target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Read the template DATABASE.DAT from init fixture
    let init_dir = init_fixture_dir();
    let template_bytes = fs::read(init_dir.join("DATABASE.DAT"))?;
    let template = DatabaseDat::parse(&template_bytes)?;

    // Read PLANETS.DAT and CONQUEST.DAT from target
    let planets_bytes = fs::read(target.join("PLANETS.DAT"))?;
    let planets = PlanetDat::parse(&planets_bytes)?;

    let conquest_bytes = fs::read(target.join("CONQUEST.DAT"))?;
    let conquest = ConquestDat::parse(&conquest_bytes)?;

    // Extract planet names
    let planet_names: Vec<String> = planets.records.iter().map(|p| p.planet_name()).collect();

    // Extract year from CONQUEST.DAT
    let game_year = conquest.game_year();

    // Generate DATABASE.DAT
    let database =
        DatabaseDat::generate_from_planets_and_year(
            &planet_names,
            game_year,
            conquest.player_count() as usize,
            Some(&template),
        );

    // Write to target
    fs::write(target.join("DATABASE.DAT"), database.to_bytes())?;

    Ok(())
}
