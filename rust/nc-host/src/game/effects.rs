use nc_nostr::claim::SeatClaimRequest;
use nc_nostr::first_join::FirstJoinSetupRequest;
use nc_nostr::invite_request::InviteRequest;
use nc_nostr::player_message::PlayerMessageRequest;
use nc_nostr::state_sync::StateRequest;
use nc_nostr::thread_message::SysopThreadMessage;
use nc_nostr::turn_commands::TurnCommands;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEffects {
    HandleStateRequest {
        request: StateRequest,
    },
    HandleInviteRequest {
        request: InviteRequest,
        game_id: String,
    },
    HandleFirstJoinSetup {
        request: FirstJoinSetupRequest,
        game_id: String,
    },
    HandleSeatClaim {
        request: SeatClaimRequest,
        game_id: String,
    },
    HandleTurnCommands {
        commands: TurnCommands,
        game_id: String,
    },
    HandleThreadMessage {
        message: SysopThreadMessage,
        game_id: String,
    },
    HandlePlayerMessage {
        message: PlayerMessageRequest,
        game_id: String,
    },
    QueueEvent {
        recipient_pubkey: String,
        kind: u32,
        content: String,
        tags: Vec<(String, String)>,
        encrypt: bool,
    },
    UpdateLobbyCatalog {
        game_id: String,
    },
    NotifySysop {
        game_id: String,
        message: String,
    },
    RunMaintenance {
        game_id: String,
    },
    InvalidEvent {
        reason: String,
    },
}
