pub fn normalize_game_id(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
