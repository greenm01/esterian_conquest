use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{ConquestDat, FleetDat, PlanetDat, PlayerDat, SetupDat};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "inspect" => {
            let dir = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(default_fixture_dir);
            inspect_dir(&dir)?;
        }
        _ => print_usage(),
    }

    Ok(())
}

fn default_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../original/v1.5")
}

fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let planets = PlanetDat::parse(&fs::read(dir.join("PLANETS.DAT"))?)?;
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    println!("SETUP version: {}", String::from_utf8_lossy(setup.version_tag()));
    println!("CONQUEST header bytes: {}", conquest.control_header().len());
    println!();

    println!("Players:");
    for (idx, record) in player.records.iter().enumerate() {
        println!(
            "  slot {}: occupied={} tax={} handle='{}' empire='{}'",
            idx + 1,
            record.occupied_flag(),
            record.tax_rate(),
            ascii_trim(record.handle_bytes()),
            ascii_trim(record.empire_name_bytes())
        );
    }
    println!();

    println!("Planets:");
    for (idx, record) in planets.records.iter().enumerate().take(5) {
        println!(
            "  planet {:02}: hdr={:02x?} len={} text='{}'",
            idx + 1,
            record.header_bytes(),
            record.string_len(),
            ascii_trim(record.status_or_name_bytes())
        );
    }
    println!("  ... {} total planet records", planets.records.len());

    match fs::read(dir.join("FLEETS.DAT")) {
        Ok(bytes) => match FleetDat::parse(&bytes) {
            Ok(fleets) => {
                println!();
                println!("Fleets:");
                for (idx, record) in fleets.records.iter().enumerate().take(4) {
                    println!(
                        "  fleet {:02}: mission={} params={:02x?}",
                        idx + 1,
                        record.mission_code(),
                        record.mission_param_bytes()
                    );
                }
                println!("  ... {} total fleet records", fleets.records.len());
            }
            Err(err) => {
                println!();
                println!("Fleets:");
                println!("  FLEETS.DAT does not match initialized 16x54 layout: {err}");
            }
        },
        Err(_) => {}
    }

    Ok(())
}

fn ascii_trim(bytes: &[u8]) -> String {
    let text = bytes
        .iter()
        .map(|b| if (32..127).contains(b) { *b as char } else { ' ' })
        .collect::<String>();
    text.trim().to_string()
}

fn print_usage() {
    println!("Usage: ec-cli inspect [dir]");
}
