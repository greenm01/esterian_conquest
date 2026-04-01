use std::collections::BTreeSet;

use rusqlite::{Connection, params};

use super::CampaignStoreError;

pub(super) fn load_planet_scorch_orders(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<BTreeSet<usize>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT planet_record_index
         FROM planet_scorch_orders
         WHERE snapshot_id = ?1
         ORDER BY planet_record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| row.get::<_, i64>(0))?;
    Ok(rows
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|value| value as usize)
        .collect())
}

pub(super) fn write_planet_scorch_orders(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    planet_record_indexes: &BTreeSet<usize>,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO planet_scorch_orders(snapshot_id, planet_record_index)
         VALUES (?1, ?2)",
    )?;
    for record_index in planet_record_indexes {
        stmt.execute(params![snapshot_id, *record_index as i64])?;
    }
    Ok(())
}
