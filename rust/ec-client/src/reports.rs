#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportsPreview {
    pub results_lines: Vec<String>,
    pub message_lines: Vec<String>,
}

impl ReportsPreview {
    pub fn from_bytes(results_bytes: &[u8], message_bytes: &[u8]) -> Self {
        Self {
            results_lines: decode_report_bytes(results_bytes),
            message_lines: decode_report_bytes(message_bytes),
        }
    }
}

pub fn clear_report_bytes(results_bytes: &mut Vec<u8>, message_bytes: &mut Vec<u8>) {
    results_bytes.clear();
    message_bytes.clear();
}

fn decode_report_bytes(bytes: &[u8]) -> Vec<String> {
    if bytes.is_empty() {
        return Vec::new();
    }

    if let Some(lines) = decode_chunked_records(bytes) {
        return lines;
    }

    let fallback = printable_runs(bytes, 8);
    if fallback.is_empty() {
        vec!["<binary data present>".to_string()]
    } else {
        fallback
    }
}

fn decode_chunked_records(bytes: &[u8]) -> Option<Vec<String>> {
    if bytes.len() % 84 != 0 {
        return None;
    }

    let text = bytes
        .chunks_exact(84)
        .flat_map(|chunk| chunk.get(1..76).unwrap_or(&[]).iter().copied())
        .filter(|byte| *byte != 0)
        .map(char::from)
        .collect::<String>();
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    Some(lines)
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
