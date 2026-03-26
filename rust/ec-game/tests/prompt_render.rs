use std::fs;
use std::path::{Path, PathBuf};

use ec_data::{EmpirePlanetEconomyRow, ProductionItemKind};
use ec_game::screen::MessageComposeScreen;
use ec_game::screen::PlanetBuildOrder;
use ec_game::screen::PlanetBuildScreen;
use ec_game::screen::PlanetCommissionDraftRow;
use ec_game::screen::PlanetCommissionPickerRow;
use ec_game::screen::PlanetCommissionScreen;
use ec_game::screen::PlanetMenuScreen;
use ec_game::screen::PlayfieldBuffer;
use ec_game::screen::layout::{
    COMMAND_LINE_ROW, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, PromptFeedback, dismiss_prompt_row,
    draw_bottom_aligned_transcript_rows, draw_command_line_default_input_at,
    draw_command_line_prompt_text_at, draw_command_prompt_at, draw_help_panel,
    draw_inline_delete_reviewables_prompt, draw_inline_planet_info_prompt, draw_plain_prompt,
    draw_prompt_error_after, draw_prompt_feedback_after, draw_table_command_prompt,
    table_dismiss_prompt_row,
};
use ec_game::theme::classic;

fn row_text(buffer: &PlayfieldBuffer, row: usize) -> String {
    buffer.row(row).iter().map(|cell| cell.ch).collect()
}

fn find_in_row(buffer: &PlayfieldBuffer, row: usize, needle: &str) -> usize {
    row_text(buffer, row)
        .find(needle)
        .unwrap_or_else(|| panic!("expected to find {needle:?} in row {}", row))
}

#[test]
fn draw_plain_prompt_highlights_square_and_angle_hotkeys() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_plain_prompt(
        &mut buffer,
        19,
        "There are more reports. Continue? [Y]es, <N>o, <NS> (non-stop) ->",
    );

    let row = buffer.row(19);
    let bracket = find_in_row(&buffer, 19, "[Y]");
    assert_eq!(row[bracket].style, classic::prompt_style());
    assert_eq!(row[bracket + 1].style, classic::prompt_hotkey_style());
    assert_eq!(row[bracket + 2].style, classic::prompt_style());

    let no = find_in_row(&buffer, 19, "<N>");
    assert_eq!(row[no].style, classic::prompt_style());
    assert_eq!(row[no + 1].style, classic::prompt_hotkey_style());
    assert_eq!(row[no + 2].style, classic::prompt_style());

    let nonstop = find_in_row(&buffer, 19, "<NS>");
    assert_eq!(row[nonstop].style, classic::prompt_style());
    assert_eq!(row[nonstop + 1].style, classic::prompt_hotkey_style());
    assert_eq!(row[nonstop + 2].style, classic::prompt_hotkey_style());
    assert_eq!(row[nonstop + 3].style, classic::prompt_style());
}

#[test]
fn draw_plain_prompt_highlights_bare_slash_separated_choices() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_plain_prompt(&mut buffer, 19, "Delete this report Y/[N] ->");

    let row = buffer.row(19);
    let choice = find_in_row(&buffer, 19, "Y/[N]");
    assert_eq!(row[choice].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 1].style, classic::prompt_style());
    assert_eq!(row[choice + 2].style, classic::prompt_style());
    assert_eq!(row[choice + 3].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 4].style, classic::prompt_style());
}

#[test]
fn draw_command_line_prompt_text_highlights_confirm_prompt_hotkeys() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_line_prompt_text_at(
        &mut buffer,
        COMMAND_LINE_ROW,
        "WORLD NAME",
        "\"Aurora\" <- Is this correct? [Y]/N ->",
    );

    let row = buffer.row(COMMAND_LINE_ROW);
    let choice = find_in_row(&buffer, COMMAND_LINE_ROW, "[Y]/N");
    assert_eq!(row[choice].style, classic::prompt_style());
    assert_eq!(row[choice + 1].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 2].style, classic::prompt_style());
    assert_eq!(row[choice + 3].style, classic::prompt_style());
    assert_eq!(row[choice + 4].style, classic::prompt_hotkey_style());
}

