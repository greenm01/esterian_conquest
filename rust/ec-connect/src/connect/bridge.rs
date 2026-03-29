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
use std::time::{Duration, Instant};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size};
use russh::keys::ssh_key::{
    HashAlg,
    private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData},
};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, client};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::connect::handshake::SessionReadyPayload;
use crate::connect::ssh_key::EphemeralKeypair;
use ec_ui::session::write_terminal_cleanup_sequence;

// ── Public entry point ────────────────────────────────────────────────────────

/// Error type for bridge operations.
pub type BridgeError = Box<dyn std::error::Error + Send + Sync>;
const POST_EXIT_DRAIN_GRACE: Duration = Duration::from_millis(10);
const STDIN_POLL_GRACE: Duration = Duration::from_millis(20);

fn terminal_channel_event(msg: &ChannelMsg) -> bool {
    matches!(msg, ChannelMsg::Eof | ChannelMsg::Close)
}

fn bridge_terminal_exit_code(msg: &ChannelMsg) -> Option<u32> {
    match msg {
        ChannelMsg::ExitStatus { exit_status } => Some(*exit_status),
        msg if terminal_channel_event(msg) => Some(0),
        _ => None,
    }
}

fn bridge_terminal_exit_label(msg: &ChannelMsg) -> Option<&'static str> {
    match msg {
        ChannelMsg::ExitStatus { .. } => Some("exit_status"),
        ChannelMsg::Eof => Some("eof"),
        ChannelMsg::Close => Some("close"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PostExitDrainSummary {
    timed_out: bool,
    saw_terminal_close: bool,
    data_messages: usize,
    data_bytes: usize,
}

fn elapsed_ms(started_at: Instant) -> u128 {
    started_at.elapsed().as_millis()
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
    let bridge_started = Instant::now();
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
    tracing::debug!(
        ssh_host = %payload.ssh_host,
        ssh_port = payload.ssh_port,
        ssh_user = ssh_username,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge ssh session connected"
    );

    // Authenticate with the ephemeral keypair.
    let privkey = keypair.to_russh_private_key()?;
    let hash_alg = session.best_supported_rsa_hash().await?.flatten();
    let auth_result = session
        .authenticate_publickey(
            ssh_username,
            PrivateKeyWithHashAlg::new(Arc::new(privkey), hash_alg),
        )
        .await?;
    tracing::debug!(
        ssh_user = ssh_username,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge ssh authentication completed"
    );

    if !auth_result.success() {
        tracing::warn!(
            ssh_host = %payload.ssh_host,
            ssh_port = payload.ssh_port,
            ssh_user = ssh_username,
            elapsed_ms = elapsed_ms(bridge_started),
            "bridge ssh public-key authentication failed"
        );
        return Err("SSH public-key authentication failed".into());
    }

    let mut channel = session.channel_open_session().await?;
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge channel opened"
    );

    let term = env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
    let (cols, rows) = terminal_size().unwrap_or((80, 24));
    channel
        .request_pty(false, &term, cols as u32, rows as u32, 0, 0, &[])
        .await?;
    tracing::debug!(
        term = %term,
        cols,
        rows,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge pty requested"
    );

    // Empty exec — the server runs its configured forced command.
    channel.exec(true, "").await?;
    tracing::info!(
        ssh_host = %payload.ssh_host,
        ssh_port = payload.ssh_port,
        ssh_user = ssh_username,
        term = %term,
        cols,
        rows,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge started"
    );

    // Enter raw mode for the duration of the session.
    enable_raw_mode()?;
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge raw mode enabled"
    );
    let exit_code = io_loop(&mut channel, bridge_started).await;
    tracing::debug!(
        exit = ?exit_code,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge io loop completed"
    );
    let _ = disable_raw_mode();
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge raw mode disabled"
    );
    let _ = restore_local_terminal_after_bridge();
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge terminal cleanup completed"
    );
    drop(session);
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge session dropped"
    );
    match &exit_code {
        Ok(code) => tracing::info!(
            exit_code = *code,
            elapsed_ms = elapsed_ms(bridge_started),
            "bridge finished"
        ),
        Err(err) => tracing::warn!(
            error = %err,
            elapsed_ms = elapsed_ms(bridge_started),
            "bridge failed"
        ),
    }

    exit_code
}

// ── I/O loop ─────────────────────────────────────────────────────────────────

