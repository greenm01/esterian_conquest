/// Structured report-block storage types and decoding logic.
///
/// This module owns the pure data decomposition of classic RESULTS.DAT bytes
/// into structured `ReportBlockRow` records suitable for SQLite storage with
/// soft-delete support.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlockRow {
    pub block_index: usize,
    pub decoded_text: String,
    pub raw_bytes: Option<Vec<u8>>,
    pub recipient_deleted: bool,
}

// ---------------------------------------------------------------------------
// Public entry point: decompose raw bytes into block rows
// ---------------------------------------------------------------------------

/// Decode raw RESULTS.DAT bytes into a list of report block rows.
///
/// Each block corresponds to one transmission (delimited by
/// `<end of transmission>` in the classic format). The `decoded_text` field
/// contains newline-joined human-readable lines; `raw_bytes` preserves the
/// original 84-byte chunked records for classic DAT export.
pub fn decode_report_block_rows(bytes: &[u8]) -> Vec<ReportBlockRow> {
    if bytes.is_empty() {
        return Vec::new();
    }

    if let Some(blocks) = decode_chunked_records(bytes) {
        return blocks
            .into_iter()
            .enumerate()
            .map(|(idx, (lines, raw))| ReportBlockRow {
                block_index: idx,
                decoded_text: lines.join("\n"),
                raw_bytes: Some(raw),
                recipient_deleted: false,
            })
            .collect();
    }

    // Fallback: extract printable runs from non-chunked binary data.
    let fallback = printable_runs(bytes, 8);
    let text = if fallback.is_empty() {
        "<binary data present>".to_string()
    } else {
        fallback.join("\n")
    };
    vec![ReportBlockRow {
        block_index: 0,
        decoded_text: text,
        raw_bytes: None,
        recipient_deleted: false,
    }]
}

/// Rebuild classic RESULTS.DAT bytes from active (non-deleted) block rows.
///
/// Concatenates `raw_bytes` from each block in order. Returns `None` if any
/// block is missing raw bytes (e.g. fallback-decoded blocks).
pub fn rebuild_results_bytes(rows: &[ReportBlockRow]) -> Option<Vec<u8>> {
    let mut rebuilt = Vec::new();
    for row in rows {
        let raw = row.raw_bytes.as_ref()?;
        rebuilt.extend_from_slice(raw);
    }
    Some(rebuilt)
}

// ---------------------------------------------------------------------------
// Internal decoding (moved from ec-client/src/reports.rs)
// ---------------------------------------------------------------------------

/// Returns Vec<(lines, raw_bytes)> per block, or None if not chunked.
fn decode_chunked_records(bytes: &[u8]) -> Option<Vec<(Vec<String>, Vec<u8>)>> {
    if bytes.len() % 84 != 0 {
        return None;
    }

    if let Some(blocks) = decode_length_prefixed_chunked_records(bytes) {
        return Some(blocks);
    }

    decode_legacy_chunked_records(bytes)
}

fn decode_length_prefixed_chunked_records(bytes: &[u8]) -> Option<Vec<(Vec<String>, Vec<u8>)>> {
    const RESULTS_TEXT_SIZE: usize = 72;
    const RESULTS_TEXT_START: usize = 2;
    const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;

    let mut blocks = Vec::new();
    let mut current_lines = Vec::new();
    let mut current_raw = Vec::new();

    for chunk in bytes.chunks_exact(84) {
        let used = chunk.get(1).copied()? as usize;
        if used > RESULTS_TEXT_SIZE {
            return None;
        }
        let text_bytes = chunk.get(RESULTS_TEXT_START..RESULTS_TEXT_START + used)?;
        if !text_bytes.iter().all(|byte| {
            byte.is_ascii_graphic() || *byte == b' ' || *byte == b'\r' || *byte == b'\n'
        }) {
            return None;
        }
        if chunk[RESULTS_TEXT_START + used..RESULTS_TEXT_END]
            .iter()
            .any(|byte| *byte != 0)
        {
            return None;
        }
        let line = String::from_utf8(text_bytes.to_vec()).ok()?;
        let line = line.trim_end_matches('\0').trim_end().to_string();
        current_lines.push(line.clone());
        current_raw.extend_from_slice(chunk);

        if line == "<end of transmission>" {
            blocks.push((
                std::mem::take(&mut current_lines),
                std::mem::take(&mut current_raw),
            ));
        }
    }

    if !current_lines.is_empty() || !current_raw.is_empty() {
        blocks.push((current_lines, current_raw));
    }

    Some(blocks)
}

fn decode_legacy_chunked_records(bytes: &[u8]) -> Option<Vec<(Vec<String>, Vec<u8>)>> {
    let mut blocks = Vec::new();
    let mut current_text = String::new();
    let mut current_raw = Vec::new();

    for chunk in bytes.chunks_exact(84) {
        let text_bytes = chunk.get(1..76).unwrap_or(&[]);
        let used = text_bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(text_bytes.len());
        current_text.extend(text_bytes[..used].iter().map(|byte| char::from(*byte)));
        current_raw.extend_from_slice(chunk);

        if used < text_bytes.len() {
            blocks.push((
                decode_text_lines(&current_text),
                std::mem::take(&mut current_raw),
            ));
            current_text.clear();
        }
    }

    if !current_text.is_empty() || !current_raw.is_empty() {
        blocks.push((decode_text_lines(&current_text), current_raw));
    }

    Some(blocks)
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

fn printable_runs(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current = String::new();

    for &byte in bytes {
        let ch = char::from(byte);
        if ch.is_ascii_graphic() || ch == ' ' {
            current.push(ch);
        } else if current.len() >= min_len {
            runs.push(current.trim_end().to_string());
            current.clear();
        } else {
            current.clear();
        }
    }

    if current.len() >= min_len {
        runs.push(current.trim_end().to_string());
    }

    runs
}
