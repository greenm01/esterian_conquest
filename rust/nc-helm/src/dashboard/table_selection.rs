#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedJumpMatch {
    pub index: usize,
    pub is_terminal_exact_match: bool,
}

pub fn find_typed_jump(
    rows: &[Vec<String>],
    selection_col: usize,
    input: &str,
) -> Option<TypedJumpMatch> {
    let raw_input = input.trim();
    if raw_input.is_empty() {
        return None;
    }

    let normalized_input = normalize_for_match(raw_input);
    if normalized_input.is_empty() {
        return None;
    }

    let mut index = None;
    let mut matched_cell_exact = false;
    let mut has_longer_prefix_match = false;

    for (row_index, row) in rows.iter().enumerate() {
        let Some(cell) = row.get(selection_col) else {
            continue;
        };
        let normalized_cell = normalize_for_match(cell);
        if normalized_cell.is_empty() || !normalized_cell.starts_with(&normalized_input) {
            continue;
        }

        if index.is_none() {
            index = Some(row_index);
            matched_cell_exact = normalized_cell == normalized_input;
        }

        if normalized_cell.len() > normalized_input.len() {
            has_longer_prefix_match = true;
        }
    }

    index.map(|index| TypedJumpMatch {
        index,
        is_terminal_exact_match: matched_cell_exact && !has_longer_prefix_match,
    })
}

pub fn find_typed_jump_index(
    rows: &[Vec<String>],
    selection_col: usize,
    input: &str,
) -> Option<usize> {
    find_typed_jump(rows, selection_col, input).map(|matched| matched.index)
}

pub fn selection_key_matches(cell: &str, raw_input: &str) -> bool {
    let cell = normalize_for_match(cell);
    let raw_input = normalize_for_match(raw_input);
    if cell.is_empty() || raw_input.is_empty() {
        return false;
    }

    cell.starts_with(&raw_input)
}

pub fn is_coordinate_input_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, ',' | ' ' | '(' | ')' | '[' | ']' | '{' | '}')
}

pub fn wrap_prev_index(selected: usize, total_rows: usize) -> usize {
    if total_rows == 0 {
        0
    } else if selected == 0 {
        total_rows - 1
    } else {
        selected - 1
    }
}

pub fn wrap_next_index(selected: usize, total_rows: usize) -> usize {
    if total_rows == 0 {
        0
    } else if selected + 1 >= total_rows {
        0
    } else {
        selected + 1
    }
}

pub fn sync_scroll_to_cursor(scroll_offset: &mut usize, cursor: usize, visible: usize) {
    if cursor < *scroll_offset {
        *scroll_offset = cursor;
    } else if cursor >= scroll_offset.saturating_add(visible) {
        *scroll_offset = cursor + 1 - visible;
    }
}

fn normalize_for_match(value: &str) -> String {
    let mut normalized = String::new();
    for token in value.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            let stripped = token.trim_start_matches('0');
            normalized.push_str(if stripped.is_empty() { "0" } else { stripped });
        } else {
            normalized.push_str(&token.to_ascii_lowercase());
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{
        TypedJumpMatch, find_typed_jump, find_typed_jump_index, is_coordinate_input_char,
        selection_key_matches, sync_scroll_to_cursor, wrap_next_index, wrap_prev_index,
    };

    #[test]
    fn numeric_jump_matches_zero_padded_prefixes() {
        let rows = vec![
            vec!["01".to_string()],
            vec!["03".to_string()],
            vec!["12".to_string()],
        ];

        assert_eq!(find_typed_jump_index(&rows, 0, "3"), Some(1));
        assert_eq!(find_typed_jump_index(&rows, 0, "12"), Some(2));
    }

    #[test]
    fn text_jump_matches_case_insensitive_prefixes() {
        let rows = vec![
            vec!["Alpha Prime".to_string()],
            vec!["beta minor".to_string()],
        ];

        assert_eq!(find_typed_jump_index(&rows, 0, "BE"), Some(1));
        assert!(selection_key_matches("Alpha Prime", "alp"));
    }

    #[test]
    fn coordinate_jump_ignores_render_punctuation() {
        let rows = vec![vec!["(01,09)".to_string()], vec!["(12,03)".to_string()]];

        assert_eq!(find_typed_jump_index(&rows, 0, "1,9"), Some(0));
        assert_eq!(find_typed_jump_index(&rows, 0, "12,3"), Some(1));
        assert_eq!(find_typed_jump_index(&rows, 0, "{1,9}"), Some(0));
        assert_eq!(find_typed_jump_index(&rows, 0, "12 3"), Some(1));
        assert!(selection_key_matches("(12,03)", "[12, 3]"));
    }

    #[test]
    fn jump_can_target_non_first_selection_column() {
        let rows = vec![
            vec!["".to_string(), "Tokyo Night".to_string()],
            vec!["*".to_string(), "Mono".to_string()],
        ];

        assert_eq!(find_typed_jump_index(&rows, 1, "mon"), Some(1));
    }

    #[test]
    fn terminal_exact_match_only_clears_when_no_longer_prefix_exists() {
        let rows = vec![
            vec!["09".to_string()],
            vec!["12".to_string()],
            vec!["123".to_string()],
        ];

        assert_eq!(
            find_typed_jump(&rows, 0, "1"),
            Some(TypedJumpMatch {
                index: 1,
                is_terminal_exact_match: false,
            })
        );
        assert_eq!(
            find_typed_jump(&rows, 0, "12"),
            Some(TypedJumpMatch {
                index: 1,
                is_terminal_exact_match: false,
            })
        );
        assert_eq!(
            find_typed_jump(&rows, 0, "123"),
            Some(TypedJumpMatch {
                index: 2,
                is_terminal_exact_match: true,
            })
        );
    }

    #[test]
    fn coordinate_input_chars_match_nc_game_convention() {
        assert!(is_coordinate_input_char('1'));
        assert!(is_coordinate_input_char(','));
        assert!(is_coordinate_input_char('['));
        assert!(!is_coordinate_input_char('A'));
    }

    #[test]
    fn wrapped_index_navigation_matches_dashboard_tables() {
        assert_eq!(wrap_prev_index(0, 4), 3);
        assert_eq!(wrap_next_index(3, 4), 0);
        assert_eq!(wrap_prev_index(0, 0), 0);
        assert_eq!(wrap_next_index(0, 0), 0);
    }

    #[test]
    fn scroll_sync_keeps_cursor_visible() {
        let mut scroll = 0;
        sync_scroll_to_cursor(&mut scroll, 5, 4);
        assert_eq!(scroll, 2);

        sync_scroll_to_cursor(&mut scroll, 1, 4);
        assert_eq!(scroll, 1);
    }
}
