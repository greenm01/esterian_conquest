use crate::private_payload::decrypt_private_json_from_event;
use crate::pubkeys::event_pubkey_hex;
use nostr_sdk::{Event, SecretKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnCommands {
    pub submit_id: String,
    pub game_id: String,
    pub turn: u32,
    pub player_pubkey: String,
    pub commands: String,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnCommandsPayload {
    pub commands: String,
    pub handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnReceiptStatus {
    Accepted,
    Rejected,
    Superseded,
    NotClaimed,
    WrongTurn,
}

impl TurnReceiptStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "accepted" => Some(TurnReceiptStatus::Accepted),
            "rejected" => Some(TurnReceiptStatus::Rejected),
            "superseded" => Some(TurnReceiptStatus::Superseded),
            "not_claimed" => Some(TurnReceiptStatus::NotClaimed),
            "wrong_turn" => Some(TurnReceiptStatus::WrongTurn),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TurnReceiptStatus::Accepted => "accepted",
            TurnReceiptStatus::Rejected => "rejected",
            TurnReceiptStatus::Superseded => "superseded",
            TurnReceiptStatus::NotClaimed => "not_claimed",
            TurnReceiptStatus::WrongTurn => "wrong_turn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnReceiptError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnReceipt {
    pub submit_id: String,
    pub game_id: String,
    pub turn: u32,
    pub status: TurnReceiptStatus,
    pub message: Option<String>,
    pub errors: Vec<TurnReceiptError>,
}

pub fn parse_turn_commands(secret_key: &SecretKey, event: &Event) -> Option<TurnCommands> {
    let player_pubkey = event_pubkey_hex(event);
    let mut submit_id = None;
    let mut game_id = None;
    let mut turn = None;
    let payload: TurnCommandsPayload = decrypt_private_json_from_event(secret_key, event).ok()?;

    for tag in event.tags.iter() {
        let values = tag.clone().to_vec();
        let Some(kind) = values.first().map(String::as_str) else {
            continue;
        };
        match kind {
            "d" if values.len() >= 2 => submit_id = Some(values[1].clone()),
            "game-id" if values.len() >= 2 => game_id = Some(values[1].clone()),
            "turn" if values.len() >= 2 => turn = values[1].parse().ok(),
            _ => {}
        }
    }

    Some(TurnCommands {
        submit_id: submit_id?,
        game_id: game_id?,
        turn: turn?,
        player_pubkey,
        commands: payload.commands,
        handle: payload.handle.filter(|value| !value.trim().is_empty()),
    })
}

pub fn build_turn_receipt_tags(receipt: &TurnReceipt) -> Vec<(&'static str, String)> {
    let mut tags = vec![
        ("d", receipt.submit_id.clone()),
        ("game-id", receipt.game_id.clone()),
        ("turn", receipt.turn.to_string()),
        ("status", receipt.status.as_str().to_string()),
    ];

    if let Some(ref msg) = receipt.message {
        tags.push(("message", msg.clone()));
    }

    for err in &receipt.errors {
        tags.push(("error", format!("{}: {}", err.path, err.message)));
    }

    tags
}
