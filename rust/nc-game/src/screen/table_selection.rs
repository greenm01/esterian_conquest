pub fn find_typed_jump_index(
    rows: &[Vec<String>],
    selection_col: usize,
    input: &str,
) -> Option<usize> {
    let raw_input = input.trim();
    if raw_input.is_empty() {
        return None;
    }

    rows.iter().position(|row| {
        row.get(selection_col)
            .is_some_and(|cell| selection_key_matches(cell, raw_input))
    })
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
