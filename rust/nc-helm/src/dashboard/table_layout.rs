#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableWidthMode {
    Compact,
    Expand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutRect {
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
}

impl LayoutRect {
    pub const fn new(col: usize, row: usize, width: usize, height: usize) -> Self {
        Self {
            col,
            row,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnWidthSpec {
    pub base_width: usize,
    pub flex: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableBlockLayout {
    pub table_col: usize,
    pub table_row: usize,
    pub table_width: usize,
    pub table_height: usize,
    pub bottom_row: usize,
    pub title_row: Option<usize>,
    pub title_col: usize,
    pub command_row: Option<usize>,
    pub command_col: usize,
    pub scrollbar_col: Option<usize>,
}

pub const TABLE_TEXT_INSET: usize = 1;

pub fn table_render_width_from_cells(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() + 1
}

pub fn distribute_column_widths(
    specs: &[ColumnWidthSpec],
    available_width: usize,
    scrollbar_visible: bool,
    width_mode: TableWidthMode,
) -> Vec<usize> {
    let widths = specs.iter().map(|spec| spec.base_width).collect::<Vec<_>>();
    let compact_width = table_render_width_from_cells(&widths);
    if widths.is_empty() || width_mode == TableWidthMode::Compact {
        return widths;
    }

    let gutter = usize::from(scrollbar_visible);
    let target_width = available_width.saturating_sub(gutter);
    if target_width <= compact_width {
        return widths;
    }

    let total_flex = specs
        .iter()
        .map(|spec| usize::from(spec.flex))
        .sum::<usize>();
    if total_flex == 0 {
        return widths;
    }

    let extra = target_width - compact_width;
    let mut resolved = widths;
    let mut assigned = 0usize;
    for (idx, spec) in specs.iter().enumerate() {
        let share = extra * usize::from(spec.flex) / total_flex;
        resolved[idx] += share;
        assigned += share;
    }

    let mut remainder = extra.saturating_sub(assigned);
    if remainder > 0 {
        for (idx, spec) in specs.iter().enumerate() {
            if spec.flex == 0 {
                continue;
            }
            resolved[idx] += 1;
            remainder -= 1;
            if remainder == 0 {
                break;
            }
        }
    }

    resolved
}

pub fn layout_table_block(
    area: LayoutRect,
    table_width: usize,
    table_height: usize,
    minimum_block_width: usize,
    title: bool,
    command: bool,
    scrollbar_visible: bool,
    horizontal_align: HorizontalAlign,
    vertical_align: VerticalAlign,
) -> TableBlockLayout {
    let total_width = (table_width + usize::from(scrollbar_visible)).max(minimum_block_width);
    let total_height = table_height + usize::from(title) + usize::from(command);
    let col = area.col
        + align_offset(
            area.width,
            total_width,
            match horizontal_align {
                HorizontalAlign::Left => AlignKind::Start,
                HorizontalAlign::Center => AlignKind::Center,
                HorizontalAlign::Right => AlignKind::End,
            },
        );
    let row = area.row
        + align_offset(
            area.height,
            total_height,
            match vertical_align {
                VerticalAlign::Top => AlignKind::Start,
                VerticalAlign::Center => AlignKind::Center,
                VerticalAlign::Bottom => AlignKind::End,
            },
        );
    let table_row = row + usize::from(title);
    let bottom_row = table_row + table_height.saturating_sub(1);

    TableBlockLayout {
        table_col: col,
        table_row,
        table_width,
        table_height,
        bottom_row,
        title_row: title.then_some(row),
        title_col: col + TABLE_TEXT_INSET,
        command_row: command.then_some(bottom_row + 1),
        command_col: col + TABLE_TEXT_INSET,
        scrollbar_col: scrollbar_visible.then_some(col + table_width),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AlignKind {
    Start,
    Center,
    End,
}

fn align_offset(available: usize, size: usize, align: AlignKind) -> usize {
    if size >= available {
        return 0;
    }
    match align {
        AlignKind::Start => 0,
        AlignKind::Center => (available - size) / 2,
        AlignKind::End => available - size,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ColumnWidthSpec, HorizontalAlign, LayoutRect, TableWidthMode, VerticalAlign,
        distribute_column_widths, layout_table_block, table_render_width_from_cells,
    };

    #[test]
    fn compact_mode_leaves_widths_unchanged() {
        let specs = [
            ColumnWidthSpec {
                base_width: 10,
                flex: 1,
            },
            ColumnWidthSpec {
                base_width: 5,
                flex: 0,
            },
        ];
        assert_eq!(
            distribute_column_widths(&specs, 40, false, TableWidthMode::Compact),
            vec![10, 5]
        );
    }

    #[test]
    fn expand_mode_distributes_extra_width_by_flex() {
        let specs = [
            ColumnWidthSpec {
                base_width: 10,
                flex: 2,
            },
            ColumnWidthSpec {
                base_width: 5,
                flex: 1,
            },
        ];
        let widths = distribute_column_widths(&specs, 25, false, TableWidthMode::Expand);
        assert_eq!(table_render_width_from_cells(&widths), 25);
        assert!(widths[0] > widths[1]);
    }

    #[test]
    fn scrollbar_reduces_target_table_width_by_one_column() {
        let specs = [ColumnWidthSpec {
            base_width: 10,
            flex: 1,
        }];
        let without_scroll = distribute_column_widths(&specs, 20, false, TableWidthMode::Expand);
        let with_scroll = distribute_column_widths(&specs, 20, true, TableWidthMode::Expand);
        assert_eq!(
            table_render_width_from_cells(&without_scroll),
            table_render_width_from_cells(&with_scroll) + 1
        );
    }

    #[test]
    fn centered_block_accounts_for_scrollbar_gutter() {
        let layout = layout_table_block(
            LayoutRect::new(0, 0, 80, 25),
            30,
            10,
            31,
            true,
            true,
            true,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        assert_eq!(layout.table_col, 24);
        assert_eq!(layout.scrollbar_col, Some(54));
        assert_eq!(layout.title_row, Some(6));
        assert_eq!(layout.table_row, 7);
        assert_eq!(layout.command_row, Some(17));
    }

    #[test]
    fn block_width_can_expand_beyond_table_width_for_footer_or_title() {
        let layout = layout_table_block(
            LayoutRect::new(0, 0, 80, 25),
            20,
            10,
            36,
            true,
            true,
            false,
            HorizontalAlign::Center,
            VerticalAlign::Center,
        );
        assert_eq!(layout.table_col, 22);
        assert_eq!(layout.title_col, 23);
        assert_eq!(layout.command_col, 23);
    }
}