#[test]
fn draw_plain_prompt_highlights_general_letter_commands() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_plain_prompt(
        &mut buffer,
        19,
        "Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->",
    );

    let row = buffer.row(19);
    for token in ["<C>", "<L>", "<M>", "<Q>"] {
        let start = find_in_row(&buffer, 19, token);
        assert_eq!(row[start].style, classic::prompt_style());
        assert_eq!(row[start + 1].style, classic::prompt_hotkey_style());
        assert_eq!(row[start + 2].style, classic::prompt_style());
    }

    let default_choice = find_in_row(&buffer, 19, "[C]");
    assert_eq!(row[default_choice].style, classic::prompt_style());
    assert_eq!(
        row[default_choice + 1].style,
        classic::prompt_hotkey_style()
    );
    assert_eq!(row[default_choice + 2].style, classic::prompt_style());
}

#[test]
fn draw_command_line_default_input_highlights_prompt_choices_and_default() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_line_default_input_at(
        &mut buffer,
        COMMAND_LINE_ROW,
        "FLEET COMMAND",
        "Change <R>OE, <I>D, or <S>peed ",
        "R",
        "",
    );

    let row = buffer.row(COMMAND_LINE_ROW);
    for token in ["<R>", "<I>", "<S>"] {
        let start = find_in_row(&buffer, COMMAND_LINE_ROW, token);
        assert_eq!(row[start].style, classic::prompt_style());
        assert_eq!(row[start + 1].style, classic::prompt_hotkey_style());
        assert_eq!(row[start + 2].style, classic::prompt_style());
    }

    let default_choice = find_in_row(&buffer, COMMAND_LINE_ROW, "[R]");
    assert_eq!(row[default_choice].style, classic::prompt_style());
    assert_eq!(
        row[default_choice + 1].style,
        classic::prompt_hotkey_style()
    );
    assert_eq!(row[default_choice + 2].style, classic::prompt_style());

    let quit = find_in_row(&buffer, COMMAND_LINE_ROW, "<Q>");
    assert_eq!(row[quit].style, classic::prompt_style());
    assert_eq!(row[quit + 1].style, classic::prompt_hotkey_style());
    assert_eq!(row[quit + 2].style, classic::prompt_style());
}

#[test]
fn draw_prompt_feedback_after_renders_notice_hanger() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_line_default_input_at(
        &mut buffer,
        COMMAND_LINE_ROW,
        "FLEET COMMAND",
        "Order Fleet # ",
        "2",
        "",
    );
    draw_prompt_feedback_after(
        &mut buffer,
        COMMAND_LINE_ROW,
        &PromptFeedback::notice("Applied move to Fleet #2 for sector [14,9]."),
    );

    assert!((0..PLAYFIELD_HEIGHT).any(|row| {
        row_text(&buffer, row).contains("Notice: Applied move to Fleet #2 for sector [14,9].")
    }));
}

#[test]
fn draw_table_command_prompt_keeps_cursor_inside_playfield_and_highlights_default() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_table_command_prompt(
        &mut buffer,
        "Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->",
    );

    let line = row_text(&buffer, COMMAND_LINE_ROW);
    assert!(line.trim_end().len() < PLAYFIELD_WIDTH);
    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    assert_eq!(cursor_row as usize, COMMAND_LINE_ROW);
    assert!((cursor_col as usize) < PLAYFIELD_WIDTH);

    let row = buffer.row(COMMAND_LINE_ROW);
    let default_choice = find_in_row(&buffer, COMMAND_LINE_ROW, "[C]");
    assert_eq!(
        row[default_choice + 1].style,
        classic::prompt_hotkey_style()
    );
}

#[test]
fn draw_table_command_prompt_inserts_space_after_arrow_before_cursor() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_table_command_prompt(
        &mut buffer,
        "Sort by <C>urrent Prod, <L>ocation, <M>ax, or <Q>uit? [C] ->",
    );

    let line = row_text(&buffer, COMMAND_LINE_ROW);
    assert!(line.contains("[C] -> "));
    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    assert_eq!(cursor_row as usize, COMMAND_LINE_ROW);
    assert_eq!(line.as_bytes()[cursor_col as usize - 1], b' ');
}

#[test]
fn draw_plain_prompt_highlights_command_rail_inside_angle_brackets() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_plain_prompt(&mut buffer, 19, "COMMAND <ARROWS J K S Q> [03,03] ->");

    let row = buffer.row(19);
    let rail = find_in_row(&buffer, 19, "<ARROWS J K S Q>");
    assert_eq!(row[rail].style, classic::prompt_style());
    for idx in rail + 1..rail + "<ARROWS J K S Q>".len() - 1 {
        assert_eq!(row[idx].style, classic::prompt_hotkey_style());
    }
    assert_eq!(
        row[rail + "<ARROWS J K S Q>".len() - 1].style,
        classic::prompt_style()
    );
}

