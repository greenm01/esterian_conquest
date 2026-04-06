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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReportsPanelSummary {
    pub pending_count: usize,
    pub report_count: usize,
    pub message_count: usize,
    pub current_count: usize,
    pub backlog_count: usize,
    pub combat_count: usize,
    pub intel_count: usize,
    pub ops_count: usize,
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

pub fn summarize_reports_panel(items: &[DashInboxItem], current_year: u16) -> ReportsPanelSummary {
    let mut summary = ReportsPanelSummary::default();
    for item in items {
        summary.pending_count += 1;
        if item.year == current_year {
            summary.current_count += 1;
        } else {
            summary.backlog_count += 1;
        }
        match item.item_type {
            DashInboxItemType::Report => {
                summary.report_count += 1;
                match item.report_bucket.unwrap_or(ReportSummaryBucket::Ops) {
                    ReportSummaryBucket::Combat => summary.combat_count += 1,
                    ReportSummaryBucket::Intel => summary.intel_count += 1,
                    ReportSummaryBucket::Ops => summary.ops_count += 1,
                }
            }
            DashInboxItemType::Message => {
                summary.message_count += 1;
            }
        }
    }
    summary
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

    #[test]
    fn summarize_reports_panel_counts_current_and_backlog_items() {
        let items = vec![
            DashInboxItem {
                source: DashInboxItemSource::ReportBlock(0),
                item_type: DashInboxItemType::Report,
                year: 3002,
                week: Some(2),
                subject: String::from("Combat"),
                body_lines: vec![String::from("Body")],
                report_bucket: Some(ReportSummaryBucket::Combat),
            },
            DashInboxItem {
                source: DashInboxItemSource::ReportBlock(1),
                item_type: DashInboxItemType::Report,
                year: 3001,
                week: Some(12),
                subject: String::from("Scout"),
                body_lines: vec![String::from("Body")],
                report_bucket: Some(ReportSummaryBucket::Intel),
            },
            DashInboxItem {
                source: DashInboxItemSource::QueuedMail(0),
                item_type: DashInboxItemType::Message,
                year: 3002,
                week: None,
                subject: String::from("Scout"),
                body_lines: vec![String::from("Body")],
                report_bucket: None,
            },
        ];

        let summary = summarize_reports_panel(&items, 3002);
        assert_eq!(summary.pending_count, 3);
        assert_eq!(summary.report_count, 2);
        assert_eq!(summary.message_count, 1);
        assert_eq!(summary.current_count, 2);
        assert_eq!(summary.backlog_count, 1);
        assert_eq!(summary.combat_count, 1);
        assert_eq!(summary.intel_count, 1);
        assert_eq!(summary.ops_count, 0);
    }
}
