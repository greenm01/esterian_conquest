pub fn format_sector_coords_table(coords: [u8; 2]) -> String {
    format!("({:02},{:02})", coords[0], coords[1])
}

pub fn format_sector_coords_default(coords: [u8; 2]) -> String {
    format!("{:02},{:02}", coords[0], coords[1])
}

#[cfg(test)]
mod tests {
    use super::{format_sector_coords_default, format_sector_coords_table};

    #[test]
    fn sector_coords_table_use_parenthesized_format() {
        assert_eq!(format_sector_coords_table([2, 3]), "(02,03)");
    }

    #[test]
    fn sector_coords_default_uses_bare_format() {
        assert_eq!(format_sector_coords_default([2, 3]), "02,03");
    }
}
