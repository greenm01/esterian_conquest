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
