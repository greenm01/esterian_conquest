use nc_ui::buffer::{CellStyle, GameColor, PlayfieldBuffer};
use nc_ui::modal::{ModalTheme, render_modal_box};

fn theme() -> ModalTheme {
    let style = CellStyle::new(GameColor::White, GameColor::Black, false);
    ModalTheme {
        body_style: style,
        pad_style: style,
        chrome_style: style,
        title_style: style,
    }
}

#[test]
fn render_modal_box_keeps_bottom_border_when_content_is_too_tall() {
    let mut buffer = PlayfieldBuffer::new(
        20,
        6,
        CellStyle::new(GameColor::White, GameColor::Black, false),
    );
    let lines = vec![
        "one".to_string(),
        "two".to_string(),
        "three".to_string(),
        "four".to_string(),
        "five".to_string(),
        "six".to_string(),
        "seven".to_string(),
    ];

    let popup = render_modal_box(&mut buffer, "TITLE", &lines, theme());
    let bottom = popup.y as usize + popup.height as usize - 1;
    let line = buffer.plain_line(bottom);

    assert!(line.contains('└'));
    assert!(line.contains('┘'));
    assert!(!line.contains("seven"));
}
