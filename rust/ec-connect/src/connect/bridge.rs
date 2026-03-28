//! SSH terminal bridge.
//!
//! Establishes an SSH connection to the game server using the ephemeral
//! Ed25519 keypair from the session handshake.  The local terminal is put
//! into raw mode; all stdin bytes are forwarded to the SSH channel and all
//! channel output bytes are written to stdout.  Terminal resize events are
//! forwarded as SSH window-change requests.
//!
//! # Lifecycle
//!
//! 1. Convert `EphemeralKeypair` → `russh::keys::PrivateKey` (in memory).
//! 2. Call `run_bridge` — raw mode on, SSH connect, PTY request, exec,
//!    I/O loop, raw mode off.
//! 3. Returns the SSH exit status code.
//!
//! # Host key verification
//!
//! The `host_fingerprint` field from `SessionReadyPayload` (e.g.
//! `"SHA256:…"`) is passed in and checked against the server's advertised
//! key in `FingerprintHandler::check_server_key`.  If the fingerprint is
//! empty (old gate version), any key is accepted.

use std::env;
use std::io::{Read, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size};
use russh::keys::ssh_key::{
    HashAlg,
    private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData},
};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, Disconnect, client};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::connect::handshake::SessionReadyPayload;
use crate::connect::ssh_key::EphemeralKeypair;
use ec_ui::session::write_terminal_cleanup_sequence;

// ── Public entry point ────────────────────────────────────────────────────────

/// Error type for bridge operations.
pub type BridgeError = Box<dyn std::error::Error + Send + Sync>;
const POST_EXIT_DRAIN_GRACE: Duration = Duration::from_millis(150);
const SESSION_DISCONNECT_GRACE: Duration = Duration::from_secs(1);
const STDIN_POLL_GRACE: Duration = Duration::from_millis(20);

fn terminal_channel_event(msg: &ChannelMsg) -> bool {
    matches!(msg, ChannelMsg::Eof | ChannelMsg::Close)
}

enum StdinEvent {
    Data(Vec<u8>),
    Eof,
    Error(String),
}

struct StdinPump {
    rx: mpsc::UnboundedReceiver<StdinEvent>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl StdinPump {
    fn spawn() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let handle = thread::spawn(move || pump_stdin(tx, thread_stop));
        Self {
            rx,
            stop,
            handle: Some(handle),
        }
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        #[cfg(unix)]
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        #[cfg(not(unix))]
        let _ = self.handle.take();
    }
}

impl Drop for StdinPump {
    fn drop(&mut self) {
        self.stop();
    }
}

fn pump_stdin(tx: mpsc::UnboundedSender<StdinEvent>, stop: Arc<AtomicBool>) {
    let mut stdin = std::io::stdin();
    let mut buf = [0u8; 4096];
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        #[cfg(unix)]
        {
            match wait_for_stdin_or_stop(&stop) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(err) => {
                    let _ = tx.send(StdinEvent::Error(err));
                    break;
                }
            }
        }
        match stdin.read(&mut buf) {
            Ok(0) => {
                let _ = tx.send(StdinEvent::Eof);
                break;
            }
            Ok(n) => {
                if tx.send(StdinEvent::Data(buf[..n].to_vec())).is_err() {
                    break;
                }
            }
            Err(err) => {
                let _ = tx.send(StdinEvent::Error(err.to_string()));
                break;
            }
        }
    }
}

