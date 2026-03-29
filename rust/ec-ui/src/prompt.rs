use crate::buffer::{PlayfieldBuffer, StyledSpan};
use crate::theme::classic;

const COMMAND_LABEL: &str = "COMMAND";
const COMMAND_ARROW_PREFIX: &str = " <- ";
const COMMAND_ARROW_SUFFIX: &str = " -> ";
const DEFAULT_OPEN: &str = "[";
const DEFAULT_CLOSE_WITH_SPACE: &str = "] ";
const DEFAULT_CLOSE_WITH_SUFFIX: &str = "] -> ";
const SLAP_A_KEY_PROMPT: &str = "(slap a key)";

pub fn plain_prompt_width(prompt: &str) -> usize {
    ensure_cursor_gap(prompt).chars().count()
}

pub fn dismiss_prompt_width() -> usize {
    plain_prompt_width(SLAP_A_KEY_PROMPT)
}

pub fn command_line_text_width(label: &str, text: &str) -> usize {
    label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + text.chars().count()
}

pub fn command_line_prompt_text_width(label: &str, prompt: &str) -> usize {
    label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + plain_prompt_width(prompt)
}

pub fn command_line_default_input_width(
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) -> usize {
    command_line_default_input_scaffold_width(label, prompt, default) + input.chars().count()
}

pub fn command_line_default_input_scaffold_width(
    label: &str,
    prompt: &str,
    default: &str,
) -> usize {
    let mut width =
        label.chars().count() + COMMAND_ARROW_PREFIX.chars().count() + plain_prompt_width(prompt);
    if !default.is_empty() {
        width += DEFAULT_OPEN.chars().count()
            + default.chars().count()
            + DEFAULT_CLOSE_WITH_SPACE.chars().count();
    }
    width + "<Q> -> ".chars().count()
}

pub fn table_command_bar_width(hotkeys_markup: &str, default: Option<&str>, input: &str) -> usize {
    table_command_bar_scaffold_width(hotkeys_markup, default) + input.chars().count()
}

pub fn table_command_bar_scaffold_width(hotkeys_markup: &str, default: Option<&str>) -> usize {
    let mut width = COMMAND_LABEL.chars().count()
        + COMMAND_ARROW_PREFIX.chars().count()
        + command_rail_width(hotkeys_markup);
    if let Some(default) = default {
        width += " ".chars().count()
            + DEFAULT_OPEN.chars().count()
            + default.chars().count()
            + DEFAULT_CLOSE_WITH_SUFFIX.chars().count();
    } else {
        width += COMMAND_ARROW_SUFFIX.chars().count();
    }
    width
}

pub fn table_command_prompt_width(prompt: &str) -> usize {
    COMMAND_LABEL.chars().count()
        + COMMAND_ARROW_PREFIX.chars().count()
        + plain_prompt_width(prompt)
}

pub fn draw_command_prompt_at(buffer: &mut PlayfieldBuffer, row: usize, label: &str, keys: &str) {
    draw_command_prompt_at_col(buffer, row, 0, label, keys);
}

pub fn draw_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    keys: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let suffix = "-> ";
    if keys == "SLAP A KEY" {
        buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <-", classic::prompt_style()),
            ],
        );
        let slap_width = SLAP_A_KEY_PROMPT.chars().count();
        let suffix_col = buffer.width().saturating_sub(suffix.chars().count() + 1);
        let slap_col = suffix_col.saturating_sub(slap_width);
        write_slap_a_key(buffer, row, slap_col);
        let written = buffer.write_text(row, suffix_col, suffix, classic::prompt_style());
        let cursor_col = suffix_col + written;
        if cursor_col < buffer.width() {
            buffer.set_cursor(cursor_col as u16, row as u16);
        }
    } else {
        let prefix_col = col
            + buffer.write_spans(
                row,
                col,
                &[
                    StyledSpan::new(label, classic::title_style()),
                    StyledSpan::new(" <- ", classic::prompt_style()),
                ],
            );
        let mut cursor_col = write_command_rail_tokens(buffer, row, prefix_col, keys);
        cursor_col += buffer.write_text(row, cursor_col, " -> ", classic::prompt_style());
        if cursor_col < buffer.width() {
            buffer.set_cursor(cursor_col as u16, row as u16);
        }
    }
}

pub fn draw_command_line_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    text: &str,
) {
    draw_command_line_text_at_col(buffer, row, 0, label, text);
}

