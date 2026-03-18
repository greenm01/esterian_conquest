#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBlock {
    pub lines: Vec<String>,
    pub raw_chunked_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportsPreview {
    pub results_lines: Vec<String>,
    pub message_lines: Vec<String>,
    pub result_blocks: Vec<ReviewBlock>,
    pub message_blocks: Vec<ReviewBlock>,
}

impl ReportsPreview {
    pub fn from_bytes(results_bytes: &[u8], message_bytes: &[u8]) -> Self {
        let result_blocks = decode_report_blocks(results_bytes);
        let message_blocks = decode_report_blocks(message_bytes);
        Self {
            results_lines: flatten_block_lines(&result_blocks),
            message_lines: flatten_block_lines(&message_blocks),
            result_blocks,
            message_blocks,
        }
    }
}

pub fn clear_report_bytes(results_bytes: &mut Vec<u8>, message_bytes: &mut Vec<u8>) {
    results_bytes.clear();
    message_bytes.clear();
}

pub fn rebuild_chunked_bytes(blocks: &[ReviewBlock]) -> Option<Vec<u8>> {
    let mut rebuilt = Vec::new();
    for block in blocks {
        let raw = block.raw_chunked_bytes.as_ref()?;
        rebuilt.extend_from_slice(raw);
    }
    Some(rebuilt)
}

fn decode_report_blocks(bytes: &[u8]) -> Vec<ReviewBlock> {
    if bytes.is_empty() {
        return Vec::new();
    }

    if let Some(blocks) = decode_chunked_records(bytes) {
        return blocks;
    }

    let fallback = printable_runs(bytes, 8);
    if fallback.is_empty() {
        vec![ReviewBlock {
            lines: vec!["<binary data present>".to_string()],
            raw_chunked_bytes: None,
        }]
    } else {
        vec![ReviewBlock {
            lines: fallback,
            raw_chunked_bytes: None,
        }]
    }
}

fn flatten_block_lines(blocks: &[ReviewBlock]) -> Vec<String> {
    blocks
        .iter()
        .flat_map(|block| block.lines.iter().cloned())
        .collect()
}

fn decode_chunked_records(bytes: &[u8]) -> Option<Vec<ReviewBlock>> {
    if bytes.len() % 84 != 0 {
        return None;
    }

    if let Some(blocks) = decode_length_prefixed_chunked_records(bytes) {
        return Some(blocks);
    }

    decode_legacy_chunked_records(bytes)
}

fn decode_length_prefixed_chunked_records(bytes: &[u8]) -> Option<Vec<ReviewBlock>> {
    const RESULTS_TEXT_SIZE: usize = 72;
    const RESULTS_TEXT_START: usize = 2;
    const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;

    let mut blocks = Vec::new();
    let mut current_text = String::new();
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
        current_text.extend(text_bytes.iter().map(|byte| char::from(*byte)));
        current_raw.extend_from_slice(chunk);

        if used < RESULTS_TEXT_SIZE {
            blocks.push(ReviewBlock {
                lines: decode_text_lines(&current_text),
                raw_chunked_bytes: Some(std::mem::take(&mut current_raw)),
            });
            current_text.clear();
        }
    }

    if !current_text.is_empty() || !current_raw.is_empty() {
        blocks.push(ReviewBlock {
            lines: decode_text_lines(&current_text),
            raw_chunked_bytes: Some(current_raw),
        });
    }

    Some(blocks)
}

fn decode_legacy_chunked_records(bytes: &[u8]) -> Option<Vec<ReviewBlock>> {
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
            blocks.push(ReviewBlock {
                lines: decode_text_lines(&current_text),
                raw_chunked_bytes: Some(std::mem::take(&mut current_raw)),
            });
            current_text.clear();
        }
    }

    if !current_text.is_empty() || !current_raw.is_empty() {
        blocks.push(ReviewBlock {
            lines: decode_text_lines(&current_text),
            raw_chunked_bytes: Some(current_raw),
        });
    }

    Some(blocks)
}

fn decode_text_lines(text: &str) -> Vec<String> {
    let normalized = text
        .split("<end of transmission>")
        .next()
        .unwrap_or(text)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut lines = normalized
        .split('\n')
        .map(str::trim)
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
            runs.push(current.trim().to_string());
            current.clear();
        } else {
            current.clear();
        }
    }

    if current.len() >= min_len {
        runs.push(current.trim().to_string());
    }

    runs
}