#[cfg(unix)]
fn wait_for_stdin_or_stop(stop: &AtomicBool) -> Result<bool, String> {
    use libc::{POLLIN, poll, pollfd};
    while !stop.load(Ordering::Relaxed) {
        let mut fds = [pollfd {
            fd: 0,
            events: POLLIN,
            revents: 0,
        }];
        let rc = unsafe {
            poll(
                fds.as_mut_ptr(),
                fds.len() as _,
                STDIN_POLL_GRACE.as_millis() as i32,
            )
        };
        if rc < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        if rc == 0 {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

/// Connect to the SSH server described by `payload`, authenticate with
/// `keypair`, run the forced command (empty exec string), and bridge
/// stdin/stdout until the channel closes.
///
/// Returns the SSH exit status code, or `1` if the channel closed without
/// reporting one.
pub async fn run_bridge(
    payload: &SessionReadyPayload,
    keypair: &EphemeralKeypair,
    username: &str,
) -> Result<u32, BridgeError> {
    let addr = (payload.ssh_host.as_str(), payload.ssh_port);
    let ssh_username = if payload.ssh_user.is_empty() {
        username
    } else {
        payload.ssh_user.as_str()
    };

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(300)),
        ..<_>::default()
    });

    let handler = FingerprintHandler {
        expected_fingerprint: payload.host_fingerprint.clone(),
    };

    let mut session = client::connect(config, addr, handler).await?;

    // Authenticate with the ephemeral keypair.
    let privkey = keypair.to_russh_private_key()?;
    let hash_alg = session.best_supported_rsa_hash().await?.flatten();
    let auth_result = session
        .authenticate_publickey(
            ssh_username,
            PrivateKeyWithHashAlg::new(Arc::new(privkey), hash_alg),
        )
        .await?;

    if !auth_result.success() {
        return Err("SSH public-key authentication failed".into());
    }

    let mut channel = session.channel_open_session().await?;

    let term = env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
    let (cols, rows) = terminal_size().unwrap_or((80, 24));
    channel
        .request_pty(false, &term, cols as u32, rows as u32, 0, 0, &[])
        .await?;

    // Empty exec — the server runs its configured forced command.
    channel.exec(true, "").await?;

    // Enter raw mode for the duration of the session.
    enable_raw_mode()?;
    let exit_code = io_loop(&mut channel).await;
    let _ = disable_raw_mode();
    let _ = restore_local_terminal_after_bridge();

    match timeout(
        SESSION_DISCONNECT_GRACE,
        session.disconnect(Disconnect::ByApplication, "", "English"),
    )
    .await
    {
        Ok(Ok(())) | Ok(Err(_)) | Err(_) => {}
    }

    exit_code
}

// ── I/O loop ─────────────────────────────────────────────────────────────────

/// Drive stdin → channel and channel → stdout until the remote side closes.
async fn io_loop(channel: &mut russh::Channel<client::Msg>) -> Result<u32, BridgeError> {
    let mut stdin_pump = StdinPump::spawn();
    let mut stdout = tokio::io::stdout();
    let mut stdin_closed = false;
    #[allow(unused_assignments)] // set inside select! arm; false positive
    let mut exit_code: Option<u32> = None;

    // On Unix, watch for SIGWINCH to forward terminal resize events.
    #[cfg(unix)]
    let mut sigwinch = {
        use tokio::signal::unix::{SignalKind, signal};
        signal(SignalKind::window_change())?
    };

    loop {
        #[cfg(unix)]
        let resize = sigwinch.recv();
        #[cfg(not(unix))]
        let resize = std::future::pending::<Option<()>>();

        tokio::select! {
            // Forward stdin bytes to the SSH channel.
            maybe_event = stdin_pump.rx.recv(), if !stdin_closed => {
                match maybe_event {
                    Some(StdinEvent::Eof) | None => {
                        stdin_closed = true;
                        let _ = channel.eof().await;
                    }
                    Some(StdinEvent::Data(data)) => {
                        channel.data(std::io::Cursor::new(&data)).await?;
                    }
                    Some(StdinEvent::Error(err)) => return Err(err.into()),
                }
            }
            // Handle events from the SSH channel.
            msg = channel.wait() => {
                let Some(msg) = msg else {
                    exit_code.get_or_insert(0);
                    break;
                };
                match msg {
                    ChannelMsg::Data { ref data } => {
                        stdout.write_all(data).await?;
                        stdout.flush().await?;
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        exit_code = Some(exit_status);
                        if !stdin_closed {
                            let _ = channel.eof().await;
                        }
                        break;
                    }
                    msg if terminal_channel_event(&msg) => {
                        exit_code.get_or_insert(0);
                        break;
                    }
                    ChannelMsg::WindowAdjusted { .. }
                    | ChannelMsg::Success
                    | ChannelMsg::Failure => {}
                    _ => {}
                }
            }
            // Forward terminal resize events to the SSH channel.
            _ = resize => {
                if let Ok((cols, rows)) = terminal_size() {
                    let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                }
            }
        }
    }

    drain_post_exit_output(channel, &mut stdout).await;
    stdin_pump.stop();

    Ok(exit_code.unwrap_or(1))
}

