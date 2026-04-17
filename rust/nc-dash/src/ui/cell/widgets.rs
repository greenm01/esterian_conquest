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