#[test]
fn draw_plain_prompt_highlights_key_in_slap_a_key_phrase() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_plain_prompt(&mut buffer, 19, "(Slap a key for more)");

    let row = buffer.row(19);
    let phrase = find_in_row(&buffer, 19, "Slap a key");
    assert_eq!(row[phrase].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 1].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 2].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 3].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 4].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 5].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 6].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 7].style, classic::prompt_hotkey_style());
    assert_eq!(row[phrase + 8].style, classic::prompt_hotkey_style());
    assert_eq!(row[phrase + 9].style, classic::prompt_hotkey_style());
}

#[test]
fn draw_command_prompt_highlights_key_in_slap_a_key_phrase() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_prompt_at(
        &mut buffer,
        COMMAND_LINE_ROW,
        "GENERAL COMMAND",
        "SLAP A KEY",
    );

    let row = buffer.row(COMMAND_LINE_ROW);
    let phrase = find_in_row(&buffer, COMMAND_LINE_ROW, "slap a key");
    assert_eq!(row[phrase].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 1].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 2].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 3].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 4].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 5].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 6].style, classic::prompt_notice_action_style());
    assert_eq!(row[phrase + 7].style, classic::prompt_hotkey_style());
    assert_eq!(row[phrase + 8].style, classic::prompt_hotkey_style());
    assert_eq!(row[phrase + 9].style, classic::prompt_hotkey_style());
}

#[test]
fn draw_command_prompt_places_cursor_after_arrow_space() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_prompt_at(&mut buffer, COMMAND_LINE_ROW, "GENERAL COMMAND", "H,Q,X");

    let line = row_text(&buffer, COMMAND_LINE_ROW);
    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    assert_eq!(cursor_row as usize, COMMAND_LINE_ROW);
    assert_eq!(line.as_bytes()[cursor_col as usize - 1], b' ');
    assert!(line.contains("GENERAL COMMAND <-H,Q,X-> "));
}

#[test]
fn draw_command_prompt_places_cursor_after_slap_a_key_arrow() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_command_prompt_at(
        &mut buffer,
        COMMAND_LINE_ROW,
        "GENERAL COMMAND",
        "SLAP A KEY",
    );

    let line = row_text(&buffer, COMMAND_LINE_ROW);
    let (cursor_col, cursor_row) = buffer.cursor().expect("cursor set");
    assert_eq!(cursor_row as usize, COMMAND_LINE_ROW);
    assert_eq!(line.as_bytes()[cursor_col as usize - 1], b' ');
    assert!(line.contains("(slap a key)-> "));
}

#[test]
fn compose_subject_prompt_renders_below_recipient_with_single_blank_row() {
    let mut screen = MessageComposeScreen::new();
    let buffer = screen
        .render_subject("Empire 9 (Viridian Chain)", "", None)
        .expect("subject prompt renders");

    assert!(row_text(&buffer, 2).contains("To: Empire 9 (Viridian Chain)"));
    assert_eq!(row_text(&buffer, 3).trim_end(), "");
    assert!(row_text(&buffer, 4).contains("COMMAND <- Message subject <Q> -> "));
    assert_eq!(row_text(&buffer, COMMAND_LINE_ROW).trim_end(), "");
}

#[test]
fn compose_body_soft_wraps_at_spaces_instead_of_splitting_words() {
    let mut screen = MessageComposeScreen::new();
    let body = format!("{} splitword", "a".repeat(70));
    let buffer = screen
        .render_body("Empire 2 (Red Horizon Pact)", "test", &body, 0, 0, None)
        .expect("body prompt renders");

    assert_eq!(row_text(&buffer, 5).trim_end(), "a".repeat(70));
    assert!(row_text(&buffer, 6).starts_with("splitword"));
}