pub fn draw_command_line_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    text: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    buffer.write_spans(
        row,
        col,
        &[
            StyledSpan::new(label, classic::title_style()),
            StyledSpan::new(" <- ", classic::prompt_style()),
            StyledSpan::new(text, classic::prompt_style()),
        ],
    );
}

pub fn draw_command_line_prompt_text_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
) {
    draw_command_line_prompt_text_at_col(buffer, row, 0, label, prompt);
}

pub fn draw_command_line_prompt_text_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
}

pub fn draw_command_line_default_input_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    draw_command_line_default_input_at_col(buffer, row, 0, label, prompt, default, input)
}

pub fn draw_command_line_default_input_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    label: &str,
    prompt: &str,
    default: &str,
    input: &str,
) {
    buffer.fill_row(row, classic::prompt_style());
    let mut cursor_col = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new(label, classic::title_style()),
                StyledSpan::new(" <- ", classic::prompt_style()),
            ],
        );
    cursor_col = write_prompt_markup(buffer, row, cursor_col, prompt);
    if !default.is_empty() {
        cursor_col += buffer.write_spans(
            row,
            cursor_col,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(" ", classic::prompt_style()),
            ],
        );
    }
    cursor_col = write_prompt_markup(buffer, row, cursor_col, "<Q> -> ");
    let written = buffer.write_text(row, cursor_col, input, classic::prompt_hotkey_style());
    let cursor_col = cursor_col + written;
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
}

pub fn draw_table_command_bar_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    draw_table_command_bar_at_col(buffer, row, 0, hotkeys_markup, default, input)
}

pub fn draw_table_command_bar_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    hotkeys_markup: &str,
    default: Option<&str>,
    input: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let mut cursor_col = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new(COMMAND_LABEL, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    cursor_col = write_command_rail_tokens(buffer, row, cursor_col, hotkeys_markup);
    if let Some(default) = default {
        cursor_col += buffer.write_spans(
            row,
            cursor_col,
            &[
                StyledSpan::new(" ", classic::prompt_style()),
                StyledSpan::new(DEFAULT_OPEN, classic::prompt_square_delimiter_style()),
                StyledSpan::new(default, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
                StyledSpan::new(COMMAND_ARROW_SUFFIX, classic::prompt_style()),
            ],
        );
        let written = buffer.write_text(row, cursor_col, input, classic::prompt_hotkey_style());
        let final_cursor_col = cursor_col + written;
        if final_cursor_col < buffer.width() {
            buffer.set_cursor(final_cursor_col as u16, row as u16);
        }
        final_cursor_col
    } else {
        let written = buffer.write_text(
            row,
            cursor_col,
            COMMAND_ARROW_SUFFIX,
            classic::prompt_style(),
        );
        let final_cursor_col = cursor_col + written;
        if final_cursor_col < buffer.width() {
            buffer.set_cursor(final_cursor_col as u16, row as u16);
        }
        final_cursor_col
    }
}

pub fn draw_table_command_prompt_at(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    prompt: &str,
) -> usize {
    draw_table_command_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_table_command_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let prefix = col
        + buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new(COMMAND_LABEL, classic::title_style()),
                StyledSpan::new(COMMAND_ARROW_PREFIX, classic::prompt_style()),
            ],
        );
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, prefix, &prompt);
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
    cursor_col
}

pub fn draw_plain_prompt(buffer: &mut PlayfieldBuffer, row: usize, prompt: &str) -> usize {
    draw_plain_prompt_at_col(buffer, row, 0, prompt)
}

pub fn draw_plain_prompt_at_col(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    prompt: &str,
) -> usize {
    buffer.fill_row(row, classic::prompt_style());
    let prompt = ensure_cursor_gap(prompt);
    let cursor_col = write_prompt_markup(buffer, row, col, &prompt);
    if cursor_col < buffer.width() {
        buffer.set_cursor(cursor_col as u16, row as u16);
    }
    cursor_col
}

fn ensure_cursor_gap(prompt: &str) -> String {
    if prompt.ends_with("-> ") {
        prompt.to_string()
    } else if prompt.ends_with("->") {
        format!("{prompt} ")
    } else {
        prompt.to_string()
    }
}

fn command_rail_width(tokens: &str) -> usize {
    tokens
        .split_whitespace()
        .map(|token| token.chars().count())
        .sum::<usize>()
        + tokens.split_whitespace().count().saturating_sub(1)
}

