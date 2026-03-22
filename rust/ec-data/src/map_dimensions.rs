pub fn map_size_for_player_count(player_count: u8) -> u8 {
    match player_count {
        1..=4 => 18,
        5..=9 => 27,
        10..=16 => 36,
        _ => 45,
    }
}
