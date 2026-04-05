#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlockRow {
    /// `0` means a broadcast/legacy row visible to every viewer.
    pub viewer_empire_id: u8,
    pub block_index: usize,
    pub decoded_text: String,
    pub raw_bytes: Option<Vec<u8>>,
    pub recipient_deleted: bool,
}

impl ReportBlockRow {
    pub fn is_visible_to_viewer(&self, viewer_empire_id: u8) -> bool {
        self.viewer_empire_id == 0 || self.viewer_empire_id == viewer_empire_id
    }
}
