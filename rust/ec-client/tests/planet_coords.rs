use ec_client::screen::parse_planet_coords;

#[test]
fn parse_planet_coords_accepts_unpadded_coords() {
    assert_eq!(parse_planet_coords("3,3"), Some([3, 3]));
    assert_eq!(parse_planet_coords("[3,3]"), Some([3, 3]));
}

#[test]
fn parse_planet_coords_accepts_zero_padded_coords() {
    assert_eq!(parse_planet_coords("03,03"), Some([3, 3]));
    assert_eq!(parse_planet_coords("[03,03]"), Some([3, 3]));
}
