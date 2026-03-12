use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::fleet_order::{apply_fleet_order_scenario, validate_fleet_order_scenario};
use crate::commands::guard_starbase::{apply_guard_starbase_scenario, validate_guard_starbase_scenario};
use crate::commands::planet_build::{apply_planet_build_scenario, validate_planet_build_scenario};
use crate::support::paths::repo_root;
use crate::INIT_FILES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KnownScenario {
    FleetOrder,
    PlanetBuild,
    GuardStarbase,
}

impl KnownScenario {
    pub(crate) fn all() -> [Self; 3] {
        [Self::FleetOrder, Self::PlanetBuild, Self::GuardStarbase]
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::FleetOrder => "fleet-order",
            Self::PlanetBuild => "planet-build",
            Self::GuardStarbase => "guard-starbase",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "fleet-order" => Some(Self::FleetOrder),
            "planet-build" => Some(Self::PlanetBuild),
            "guard-starbase" => Some(Self::GuardStarbase),
            _ => None,
        }
    }

    pub(crate) fn description(self) -> &'static str {
        match self {
            Self::FleetOrder => "accepted fleet movement/order fixture rooted in FLEETS.DAT",
            Self::PlanetBuild => "accepted planet build-queue fixture rooted in PLANETS.DAT",
            Self::GuardStarbase => {
                "accepted one-base guard-starbase fixture spanning PLAYER/FLEETS/BASES"
            }
        }
    }

    pub(crate) fn preserved_fixture_dir(self) -> PathBuf {
        let root = repo_root().join("fixtures");
        match self {
            Self::FleetOrder => root.join("ecmaint-fleet-pre/v1.5"),
            Self::PlanetBuild => root.join("ecmaint-build-pre/v1.5"),
            Self::GuardStarbase => root.join("ecmaint-starbase-pre/v1.5"),
        }
    }

    pub(crate) fn exact_match_files(self) -> &'static [&'static str] {
        match self {
            Self::FleetOrder => &["FLEETS.DAT"],
            Self::PlanetBuild => &["PLANETS.DAT"],
            Self::GuardStarbase => &["PLAYER.DAT", "FLEETS.DAT", "BASES.DAT"],
        }
    }
}

pub(crate) fn apply_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        KnownScenario::FleetOrder => apply_fleet_order_scenario(dir),
        KnownScenario::PlanetBuild => apply_planet_build_scenario(dir),
        KnownScenario::GuardStarbase => apply_guard_starbase_scenario(dir),
    }
}

pub(crate) fn validate_known_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        KnownScenario::FleetOrder => {
            validate_fleet_order_scenario(dir, 1, 0x03, 0x0C, 0x0F, 0x0D, None, None)
        }
        KnownScenario::PlanetBuild => validate_planet_build_scenario(dir, 15, 0x03, 0x01),
        KnownScenario::GuardStarbase => validate_guard_starbase_scenario(dir),
    }
}

pub(crate) fn validate_all_known_scenarios(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        match validate_known_scenario(dir, scenario) {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => println!("FAIL {name}: {err}"),
        }
    }

    if matched == 0 {
        Err("directory does not match any known accepted scenario".into())
    } else {
        Ok(())
    }
}

pub(crate) fn validate_preserved_scenario(
    dir: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_dir = scenario.preserved_fixture_dir();
    let mut errors = Vec::new();

    for name in scenario.exact_match_files() {
        let actual = fs::read(dir.join(name))?;
        let expected = fs::read(fixture_dir.join(name))?;
        if actual != expected {
            errors.push(format!("{name} differs from preserved fixture"));
        }
    }

    if errors.is_empty() {
        println!("Exact preserved match: {}", scenario.name());
        println!("  fixture: {}", fixture_dir.display());
        for name in scenario.exact_match_files() {
            println!("  {name}");
        }
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn validate_all_preserved_scenarios(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut matched = 0usize;
    for scenario in KnownScenario::all() {
        let name = scenario.name();
        match validate_preserved_scenario(dir, scenario) {
            Ok(()) => {
                println!("OK   {name}");
                matched += 1;
            }
            Err(err) => println!("FAIL {name}: {err}"),
        }
    }

    if matched == 0 {
        Err("directory does not exactly match any preserved accepted scenario".into())
    } else {
        Ok(())
    }
}

pub(crate) fn init_known_scenario(
    source: &Path,
    target: &Path,
    scenario: KnownScenario,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target)?;
    for name in INIT_FILES {
        fs::copy(source.join(name), target.join(name))?;
    }
    apply_known_scenario(target, scenario)?;
    println!("Scenario directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_all_known_scenarios(
    source: &Path,
    target_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Known scenarios\n");
    manifest.push_str(&format!("source={}\n", source.display()));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');
    for scenario in KnownScenario::all() {
        let scenario_dir = target_root.join(scenario.name());
        init_known_scenario(source, &scenario_dir, scenario)?;
        manifest.push_str(&format!("{}\n", scenario.name()));
        manifest.push_str(&format!("  description={}\n", scenario.description()));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=ec-cli validate {} {}\n\n",
            scenario_dir.display(),
            scenario.name()
        ));
    }
    fs::write(target_root.join("SCENARIOS.txt"), manifest)?;
    println!("Initialized all known scenarios under {}", target_root.display());
    Ok(())
}

pub(crate) fn print_known_scenarios() {
    println!("Known scenarios:");
    for scenario in KnownScenario::all() {
        println!("  {}: {}", scenario.name(), scenario.description());
    }
}

pub(crate) fn print_known_scenario_details(scenario: KnownScenario) {
    println!("Scenario: {}", scenario.name());
    println!("Description: {}", scenario.description());
    println!(
        "Preserved fixture: {}",
        scenario.preserved_fixture_dir().display()
    );
    println!("Exact-match files:");
    for name in scenario.exact_match_files() {
        println!("  {name}");
    }
}