fn write_slap_a_key(buffer: &mut PlayfieldBuffer, row: usize, col: usize) -> usize {
    let after_open = col + buffer.write_text(row, col, "(", classic::prompt_hotkey_style());
    let after_text = after_open
        + buffer.write_text(
            row,
            after_open,
            "slap a ",
            classic::prompt_notice_action_style(),
        );
    let after_key =
        after_text + buffer.write_text(row, after_text, "key", classic::prompt_hotkey_style());
    after_key + buffer.write_text(row, after_key, ")", classic::prompt_hotkey_style())
}

fn write_command_rail_tokens(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    tokens: &str,
) -> usize {
    let mut col = start_col;
    let mut first = true;
    for token in tokens.split_whitespace() {
        if !first {
            col += buffer.write_text(row, col, " ", classic::prompt_style());
        }
        col += write_command_rail_token(buffer, row, col, token);
        first = false;
    }
    col
}

fn write_command_rail_token(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    col: usize,
    token: &str,
) -> usize {
    if let Some(inner) = token
        .strip_prefix('<')
        .and_then(|value| value.strip_suffix('>'))
    {
        buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new("<", classic::prompt_angle_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new(">", classic::prompt_angle_delimiter_style()),
            ],
        )
    } else if let Some(inner) = token
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        buffer.write_spans(
            row,
            col,
            &[
                StyledSpan::new("[", classic::prompt_square_delimiter_style()),
                StyledSpan::new(inner, classic::prompt_hotkey_style()),
                StyledSpan::new("]", classic::prompt_square_delimiter_style()),
            ],
        )
    } else {
        buffer.write_text(row, col, token, classic::prompt_hotkey_style())
    }
}

fn write_prompt_markup(
    buffer: &mut PlayfieldBuffer,
    row: usize,
    start_col: usize,
    text: &str,
) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut col = start_col;
    let mut plain = String::new();
    let mut idx = 0usize;

    while idx < chars.len() {
        if let Some((phrase_end, key_start, key_end)) = slap_a_key_phrase(&chars, idx) {
            if !plain.is_empty() {
                col += buffer.write_text(row, col, &plain, classic::prompt_style());
                plain.clear();
            }
            if key_start > idx {
                let prefix = chars[idx..key_start].iter().collect::<String>();
                col += buffer.write_text(row, col, &prefix, classic::prompt_notice_action_style());
            }
            let key = chars[key_start..key_end].iter().collect::<String>();
            col += buffer.write_text(row, col, &key, classic::prompt_hotkey_style());
            idx = phrase_end;
            continue;
        }

        if chars[idx] == '<'
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == '>')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_angle_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, "<", classic::prompt_angle_delimiter_style());
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text(row, col, &segment, classic::prompt_hotkey_style());
                }
                col += buffer.write_text(row, col, ">", classic::prompt_angle_delimiter_style());
                idx = close_idx + 1;
                continue;
            }
        }

        if chars[idx] == '['
            && let Some(close_idx) = chars[idx + 1..].iter().position(|&ch| ch == ']')
        {
            let close_idx = idx + 1 + close_idx;
            if is_prompt_bracket_hotkey(&chars[idx + 1..close_idx]) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, "[", classic::prompt_square_delimiter_style());
                if close_idx > idx + 1 {
                    let segment = chars[idx + 1..close_idx].iter().collect::<String>();
                    col += buffer.write_text(row, col, &segment, classic::prompt_hotkey_style());
                }
                col += buffer.write_text(row, col, "]", classic::prompt_square_delimiter_style());
                idx = close_idx + 1;
                continue;
            }
        }

        if chars[idx].is_ascii_alphanumeric() {
            let start = idx;
            while idx < chars.len() && chars[idx].is_ascii_alphanumeric() {
                idx += 1;
            }
            let token = chars[start..idx].iter().collect::<String>();
            if is_prompt_slash_hotkey_token(&chars, start, idx) {
                if !plain.is_empty() {
                    col += buffer.write_text(row, col, &plain, classic::prompt_style());
                    plain.clear();
                }
                col += buffer.write_text(row, col, &token, classic::prompt_hotkey_style());
            } else {
                plain.push_str(&token);
            }
            continue;
        }

        plain.push(chars[idx]);
        idx += 1;
    }

    if !plain.is_empty() {
        col += buffer.write_text(row, col, &plain, classic::prompt_style());
    }

    col
}

fn is_prompt_slash_hotkey_token(chars: &[char], start: usize, end: usize) -> bool {
    let token_len = end.saturating_sub(start);
    token_len > 0
        && token_len <= 3
        && (matches!(chars.get(end), Some('/')) || (start > 0 && chars[start - 1] == '/'))
}

