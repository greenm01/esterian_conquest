use nc_helm::{CellStyle, Column, GameColor, PlayfieldBuffer, Point, Row};

#[test]
fn point_keeps_column_and_row_distinct() {
    let point = Point::new(Column(12), Row(7));
    assert_eq!(point.column, Column(12));
    assert_eq!(point.row, Row(7));
}

#[test]
fn playfield_buffer_stores_typed_cursor_positions() {
    let style = CellStyle::new(GameColor::BrightWhite, GameColor::Black, false);
    let mut buffer = PlayfieldBuffer::new(40, 20, style);
    let cursor = Point::new(Column(9), Row(4));
    buffer.set_cursor(cursor);
    assert_eq!(buffer.cursor(), Some(cursor));
}
