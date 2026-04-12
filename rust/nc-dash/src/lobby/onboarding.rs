use super::state::{FirstRunField, LobbyRoute, LobbyState};
use nc_client::keychain::keychain_path;

pub fn initial_route(keychain_exists: bool) -> LobbyRoute {
    if keychain_exists {
        LobbyRoute::Locked
    } else {
        LobbyRoute::FirstRun
    }
}

pub fn first_run_lines(state: &LobbyState) -> Vec<String> {
    vec![
        "Create your local hosted identity.".to_string(),
        format!(
            "{} Handle         : {}",
            field_marker(state.first_run_field == FirstRunField::Handle),
            display_or_cursor(&state.first_run_handle_input)
        ),
        format!(
            "{} Keychain pass  : {}",
            field_marker(state.first_run_field == FirstRunField::Password),
            masked_or_cursor(&state.first_run_password_input)
        ),
        format!(
            "{} Confirm pass   : {}",
            field_marker(state.first_run_field == FirstRunField::Confirm),
            masked_or_cursor(&state.first_run_confirm_input)
        ),
        format!("Keychain path  : {}", keychain_path().display()),
        "Tab moves between fields. Enter creates the encrypted keychain.".to_string(),
    ]
}

pub fn locked_lines(state: &LobbyState) -> Vec<String> {
    vec![
        "Unlock the local keychain.".to_string(),
        format!(
            "Password       : {}",
            masked_or_cursor(&state.unlock_password_input)
        ),
        format!("Keychain path  : {}", keychain_path().display()),
        "Enter unlocks the hosted lobby.".to_string(),
    ]
}

fn masked_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        "*".repeat(value.chars().count())
    }
}

fn display_or_cursor(value: &str) -> String {
    if value.is_empty() {
        "_".to_string()
    } else {
        value.to_string()
    }
}

fn field_marker(active: bool) -> &'static str {
    if active { ">" } else { " " }
}
