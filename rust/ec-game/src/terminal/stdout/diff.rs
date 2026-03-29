use crate::screen::Cell;

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

#[cfg(test)]
mod tests {
    use super::{ChangedSpan, changed_spans};
    use crate::screen::{Cell, CellStyle, GameColor};

    fn cell(ch: char) -> Cell {
        Cell::new(
            ch,
            CellStyle::new(GameColor::White, GameColor::Black, false),
        )
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
        let current = [
            highlighted('A'),
            cell('B'),
            cell('C'),
            highlighted('D'),
            cell('E'),
        ];
        assert_eq!(
            changed_spans(&previous, &current),
            vec![
                ChangedSpan { start: 0, end: 1 },
                ChangedSpan { start: 3, end: 4 }
            ]
        );
    }

    #[test]
    fn trailing_blank_diff_is_retained() {
        let previous = [cell('A'), cell('B'), cell('C')];
        let current = [cell('A'), cell('B'), cell(' ')];
        assert_eq!(
            changed_spans(&previous, &current),
            vec![ChangedSpan { start: 2, end: 3 }]
        );
    }
}
