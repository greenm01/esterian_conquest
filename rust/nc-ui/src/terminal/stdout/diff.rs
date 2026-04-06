use crate::buffer::{Cell, GameColor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChangedSpan {
    pub start: usize,
    pub end: usize,
}

pub(super) fn changed_spans(previous_row: &[Cell], current_row: &[Cell]) -> Vec<ChangedSpan> {
    assert_eq!(
        previous_row.len(),
        current_row.len(),
        "diff rows must have matching widths"
    );
    let mut spans = Vec::new();
    let mut current_span_start = None;

    for idx in 0..current_row.len() {
        if previous_row[idx] != current_row[idx] {
            current_span_start.get_or_insert(idx);
            continue;
        }
        if let Some(start) = current_span_start.take() {
            spans.push(ChangedSpan { start, end: idx });
        }
    }

    if let Some(start) = current_span_start {
        spans.push(ChangedSpan {
            start,
            end: current_row.len(),
        });
    }

    spans
}

pub(super) fn fingerprint_row(row: &[Cell]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for cell in row {
        hash = mix_u32(hash, cell.ch as u32);
        hash = mix_u32(hash, color_code(cell.style.fg));
        hash = mix_u32(hash, color_code(cell.style.bg));
        hash = mix_u32(hash, u32::from(cell.style.bold));
    }
    hash
}

fn mix_u32(mut hash: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn color_code(color: GameColor) -> u32 {
    match color {
        GameColor::Black => 0,
        GameColor::Red => 1,
        GameColor::Green => 2,
        GameColor::Yellow => 3,
        GameColor::Blue => 4,
        GameColor::Magenta => 5,
        GameColor::Cyan => 6,
        GameColor::White => 7,
        GameColor::BrightBlack => 8,
        GameColor::BrightRed => 9,
        GameColor::BrightGreen => 10,
        GameColor::BrightYellow => 11,
        GameColor::BrightBlue => 12,
        GameColor::BrightMagenta => 13,
        GameColor::BrightCyan => 14,
        GameColor::BrightWhite => 15,
        GameColor::Indexed(idx) => 0x0100_0000 | u32::from(idx),
        GameColor::Rgb(r, g, b) => {
            0x0200_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangedSpan, changed_spans, fingerprint_row};
    use crate::buffer::{Cell, CellStyle, GameColor};

    fn cell(ch: char) -> Cell {
        Cell::new(ch, CellStyle::new(GameColor::White, GameColor::Black, false))
    }

    fn highlighted(ch: char) -> Cell {
        Cell::new(ch, CellStyle::new(GameColor::Black, GameColor::White, true))
    }

    #[test]
    fn unchanged_rows_produce_no_spans() {
        let row = [cell('A'), cell('B'), cell('C')];
        assert!(changed_spans(&row, &row).is_empty());
    }

    #[test]
    fn contiguous_changes_collapse_into_one_span() {
        let previous = [cell('A'), cell('B'), cell('C'), cell('D')];
        let current = [cell('A'), highlighted('B'), highlighted('C'), cell('D')];
        assert_eq!(
            changed_spans(&previous, &current),
            vec![ChangedSpan { start: 1, end: 3 }]
        );
    }

    #[test]
    fn separated_changes_produce_multiple_spans() {
        let previous = [cell('A'), cell('B'), cell('C'), cell('D'), cell('E')];
        let current = [highlighted('A'), cell('B'), cell('C'), highlighted('D'), cell('E')];
        assert_eq!(
            changed_spans(&previous, &current),
            vec![
                ChangedSpan { start: 0, end: 1 },
                ChangedSpan { start: 3, end: 4 }
            ]
        );
    }

    #[test]
    fn identical_rows_have_matching_fingerprints() {
        let row = [cell('A'), highlighted('B'), cell('C')];
        assert_eq!(fingerprint_row(&row), fingerprint_row(&row));
    }

    #[test]
    fn content_changes_change_the_fingerprint() {
        let previous = [cell('A'), cell('B'), cell('C')];
        let current = [cell('A'), cell('X'), cell('C')];
        assert_ne!(fingerprint_row(&previous), fingerprint_row(&current));
    }

    #[test]
    fn style_changes_change_the_fingerprint() {
        let previous = [cell('A'), cell('B'), cell('C')];
        let current = [cell('A'), highlighted('B'), cell('C')];
        assert_ne!(fingerprint_row(&previous), fingerprint_row(&current));
    }
}
