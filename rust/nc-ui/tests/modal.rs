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

#[test]
fn render_modal_box_wraps_long_lines_inside_the_border() {
    let mut buffer = PlayfieldBuffer::new(
        32,
        12,
        CellStyle::new(GameColor::White, GameColor::Black, false),
    );
    let lines = vec![String::from(
        "Message : This is a deliberately long status line that must stay inside the dialog box",
    )];

    let popup = render_modal_box(&mut buffer, "TITLE", &lines, theme());
    let left = popup.x as usize;
    let right = left + popup.width as usize - 1;
    let top = popup.y as usize;
    let bottom = top + popup.height as usize - 1;

    assert!(buffer.plain_line(top + 1).contains("Message : This is"));
    assert!(buffer.plain_line(top + 2).contains("          "));
    for row in top..=bottom {
        assert!(buffer.row(row)[..left].iter().all(|cell| cell.ch == ' '));
        assert!(
            buffer.row(row)[right + 1..]
                .iter()
                .all(|cell| cell.ch == ' ')
        );
    }
}

#[test]
#[should_panic(expected = "modal title overruns its border slot")]
fn render_modal_box_panics_when_title_does_not_fit_border_slot() {
    let mut buffer = PlayfieldBuffer::new(
        16,
        6,
        CellStyle::new(GameColor::White, GameColor::Black, false),
    );
    let lines = vec!["one".to_string()];

    let _ = render_modal_box(&mut buffer, "THIS TITLE DOES NOT FIT", &lines, theme());
}
