use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError, ReportBlockRow};

impl CampaignStore {
    pub fn load_snapshot_report_block_rows(
        &self,
        snapshot_id: i64,
        include_deleted: bool,
    ) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
        let mut conn = self.connection()?;
        load_report_block_rows_filtered(&mut conn, snapshot_id, include_deleted)
    }

    /// Soft-delete a single report block by index.
    pub fn mark_report_block_deleted(
        &self,
        snapshot_id: i64,
        block_index: usize,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE report_blocks SET recipient_deleted = 1
             WHERE snapshot_id = ?1 AND block_index = ?2",
            params![snapshot_id, block_index as i64],
        )?;
        Ok(())
    }

    /// Soft-delete all report blocks for a snapshot.
    pub fn mark_all_report_blocks_deleted(
        &self,
        snapshot_id: i64,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE report_blocks SET recipient_deleted = 1 WHERE snapshot_id = ?1",
            params![snapshot_id],
        )?;
        Ok(())
    }

    /// Check whether any active (non-deleted) report blocks exist.
    pub fn has_active_report_blocks(&self, snapshot_id: i64) -> Result<bool, CampaignStoreError> {
        let conn = self.connection()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM report_blocks
                 WHERE snapshot_id = ?1 AND recipient_deleted = 0
             )",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Load the active (non-deleted) report block rows for the latest snapshot.
    pub fn load_latest_report_block_rows(&self) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(Vec::new());
        };
        load_report_block_rows(&mut conn, snapshot_id)
    }
}

pub(super) fn write_report_block_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    rows: &[ReportBlockRow],
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO report_blocks(snapshot_id, block_index, decoded_text, raw_hex, recipient_deleted)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for row in rows {
        stmt.execute(params![
            snapshot_id,
            row.block_index as i64,
            &row.decoded_text,
            row.raw_bytes.as_ref().map(|bytes| encode_hex(bytes)),
            i64::from(u8::from(row.recipient_deleted)),
        ])?;
    }
    Ok(())
}

pub(super) fn load_report_block_rows(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    load_report_block_rows_filtered(conn, snapshot_id, false)
}

pub(super) fn load_report_block_rows_filtered(
    conn: &mut Connection,
    snapshot_id: i64,
    include_deleted: bool,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    let sql = if include_deleted {
        "SELECT block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1
         ORDER BY block_index"
    } else {
        "SELECT block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1 AND recipient_deleted = 0
         ORDER BY block_index"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(ReportBlockRow {
            block_index: row.get::<_, i64>(0)? as usize,
            decoded_text: row.get(1)?,
            raw_bytes: row
                .get::<_, Option<String>>(2)?
                .map(|hex| decode_hex(&hex))
                .transpose()
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                    )
                })?,
            recipient_deleted: row.get::<_, i64>(3)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}

fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if text.len() % 2 != 0 {
        return Err("hex text length must be even".to_string());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for idx in (0..bytes.len()).step_by(2) {
        let hi = decode_hex_nibble(bytes[idx])
            .ok_or_else(|| format!("invalid hex character at offset {idx}"))?;
        let lo = decode_hex_nibble(bytes[idx + 1])
            .ok_or_else(|| format!("invalid hex character at offset {}", idx + 1))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
