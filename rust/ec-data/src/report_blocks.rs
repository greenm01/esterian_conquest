#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlockRow {
    pub block_index: usize,
    pub decoded_text: String,
    pub raw_bytes: Option<Vec<u8>>,
    pub recipient_deleted: bool,
}

impl ReportBlockRow {
    pub fn from_classic_block(block_index: usize, block: ec_classic::ClassicReportBlock) -> Self {
        Self {
            block_index,
            decoded_text: block.decoded_text,
            raw_bytes: block.raw_bytes,
            recipient_deleted: false,
        }
    }

    pub fn to_classic_block(&self) -> ec_classic::ClassicReportBlock {
        ec_classic::ClassicReportBlock {
            decoded_text: self.decoded_text.clone(),
            raw_bytes: self.raw_bytes.clone(),
        }
    }
}
