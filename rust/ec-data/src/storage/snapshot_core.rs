use rusqlite::{Connection, params};

use super::hex::{decode_hex, encode_hex};
use super::{
    BASE_SNAPSHOTS_TABLE, CONQUEST_SNAPSHOTS_TABLE, CampaignStoreError, FLEET_SNAPSHOTS_TABLE,
    IPBM_SNAPSHOTS_TABLE, PLANET_SNAPSHOTS_TABLE, PLAYER_SNAPSHOTS_TABLE, SETUP_SNAPSHOTS_TABLE,
};
use crate::{
    BaseDat, ConquestDat, CoreGameData, FleetDat, IpbmDat, PlanetDat, PlayerDat, SetupDat,
};

pub(super) fn write_snapshot_core_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
) -> Result<(), CampaignStoreError> {
    write_player_rows(tx, snapshot_id, &game_data.player)?;
    write_planet_rows(tx, snapshot_id, &game_data.planets)?;
    write_fleet_rows(tx, snapshot_id, &game_data.fleets)?;
    write_base_rows(tx, snapshot_id, &game_data.bases)?;
    write_ipbm_rows(tx, snapshot_id, &game_data.ipbm)?;
    write_setup_row(tx, snapshot_id, &game_data.setup)?;
    write_conquest_row(tx, snapshot_id, &game_data.conquest)?;
    Ok(())
}

pub(super) fn load_snapshot_game_data(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    Ok(CoreGameData {
        player: PlayerDat::parse(&load_record_bytes(
            conn,
            PLAYER_SNAPSHOTS_TABLE,
            snapshot_id,
        )?)?,
        planets: PlanetDat::parse(&load_record_bytes(
            conn,
            PLANET_SNAPSHOTS_TABLE,
            snapshot_id,
        )?)?,
        fleets: FleetDat::parse(&load_record_bytes(
            conn,
            FLEET_SNAPSHOTS_TABLE,
            snapshot_id,
        )?)?,
        bases: BaseDat::parse(&load_record_bytes(conn, BASE_SNAPSHOTS_TABLE, snapshot_id)?)?,
        ipbm: IpbmDat::parse(&load_record_bytes(conn, IPBM_SNAPSHOTS_TABLE, snapshot_id)?)?,
        setup: SetupDat::parse(&load_singleton_hex(
            conn,
            SETUP_SNAPSHOTS_TABLE,
            snapshot_id,
        )?)?,
        conquest: ConquestDat::parse(&load_singleton_hex(
            conn,
            CONQUEST_SNAPSHOTS_TABLE,
            snapshot_id,
        )?)?,
    })
}

fn write_player_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &PlayerDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_players(
             snapshot_id, record_index, occupied_flag, owner_mode_raw, handle_text,
             empire_name_text, tax_rate, autopilot_flag, fleet_chain_head_raw,
             starbase_count_raw, ipbm_count_raw, homeworld_planet_index_raw,
             last_run_year_raw, classic_message_review_word_raw,
             classic_message_review_carry_word_raw, classic_results_review_word_raw,
             classic_results_review_carry_word_raw, classic_results_chain_flag_raw,
             classic_results_chain_next_free_raw, compat_raw_hex
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.occupied_flag()),
            i64::from(record.owner_mode_raw()),
            record.assigned_player_handle_summary(),
            record.controlled_empire_name_summary(),
            i64::from(record.tax_rate()),
            i64::from(record.autopilot_flag()),
            i64::from(record.fleet_chain_head_raw()),
            i64::from(record.starbase_count_raw()),
            i64::from(record.ipbm_count_raw()),
            i64::from(record.homeworld_planet_index_1_based_raw()),
            i64::from(record.last_run_year_raw()),
            i64::from(record.classic_message_review_word_raw()),
            i64::from(record.classic_message_review_carry_word_raw()),
            i64::from(record.classic_results_review_word_raw()),
            i64::from(record.classic_results_review_carry_word_raw()),
            i64::from(record.classic_results_chain_flag_raw()),
            i64::from(record.classic_results_chain_next_free_raw()),
            encode_hex(&record.raw),
        ])?;
    }
    Ok(())
}

fn write_planet_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &PlanetDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_planets(
             snapshot_id, record_index, coords_x, coords_y, name_text,
             potential_production_points, present_production_points,
             stored_production_points, owner_empire_slot_raw, ownership_status_raw,
             armies_raw, ground_batteries_raw, compat_raw_hex
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [x, y] = record.coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(x),
            i64::from(y),
            record.planet_name(),
            i64::from(record.potential_production_points()),
            record.present_production_points().map(i64::from),
            i64::from(record.stored_production_points()),
            i64::from(record.owner_empire_slot_raw()),
            i64::from(record.ownership_status_raw()),
            i64::from(record.army_count_raw()),
            i64::from(record.ground_batteries_raw()),
            encode_hex(&record.raw),
        ])?;
    }
    Ok(())
}

