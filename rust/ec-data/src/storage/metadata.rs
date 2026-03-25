use rusqlite::{OptionalExtension, params};

use super::{CampaignStoreError, Connection};

pub(super) fn latest_snapshot_id_and_year(
    conn: &mut Connection,
) -> Result<Option<(i64, u16)>, CampaignStoreError> {
    conn.query_row(
        "SELECT id, game_year FROM snapshots ORDER BY game_year DESC LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as u16)),
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_campaign_seed(conn: &mut Connection) -> Result<Option<u64>, CampaignStoreError> {
    conn.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<Option<u64>, CampaignStoreError> {
    tx.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn persist_campaign_seed(
    conn: &mut Connection,
    seed: u64,
) -> Result<(), CampaignStoreError> {
    conn.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}

pub(super) fn persist_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
    seed: u64,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}
