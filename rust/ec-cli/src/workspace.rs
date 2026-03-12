use std::fs;
use std::path::Path;

use crate::support::paths::{init_fixture_dir, post_maint_fixture_dir};

pub(crate) const INIT_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "IPBM.DAT",
    "MESSAGES.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "RESULTS.DAT",
    "SETUP.DAT",
];

const CURRENT_KNOWN_CORE_FILES: &[&str] = &[
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
];

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

    println!("Initialized game directory: {}", target.display());
    println!("  source snapshot: {}", source.display());
    println!("  init fixture set: {}", init_dir.display());
    println!("  overlaid files:");
    for name in INIT_FILES {
        println!("    {name}");
    }

    Ok(())
}

pub fn copy_init_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_named_files(source, target, INIT_FILES)
}

pub fn match_fixture_set(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let candidates = [
        ("original/v1.5", crate::support::paths::default_fixture_dir(), ORIGINAL_FILES),
        ("fixtures/ecutil-init/v1.5", init_fixture_dir(), INIT_FILES),
        ("fixtures/ecmaint-post/v1.5", post_maint_fixture_dir(), INIT_FILES),
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

pub fn copy_top_level_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
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
