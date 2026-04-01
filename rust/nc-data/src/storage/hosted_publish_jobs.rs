use rusqlite::params;

use super::{CampaignStore, CampaignStoreError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedPublishJobKind {
    MapPackOnFirstClaim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedPublishJobStatus {
    Pending,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedPublishJob {
    pub id: i64,
    pub kind: HostedPublishJobKind,
    pub player_record_index_1_based: usize,
    pub player_npub: String,
    pub status: HostedPublishJobStatus,
    pub created_at_unix_seconds: u64,
    pub published_at_unix_seconds: Option<u64>,
    pub last_error: Option<String>,
}

impl HostedPublishJobKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MapPackOnFirstClaim => "map_pack_on_first_claim",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "map_pack_on_first_claim" => Some(Self::MapPackOnFirstClaim),
            _ => None,
        }
    }
}

impl HostedPublishJobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Published => "published",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "published" => Some(Self::Published),
            _ => None,
        }
    }
}

impl CampaignStore {
    pub fn hosted_publish_jobs(&self) -> Result<Vec<HostedPublishJob>, CampaignStoreError> {
        let conn = self.connection()?;
        load_hosted_publish_jobs_conn(&conn, false)
    }

    pub fn pending_hosted_publish_jobs(&self) -> Result<Vec<HostedPublishJob>, CampaignStoreError> {
        let conn = self.connection()?;
        load_hosted_publish_jobs_conn(&conn, true)
    }

    pub fn mark_hosted_publish_job_published(
        &self,
        job_id: i64,
        published_at_unix_seconds: u64,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE hosted_publish_jobs
             SET status = 'published',
                 published_at = ?2,
                 last_error = NULL
             WHERE id = ?1",
            params![job_id, published_at_unix_seconds as i64],
        )?;
        Ok(())
    }

    pub fn record_hosted_publish_job_error(
        &self,
        job_id: i64,
        message: &str,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "UPDATE hosted_publish_jobs
             SET last_error = ?2
             WHERE id = ?1",
            params![job_id, message],
        )?;
        Ok(())
    }
}

pub(super) fn enqueue_hosted_publish_job_tx(
    tx: &rusqlite::Transaction<'_>,
    kind: HostedPublishJobKind,
    player_record_index_1_based: usize,
    player_npub: &str,
    created_at_unix_seconds: u64,
) -> Result<i64, CampaignStoreError> {
    tx.execute(
        "INSERT INTO hosted_publish_jobs (
             job_kind,
             player_record_index,
             player_npub,
             status,
             created_at,
             published_at,
             last_error
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
        params![
            kind.as_str(),
            player_record_index_1_based as i64,
            player_npub,
            HostedPublishJobStatus::Pending.as_str(),
            created_at_unix_seconds as i64,
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

fn load_hosted_publish_jobs_conn(
    conn: &rusqlite::Connection,
    pending_only: bool,
) -> Result<Vec<HostedPublishJob>, CampaignStoreError> {
    let sql = if pending_only {
        "SELECT id,
                job_kind,
                player_record_index,
                player_npub,
                status,
                created_at,
                published_at,
                last_error
         FROM hosted_publish_jobs
         WHERE status = 'pending'
         ORDER BY id ASC"
    } else {
        "SELECT id,
                job_kind,
                player_record_index,
                player_npub,
                status,
                created_at,
                published_at,
                last_error
         FROM hosted_publish_jobs
         ORDER BY id ASC"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        let kind_raw: String = row.get(1)?;
        let kind = HostedPublishJobKind::parse(&kind_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown hosted publish job kind: {kind_raw}"),
                )),
            )
        })?;
        let status_raw: String = row.get(4)?;
        let status = HostedPublishJobStatus::parse(&status_raw).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unknown hosted publish job status: {status_raw}"),
                )),
            )
        })?;
        Ok(HostedPublishJob {
            id: row.get(0)?,
            kind,
            player_record_index_1_based: row.get::<_, i64>(2)? as usize,
            player_npub: row.get(3)?,
            status,
            created_at_unix_seconds: row.get::<_, i64>(5)? as u64,
            published_at_unix_seconds: row.get::<_, Option<i64>>(6)?.map(|value| value as u64),
            last_error: row.get(7)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CampaignStoreError::Sql)
}
