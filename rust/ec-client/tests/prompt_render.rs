use ec_client::screen::PlayfieldBuffer;
use ec_client::screen::layout::{
    PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH, draw_command_line_prompt_text, draw_plain_prompt,
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
    for idx in no..no + "<N>".len() {
        assert_eq!(row[idx].style, classic::prompt_hotkey_style());
    }

    let nonstop = find_in_row(&buffer, 19, "<NS>");
    for idx in nonstop..nonstop + "<NS>".len() {
        assert_eq!(row[idx].style, classic::prompt_hotkey_style());
    }
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

    let row = buffer.row(19);
    let choice = find_in_row(&buffer, 19, "[Y]/N");
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
        "List by <C>urrent Production, <L>ocation, <P>otential or <A>bort? [C] -> ",
    );

    let row = buffer.row(19);
    for token in ["<C>", "<L>", "<P>", "<A>"] {
        let start = find_in_row(&buffer, 19, token);
        for idx in start..start + token.len() {
            assert_eq!(row[idx].style, classic::prompt_hotkey_style());
        }
    }

    let default_choice = find_in_row(&buffer, 19, "[C]");
    assert_eq!(row[default_choice].style, classic::prompt_style());
    assert_eq!(
        row[default_choice + 1].style,
        classic::prompt_hotkey_style()
    );
    assert_eq!(row[default_choice + 2].style, classic::prompt_style());
}