async fn drain_post_exit_output(
    channel: &mut russh::Channel<client::Msg>,
    stdout: &mut tokio::io::Stdout,
) {
    loop {
        let Ok(message) = timeout(POST_EXIT_DRAIN_GRACE, channel.wait()).await else {
            break;
        };
        let Some(msg) = message else {
            break;
        };
        match msg {
            ChannelMsg::Data { ref data } => {
                let _ = stdout.write_all(data).await;
                let _ = stdout.flush().await;
            }
            ChannelMsg::Eof | ChannelMsg::Close => break,
            _ => {}
        }
    }
}

fn restore_local_terminal_after_bridge() -> Result<(), BridgeError> {
    let mut stdout = std::io::stdout();
    write_bridge_cleanup_sequence(&mut stdout)?;
    stdout.flush()?;
    Ok(())
}

fn write_bridge_cleanup_sequence(
    out: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    write_terminal_cleanup_sequence(out)?;
    out.write_all(b"\r\n")?;
    Ok(())
}

// ── Handler ───────────────────────────────────────────────────────────────────

/// russh client handler that verifies the server host key fingerprint.
struct FingerprintHandler {
    /// Expected `"SHA256:…"` fingerprint from the handshake payload.
    /// Empty string means "accept any key" (old gate without fingerprint).
    expected_fingerprint: String,
}

impl client::Handler for FingerprintHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        if self.expected_fingerprint.is_empty() {
            return Ok(true);
        }
        let actual = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        Ok(actual == self.expected_fingerprint)
    }
}

#[cfg(test)]
mod tests {
    use super::{terminal_channel_event, write_bridge_cleanup_sequence};
    use russh::ChannelMsg;

    #[test]
    fn bridge_cleanup_restores_cursor_and_ends_on_new_line() {
        let mut actual = Vec::new();
        write_bridge_cleanup_sequence(&mut actual).expect("cleanup sequence should serialize");
        assert!(
            actual.ends_with(b"\r\n"),
            "cleanup should leave the shell prompt on a fresh line: {actual:?}"
        );
        assert!(
            actual.starts_with(b"\x1b["),
            "cleanup should emit terminal reset/show escapes before the newline: {actual:?}"
        );
    }

    #[test]
    fn close_events_are_terminal_conditions() {
        assert!(terminal_channel_event(&ChannelMsg::Eof));
        assert!(terminal_channel_event(&ChannelMsg::Close));
        assert!(!terminal_channel_event(&ChannelMsg::Success));
    }
}

// ── Key conversion ────────────────────────────────────────────────────────────

impl EphemeralKeypair {
    /// Convert this keypair into a `russh::keys::PrivateKey` for SSH
    /// public-key authentication.
    ///
    /// The conversion goes:
    /// `SigningKey → Ed25519PrivateKey → Ed25519Keypair → KeypairData → PrivateKey`
    pub fn to_russh_private_key(&self) -> Result<PrivateKey, BridgeError> {
        let seed = self.signing_key_bytes();
        let ed_priv = Ed25519PrivateKey::from_bytes(&seed);
        let ed_kp = Ed25519Keypair::from(ed_priv);
        let key_data = KeypairData::from(ed_kp);
        let privkey = PrivateKey::new(key_data, "")
            .map_err(|e| format!("failed to construct PrivateKey: {e}"))?;
        Ok(privkey)
    }
}
