use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{ConquestDat, FleetDat, PlanetDat, PlayerDat, SetupDat};

const INIT_FILES: &[&str] = &[
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

const ORIGINAL_FILES: &[&str] = &[
    "BASES.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "FLEETS.DAT",
    "PLANETS.DAT",
    "PLAYER.DAT",
    "SETUP.DAT",
];

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
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            inspect_dir(&dir)?;
        }
        "headers" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            dump_headers(&dir)?;
        }
        "match" => {
            let dir = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            match_fixture_set(&dir)?;
        }
        "compare" => {
            let Some(left) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            let Some(right) = args.next().map(|arg| resolve_repo_path(&arg)) else {
                print_usage();
                return Ok(());
            };
            compare_dirs(&left, &right)?;
        }
        "init" => {
            let source = args
                .next()
                .map(|arg| resolve_repo_path(&arg))
                .unwrap_or_else(default_fixture_dir);
            let Some(target) = args.next().map(PathBuf::from) else {
                print_usage();
                return Ok(());
            };
            initialize_dir(&source, &target)?;
        }
        _ => print_usage(),
    }

    Ok(())
}

fn default_fixture_dir() -> PathBuf {
    repo_root().join("original/v1.5")
}

fn init_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecutil-init/v1.5")
}

fn post_maint_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecmaint-post/v1.5")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        repo_root().join(path)
    }
}

fn inspect_dir(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let player = PlayerDat::parse(&fs::read(dir.join("PLAYER.DAT"))?)?;
    let planets = PlanetDat::parse(&fs::read(dir.join("PLANETS.DAT"))?)?;
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    print_header_summary(&setup, &conquest);
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

fn dump_headers(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let setup = SetupDat::parse(&fs::read(dir.join("SETUP.DAT"))?)?;
    let conquest = ConquestDat::parse(&fs::read(dir.join("CONQUEST.DAT"))?)?;

    println!("Directory: {}", dir.display());
    println!("SETUP.version={}", String::from_utf8_lossy(setup.version_tag()));
    println!("SETUP.option_prefix={:02x?}", setup.option_prefix());
    println!("CONQUEST.game_year={}", conquest.game_year());
    println!("CONQUEST.player_count={}", conquest.player_count());
    println!("CONQUEST.player_config_word={:04x}", conquest.player_config_word());
    println!(
        "CONQUEST.maintenance_schedule={:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST.header_len={}", conquest.control_header().len());
    println!("CONQUEST.header_words={:04x?}", conquest.header_words());

    Ok(())
}

fn print_header_summary(setup: &SetupDat, conquest: &ConquestDat) {
    println!("SETUP version: {}", String::from_utf8_lossy(setup.version_tag()));
    println!("SETUP option prefix: {:02x?}", setup.option_prefix());
    println!("CONQUEST game year: {}", conquest.game_year());
    println!("CONQUEST player count: {}", conquest.player_count());
    println!(
        "CONQUEST maintenance schedule: {:02x?}",
        conquest.maintenance_schedule_bytes()
    );
    println!("CONQUEST header bytes: {}", conquest.control_header().len());
    println!(
        "CONQUEST first header words: {:04x?}",
        &conquest.header_words()[..8]
    );
}

fn initialize_dir(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;

    let init_dir = init_fixture_dir();
    for name in INIT_FILES {
        fs::copy(init_dir.join(name), target.join(name))?;
    }

    println!("Initialized game directory: {}", target.display());
    println!("  source snapshot: {}", source.display());
    println!("  init fixture set: {}", init_dir.display());
    println!("  overlaid files:");
    for name in INIT_FILES {
        println!("    {name}");
    }

    Ok(())
}

fn compare_dirs(left: &Path, right: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Left:  {}", left.display());
    println!("Right: {}", right.display());
    println!();

    compare_raw_file(left, right, "SETUP.DAT")?;
    compare_raw_file(left, right, "CONQUEST.DAT")?;
    compare_raw_file(left, right, "DATABASE.DAT")?;
    compare_player(left, right)?;
    compare_planets(left, right)?;
    compare_fleets(left, right)?;

    Ok(())
}

fn match_fixture_set(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let candidates = [
        ("original/v1.5", default_fixture_dir(), ORIGINAL_FILES),
        ("fixtures/ecutil-init/v1.5", init_fixture_dir(), INIT_FILES),
        ("fixtures/ecmaint-post/v1.5", post_maint_fixture_dir(), INIT_FILES),
    ];

    println!("Directory: {}", dir.display());
    let mut matched_any = false;
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

fn compare_raw_file(
    left_dir: &Path,
    right_dir: &Path,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let left = fs::read(left_dir.join(name))?;
    let right = fs::read(right_dir.join(name))?;
    println!(
        "{name}: size {} vs {}, differing bytes {}",
        left.len(),
        right.len(),
        diff_count(&left, &right)
    );
    Ok(())
}

fn compare_player(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlayerDat::parse(&fs::read(left_dir.join("PLAYER.DAT"))?)?;
    let right = PlayerDat::parse(&fs::read(right_dir.join("PLAYER.DAT"))?)?;
    println!("PLAYER.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.raw, &b.raw);
        if count == 0 {
            continue;
        }
        println!(
            "  record {}: {} differing bytes, tax {} -> {}",
            idx + 1,
            count,
            a.tax_rate(),
            b.tax_rate()
        );
    }
    Ok(())
}

fn compare_planets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left = PlanetDat::parse(&fs::read(left_dir.join("PLANETS.DAT"))?)?;
    let right = PlanetDat::parse(&fs::read(right_dir.join("PLANETS.DAT"))?)?;
    println!("PLANETS.DAT:");
    for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
        let count = diff_count(&a.raw, &b.raw);
        if count == 0 {
            continue;
        }
        println!(
            "  record {:02}: {} differing bytes, text '{}' -> '{}'",
            idx + 1,
            count,
            ascii_trim(a.status_or_name_bytes()),
            ascii_trim(b.status_or_name_bytes())
        );
    }
    Ok(())
}

