use super::state::{LobbyRoute, LobbyState};

pub fn initial_route(keychain_exists: bool) -> LobbyRoute {
    if keychain_exists {
        LobbyRoute::Locked
    } else {
        LobbyRoute::FirstRun
    }
}

pub fn first_run_lines(state: &LobbyState) -> Vec<String> {
    vec![
        "First launch should collect a player handle and keychain password.".to_string(),
        "This scaffold does not create keys or write encrypted files yet.".to_string(),
        format!(
            "Default placeholder handle: {}",
            state.player_handle.as_deref().unwrap_or("<unset>")
        ),
        "Press Enter to continue into the stub lobby.".to_string(),
    ]
}

pub fn locked_lines(state: &LobbyState) -> Vec<String> {
    vec![
        "A local keychain already exists for this client root.".to_string(),
        "Unlock flow is stubbed in this pass.".to_string(),
        format!(
            "Active placeholder handle: {}",
            state.player_handle.as_deref().unwrap_or("<unset>")
        ),
        "Press Enter to continue into the stub lobby.".to_string(),
    ]
}
