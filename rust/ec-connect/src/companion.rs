use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rand::RngCore;

use crate::cache::CachedGame;

pub const PASSWORD_FILE_FLAG: &str = "--password-file";
pub const GAME_ID_FLAG: &str = "--game-id";
pub const NO_CONSOLE_SETUP_FLAG: &str = "--no-console-setup";

pub fn companion_binary_name() -> &'static str {
    if cfg!(windows) {
        "ec-connect-cli.exe"
    } else {
        "ec-connect-cli"
    }
}

pub fn companion_binary_path(current_exe: &Path) -> PathBuf {
    current_exe.with_file_name(companion_binary_name())
}

pub fn write_password_handoff_file(password: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut random = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut random);
    let path = std::env::temp_dir().join(format!(
        "ec-connect-pass-{}.txt",
        random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ));
    fs::write(&path, password.as_bytes())?;
    Ok(path)
}

pub fn command_for_cached_game(
    current_exe: &Path,
    game: &CachedGame,
    password_file: &Path,
) -> Command {
    let mut command = Command::new(companion_binary_path(current_exe));
    command.arg(format!("{}:{}", game.server, game.port));
    if let Some(relay_url) = game.relay_url.as_deref().filter(|value| !value.is_empty()) {
        command.arg("--relay").arg(relay_url);
    }
    command
        .arg(GAME_ID_FLAG)
        .arg(&game.id)
        .arg(PASSWORD_FILE_FLAG)
        .arg(password_file)
        .arg(NO_CONSOLE_SETUP_FLAG);
    if !game.gate_npub.trim().is_empty() {
        command.arg("--gate").arg(&game.gate_npub);
    }
    command
}

pub fn command_for_invite(current_exe: &Path, invite_code: &str, password_file: &Path) -> Command {
    let mut command = Command::new(companion_binary_path(current_exe));
    command
        .arg("--join")
        .arg(invite_code)
        .arg(PASSWORD_FILE_FLAG)
        .arg(password_file)
        .arg(NO_CONSOLE_SETUP_FLAG);
    command
}

#[cfg(test)]
mod tests {
    use super::{
        GAME_ID_FLAG, NO_CONSOLE_SETUP_FLAG, PASSWORD_FILE_FLAG, command_for_cached_game,
        command_for_invite, companion_binary_name, companion_binary_path,
    };
    use crate::cache::{CachedGame, CachedGameStatus};
    use std::path::Path;

    fn sample_game() -> CachedGame {
        CachedGame {
            id: "friday-night".to_string(),
            name: "Friday Night".to_string(),
            player_name: Some("House Vale".to_string()),
            server: "play.example.com".to_string(),
            port: 2222,
            relay_url: Some("wss://relay.example.com".to_string()),
            seat: 2,
            npub: "npub1sample".to_string(),
            gate_npub: "npub1gate".to_string(),
            status: CachedGameStatus::Joined,
            invite_code: None,
            joined: "2026-03-30T12:00:00Z".to_string(),
            last_connected: None,
        }
    }

    #[test]
    fn companion_path_reuses_current_directory() {
        let exe = Path::new("/tmp/ec-connect.exe");
        let path = companion_binary_path(exe);
        assert_eq!(path.parent(), exe.parent());
        assert_eq!(
            path.file_name().and_then(|value| value.to_str()),
            Some(companion_binary_name())
        );
    }

    #[test]
    fn cached_game_command_includes_direct_connect_hints() {
        let game = sample_game();
        let command = command_for_cached_game(
            Path::new("/tmp/ec-connect.exe"),
            &game,
            Path::new("/tmp/pass.txt"),
        );
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"play.example.com:2222".to_string()));
        assert!(args.contains(&"--relay".to_string()));
        assert!(args.contains(&"wss://relay.example.com".to_string()));
        assert!(args.contains(&GAME_ID_FLAG.to_string()));
        assert!(args.contains(&"friday-night".to_string()));
        assert!(args.contains(&PASSWORD_FILE_FLAG.to_string()));
        assert!(args.contains(&NO_CONSOLE_SETUP_FLAG.to_string()));
    }

    #[test]
    fn invite_command_uses_join_mode() {
        let command = command_for_invite(
            Path::new("/tmp/ec-connect.exe"),
            "amber-river@relay.example.com",
            Path::new("/tmp/pass.txt"),
        );
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args[0], "--join");
        assert_eq!(args[1], "amber-river@relay.example.com");
        assert!(args.contains(&PASSWORD_FILE_FLAG.to_string()));
        assert!(args.contains(&NO_CONSOLE_SETUP_FLAG.to_string()));
    }
}
