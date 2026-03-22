#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlockRow {
    pub block_index: usize,
    pub decoded_text: String,
    pub raw_bytes: Option<Vec<u8>>,
    pub recipient_deleted: bool,
}
