pub mod style {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Color {
        Reset,
        Black,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        Gray,
        DarkGray,
        LightRed,
        LightGreen,
        LightYellow,
        LightBlue,
        LightMagenta,
        LightCyan,
        White,
        Indexed(u8),
        Rgb(u8, u8, u8),
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Modifier(u16);

    impl Modifier {
        pub const BOLD: Self = Self(1);

        pub const fn empty() -> Self {
            Self(0)
        }

        pub const fn contains(self, other: Self) -> bool {
            self.0 & other.0 == other.0
        }
    }

    impl std::ops::BitOr for Modifier {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Style {
        pub fg: Option<Color>,
        pub bg: Option<Color>,
        pub modifier: Modifier,
    }

    impl Default for Style {
        fn default() -> Self {
            Self {
                fg: None,
                bg: None,
                modifier: Modifier::empty(),
            }
        }
    }

    impl Style {
        pub const fn fg(mut self, color: Color) -> Self {
            self.fg = Some(color);
            self
        }

        pub const fn bg(mut self, color: Color) -> Self {
            self.bg = Some(color);
            self
        }

        pub fn add_modifier(mut self, modifier: Modifier) -> Self {
            self.modifier = self.modifier | modifier;
            self
        }
    }
}

pub mod layout {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Rect {
        pub x: u16,
        pub y: u16,
        pub width: u16,
        pub height: u16,
    }

    impl Rect {
        pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
            Self {
                x,
                y,
                width,
                height,
            }
        }

        pub const fn top(self) -> u16 {
            self.y
        }

        pub const fn bottom(self) -> u16 {
            self.y.saturating_add(self.height)
        }

