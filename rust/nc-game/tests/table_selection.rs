use nc_game::screen::table_selection::{find_typed_jump_index, selection_key_matches};

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
}

#[test]
fn jump_can_target_non_first_selection_column() {
    let rows = vec![
        vec!["".to_string(), "Tokyo Night".to_string()],
        vec!["*".to_string(), "Mono".to_string()],
    ];

    assert_eq!(find_typed_jump_index(&rows, 1, "mon"), Some(1));
}
