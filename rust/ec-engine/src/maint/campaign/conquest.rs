use crate::CoreGameData;

pub(super) fn process_conquest_header(
    game_data: &mut CoreGameData,
    should_accumulate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if should_accumulate {
        let prod_total = game_data
            .conquest
            .inactive_production_slot_raw(0)
            .unwrap_or_default();
        let new_prod_total = prod_total.saturating_add(100);
        game_data
            .conquest
            .set_inactive_production_slot_raw(0, new_prod_total);
        game_data
            .conquest
            .set_control_byte_3d_raw(game_data.conquest.control_byte_3d_raw().saturating_add(1));
    }

    let offsets_to_clear = [
        0x14, 0x16, 0x18, 0x1c, 0x1e, 0x24, 0x2a, 0x2c, 0x2e, 0x30, 0x32, 0x34,
    ];

    for offset in offsets_to_clear {
        game_data.conquest.clear_control_byte_if_equal(offset, 0x64);
    }

    if game_data.conquest.inactive_production_slot_raw(1) == Some(0) {
        let non_active_prods: Vec<u16> = game_data
            .player
            .records
            .iter()
            .filter(|p| !p.is_active_player())
            .map(|p| p.production_score_raw())
            .collect();

        for (slot, prod) in non_active_prods.iter().take(3).enumerate() {
            game_data
                .conquest
                .set_inactive_production_slot_raw(slot, *prod);
        }
    }

    game_data.conquest.set_control_word_12_raw(0xFFFF);
    game_data.conquest.set_control_word_1a_raw(0x3374);

    if game_data.conquest.control_word_20_raw() == 0x0064 {
        game_data.conquest.set_control_word_20_raw(0x0375);
    }

    if game_data.conquest.control_word_22_raw() == 0x0064 {
        game_data.conquest.set_control_word_22_raw(0x2065);
    }

    if game_data.conquest.control_word_26_raw() == 0x0064 {
        game_data.conquest.set_control_word_26_raw(0x047e);
    }

    if game_data.conquest.control_word_28_raw() == 0x0064 {
        game_data.conquest.set_control_word_28_raw(0x7420);
    }

    if game_data.conquest.control_word_36_raw() == 0x0064 {
        game_data.conquest.set_control_word_36_raw(0x863b);
    }

    if game_data.conquest.control_word_38_raw() == 0x0064 {
        game_data.conquest.set_control_word_38_raw(0xfcfe);
    }

    if game_data.conquest.control_word_3a_raw() == 0x0064 {
        game_data.conquest.set_control_word_3a_raw(0x8b28);
    }

    for offset in 0x42..=0x54 {
        game_data.conquest.clear_control_byte_if_equal(offset, 0x01);
    }

    if game_data.conquest.control_word_40_raw() == 0x0101 {
        game_data.conquest.set_control_word_40_raw(0x00FF);
    }

    if game_data.conquest.control_byte_44_raw() == 0x00 {
        game_data.conquest.set_control_byte_44_raw(0xc2);
    }

    if game_data.conquest.control_byte_47_raw() == 0x00
        && game_data.conquest.control_byte_48_raw() == 0x00
    {
        game_data.conquest.set_control_byte_47_raw(0x08);
        game_data.conquest.set_control_byte_48_raw(0x6f);
    }

    if game_data.conquest.control_byte_4a_raw() == 0x00 {
        game_data.conquest.set_control_byte_4a_raw(0x01);
    }
    if game_data.conquest.control_byte_4b_raw() == 0x00 {
        game_data.conquest.set_control_byte_4b_raw(0x6f);
    }

    if game_data.conquest.control_word_52_raw() == 0x0000 {
        game_data.conquest.set_control_word_52_raw(0x8d6a);
    }

    if game_data.conquest.control_byte_54_raw() == 0x00 {
        game_data.conquest.set_control_byte_54_raw(0x35);
    }

    Ok(())
}