#[test]
fn compose_body_uses_full_80x25_vertical_editor_space() {
    let mut screen = MessageComposeScreen::new();
    let body = (1..=20)
        .map(|idx| format!("line {idx:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let buffer = screen
        .render_body("Empire 2 (Red Horizon Pact)", "test", &body, 0, 0, None)
        .expect("body prompt renders");
    assert!(row_text(&buffer, 20).contains("line 16"));
    assert!(row_text(&buffer, 21).trim().is_empty());
    assert!(row_text(&buffer, 22).contains("Chars:"));
    assert!(row_text(&buffer, 23).trim().is_empty());
    assert!(row_text(&buffer, 24).contains("GENERAL COMMAND <-CTRL-E CTRL-X->"));
}

#[test]
fn compose_discard_confirm_uses_default_no_prompt_markup() {
    let mut screen = MessageComposeScreen::new();
    let buffer = screen
        .render_discard_confirm("Empire 2 (Red Horizon Pact)", "test", "hello")
        .expect("discard confirm renders");

    assert!(!row_text(&buffer, 20).contains("Discard this unsent message draft?"));
    assert!(row_text(&buffer, 21).trim().is_empty());
    assert!(row_text(&buffer, 24).contains("CANCEL MESSAGE <- Y/[N] ->"));
    let row = buffer.row(24);
    let choice = find_in_row(&buffer, 24, "Y/[N]");
    assert_eq!(row[choice].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 1].style, classic::prompt_style());
    assert_eq!(row[choice + 2].style, classic::prompt_style());
    assert_eq!(row[choice + 3].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 4].style, classic::prompt_style());
}

#[test]
fn compose_send_confirm_uses_default_no_prompt_markup() {
    let mut screen = MessageComposeScreen::new();
    let buffer = screen
        .render_send_confirm("Empire 2 (Red Horizon Pact)", "test", "hello")
        .expect("send confirm renders");

    assert!(!row_text(&buffer, 20).contains("Send this message after turn maintenance?"));
    assert!(row_text(&buffer, 21).trim().is_empty());
    assert!(row_text(&buffer, 24).contains("SEND MESSAGE <- Y/[N] ->"));
    let row = buffer.row(24);
    let choice = find_in_row(&buffer, 24, "Y/[N]");
    assert_eq!(row[choice].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 1].style, classic::prompt_style());
    assert_eq!(row[choice + 2].style, classic::prompt_style());
    assert_eq!(row[choice + 3].style, classic::prompt_hotkey_style());
    assert_eq!(row[choice + 4].style, classic::prompt_style());
}

#[test]
fn inline_planet_info_prompt_zero_pads_default_coords() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_inline_planet_info_prompt(&mut buffer, COMMAND_LINE_ROW, [3, 3], "", None, None);

    let line = row_text(&buffer, COMMAND_LINE_ROW);
    assert!(line.contains("COMMAND <- Planet coords [03,03] <Q> -> "));
}

#[test]
fn inline_delete_reviewables_prompt_uses_notice_style_and_cursor_gap() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_inline_delete_reviewables_prompt(&mut buffer, 10, None);

    assert!(row_text(&buffer, 10).contains("COMMAND <- Y/[N] -> "));
    assert_eq!(buffer.cursor().expect("cursor set"), (20u16, 10u16),);
    let prompt_row = buffer.row(10);
    let choice = find_in_row(&buffer, 10, "Y/[N]");
    assert_eq!(prompt_row[choice].style, classic::prompt_hotkey_style());
    assert_eq!(prompt_row[choice + 1].style, classic::prompt_style());
    assert_eq!(prompt_row[choice + 2].style, classic::prompt_style());
    assert_eq!(prompt_row[choice + 3].style, classic::prompt_hotkey_style());
    assert_eq!(prompt_row[choice + 4].style, classic::prompt_style());

    let title = "DELETE ALL MESSAGES / RESULTS:";
    let title_col = find_in_row(&buffer, 12, title);
    let row = buffer.row(12);
    assert_eq!(row[title_col].style, classic::notice_style());
    assert!(
        row_text(&buffer, 13)
            .contains("This will clear all currently reviewable messages and results.")
    );
}

#[test]
fn planet_menu_inline_auto_commission_uses_standard_confirm_layout() {
    let mut screen = PlanetMenuScreen::new();
    let buffer = screen
        .render_with_notice(
            None,
            false,
            false,
            [0, 0],
            "",
            None,
            false,
            "0",
            "",
            None,
            None,
            true,
            None,
            &[],
            None,
            "",
            "",
            None,
            None,
            None,
        )
        .expect("planet menu inline auto-commission renders");

    assert!(row_text(&buffer, 5).contains("COMMAND <- Y/[N] -> "));
    assert!(row_text(&buffer, 6).trim().is_empty());
    assert!(row_text(&buffer, 7).contains("AUTO-COMMISSION SHIPS:"));
    assert!(
        row_text(&buffer, 8)
            .contains("Automatically commission all ships and starbases in stardock?")
    );
}

