use nostr_sdk::{Event, ToBech32};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRequest {
    pub request_id: String,
    pub game_id: String,
    pub player_pubkey: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InviteRequestReceiptStatus {
    Received,
    NotRecruiting,
    GameClosed,
    RateLimited,
    UnknownGame,
}

impl InviteRequestReceiptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InviteRequestReceiptStatus::Received => "received",
            InviteRequestReceiptStatus::NotRecruiting => "not_recruiting",
            InviteRequestReceiptStatus::GameClosed => "game_closed",
            InviteRequestReceiptStatus::RateLimited => "rate_limited",
            InviteRequestReceiptStatus::UnknownGame => "unknown_game",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteRequestReceipt {
    pub request_id: String,
    pub game_id: String,
    pub status: InviteRequestReceiptStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteDecision {
    Approved { invite: String },
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteDecisionPayload {
    pub request_id: String,
    pub game_id: String,
    pub decision: InviteDecision,
    pub message: String,
}

pub fn parse_invite_request(event: &Event) -> Option<InviteRequest> {
    let player_pubkey = event.pubkey.to_bech32().ok()?;
    let mut request_id = None;
    let mut game_id = None;
    let message = event.content.clone();

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => request_id = Some(values[1].clone()),
            "game-id" if values.len() >= 2 => game_id = Some(values[1].clone()),
            _ => {}
        }
    }

    Some(InviteRequest {
        request_id: request_id?,
        game_id: game_id?,
        player_pubkey,
        message,
    })
}

pub fn build_invite_request_receipt_tags(
    receipt: &InviteRequestReceipt,
) -> Vec<(&'static str, String)> {
    vec![
        ("d", receipt.request_id.clone()),
        ("game-id", receipt.game_id.clone()),
        ("status", receipt.status.as_str().to_string()),
    ]
}

pub fn build_invite_decision_tags(decision: &InviteDecisionPayload) -> Vec<(&'static str, String)> {
    let decision_str = match decision.decision {
        InviteDecision::Approved { .. } => "approved",
        InviteDecision::Rejected => "rejected",
    };

    let mut tags = vec![
        ("d", decision.request_id.clone()),
        ("game-id", decision.game_id.clone()),
        ("decision", decision_str.to_string()),
    ];

    if let InviteDecision::Approved { ref invite } = decision.decision {
        tags.push(("invite", invite.clone()));
    }

    tags
}
