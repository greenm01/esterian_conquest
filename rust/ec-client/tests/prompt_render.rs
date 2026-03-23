use ec_client::screen::PlayfieldBuffer;
use ec_client::screen::layout::{
    COMMAND_LINE_ROW, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, draw_command_line_prompt_text,
    draw_command_prompt, draw_plain_prompt, draw_table_command_prompt,
};
use ec_client::theme::classic;

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
    draw_command_line_prompt_text(
        &mut buffer,
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
    draw_command_prompt(&mut buffer, 19, "GENERAL COMMAND", "SLAP A KEY");

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