fn write_fleet_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &FleetDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_fleets(
             snapshot_id, record_index, local_slot_raw, owner_empire_raw, fleet_id_raw,
             current_x, current_y, target_x, target_y, standing_order_code_raw,
             max_speed, current_speed, rules_of_engagement, scout_count,
             battleship_count, cruiser_count, destroyer_count,
             troop_transport_count, army_count, etac_count, compat_raw_hex
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
             ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [current_x, current_y] = record.current_location_coords_raw();
        let [target_x, target_y] = record.standing_order_target_coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.local_slot()),
            i64::from(record.owner_empire_raw()),
            i64::from(record.fleet_id()),
            i64::from(current_x),
            i64::from(current_y),
            i64::from(target_x),
            i64::from(target_y),
            i64::from(record.standing_order_code_raw()),
            i64::from(record.max_speed()),
            i64::from(record.current_speed()),
            i64::from(record.rules_of_engagement()),
            i64::from(record.scout_count()),
            i64::from(record.battleship_count()),
            i64::from(record.cruiser_count()),
            i64::from(record.destroyer_count()),
            i64::from(record.troop_transport_count()),
            i64::from(record.army_count()),
            i64::from(record.etac_count()),
            encode_hex(&record.raw),
        ])?;
    }
    Ok(())
}

fn write_base_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &BaseDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_bases(
             snapshot_id, record_index, local_slot_raw, active_flag_raw, base_id_raw,
             coords_x, coords_y, trailing_x, trailing_y, owner_empire_raw, compat_raw_hex
         ) VALUES (
             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
         )",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        let [x, y] = record.coords_raw();
        let [trailing_x, trailing_y] = record.trailing_coords_raw();
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.local_slot_raw()),
            i64::from(record.active_flag_raw()),
            i64::from(record.base_id_raw()),
            i64::from(x),
            i64::from(y),
            i64::from(trailing_x),
            i64::from(trailing_y),
            i64::from(record.owner_empire_raw()),
            encode_hex(&record.raw),
        ])?;
    }
    Ok(())
}

fn write_ipbm_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &IpbmDat,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO snapshot_ipbms(
             snapshot_id, record_index, owner_empire_raw, tuple_a_tag_raw,
             tuple_b_tag_raw, compat_raw_hex
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (idx, record) in data.records.iter().enumerate() {
        stmt.execute(params![
            snapshot_id,
            (idx + 1) as i64,
            i64::from(record.owner_empire_raw()),
            i64::from(record.tuple_a_tag_raw()),
            i64::from(record.tuple_b_tag_raw()),
            encode_hex(&record.raw),
        ])?;
    }
    Ok(())
}

fn write_setup_row(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &SetupDat,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "INSERT INTO snapshot_setup(
             snapshot_id, version_tag_text, snoop_enabled,
             max_time_between_keys_minutes_raw, remote_timeout_enabled,
             local_timeout_enabled, minimum_time_granted_minutes_raw,
             purge_after_turns_raw, autopilot_inactive_turns_raw, compat_raw_hex
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            snapshot_id,
            String::from_utf8_lossy(data.version_tag()).to_string(),
            i64::from(u8::from(data.snoop_enabled())),
            i64::from(data.max_time_between_keys_minutes_raw()),
            i64::from(u8::from(data.remote_timeout_enabled())),
            i64::from(u8::from(data.local_timeout_enabled())),
            i64::from(data.minimum_time_granted_minutes_raw()),
            i64::from(data.purge_after_turns_raw()),
            i64::from(data.autopilot_inactive_turns_raw()),
            encode_hex(&data.raw),
        ],
    )?;
    Ok(())
}

fn write_conquest_row(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    data: &ConquestDat,
) -> Result<(), CampaignStoreError> {
    let days = data.maintenance_schedule_enabled();
    tx.execute(
        "INSERT INTO snapshot_conquest(
             snapshot_id, game_year, player_count,
             maintenance_day_0_enabled, maintenance_day_1_enabled,
             maintenance_day_2_enabled, maintenance_day_3_enabled,
             maintenance_day_4_enabled, maintenance_day_5_enabled,
             maintenance_day_6_enabled, compat_raw_hex
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            snapshot_id,
            i64::from(data.game_year()),
            i64::from(data.player_count()),
            i64::from(u8::from(days[0])),
            i64::from(u8::from(days[1])),
            i64::from(u8::from(days[2])),
            i64::from(u8::from(days[3])),
            i64::from(u8::from(days[4])),
            i64::from(u8::from(days[5])),
            i64::from(u8::from(days[6])),
            encode_hex(&data.raw),
        ],
    )?;
    Ok(())
}

fn load_record_bytes(
    conn: &mut Connection,
    table: &str,
    snapshot_id: i64,
) -> Result<Vec<u8>, CampaignStoreError> {
    let sql = format!(
        "SELECT compat_raw_hex
         FROM {table}
         WHERE snapshot_id = ?1
         ORDER BY record_index"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| row.get::<_, String>(0))?;
    let mut bytes = Vec::new();
    for row in rows {
        let hex = row?;
        let record = decode_hex(&hex).map_err(invalid_hex_sql_error)?;
        bytes.extend_from_slice(&record);
    }
    Ok(bytes)
}

fn load_singleton_hex(
    conn: &mut Connection,
    table: &str,
    snapshot_id: i64,
) -> Result<Vec<u8>, CampaignStoreError> {
    let sql = format!(
        "SELECT compat_raw_hex
         FROM {table}
         WHERE snapshot_id = ?1"
    );
    let hex = conn.query_row(&sql, params![snapshot_id], |row| row.get::<_, String>(0))?;
    decode_hex(&hex).map_err(invalid_hex_sql_error)
}

fn invalid_hex_sql_error(err: String) -> CampaignStoreError {
    CampaignStoreError::Sql(rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
    ))
}