#[test]
fn build_menu_inline_abort_uses_standard_confirm_layout() {
    let mut screen = PlanetBuildScreen::new();
    let view = ec_game::screen::PlanetBuildMenuView {
        row: EmpirePlanetEconomyRow {
            planet_record_index_1_based: 1,
            coords: [6, 5],
            planet_name: "Not Named Yet".to_string(),
            present_production: 100,
            potential_production: 100,
            stored_production_points: 50,
            yearly_tax_revenue: 50,
            yearly_growth_delta: 0,
            build_capacity: 100,
            has_friendly_starbase: false,
            armies: 10,
            ground_batteries: 4,
            is_homeworld_seed: true,
        },
        committed_points: 50,
        available_points: 50,
        points_left: 0,
        queue_used: 2,
        queue_capacity: 10,
        stardock_used: 3,
        stardock_capacity: 10,
    };
    let orders = vec![
        PlanetBuildOrder {
            kind: ProductionItemKind::Etac,
            points_remaining: 20,
        },
        PlanetBuildOrder {
            kind: ProductionItemKind::Etac,
            points_remaining: 20,
        },
        PlanetBuildOrder {
            kind: ProductionItemKind::Destroyer,
            points_remaining: 10,
        },
    ];

    let buffer = screen
        .render_menu(&view, &orders, None, false, false, [0, 0], "", None, true)
        .expect("build menu inline abort renders");

    assert!(row_text(&buffer, 7).contains("COMMAND <- Y/[N] -> "));
    assert!(row_text(&buffer, 8).trim().is_empty());
    assert!(row_text(&buffer, 9).contains("ABORT BUILD ORDERS:"));
    assert!(row_text(&buffer, 10).contains("Queued orders to be cancelled:"));
    assert!(row_text(&buffer, 11).contains("2 ETACs (40 pts)"));
    assert!(row_text(&buffer, 12).contains("2 Destroyers (10 pts)"));
    assert!(row_text(&buffer, 13).contains("All 50 committed points will be fully refunded."));
}

#[test]
fn commission_picker_renders_planets_with_stardock_counts() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        PlanetCommissionPickerRow {
            coords: [8, 9],
            planet_name: "Aurora Prime".to_string(),
            destroyers: 4,
            cruisers: 2,
            battleships: 0,
            scouts: 0,
            troop_transports: 3,
            etacs: 0,
            starbases: 1,
        },
        PlanetCommissionPickerRow {
            coords: [11, 22],
            planet_name: "Cobalt Rise".to_string(),
            destroyers: 0,
            cruisers: 0,
            battleships: 1,
            scouts: 0,
            troop_transports: 0,
            etacs: 1,
            starbases: 0,
        },
    ];

    let buffer = screen
        .render_picker(&rows, 0, 0)
        .expect("commission picker renders");

    assert!(row_text(&buffer, 1).trim().is_empty());
    assert!(row_text(&buffer, 2).starts_with("┌"));
    assert!(row_text(&buffer, 3).contains("Planet Name"));
    assert!(row_text(&buffer, 3).contains("ET"));
    assert!(row_text(&buffer, 5).contains("(08,09)"));
    assert!(row_text(&buffer, 5).contains("Aurora Prime"));
    assert!(row_text(&buffer, 5).contains("│04│02│"));
    assert!(row_text(&buffer, 5).contains("│03│"));
    assert!(row_text(&buffer, 5).contains("│01│"));
    assert!(row_text(&buffer, 6).contains("│01│"));
}