fn is_prompt_bracket_hotkey(chars: &[char]) -> bool {
    !chars.is_empty() && chars.len() <= 5 && chars.iter().all(|ch| ch.is_ascii_alphanumeric())
}

fn is_prompt_angle_hotkey(chars: &[char]) -> bool {
    !chars.is_empty()
        && chars
            .iter()
            .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace())
}

fn slap_a_key_phrase(chars: &[char], start: usize) -> Option<(usize, usize, usize)> {
    const KEYWORD: [&str; 2] = ["slap a key", "Slap a key"];
    for keyword in KEYWORD {
        let kw_chars: Vec<char> = keyword.chars().collect();
        let end = start + kw_chars.len();
        if end > chars.len() || chars[start..end] != kw_chars[..] {
            continue;
        }
        let key_start = start + kw_chars.len() - 3;
        return Some((end, key_start, end));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        command_line_default_input_scaffold_width, command_line_default_input_width,
        draw_command_line_prompt_text_at, draw_plain_prompt, draw_table_command_bar_at,
        table_command_bar_scaffold_width, table_command_bar_width, table_command_prompt_width,
    };
    use crate::buffer::PlayfieldBuffer;
    use crate::theme::classic;

    fn buffer() -> PlayfieldBuffer {
        PlayfieldBuffer::new(80, 25, classic::body_style())
    }

    fn find_in_row(buffer: &PlayfieldBuffer, row: usize, needle: &str) -> usize {
        buffer
            .plain_line(row)
            .find(needle)
            .expect("needle should be present")
    }

    #[test]
    fn plain_prompt_highlights_angle_and_square_tokens() {
        let mut buffer = buffer();
        draw_plain_prompt(&mut buffer, 24, "Delete report [Y]/N <Q> ->");
        let row = buffer.row(24);
        let bracket = find_in_row(&buffer, 24, "[Y]");
        assert_eq!(row[bracket].style, classic::prompt_square_delimiter_style());
        assert_eq!(row[bracket + 1].style, classic::prompt_hotkey_style());
        let quit = find_in_row(&buffer, 24, "<Q>");
        assert_eq!(row[quit].style, classic::prompt_angle_delimiter_style());
        assert_eq!(row[quit + 1].style, classic::prompt_hotkey_style());
    }

    #[test]
    fn table_command_bar_renders_commands_label() {
        let mut buffer = buffer();
        draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <N> <Q>", None, "");
        assert!(buffer.plain_line(24).starts_with("COMMAND <- "));
    }

    #[test]
    fn command_line_prompt_keeps_one_space_after_arrow_for_cursor() {
        let mut buffer = buffer();
        draw_command_line_prompt_text_at(&mut buffer, 24, "COMMAND", "Are you sure? Y/[N] ->");
        let (col, row) = buffer.cursor().expect("cursor");
        let line = buffer.plain_line(row as usize);
        let arrow = line.find("->").expect("arrow");
        assert_eq!(col as usize, arrow + "->".chars().count() + 1);
    }

    #[test]
    fn table_command_bar_width_matches_rendered_footer_width() {
        let mut buffer = buffer();
        let end_col = draw_table_command_bar_at(&mut buffer, 24, "J K ^U ^D <Q>", Some("02"), "");
        assert_eq!(
            end_col,
            table_command_bar_width("J K ^U ^D <Q>", Some("02"), "")
        );
    }

    #[test]
    fn prompt_width_helpers_include_default_and_input_span() {
        let width = command_line_default_input_width("COMMAND", "Qty ", "12", "345");
        assert!(width > table_command_prompt_width("Qty -> "));
        assert_eq!(width, "COMMAND <- Qty [12] <Q> -> 345".chars().count());
    }

    #[test]
    fn scaffold_width_helpers_ignore_live_input_text() {
        assert_eq!(
            table_command_bar_scaffold_width("J K ^U ^D <Q>", Some("Matrix")),
            table_command_bar_width("J K ^U ^D <Q>", Some("Matrix"), "")
        );
        assert_eq!(
            table_command_bar_scaffold_width("J K ^U ^D <Q>", Some("Matrix")),
            table_command_bar_width("J K ^U ^D <Q>", Some("Matrix"), "tokyo")
                - "tokyo".chars().count()
        );
        assert_eq!(
            command_line_default_input_scaffold_width("COMMAND", "Qty ", "12"),
            command_line_default_input_width("COMMAND", "Qty ", "12", "345")
                - "345".chars().count()
        );
    }
}
