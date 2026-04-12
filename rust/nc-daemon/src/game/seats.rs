#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedSeat {
    pub player_record_index_1_based: usize,
    pub status: String,
}
