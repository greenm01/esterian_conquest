pub mod invite_requests;
pub mod outbox;
pub mod schema;
pub mod seats;
pub mod settings;
pub mod store;
pub mod sysop_notifications;
pub mod threads;
pub mod turn_queue;

pub use invite_requests::{
    approve_request, approve_request_for_seat, create_request, get_pending_request_count,
    get_request, list_pending_decisions, list_requests, mark_decision_published, reject_request,
    count_pending_requests, count_unpublished_decisions, InviteRequest, InviteRequestStatus,
};
pub use outbox::{
    count_by_status, delete_published_older_than, enqueue, get_pending, increment_retry,
    mark_failed, mark_published, OutboxEvent, OutboxItem, OutboxStatus,
};
pub use schema::INIT_SQL;
pub use seats::{
    claim_seat, close_seat, create_seats, find_seat_by_invite_hash, get_seat_by_number,
    get_seat_by_pubkey, list_seats, open_seat, reissue_seat, reset_seat, Seat, SeatStatus,
};
pub use settings::{
    clear_catalog_dirty, get_catalog_dirty_since, get_game_metadata, get_settings,
    mark_catalog_dirty, update_settings, GameMetadata, GameSettings, LobbyVisibility,
    RecruitingMode,
};
pub use store::HostedStore;
pub use sysop_notifications::{
    enqueue as enqueue_sysop_notification, get_pending as get_pending_sysop_notifications,
    mark_failed as mark_sysop_notification_failed, mark_sent as mark_sysop_notification_sent,
    SysopNotification, SysopNotificationStatus,
};
pub use threads::{list_messages as list_thread_messages, list_thread_players, store_message as store_thread_message, ThreadMessage as HostedThreadMessage};
pub use turn_queue::{
    accept_turn, count_pending_turns, enqueue_turn, get_pending_turn, list_pending_turns,
    mark_superseded, reject_turn, TurnSubmission, TurnSubmissionStatus,
};