#[test]
fn commission_draft_starts_table_under_title_and_defaults_ship_prompt_to_remaining_qty() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            remaining_qty: 4,
            fleet_qty: 0,
        },
        PlanetCommissionDraftRow {
            direct_slot_0_based: Some(5),
            kind: ProductionItemKind::Starbase,
            unit_label: "Starbases".to_string(),
            remaining_qty: 1,
            fleet_qty: 0,
        },
    ];

    let buffer = screen
        .render_draft(
            "DRAFT COMMISSION FLEET: \"Aurora Prime\" IN SYSTEM [08,09]:",
            &rows,
            0,
            "",
            None,
            None,
        )
        .expect("commission draft renders");

    assert!(row_text(&buffer, 1).trim().is_empty());
    assert!(row_text(&buffer, 2).starts_with("┌"));
    assert!(row_text(&buffer, 5).contains("│Destroyers"));
    assert!(row_text(&buffer, 5).contains("│       04│         00│"));
    assert!(row_text(&buffer, 6).contains("│Starbases"));
    assert!(row_text(&buffer, 6).contains("│       01│           │"));
    assert!(row_text(&buffer, 8).contains("COMMAND <- Qty for Destroyers [04] <Q> ->"));
    assert!(row_text(&buffer, 9).trim().is_empty());
    assert!(row_text(&buffer, 10).contains("Set quantities for the ships you want in this fleet."));
}

#[test]
fn commission_draft_switches_prompt_for_starbase_rows() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            remaining_qty: 4,
            fleet_qty: 0,
        },
        PlanetCommissionDraftRow {
            direct_slot_0_based: Some(5),
            kind: ProductionItemKind::Starbase,
            unit_label: "Starbases".to_string(),
            remaining_qty: 1,
            fleet_qty: 0,
        },
    ];

    let buffer = screen
        .render_draft(
            "DRAFT COMMISSION FLEET: \"Aurora Prime\" IN SYSTEM [08,09]:",
            &rows,
            1,
            "",
            None,
            None,
        )
        .expect("commission draft renders");

    assert!(
        row_text(&buffer, 8)
            .contains("COMMAND <- <ENTER> commissions the highlighted starbase. <Q> -> ")
    );
    assert!(
        row_text(&buffer, 10)
            .contains("ENTER commissions the highlighted starbase directly to the planet.")
    );
}

#[test]
fn commission_draft_renders_inline_notice_below_command_row() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Battleship,
            unit_label: "Battleships".to_string(),
            remaining_qty: 3,
            fleet_qty: 2,
        },
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            remaining_qty: 5,
            fleet_qty: 5,
        },
    ];

    let buffer = screen
        .render_draft(
            "DRAFT COMMISSION FLEET: \"Aurora Prime\" IN SYSTEM [08,09]:",
            &rows,
            0,
            "",
            None,
            Some("Commissioned selected ships into Fleet 02."),
        )
        .expect("commission draft renders");

    assert!(row_text(&buffer, 2).starts_with("┌"));
    assert!(row_text(&buffer, 8).contains("COMMAND <- Qty for Battleships [03] <Q> ->"));
    assert!(row_text(&buffer, 9).trim().is_empty());
    assert!(row_text(&buffer, 10).contains("ENTER commissions the current fleet draft."));
    assert!(row_text(&buffer, 11).trim().is_empty());
    assert!(row_text(&buffer, 12).contains("Notice: Commissioned selected ships into Fleet 02."));
}

#[test]
fn commission_draft_zero_pads_live_input_in_this_fleet_column() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Destroyer,
            unit_label: "Destroyers".to_string(),
            remaining_qty: 4,
            fleet_qty: 1,
        },
        PlanetCommissionDraftRow {
            direct_slot_0_based: None,
            kind: ProductionItemKind::Cruiser,
            unit_label: "Cruisers".to_string(),
            remaining_qty: 2,
            fleet_qty: 2,
        },
    ];

    let buffer = screen
        .render_draft(
            "DRAFT COMMISSION FLEET: \"Aurora Prime\" IN SYSTEM [08,09]:",
            &rows,
            0,
            "3",
            None,
            None,
        )
        .expect("commission draft renders");

    assert!(row_text(&buffer, 5).contains("│Destroyers              │       04│         03│"));
}

#[test]
fn commission_result_renders_notice_with_dismiss_prompt() {
    let mut screen = PlanetCommissionScreen::new();

    let buffer = screen
        .render_result(
            "DRAFT COMMISSION FLEET: \"Aurora Prime\" IN SYSTEM [08,09]:",
            "Commissioned selected ships into Fleet 02.",
        )
        .expect("commission result renders");

    assert!(row_text(&buffer, 0).contains("DRAFT COMMISSION FLEET:"));
    assert!(row_text(&buffer, 2).contains("Notice: Commissioned selected ships into Fleet 02."));
    assert!(row_text(&buffer, 3).trim().is_empty());
    assert_eq!(row_text(&buffer, 4).trim_end(), "(slap a key)");
}

