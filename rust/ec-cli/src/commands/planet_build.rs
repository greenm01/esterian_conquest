use std::fs;
use std::path::Path;

use ec_data::CoreGameData;

use crate::workspace::copy_init_files;

pub(crate) fn set_planet_build(
    dir: &Path,
    record_index_1_based: usize,
    slot_raw: u8,
    kind_raw: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = CoreGameData::load(dir)?;
    data.set_planet_build(record_index_1_based, slot_raw, kind_raw)
        .map_err(|err| err.to_string())?;
    data.save(dir)?;

    println!(
        "Planet record {} updated: build_slot={:#04x} build_kind={:#04x}",
        record_index_1_based, slot_raw, kind_raw
    );
    Ok(())
}

pub(crate) fn apply_planet_build_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_planet_build(dir, 15, 0x03, 0x01)?;
    println!("Applied scenario: planet-build");
    Ok(())
}

pub(crate) fn print_planet_build_report(
    dir: &Path,
    record_index_1_based: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let record = data
        .planets
        .records
        .get(record_index_1_based - 1)
        .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
    println!("Planet Build Report");
    println!("  dir={}", dir.display());
    println!("  record={}", record_index_1_based);
    println!("  build_slot={:#04x}", record.build_count_raw(0));
    println!("  build_kind={:#04x}", record.build_kind_raw(0));
    println!("  stardock_count={:#04x}", record.stardock_count_raw(0));
    println!("  stardock_kind={:#04x}", record.stardock_kind_raw(0));
    println!("  owner={:#04x}", record.owner_empire_slot_raw());
    println!("  coords={:?}", record.coords_raw());
    Ok(())
}

pub(crate) fn init_planet_build_scenario(
    source: &Path,
    target: &Path,
    record_index_1_based: usize,
    slot_raw: u8,
    kind_raw: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_planet_build(target, record_index_1_based, slot_raw, kind_raw)?;
    println!("Planet-build directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_planet_build_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(usize, u8, u8)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Planet-build batch\n");
    manifest.push_str(&format!("source={}\n", source.display()));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for (record_index, slot_raw, kind_raw) in specs {
        let name = format!("p{:02}-s{:02x}-k{:02x}", record_index, slot_raw, kind_raw);
        let scenario_dir = target_root.join(&name);
        init_planet_build_scenario(source, &scenario_dir, *record_index, *slot_raw, *kind_raw)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!(
            "  spec={}:{:#04x}:{:#04x}\n",
            record_index, slot_raw, kind_raw
        ));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=ec-cli validate {} planet-build\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("PLANET_BUILDS.txt"), manifest)?;
    println!(
        "Initialized {} planet-build directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}
