use rusqlite::params;

use super::{
    CampaignStoreError, Connection, BASE_RECORD_FIELDS_TABLE, CONQUEST_RECORD_FIELDS_TABLE,
    FLEET_RECORD_FIELDS_TABLE, IPBM_RECORD_FIELDS_TABLE, PLANET_RECORD_FIELDS_TABLE,
    PLAYER_RECORD_FIELDS_TABLE, SETUP_RECORD_FIELDS_TABLE,
};
use crate::{
    BaseDat, CoreGameData, FleetDat, IpbmDat, PlanetDat, PlayerDat, SetupDat, BASE_RECORD_SIZE,
    FLEET_RECORD_SIZE, IPBM_RECORD_SIZE, PLANET_RECORD_SIZE, PLAYER_RECORD_SIZE,
};

pub(super) fn write_typed_record_rows(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    snapshot_id: i64,
    bytes: &[u8],
    record_size: usize,
) -> Result<(), CampaignStoreError> {
    let sql = format!(
        "INSERT INTO {table}(snapshot_id, record_index, byte_offset, byte_value)
         VALUES (?1, ?2, ?3, ?4)"
    );
    let mut stmt = tx.prepare(&sql)?;
    for (idx, chunk) in bytes.chunks_exact(record_size).enumerate() {
        let record_index = (idx + 1) as i64;
        for (byte_offset, byte_value) in chunk.iter().copied().enumerate() {
            stmt.execute(params![
                snapshot_id,
                record_index,
                byte_offset as i64,
                i64::from(byte_value),
            ])?;
        }
    }
    Ok(())
}

fn read_typed_record_rows(
    conn: &mut Connection,
    table: &str,
    snapshot_id: i64,
    expected_size: usize,
) -> Result<Vec<u8>, CampaignStoreError> {
    let sql = format!(
        "SELECT record_index, byte_offset, byte_value
         FROM {table}
         WHERE snapshot_id = ?1
         ORDER BY record_index, byte_offset"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut bytes = Vec::new();
    let mut current_record_index: Option<i64> = None;
    let mut expected_offset = 0usize;
    for row in rows {
        let (record_index, byte_offset, byte_value) = row?;
        let byte_offset = byte_offset as usize;
        if current_record_index != Some(record_index) {
            if current_record_index.is_some() && expected_offset != expected_size {
                return Err(CampaignStoreError::Parse(
                    crate::ParseError::WrongRecordMultiple {
                        file_type: "sqlite-record",
                        record_size: expected_size,
                        actual: expected_offset,
                    },
                ));
            }
            current_record_index = Some(record_index);
            expected_offset = 0;
        }
        if byte_offset != expected_offset {
            return Err(CampaignStoreError::Parse(
                crate::ParseError::WrongRecordMultiple {
                    file_type: "sqlite-record",
                    record_size: expected_size,
                    actual: byte_offset,
                },
            ));
        }
        bytes.push(byte_value as u8);
        expected_offset += 1;
    }
    if current_record_index.is_some() && expected_offset != expected_size {
        return Err(CampaignStoreError::Parse(
            crate::ParseError::WrongRecordMultiple {
                file_type: "sqlite-record",
                record_size: expected_size,
                actual: expected_offset,
            },
        ));
    }
    Ok(bytes)
}

pub(super) fn load_snapshot_game_data(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<CoreGameData, CampaignStoreError> {
    Ok(CoreGameData {
        player: PlayerDat::parse(&read_typed_record_rows(
            conn,
            PLAYER_RECORD_FIELDS_TABLE,
            snapshot_id,
            PLAYER_RECORD_SIZE,
        )?)?,
        planets: PlanetDat::parse(&read_typed_record_rows(
            conn,
            PLANET_RECORD_FIELDS_TABLE,
            snapshot_id,
            PLANET_RECORD_SIZE,
        )?)?,
        fleets: FleetDat::parse(&read_typed_record_rows(
            conn,
            FLEET_RECORD_FIELDS_TABLE,
            snapshot_id,
            FLEET_RECORD_SIZE,
        )?)?,
        bases: BaseDat::parse(&read_typed_record_rows(
            conn,
            BASE_RECORD_FIELDS_TABLE,
            snapshot_id,
            BASE_RECORD_SIZE,
        )?)?,
        ipbm: IpbmDat::parse(&read_typed_record_rows(
            conn,
            IPBM_RECORD_FIELDS_TABLE,
            snapshot_id,
            IPBM_RECORD_SIZE,
        )?)?,
        setup: SetupDat::parse(&read_typed_record_rows(
            conn,
            SETUP_RECORD_FIELDS_TABLE,
            snapshot_id,
            crate::SETUP_DAT_SIZE,
        )?)?,
        conquest: crate::ConquestDat::parse(&read_typed_record_rows(
            conn,
            CONQUEST_RECORD_FIELDS_TABLE,
            snapshot_id,
            crate::CONQUEST_DAT_SIZE,
        )?)?,
    })
}