#[test]
fn auto_commission_report_bottom_aligns_text_and_leaves_blank_row_above_prompt() {
    let mut screen = PlanetCommissionScreen::new();
    let rows = vec![
        "Fleet 03 commissioned from \"Aurora Prime\" in sector (08,09) with DD 04, CA 02."
            .to_string(),
    ];

    let buffer = screen
        .render_auto_commission_report(&rows, rows.len())
        .expect("auto commission report renders");

    assert!(row_text(&buffer, 22).contains("Fleet 03 commissioned from"));
    assert!(row_text(&buffer, 23).trim().is_empty());
    assert_eq!(row_text(&buffer, 24).trim_end(), "(slap a key)");

    let row = buffer.row(22);
    let fleet_digits = find_in_row(&buffer, 22, "03 commissioned");
    assert_eq!(row[fleet_digits].style, classic::status_value_style());
    assert_eq!(row[fleet_digits + 1].style, classic::status_value_style());

    let coords = find_in_row(&buffer, 22, "(08,09)");
    assert_eq!(row[coords + 1].style, classic::status_value_style());
    assert_eq!(row[coords + 2].style, classic::status_value_style());
    assert_eq!(row[coords + 4].style, classic::status_value_style());
    assert_eq!(row[coords + 5].style, classic::status_value_style());
}

#[test]
fn bottom_aligned_transcript_rows_can_fill_content_through_row_22() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    let rows = (1..=23)
        .map(|idx| format!("line {idx:02}"))
        .collect::<Vec<_>>();

    draw_bottom_aligned_transcript_rows(
        &mut buffer,
        &rows,
        rows.len(),
        0,
        22,
        |buffer, row, line| {
            buffer.write_text(row, 0, line, classic::body_style());
        },
    );

    assert!(row_text(&buffer, 0).contains("line 01"));
    assert!(row_text(&buffer, 22).contains("line 23"));
}

#[test]
fn draw_prompt_error_after_places_error_hanger_two_rows_below_command_row() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    draw_prompt_error_after(&mut buffer, 10, "No fleets are ready.");

    assert!(row_text(&buffer, 11).trim().is_empty());
    assert!(row_text(&buffer, 12).contains("Error: "));
    assert!(row_text(&buffer, 12).contains("No fleets are ready."));
}

#[test]
#[should_panic(expected = "write_text overflow")]
fn playfield_write_text_panics_when_text_overflows_row() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    buffer.write_text(0, PLAYFIELD_WIDTH - 1, "AB", classic::body_style());
}

#[test]
#[should_panic(expected = "cursor column")]
fn playfield_set_cursor_panics_when_cursor_is_out_of_bounds() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    buffer.set_cursor(PLAYFIELD_WIDTH as u16, 0);
}

#[test]
fn source_tree_does_not_use_removed_inline_status_helper() {
    let src_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src_root];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).expect("read source dir") {
            let entry = entry.expect("read source entry");
            let entry_path = entry.path();
            if entry_path.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if entry_path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let contents = fs::read_to_string(&entry_path).expect("read source file");
            assert!(
                !contents.contains("draw_inline_status_after("),
                "old inline status helper still used in {}",
                display_rel(&entry_path)
            );
        }
    }
}

fn display_rel(path: &Path) -> String {
    path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn dismiss_prompt_row_leaves_one_blank_row_above_prompt() {
    assert_eq!(dismiss_prompt_row(16), 18);
    assert_eq!(dismiss_prompt_row(0), 2);
}

#[test]
fn table_dismiss_prompt_row_attaches_prompt_to_table_bottom() {
    assert_eq!(table_dismiss_prompt_row(10), 11);
    assert_eq!(table_dismiss_prompt_row(23), 24);
}

#[test]
fn help_panel_reserves_one_blank_row_above_dismiss_prompt() {
    let mut buffer = PlayfieldBuffer::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT, classic::body_style());
    let lines = vec!["line"; 40];
    draw_help_panel(&mut buffer, "HELP:", "Header", &lines, "GENERAL COMMAND");

    assert!(!row_text(&buffer, COMMAND_LINE_ROW - 2).trim().is_empty());
    assert!(row_text(&buffer, COMMAND_LINE_ROW - 1).trim().is_empty());
    assert!(row_text(&buffer, COMMAND_LINE_ROW).contains("(slap a key)"));
}
