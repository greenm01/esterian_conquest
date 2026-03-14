use ec_data::{
    BaseDat, ConquestDat, CoreGameData, EmpireProductionRankingSort, FleetDat, IpbmDat, PlanetDat,
    PlanetRecord, PlayerDat, PlayerRecord, SetupDat, decode_real48,
};

fn zeroed_setup() -> SetupDat {
    SetupDat::parse(&vec![0; ec_data::SETUP_DAT_SIZE]).expect("zeroed setup should parse")
}

fn zeroed_conquest() -> ConquestDat {
    ConquestDat::parse(&vec![0; ec_data::CONQUEST_DAT_SIZE]).expect("zeroed conquest should parse")
}

fn player_with_empire_name(name: &str, tax_rate: u8, stored_points: u16) -> PlayerRecord {
    let mut record = PlayerRecord::new_zeroed();
    record.set_owner_empire_raw(1);
    record.set_tax_rate_raw(tax_rate);
    record.raw[0x4E..0x50].copy_from_slice(&stored_points.to_le_bytes());
    let bytes = name.as_bytes();
    let len = bytes.len().min(19);
    record.raw[27] = len as u8;
    record.raw[28..28 + len].copy_from_slice(&bytes[..len]);
    record
}

fn owned_planet(
    owner_empire_slot: u8,
    potential_production: u8,
    factories_real48: [u8; 6],
    stored_points: u32,
    armies: u8,
    batteries: u8,
) -> PlanetRecord {
    let mut record = PlanetRecord::new_zeroed();
    record.set_owner_empire_slot_raw(owner_empire_slot);
    record.set_potential_production_raw([potential_production, 0]);
    record.set_factories_raw(factories_real48);
    record.set_stored_goods_raw(stored_points);
    record.set_army_count_raw(armies);
    record.set_ground_batteries_raw(batteries);
    record
}

fn owned_homeworld_seed(
    owner_empire_slot: u8,
    potential_production: u8,
    factories_real48: [u8; 6],
    armies: u8,
    batteries: u8,
) -> PlanetRecord {
    let mut record = owned_planet(
        owner_empire_slot,
        potential_production,
        factories_real48,
        0,
        armies,
        batteries,
    );
    record.set_ownership_status_raw(2);
    record
}

#[test]
fn decode_real48_matches_current_known_homeworld_values() {
    let fifty = decode_real48([0x00, 0x00, 0x00, 0x00, 0x48, 0x86]).expect("real should decode");
    let hundred =
        decode_real48([0x00, 0x00, 0x00, 0x00, 0x48, 0x87]).expect("real should decode");

    assert!((fifty - 50.0).abs() < 0.001, "expected 50.0, got {fifty}");
    assert!(
        (hundred - 100.0).abs() < 0.001,
        "expected 100.0, got {hundred}"
    );
}

#[test]
fn current_known_empire_economy_helpers_use_classic_production_terms() {
    let mut player1 = player_with_empire_name("Alpha", 50, 0);
    let mut player2 = player_with_empire_name("Beta", 60, 20);
    player1.set_owner_empire_raw(1);
    player2.set_owner_empire_raw(2);

    let game = CoreGameData {
        player: PlayerDat {
            records: vec![player1, player2],
        },
        planets: PlanetDat {
            records: vec![
                owned_planet(1, 100, [0x00, 0x00, 0x00, 0x00, 0x48, 0x87], 20, 10, 4),
                owned_planet(1, 50, [0x00, 0x00, 0x00, 0x00, 0x48, 0x85], 15, 5, 1),
                owned_planet(2, 200, [0x00, 0x00, 0x00, 0x00, 0x48, 0x88], 25, 8, 2),
            ],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: zeroed_conquest(),
    };

    let economy = game.empire_economy_summary_current_known(1);
    assert_eq!(economy.owned_planets, 2);
    assert_eq!(economy.present_production, 125);
    assert_eq!(economy.potential_production, 150);
    assert_eq!(economy.total_available_points, 62);
    assert!((economy.efficiency_percent - 83.333).abs() < 0.01);
    assert_eq!(economy.rank_by_planets, 1);
    assert_eq!(economy.rank_by_present_production, 2);

    let rankings = game.empire_production_ranking_rows_current_known(
        EmpireProductionRankingSort::Production,
    );
    assert_eq!(rankings[0].empire_name, "Beta");
    assert_eq!(rankings[0].current_production, 200);
    assert_eq!(rankings[1].empire_name, "Alpha");
    assert_eq!(rankings[1].current_production, 125);
}

#[test]
fn current_known_total_available_points_matches_first_turn_tax_budget() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let game = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![owned_homeworld_seed(
                1,
                100,
                [0x00, 0x00, 0x00, 0x00, 0x48, 0x86],
                10,
                4,
            )],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: zeroed_conquest(),
    };

    assert_eq!(game.empire_present_production_current_known(1), 100);
    assert_eq!(game.empire_total_available_points_current_known(1), 50);
}

#[test]
fn current_known_homeworld_present_production_clamps_to_potential() {
    let planet = owned_homeworld_seed(1, 100, [0x00, 0x00, 0x00, 0x00, 0x48, 0x86], 10, 4);
    assert_eq!(planet.present_production_points_current_known(), Some(100));
}