fn compare_fleets(left_dir: &Path, right_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let left_bytes = fs::read(left_dir.join("FLEETS.DAT"))?;
    let right_bytes = fs::read(right_dir.join("FLEETS.DAT"))?;
    println!(
        "FLEETS.DAT: size {} vs {}, differing bytes {}",
        left_bytes.len(),
        right_bytes.len(),
        diff_count(&left_bytes, &right_bytes)
    );

    let left = FleetDat::parse(&left_bytes);
    let right = FleetDat::parse(&right_bytes);
    if let (Ok(left), Ok(right)) = (left, right) {
        for (idx, (a, b)) in left.records.iter().zip(right.records.iter()).enumerate() {
            let count = diff_count(&a.raw, &b.raw);
            if count == 0 {
                continue;
            }
            println!(
                "  record {:02}: {} differing bytes, mission {} -> {}, params {:02x?} -> {:02x?}",
                idx + 1,
                count,
                a.mission_code(),
                b.mission_code(),
                a.mission_param_bytes(),
                b.mission_param_bytes()
            );
        }
    }

    Ok(())
}

fn copy_top_level_files(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn ascii_trim(bytes: &[u8]) -> String {
    let text = bytes
        .iter()
        .map(|b| if (32..127).contains(b) { *b as char } else { ' ' })
        .collect::<String>();
    text.trim().to_string()
}

fn diff_count(left: &[u8], right: &[u8]) -> usize {
    let shared = left.iter().zip(right.iter()).filter(|(a, b)| a != b).count();
    shared + left.len().abs_diff(right.len())
}

fn print_usage() {
    println!("Usage:");
    println!("  ec-cli inspect [dir]");
    println!("  ec-cli headers [dir]");
    println!("  ec-cli match [dir]");
    println!("  ec-cli compare <left_dir> <right_dir>");
    println!("  ec-cli init [source_dir] <target_dir>");
}
