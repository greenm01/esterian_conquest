//! Dash-local inbox projection and summary helpers.

use nc_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

use crate::app::state::InboxFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashInboxItemType {
    Report,
    Message,
}

impl DashInboxItemType {
    pub const fn code(self) -> char {
        match self {
            Self::Report => 'R',
            Self::Message => 'M',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashInboxItemSource {
    ReportBlock(usize),
    QueuedMail(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportSummaryBucket {
    Combat,
    Intel,
    Ops,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashInboxItem {
    pub source: DashInboxItemSource,
    pub item_type: DashInboxItemType,
    pub year: u16,
    pub week: Option<u8>,
    pub subject: String,
    pub body_lines: Vec<String>,
    pub report_bucket: Option<ReportSummaryBucket>,
}

impl DashInboxItem {
    pub fn matches_filter(
        &self,
        filter: InboxFilter,
        current_year_only: bool,
        current_year: u16,
    ) -> bool {
        if current_year_only && self.year != current_year {
            return false;
        }
        match filter {
            InboxFilter::All => true,
            InboxFilter::Reports => matches!(self.item_type, DashInboxItemType::Report),
            InboxFilter::Messages => matches!(self.item_type, DashInboxItemType::Message),
        }
    }
}

pub fn project_inbox_items(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    report_blocks: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> Vec<DashInboxItem> {
    let current_year = game_data.conquest.game_year();
    let mut items = report_blocks
        .iter()
        .enumerate()
        .filter(|(_, row)| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|(idx, row)| report_item(idx, row, current_year))
        .collect::<Vec<_>>();

    items.extend(
        queued_mail
            .iter()
            .enumerate()
            .filter(|(_, mail)| mail.is_visible_to_recipient(viewer_empire_id))
            .map(|(idx, mail)| mail_item(game_data, idx, mail)),
    );

    items
}

fn report_item(idx: usize, row: &ReportBlockRow, current_year: u16) -> DashInboxItem {
    let body_lines = decode_text_lines(&row.decoded_text);
    let subject = body_lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| String::from("(report)"));
    let (week, year) = parse_stardate_week_year(&row.decoded_text)
        .map(|(week, year)| (Some(week), year))
        .unwrap_or((None, current_year));
    DashInboxItem {
        source: DashInboxItemSource::ReportBlock(idx),
        item_type: DashInboxItemType::Report,
        year,
        week,
        subject,
        body_lines,
        report_bucket: Some(classify_report_bucket(&row.decoded_text)),
    }
}

fn mail_item(game_data: &CoreGameData, idx: usize, mail: &QueuedPlayerMail) -> DashInboxItem {
    let subject = if mail.subject.trim().is_empty() {
        String::from("<no subject>")
    } else {
        mail.subject.trim().to_string()
    };
    let mut body_lines = vec![format!(
        "From {} (Empire #{})",
        sender_label(game_data, mail.sender_empire_id),
        mail.sender_empire_id
    )];
    if !mail.subject.trim().is_empty() {
        body_lines.push(format!("Subject: {}", mail.subject.trim()));
    }
    let decoded = decode_text_lines(&mail.body);
    if decoded.is_empty() {
        body_lines.push(String::new());
    } else {
        body_lines.extend(decoded);
    }
    DashInboxItem {
        source: DashInboxItemSource::QueuedMail(idx),
        item_type: DashInboxItemType::Message,
        year: mail.year,
        week: None,
        subject,
        body_lines,
        report_bucket: None,
    }
}

fn classify_report_bucket(text: &str) -> ReportSummaryBucket {
    let normalized = text.to_ascii_lowercase();

    if contains_any(
        &normalized,
        &[
            "bombard",
            "blitz",
            "invade",
            "invasion",
            "assault",
            "we were attacked",
            "we intercepted",
            "lost all contact",
            "captured by",
            "has been captured",
            "destroyed in combat",
            "engaged in combat",
            "battle at",
        ],
    ) {
        return ReportSummaryBucket::Combat;
    }

    if contains_any(
        &normalized,
        &[
            "contact report",
            "scout report",
            "sighted",
            "detected",
            "sensor contact",
            "contact with",
            "patrol",
        ],
    ) {
        return ReportSummaryBucket::Intel;
    }

    ReportSummaryBucket::Ops
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn parse_stardate_week_year(text: &str) -> Option<(u8, u16)> {
    let marker = text.find("Stardate")?;
    let tail = &text[marker + "Stardate".len()..];
    let digit_start = tail.find(|ch: char| ch.is_ascii_digit())?;
    let digits = &tail[digit_start..];
    let slash = digits.find('/')?;
    let week_raw = digits[..slash]
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let year_raw = digits[slash + 1..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let week = week_raw.parse::<u8>().ok()?;
    let year = year_raw.parse::<u16>().ok()?;
    Some((week, year))
}

fn decode_text_lines(text: &str) -> Vec<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized
        .split('\n')
        .map(str::trim_end)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn sender_label(game_data: &CoreGameData, sender_empire_id: u8) -> String {
    let Some(player) = game_data
        .player
        .records
        .get(sender_empire_id.saturating_sub(1) as usize)
    else {
        return format!("Empire #{sender_empire_id}");
    };
    let controlled = player.controlled_empire_name_summary();
    if !controlled.is_empty() {
        return controlled;
    }
    let legacy = player.legacy_status_name_summary();
    if !legacy.is_empty() {
        return legacy;
    }
    format!("Empire #{sender_empire_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_report_bucket_groups_combat_and_intel_text() {
        assert_eq!(
            classify_report_bucket("We were attacked by an alien force."),
            ReportSummaryBucket::Combat
        );
        assert_eq!(
            classify_report_bucket("Sensor contact shows an alien fleet in System(5,13)."),
            ReportSummaryBucket::Intel
        );
        assert_eq!(
            classify_report_bucket("Starbase has halted at System(5,13)."),
            ReportSummaryBucket::Ops
        );
    }

    #[test]
    fn parse_stardate_week_year_extracts_report_date() {
        assert_eq!(
            parse_stardate_week_year("Stardate: 02/3002"),
            Some((2, 3002))
        );
    }
}
