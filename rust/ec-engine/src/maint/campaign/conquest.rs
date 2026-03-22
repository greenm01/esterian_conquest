use crate::CoreGameData;

pub(super) fn process_conquest_header(
    game_data: &mut CoreGameData,
    should_accumulate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if should_accumulate {
        let prod_total =
            u16::from_le_bytes([game_data.conquest.raw[0x0c], game_data.conquest.raw[0x0d]]);
        let new_prod_total = prod_total.saturating_add(100);
        let [lo, hi] = new_prod_total.to_le_bytes();
        game_data.conquest.raw[0x0c] = lo;
        game_data.conquest.raw[0x0d] = hi;
        game_data.conquest.raw[0x3d] = game_data.conquest.raw[0x3d].saturating_add(1);
    }

    let offsets_to_clear = [
        0x14, 0x16, 0x18, 0x1c, 0x1e, 0x24, 0x2a, 0x2c, 0x2e, 0x30, 0x32, 0x34,
    ];

    for offset in offsets_to_clear {
        if game_data.conquest.raw[offset] == 0x64 {
            game_data.conquest.raw[offset] = 0x00;
        }
    }

    if game_data.conquest.raw[0x0e] == 0x00 {
        let non_active_prods: Vec<u16> = game_data
            .player
            .records
            .iter()
            .filter(|p| p.raw[0x00] != 0x01)
            .map(|p| p.raw[0x52] as u16)
            .collect();

        let mut write_offset = 0x0cusize;
        for prod in non_active_prods.iter().take(3) {
            game_data.conquest.raw[write_offset] = (*prod & 0xFF) as u8;
            game_data.conquest.raw[write_offset + 1] = (*prod >> 8) as u8;
            write_offset += 2;
        }
    }

    game_data.conquest.raw[0x12] = 0xFF;
    game_data.conquest.raw[0x13] = 0xFF;
    game_data.conquest.raw[0x1a] = 0x74;
    game_data.conquest.raw[0x1b] = 0x33;

    if game_data.conquest.raw[0x20] == 0x64 {
        game_data.conquest.raw[0x20] = 0x75;
        game_data.conquest.raw[0x21] = 0x03;
    }

    if game_data.conquest.raw[0x22] == 0x64 && game_data.conquest.raw[0x23] == 0x00 {
        game_data.conquest.raw[0x22] = 0x65;
        game_data.conquest.raw[0x23] = 0x20;
    }

    if game_data.conquest.raw[0x26] == 0x64 {
        game_data.conquest.raw[0x26] = 0x7e;
        game_data.conquest.raw[0x27] = 0x04;
    }

    if game_data.conquest.raw[0x28] == 0x64 && game_data.conquest.raw[0x29] == 0x00 {
        game_data.conquest.raw[0x28] = 0x20;
        game_data.conquest.raw[0x29] = 0x74;
    }

    if game_data.conquest.raw[0x36] == 0x64 {
        game_data.conquest.raw[0x36] = 0x3b;
        game_data.conquest.raw[0x37] = 0x86;
    }

    if game_data.conquest.raw[0x38] == 0x64 && game_data.conquest.raw[0x39] == 0x00 {
        game_data.conquest.raw[0x38] = 0xfe;
        game_data.conquest.raw[0x39] = 0xfc;
    }

    if game_data.conquest.raw[0x3a] == 0x64 && game_data.conquest.raw[0x3b] == 0x00 {
        game_data.conquest.raw[0x3a] = 0x28;
        game_data.conquest.raw[0x3b] = 0x8b;
    }

    for offset in 0x42..=0x54 {
        if game_data.conquest.raw[offset] == 0x01 {
            game_data.conquest.raw[offset] = 0x00;
        }
    }

    if game_data.conquest.raw[0x40] == 0x01 && game_data.conquest.raw[0x41] == 0x01 {
        game_data.conquest.raw[0x40] = 0xFF;
        game_data.conquest.raw[0x41] = 0x00;
    }

    if game_data.conquest.raw[0x44] == 0x00 {
        game_data.conquest.raw[0x44] = 0xc2;
    }

    if game_data.conquest.raw[0x47] == 0x00 && game_data.conquest.raw[0x48] == 0x00 {
        game_data.conquest.raw[0x47] = 0x08;
        game_data.conquest.raw[0x48] = 0x6f;
    }

    if game_data.conquest.raw[0x4a] == 0x00 {
        game_data.conquest.raw[0x4a] = 0x01;
    }
    if game_data.conquest.raw[0x4b] == 0x00 {
        game_data.conquest.raw[0x4b] = 0x6f;
    }

    if game_data.conquest.raw[0x52] == 0x00 && game_data.conquest.raw[0x53] == 0x00 {
        game_data.conquest.raw[0x52] = 0x6a;
        game_data.conquest.raw[0x53] = 0x8d;
    }

    if game_data.conquest.raw[0x54] == 0x00 {
        game_data.conquest.raw[0x54] = 0x35;
    }

    Ok(())
}
