use ratatui::layout::Rect;

use crate::modal;
use crate::overlays::frame::RelativePopupOrigin;

pub fn placed_popup_rect(
    parent: Rect,
    preferred: (u16, u16),
    origin: Option<RelativePopupOrigin>,
) -> Rect {
    let placement = origin
        .map(|origin| modal::ModalPlacement::Origin {
            x: parent.x.saturating_add(origin.col_offset as u16),
            y: parent.y.saturating_add(origin.row_offset as u16),
        })
        .unwrap_or(modal::ModalPlacement::Centered);
    let rect = modal::placed_rect(
        preferred.0.min(parent.width.saturating_sub(2)).max(10),
        preferred.1.min(parent.height.saturating_sub(2)).max(5),
        modal::Rect::new(parent.x, parent.y, parent.width, parent.height),
        placement,
    );
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}
