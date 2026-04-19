//! Dash-local inbox adapters over the shared nc-data runtime projection.

use nc_data::{CoreGameData, QueuedPlayerMail, ReportBlockRow, runtime_inbox_items};

use crate::dashboard::app::state::InboxFilter;

pub use nc_data::{
    InboxItem as DashInboxItem, InboxItemSource as DashInboxItemSource,
    InboxItemType as DashInboxItemType, ReportSummaryBucket,
};

pub fn project_inbox_items(
    game_data: &CoreGameData,
    viewer_empire_id: u8,
    report_blocks: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
) -> Vec<DashInboxItem> {
    runtime_inbox_items(game_data, viewer_empire_id, report_blocks, queued_mail)
}

pub fn matches_filter(
    item: &DashInboxItem,
    filter: InboxFilter,
    current_year_only: bool,
    current_year: u16,
) -> bool {
    if current_year_only && item.year != current_year {
        return false;
    }
    match filter {
        InboxFilter::All => true,
        InboxFilter::Reports => matches!(item.item_type, DashInboxItemType::Report),
        InboxFilter::Messages => matches!(item.item_type, DashInboxItemType::Message),
    }
}