/// Drive stdin → channel and channel → stdout until the remote side closes.
async fn io_loop(
    channel: &mut russh::Channel<client::Msg>,
    bridge_started: Instant,
) -> Result<u32, BridgeError> {
    let mut stdin_pump = StdinPump::spawn();
    let mut stdout = tokio::io::stdout();
    let mut stdin_closed = false;
    #[allow(unused_assignments)] // set inside select! arm; false positive
    let mut exit_code: Option<u32> = None;
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge io loop entered"
    );

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
                        tracing::debug!(
                            elapsed_ms = elapsed_ms(bridge_started),
                            "bridge stdin eof observed"
                        );
                        let _ = channel.eof().await;
                    }
                    Some(StdinEvent::Data(data)) => {
                        channel.data(std::io::Cursor::new(&data)).await?;
                    }
                    Some(StdinEvent::Error(err)) => {
                        tracing::warn!(
                            error = %err,
                            elapsed_ms = elapsed_ms(bridge_started),
                            "bridge stdin pump error"
                        );
                        return Err(err.into());
                    }
                }
            }
            // Handle events from the SSH channel.
            msg = channel.wait() => {
                let Some(msg) = msg else {
                    exit_code.get_or_insert(0);
                    tracing::debug!(
                        exit_code = exit_code.unwrap_or(0),
                        elapsed_ms = elapsed_ms(bridge_started),
                        "bridge channel stream ended"
                    );
                    stdin_pump.stop();
                    tracing::debug!(
                        elapsed_ms = elapsed_ms(bridge_started),
                        "bridge stdin pump stopped after channel end"
                    );
                    break;
                };
                if let Some(code) = bridge_terminal_exit_code(&msg) {
                    exit_code.get_or_insert(code);
                    tracing::debug!(
                        event = bridge_terminal_exit_label(&msg).unwrap_or("unknown"),
                        exit_code = code,
                        elapsed_ms = elapsed_ms(bridge_started),
                        "bridge terminal exit event received"
                    );
                    stdin_pump.stop();
                    tracing::debug!(
                        elapsed_ms = elapsed_ms(bridge_started),
                        "bridge stdin pump stopped after terminal exit event"
                    );
                    break;
                }
                match msg {
                    ChannelMsg::Data { ref data } => {
                        stdout.write_all(data).await?;
                        stdout.flush().await?;
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
                    tracing::trace!(
                        cols,
                        rows,
                        elapsed_ms = elapsed_ms(bridge_started),
                        "bridge forwarding terminal resize"
                    );
                    let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                }
            }
        }
    }

    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge post-exit drain started"
    );
    let drain = drain_post_exit_output(channel, &mut stdout).await;
    tracing::debug!(
        timed_out = drain.timed_out,
        saw_terminal_close = drain.saw_terminal_close,
        data_messages = drain.data_messages,
        data_bytes = drain.data_bytes,
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge post-exit drain completed"
    );
    stdin_pump.stop();
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge stdin pump joined"
    );

    Ok(exit_code.unwrap_or(1))
}

async fn drain_post_exit_output(
    channel: &mut russh::Channel<client::Msg>,
    stdout: &mut tokio::io::Stdout,
) -> PostExitDrainSummary {
    let mut summary = PostExitDrainSummary::default();
    loop {
        let Ok(message) = timeout(POST_EXIT_DRAIN_GRACE, channel.wait()).await else {
            summary.timed_out = true;
            break;
        };
        let Some(msg) = message else {
            break;
        };
        match msg {
            ChannelMsg::Data { ref data } => {
                summary.data_messages += 1;
                summary.data_bytes += data.len();
                let _ = stdout.write_all(data).await;
                let _ = stdout.flush().await;
            }
            ChannelMsg::Eof | ChannelMsg::Close => {
                summary.saw_terminal_close = true;
                break;
            }
            _ => {}
        }
    }
    summary
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
    use super::{
        PostExitDrainSummary, bridge_terminal_exit_code, bridge_terminal_exit_label,
        terminal_channel_event, write_bridge_cleanup_sequence,
    };
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

    #[test]
    fn bridge_exit_status_returns_the_remote_code() {
        assert_eq!(
            bridge_terminal_exit_code(&ChannelMsg::ExitStatus { exit_status: 7 }),
            Some(7)
        );
    }

    #[test]
    fn bridge_close_events_default_to_zero() {
        assert_eq!(bridge_terminal_exit_code(&ChannelMsg::Eof), Some(0));
        assert_eq!(bridge_terminal_exit_code(&ChannelMsg::Close), Some(0));
        assert_eq!(bridge_terminal_exit_code(&ChannelMsg::Success), None);
    }

    #[test]
    fn bridge_exit_labels_match_terminal_messages() {
        assert_eq!(
            bridge_terminal_exit_label(&ChannelMsg::ExitStatus { exit_status: 7 }),
            Some("exit_status")
        );
        assert_eq!(bridge_terminal_exit_label(&ChannelMsg::Eof), Some("eof"));
        assert_eq!(
            bridge_terminal_exit_label(&ChannelMsg::Close),
            Some("close")
        );
        assert_eq!(bridge_terminal_exit_label(&ChannelMsg::Success), None);
    }

    #[test]
    fn post_exit_drain_summary_defaults_to_no_activity() {
        assert_eq!(
            PostExitDrainSummary::default(),
            PostExitDrainSummary {
                timed_out: false,
                saw_terminal_close: false,
                data_messages: 0,
                data_bytes: 0,
            }
        );
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
