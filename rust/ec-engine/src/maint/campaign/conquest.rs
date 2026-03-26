use crate::CoreGameData;

pub(super) fn process_conquest_header(
    game_data: &mut CoreGameData,
    should_accumulate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if should_accumulate {
        let prod_total = game_data.conquest.raw_word(0x0c);
        let new_prod_total = prod_total.saturating_add(100);
        game_data.conquest.set_raw_word(0x0c, new_prod_total);
        game_data
            .conquest
            .set_raw_byte(0x3d, game_data.conquest.raw_byte(0x3d).saturating_add(1));
    }

    let offsets_to_clear = [
        0x14, 0x16, 0x18, 0x1c, 0x1e, 0x24, 0x2a, 0x2c, 0x2e, 0x30, 0x32, 0x34,
    ];

    for offset in offsets_to_clear {
        game_data.conquest.clear_raw_byte_if_equal(offset, 0x64);
    }

    if game_data.conquest.raw_byte(0x0e) == 0x00 {
        let non_active_prods: Vec<u16> = game_data
            .player
            .records
            .iter()
            .filter(|p| !p.is_active_player())
            .map(|p| p.production_score_raw())
            .collect();

        let mut write_offset = 0x0cusize;
        for prod in non_active_prods.iter().take(3) {
            game_data.conquest.set_raw_word(write_offset, *prod);
            write_offset += 2;
        }
    }

    game_data.conquest.set_raw_word(0x12, 0xFFFF);
    game_data.conquest.set_raw_word(0x1a, 0x3374);

    if game_data.conquest.raw_byte(0x20) == 0x64 {
        game_data.conquest.set_raw_word(0x20, 0x0375);
    }

    if game_data.conquest.raw_word(0x22) == 0x0064 {
        game_data.conquest.set_raw_word(0x22, 0x2065);
    }

    if game_data.conquest.raw_byte(0x26) == 0x64 {
        game_data.conquest.set_raw_word(0x26, 0x047e);
    }

    if game_data.conquest.raw_word(0x28) == 0x0064 {
        game_data.conquest.set_raw_word(0x28, 0x7420);
    }

    if game_data.conquest.raw_byte(0x36) == 0x64 {
        game_data.conquest.set_raw_word(0x36, 0x863b);
    }

    if game_data.conquest.raw_word(0x38) == 0x0064 {
        game_data.conquest.set_raw_word(0x38, 0xfcfe);
    }

    if game_data.conquest.raw_word(0x3a) == 0x0064 {
        game_data.conquest.set_raw_word(0x3a, 0x8b28);
    }

    for offset in 0x42..=0x54 {
        game_data.conquest.clear_raw_byte_if_equal(offset, 0x01);
    }

    if game_data.conquest.raw_word(0x40) == 0x0101 {
        game_data.conquest.set_raw_word(0x40, 0x00FF);
    }

    if game_data.conquest.raw_byte(0x44) == 0x00 {
        game_data.conquest.set_raw_byte(0x44, 0xc2);
    }

    if game_data.conquest.raw_byte(0x47) == 0x00 && game_data.conquest.raw_byte(0x48) == 0x00 {
        game_data.conquest.set_raw_byte(0x47, 0x08);
        game_data.conquest.set_raw_byte(0x48, 0x6f);
    }

    if game_data.conquest.raw_byte(0x4a) == 0x00 {
        game_data.conquest.set_raw_byte(0x4a, 0x01);
    }
    if game_data.conquest.raw_byte(0x4b) == 0x00 {
        game_data.conquest.set_raw_byte(0x4b, 0x6f);
    }

    if game_data.conquest.raw_word(0x52) == 0x0000 {
        game_data.conquest.set_raw_word(0x52, 0x8d6a);
    }

    if game_data.conquest.raw_byte(0x54) == 0x00 {
        game_data.conquest.set_raw_byte(0x54, 0x35);
    }

    Ok(())
}