        pub const fn right(self) -> u16 {
            self.x.saturating_add(self.width)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Constraint {
        Fill(u16),
        Max(u16),
        Min(u16),
        Length(u16),
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Axis {
        Horizontal,
        Vertical,
    }

    #[derive(Clone, Debug)]
    pub struct Layout {
        axis: Axis,
        constraints: Vec<Constraint>,
        spacing: u16,
    }

    impl Layout {
        pub fn horizontal(constraints: impl Into<Vec<Constraint>>) -> Self {
            Self {
                axis: Axis::Horizontal,
                constraints: constraints.into(),
                spacing: 0,
            }
        }

        pub fn vertical(constraints: impl Into<Vec<Constraint>>) -> Self {
            Self {
                axis: Axis::Vertical,
                constraints: constraints.into(),
                spacing: 0,
            }
        }

        pub const fn spacing(mut self, spacing: u16) -> Self {
            self.spacing = spacing;
            self
        }

        pub fn split(&self, area: Rect) -> Vec<Rect> {
            let count = self.constraints.len();
            if count == 0 {
                return Vec::new();
            }
            let gap_total = self.spacing.saturating_mul(count.saturating_sub(1) as u16);
            let axis_len = match self.axis {
                Axis::Horizontal => area.width,
                Axis::Vertical => area.height,
            }
            .saturating_sub(gap_total);
            let lengths = axis_lengths(&self.constraints, axis_len);
            let mut offset = 0u16;
            let mut rects = Vec::with_capacity(count);
            for length in lengths {
                let rect = match self.axis {
                    Axis::Horizontal => Rect::new(
                        area.x.saturating_add(offset),
                        area.y,
                        length,
                        area.height,
                    ),
                    Axis::Vertical => {
                        Rect::new(area.x, area.y.saturating_add(offset), area.width, length)
                    }
                };
                rects.push(rect);
                offset = offset.saturating_add(length).saturating_add(self.spacing);
            }
            rects
        }

        pub fn areas<const N: usize>(&self, area: Rect) -> [Rect; N] {
            self.split(area)
                .try_into()
                .expect("layout areas length should match constraints")
        }
    }

    fn axis_lengths(constraints: &[Constraint], axis_len: u16) -> Vec<u16> {
        let mut lengths = vec![0u16; constraints.len()];
        let mut remaining = axis_len;
        let mut max_slots = Vec::new();
        let mut min_slots = Vec::new();
        let mut fill_total = 0u32;

        for (idx, constraint) in constraints.iter().enumerate() {
            match *constraint {
                Constraint::Length(value) | Constraint::Min(value) => {
                    let allocated = value.min(remaining);
                    lengths[idx] = allocated;
                    remaining = remaining.saturating_sub(allocated);
                    if matches!(constraint, Constraint::Min(_)) {
                        min_slots.push(idx);
                    }
                }
                Constraint::Max(limit) => max_slots.push((idx, limit)),
                Constraint::Fill(weight) => fill_total += u32::from(weight.max(1)),
            }
        }

        for (idx, limit) in max_slots {
            if remaining == 0 {
                break;
            }
            let allocated = limit.min(remaining);
            lengths[idx] = allocated;
            remaining = remaining.saturating_sub(allocated);
        }

        if fill_total > 0 && remaining > 0 {
            let mut carry = remaining;
            let mut fill_indices = constraints
                .iter()
                .enumerate()
                .filter_map(|(idx, constraint)| match constraint {
                    Constraint::Fill(weight) => Some((idx, u32::from((*weight).max(1)))),
                    _ => None,
                })
                .collect::<Vec<_>>();
            while let Some((idx, weight)) = fill_indices.first().copied() {
                fill_indices.remove(0);
                let allocated = if fill_indices.is_empty() {
                    carry
                } else {
                    ((u32::from(remaining) * weight) / fill_total) as u16
                }
                .min(carry);
                lengths[idx] = lengths[idx].saturating_add(allocated);
                carry = carry.saturating_sub(allocated);
            }
            remaining = carry;
        }

        if remaining > 0 && !min_slots.is_empty() {
            let share = remaining / min_slots.len() as u16;
            let mut carry = remaining;
            let min_count = min_slots.len();
            for (slot_idx, idx) in min_slots.into_iter().enumerate() {
                let extra = if slot_idx + 1 == min_count {
                    carry
                } else {
                    share.min(carry)
                };
                lengths[idx] = lengths[idx].saturating_add(extra);
                carry = carry.saturating_sub(extra);
            }
        }

        lengths
    }
}

pub mod buffer {
    use super::layout::Rect;
    use super::style::{Color, Modifier, Style};

    #[derive(Clone, Debug)]
    pub struct Cell {
        symbol: String,
        pub fg: Color,
        pub bg: Color,
        pub modifier: Modifier,
    }

    impl Default for Cell {
        fn default() -> Self {
            Self {
                symbol: " ".to_string(),
                fg: Color::Reset,
                bg: Color::Reset,
                modifier: Modifier::empty(),
            }
        }
    }

    impl Cell {
        pub fn symbol(&self) -> &str {
            &self.symbol
        }

        fn set_char(&mut self, ch: char, style: Style) {
            self.symbol.clear();
            self.symbol.push(ch);
            self.fg = style.fg.unwrap_or(Color::Reset);
            self.bg = style.bg.unwrap_or(Color::Reset);
            self.modifier = style.modifier;
        }
    }

    #[derive(Clone, Debug)]
    pub struct Buffer {
        pub area: Rect,
        cells: Vec<Cell>,
    }

    impl Buffer {
        pub fn empty(area: Rect) -> Self {
            let len = usize::from(area.width) * usize::from(area.height);
            Self {
                area,
                cells: vec![Cell::default(); len],
            }
        }

        pub fn cell(&self, position: (u16, u16)) -> Option<&Cell> {
            let (x, y) = position;
            if x < self.area.x || y < self.area.y || x >= self.area.right() || y >= self.area.bottom()
            {
                return None;
            }
            let local_x = usize::from(x - self.area.x);
            let local_y = usize::from(y - self.area.y);
            self.cells
                .get(local_y * usize::from(self.area.width) + local_x)
        }

        pub fn set_stringn(
            &mut self,
            x: u16,
            y: u16,
            text: impl AsRef<str>,
            max_width: usize,
            style: Style,
        ) {
            if y < self.area.y || y >= self.area.bottom() || max_width == 0 {
                return;
            }
            let start = x.max(self.area.x);
            let end = self.area.right();
            let mut col = start;
            for ch in text.as_ref().chars().take(max_width) {
                if col >= end {
                    break;
                }
                self.set_cell(col, y, ch, style);
                col += 1;
            }
        }

        pub fn fill_rect(&mut self, area: Rect, style: Style) {
            for row in area.y..area.bottom() {
                for col in area.x..area.right() {
                    self.set_cell(col, row, ' ', style);
                }
            }
        }

        fn set_cell(&mut self, x: u16, y: u16, ch: char, style: Style) {
            if x < self.area.x || y < self.area.y || x >= self.area.right() || y >= self.area.bottom()
            {
                return;
            }
            let local_x = usize::from(x - self.area.x);
            let local_y = usize::from(y - self.area.y);
            if let Some(cell) = self
                .cells
                .get_mut(local_y * usize::from(self.area.width) + local_x)
            {
                cell.set_char(ch, style);
            }
        }
    }
}

pub mod widgets {
    use std::borrow::Cow;

    use super::buffer::Buffer;
    use super::layout::Rect;
    use super::style::Style;

    pub trait Widget {
        fn render(self, area: Rect, buffer: &mut Buffer);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Borders(u8);

    impl Borders {
        pub const ALL: Self = Self(0b1111);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BorderType {
        Plain,
        Double,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Padding {
        pub left: u16,
        pub right: u16,
        pub top: u16,
        pub bottom: u16,
    }

    impl Padding {
        pub const fn horizontal(value: u16) -> Self {
            Self {
                left: value,
                right: value,
                top: 0,
                bottom: 0,
            }
        }
    }

    #[derive(Clone, Debug)]
    pub struct Block<'a> {
        borders: Borders,
        border_type: BorderType,
        padding: Padding,
        style: Style,
        border_style: Style,
        title_style: Style,
        title: Option<Cow<'a, str>>,
    }

    impl<'a> Default for Block<'a> {
        fn default() -> Self {
            Self {
                borders: Borders::ALL,
                border_type: BorderType::Plain,
                padding: Padding::default(),
                style: Style::default(),
                border_style: Style::default(),
                title_style: Style::default(),
                title: None,
            }
        }
    }

    impl<'a> Block<'a> {
        pub fn borders(mut self, borders: Borders) -> Self {
            self.borders = borders;
            self
        }

        pub fn border_type(mut self, border_type: BorderType) -> Self {
            self.border_type = border_type;
            self
        }

        pub fn padding(mut self, padding: Padding) -> Self {
            self.padding = padding;
            self
        }

        pub fn title(mut self, title: impl Into<Cow<'a, str>>) -> Self {
            self.title = Some(title.into());
            self
        }

        pub fn style(mut self, style: Style) -> Self {
            self.style = style;
            self
        }

        pub fn border_style(mut self, style: Style) -> Self {
            self.border_style = style;
            self
        }

        pub fn title_style(mut self, style: Style) -> Self {
            self.title_style = style;
            self
        }

        pub fn inner(&self, area: Rect) -> Rect {
            let border_x = if self.borders == Borders::ALL { 1 } else { 0 };
            let border_y = if self.borders == Borders::ALL { 1 } else { 0 };
            let x = area
                .x
                .saturating_add(border_x)
                .saturating_add(self.padding.left);
            let y = area
                .y
                .saturating_add(border_y)
                .saturating_add(self.padding.top);
            let width = area
                .width
                .saturating_sub(border_x * 2)
                .saturating_sub(self.padding.left)
                .saturating_sub(self.padding.right);
            let height = area
                .height
                .saturating_sub(border_y * 2)
                .saturating_sub(self.padding.top)
                .saturating_sub(self.padding.bottom);
            Rect::new(x, y, width, height)
        }
    }

    impl Widget for Block<'_> {
        fn render(self, area: Rect, buffer: &mut Buffer) {
            buffer.fill_rect(area, self.style);
            if self.borders != Borders::ALL || area.width == 0 || area.height == 0 {
                return;
            }

            let (h, v, tl, tr, bl, br) = match self.border_type {
                BorderType::Plain => ('─', '│', '┌', '┐', '└', '┘'),
                BorderType::Double => ('═', '║', '╔', '╗', '╚', '╝'),
            };
            let top = area.y;
            let bottom = area.bottom().saturating_sub(1);
            let left = area.x;
            let right = area.right().saturating_sub(1);
            for col in left..=right {
                buffer.set_stringn(col, top, &h.to_string(), 1, self.border_style);
                buffer.set_stringn(col, bottom, &h.to_string(), 1, self.border_style);
            }
            for row in top..=bottom {
                buffer.set_stringn(left, row, &v.to_string(), 1, self.border_style);
                buffer.set_stringn(right, row, &v.to_string(), 1, self.border_style);
            }
            buffer.set_stringn(left, top, &tl.to_string(), 1, self.border_style);
            buffer.set_stringn(right, top, &tr.to_string(), 1, self.border_style);
            buffer.set_stringn(left, bottom, &bl.to_string(), 1, self.border_style);
            buffer.set_stringn(right, bottom, &br.to_string(), 1, self.border_style);

            if let Some(title) = self.title {
                let title_width = title.chars().count().min(area.width.saturating_sub(4) as usize);
                if title_width > 0 {
                    buffer.set_stringn(
                        area.x.saturating_add(2),
                        area.y,
                        &title,
                        title_width,
                        self.title_style,
                    );
                }
            }
        }
    }

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Clear;

    impl Widget for Clear {
        fn render(self, area: Rect, buffer: &mut Buffer) {
            buffer.fill_rect(area, Style::default());
        }
    }
}
