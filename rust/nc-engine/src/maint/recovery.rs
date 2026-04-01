//! Crash recovery helpers for the maintenance engine.
//!
//! Classic ECMAINT creates `Move.Tok` before the movement phase and removes it
//! after a successful run.  If a previous run was interrupted, `Move.Tok`
//! exists at startup and the engine restores `.DAT` files from their `.SAV`
//! backups before proceeding.
//!
//! This module provides the Rust equivalents.

use std::path::Path;

/// Return `true` if a `Move.Tok` crash marker exists in `dir`, indicating that
/// a previous maintenance run was interrupted mid-movement.
pub fn check_crash_marker(dir: &Path) -> bool {
    dir.join("Move.Tok").exists()
}

/// Restore `.DAT` files from their `.SAV` backups when a crash is detected.
///
/// Copies `FLEETS.SAV` → `FLEETS.DAT`, `PLANETS.SAV` → `PLANETS.DAT`, etc.
/// Missing `.SAV` files are silently skipped.
pub fn restore_from_sav(dir: &Path) -> std::io::Result<()> {
    const FILES: &[&str] = &["FLEETS", "PLANETS", "PLAYER", "BASES", "IPBM"];
    for name in FILES {
        let sav = dir.join(format!("{name}.SAV"));
        let dat = dir.join(format!("{name}.DAT"));
        if sav.exists() {
            std::fs::copy(&sav, &dat)?;
        }
    }
    Ok(())
}

/// Create `.SAV` backups of the movement-phase `.DAT` files before movement
/// begins.  These are the files that movement modifies and that must be
/// restorable on crash.
pub fn create_movement_backups(dir: &Path) -> std::io::Result<()> {
    const FILES: &[&str] = &["FLEETS", "PLANETS", "PLAYER", "BASES", "IPBM"];
    for name in FILES {
        let dat = dir.join(format!("{name}.DAT"));
        let sav = dir.join(format!("{name}.SAV"));
        if dat.exists() {
            std::fs::copy(&dat, &sav)?;
        }
    }
    // Create the crash marker after backups are in place.
    std::fs::write(dir.join("Move.Tok"), b"")
}

/// Remove the `Move.Tok` crash marker after a successful run.
pub fn remove_crash_marker(dir: &Path) {
    let _ = std::fs::remove_file(dir.join("Move.Tok"));
}
