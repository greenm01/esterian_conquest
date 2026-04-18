mod caret;
mod mapping;
mod metrics;

pub use caret::caret_rect;
pub use mapping::GridMapper;
pub use metrics::{
    CellMetrics, GridMetrics, TextMetrics, logical_window_size_for_grid,
};
