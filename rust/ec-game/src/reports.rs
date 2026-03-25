use ec_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBlock {
    pub lines: Vec<String>,
    pub raw_chunked_bytes: Option<Vec<u8>>,
    pub runtime_mail_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportsPreview {
    pub results_lines: Vec<String>,
    pub message_lines: Vec<String>,
    pub result_blocks: Vec<ReviewBlock>,
    pub message_blocks: Vec<ReviewBlock>,
}

impl ReportsPreview {
    pub fn from_block_rows(
        game_data: &CoreGameData,
        viewer_empire_id: u8,
        report_blocks: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Self {
        let result_blocks = review_blocks_from_rows(report_blocks);
        let message_blocks = runtime_message_blocks(game_data, viewer_empire_id, queued_mail);
        Self {
            results_lines: flatten_block_lines(&result_blocks),
            message_lines: flatten_block_lines(&message_blocks),
            result_blocks,
            message_blocks,
        }
    }
}

pub fn has_visible_runtime_messages(
    viewer_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
) -> bool {
    queued_mail
        .iter()
        .any(|mail| mail.is_visible_to_recipient(viewer_empire_id))
}

pub fn wrap_review_text_preserving_spacing(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() || width == 0 {
        return vec![String::new()];
    }

    let mut rows = Vec::new();
    let mut remaining = text;
    while remaining.len() > width {
        let prefix = &remaining[..width];
        let break_at = prefix.rfind(' ').filter(|idx| *idx > 0);
        match break_at {
            Some(idx) => {
                rows.push(remaining[..idx].to_string());
                remaining = &remaining[idx + 1..];
            }
            None => {
                rows.push(prefix.to_string());
                remaining = &remaining[width..];
            }
        }
    }
    rows.push(remaining.to_string());
    rows
}

// ---------------------------------------------------------------------------
// Conversion: ec-data ReportBlockRow -> ec-game ReviewBlock
// ---------------------------------------------------------------------------

fn review_blocks_from_rows(rows: &[ReportBlockRow]) -> Vec<ReviewBlock> {
    rows.iter()
        .filter(|row| !row.recipient_deleted)
        .map(|row| ReviewBlock {
            lines: row
                .decoded_text
                .split('\n')
                .map(ToOwned::to_owned)
                .collect(),
            raw_chunked_bytes: row.raw_bytes.clone(),
            runtime_mail_index: None,
        })
        .collect()
}

fn flatten_block_lines(blocks: &[ReviewBlock]) -> Vec<String> {
    blocks
        .iter()
        .flat_map(|block| block.lines.iter().cloned())
        .collect()
}

fn runtime_message_blocks(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    queued_mail: &[QueuedPlayerMail],
) -> Vec<ReviewBlock> {
    queued_mail
        .iter()
        .enumerate()
        .filter(|(_, mail)| mail.is_visible_to_recipient(viewer_empire_id))
        .map(|(idx, mail)| ReviewBlock {
            lines: runtime_message_lines(game_data, mail),
            raw_chunked_bytes: None,
            runtime_mail_index: Some(idx),
        })
        .collect()
}

fn runtime_message_lines(game_data: &CoreGameData, mail: &QueuedPlayerMail) -> Vec<String> {
    let mut lines = vec![format!(
        "From {} (Empire #{})",
        sender_label(game_data, mail.sender_empire_id),
        mail.sender_empire_id
    )];
    if !mail.subject.trim().is_empty() {
        lines.push(format!("Subject: {}", mail.subject.trim()));
    }
    let body_lines = decode_text_lines(&mail.body);
    if body_lines.is_empty() {
        lines.push(String::new());
    } else {
        lines.extend(body_lines);
    }
    lines.push("<end of message>".to_string());
    lines
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
