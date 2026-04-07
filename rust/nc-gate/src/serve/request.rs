//! Session request parsing re-exported from `nc-nostr`.

pub use nc_nostr::session::{
    ParseSessionRequestError as ParseError, SessionRequest, parse_session_request,
};
