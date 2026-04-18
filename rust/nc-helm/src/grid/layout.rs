#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenGeometry {
    width: usize,
    height: usize,
}

impl ScreenGeometry {
    pub const fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }

    pub const fn width(self) -> usize {
        self.width
    }

    pub const fn height(self) -> usize {
        self.height
    }
}
