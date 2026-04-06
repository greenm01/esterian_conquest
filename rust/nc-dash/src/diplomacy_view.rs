use nc_data::{DiplomaticRelation, PlayerRecord};
use nc_ui::{CellStyle, GameColor};

use crate::theme;

pub(crate) fn display_name(player: &PlayerRecord, empire_slot: u8) -> String {
    let name = player.controlled_empire_name_summary();
    let fallback = player.legacy_status_name_summary();
    if !name.is_empty() {
        name
    } else if !fallback.is_empty() {
        fallback
    } else {
        format!("Empire #{empire_slot}")
    }
}

pub(crate) fn state_label_and_style(
    player: &PlayerRecord,
    viewer_slot: u8,
    empire_slot: u8,
) -> (&'static str, CellStyle) {
    if player.is_civil_disorder_player() {
        ("Civil Dis", theme::icd_style())
    } else if player.is_rogue_player() {
        ("Rogue", theme::alert_style())
    } else if empire_slot == viewer_slot {
        ("(you)", theme::friendly_style())
    } else {
        ("Stable", theme::dim_style())
    }
}

pub(crate) fn relation_label_and_style(
    viewer: Option<&PlayerRecord>,
    viewer_slot: u8,
    empire_slot: u8,
) -> (&'static str, CellStyle) {
    if empire_slot == viewer_slot {
        ("—", theme::dim_style())
    } else if viewer.and_then(|row| row.diplomatic_relation_toward(empire_slot))
        == Some(DiplomaticRelation::Enemy)
    {
        ("Enemy", theme::enemy_style())
    } else {
        ("Neutral", theme::dim_style())
    }
}

pub(crate) fn panel_status_label_and_style(
    player: &PlayerRecord,
    viewer: Option<&PlayerRecord>,
    viewer_slot: u8,
    empire_slot: u8,
) -> (&'static str, CellStyle) {
    if player.is_civil_disorder_player() {
        ("ICD", theme::icd_style())
    } else if player.is_rogue_player() {
        ("Rogue", theme::alert_style())
    } else if empire_slot == viewer_slot {
        ("You", theme::friendly_style())
    } else if viewer.and_then(|row| row.diplomatic_relation_toward(empire_slot))
        == Some(DiplomaticRelation::Enemy)
    {
        ("Enemy", theme::enemy_style())
    } else {
        ("Neut", theme::dim_style())
    }
}

pub(crate) fn empire_name_style(empire_slot: u8, bg: GameColor, bold: bool) -> CellStyle {
    theme::empire_slot_style_on(empire_slot, bg, bold)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nc_ui::GameColor;

    #[test]
    fn blank_display_name_falls_back_to_empire_slot() {
        let player = PlayerRecord::new_zeroed();
        assert_eq!(display_name(&player, 4), "Empire #4");
    }

    #[test]
    fn empire_name_style_uses_theme_slot_color() {
        let style = empire_name_style(3, GameColor::Black, false);
        assert_eq!(style.fg, theme::empire_slot_color(3));
        assert_eq!(style.bg, GameColor::Black);
    }
}
