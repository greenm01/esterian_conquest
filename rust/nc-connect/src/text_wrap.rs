use nc_ui::buffer::{CellStyle, PlayfieldBuffer};

pub(crate) fn normalize_message_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    let mut paragraphs = Vec::new();
    let mut current = Vec::new();

    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        current.push(trimmed.to_string());
    }

    if !current.is_empty() {
        paragraphs.push(current.join(" "));
    }

    if paragraphs.is_empty() {
        String::new()
    } else {
        paragraphs.join("\n\n")
    }
}

pub(crate) fn wrapped_lines(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }

    let normalized = normalize_message_text(text);
    let mut lines = Vec::new();

    for raw_line in normalized.split('\n') {
        wrap_single_line(raw_line, max_width, &mut lines);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

pub(crate) fn write_lines_clamped(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_width: usize,
    max_rows: usize,
    lines: &[String],
    style: CellStyle,
) -> usize {
    if max_width == 0 || max_rows == 0 {
        return 0;
    }

    let visible_rows = lines.len().min(max_rows);
    for idx in 0..visible_rows {
        let is_last_visible = idx + 1 == max_rows;
        let overflow_hidden = lines.len() > max_rows;
        let line = if is_last_visible && overflow_hidden {
            truncate_with_continuation(&lines[idx], max_width)
        } else {
            clip_to_width(&lines[idx], max_width)
        };
        buffer.write_text_clipped(start_row + idx, col, &line, style);
    }
    visible_rows
}

pub(crate) fn write_wrapped_lines_clamped(
    buffer: &mut PlayfieldBuffer,
    start_row: usize,
    col: usize,
    max_width: usize,
    max_rows: usize,
    text: &str,
    style: CellStyle,
) -> usize {
    let lines = wrapped_lines(text, max_width);
    write_lines_clamped(buffer, start_row, col, max_width, max_rows, &lines, style)
}

fn wrap_single_line(text: &str, max_width: usize, out: &mut Vec<String>) {
    if text.trim().is_empty() {
        out.push(String::new());
        return;
    }

    let mut current = String::new();

    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        if word_len > max_width {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            push_split_word(word, max_width, out, &mut current);
            continue;
        }

        let needed = if current.is_empty() {
            word_len
        } else {
            current.chars().count() + 1 + word_len
        };
        if needed > max_width && !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        out.push(current);
    }
}

fn push_split_word(word: &str, max_width: usize, out: &mut Vec<String>, current: &mut String) {
    let chars: Vec<char> = word.chars().collect();
    for chunk in chars.chunks(max_width) {
        let piece: String = chunk.iter().collect();
        if chunk.len() == max_width {
            out.push(piece);
        } else {
            current.push_str(&piece);
        }
    }
}

fn clip_to_width(text: &str, max_width: usize) -> String {
    text.chars().take(max_width).collect()
}

fn truncate_with_continuation(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let clipped = clip_to_width(text, max_width.saturating_sub(3));
    format!("{clipped}...")
}
