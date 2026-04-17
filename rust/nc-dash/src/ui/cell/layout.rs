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
                Axis::Horizontal => {
                    Rect::new(area.x.saturating_add(offset), area.y, length, area.height)
                }
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
