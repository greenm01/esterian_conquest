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
use std::time::Instant;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::connect::handshake::SessionReadyPayload;
use crate::connect::live::{LiveEvent, LiveSession, TerminalSpec};
use crate::connect::ssh_key::EphemeralKeypair;
use nc_ui::session::write_terminal_cleanup_sequence;

// ── Public entry point ────────────────────────────────────────────────────────

/// Error type for bridge operations.
pub type BridgeError = Box<dyn std::error::Error + Send + Sync>;
#[cfg(unix)]
use std::time::Duration;

#[cfg(unix)]
const STDIN_POLL_GRACE: Duration = Duration::from_millis(20);

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
    let term = env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
    let (cols, rows) = terminal_size().unwrap_or((80, 24));
    let mut live = LiveSession::start(
        payload.clone(),
        EphemeralKeypair::from_signing_key_bytes(keypair.signing_key_bytes()),
        username.to_string(),
        TerminalSpec { term, cols, rows },
    );

    // Enter raw mode for the duration of the session.
    enable_raw_mode()?;
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge raw mode enabled"
    );
    let exit_code = io_loop(&mut live, bridge_started).await;
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
async fn io_loop(live: &mut LiveSession, bridge_started: Instant) -> Result<u32, BridgeError> {
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
                        live.send_eof();
                    }
                    Some(StdinEvent::Data(data)) => {
                        live.send_input(data);
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
            event = live.recv() => {
                let Some(event) = event else {
                    exit_code.get_or_insert(0);
                    break;
                };
                match event {
                    LiveEvent::Output(data) => {
                        stdout.write_all(&data).await?;
                        stdout.flush().await?;
                    }
                    LiveEvent::Exit(code) => {
                        exit_code.get_or_insert(code);
                        stdin_pump.stop();
                        break;
                    }
                    LiveEvent::Error(err) => {
                        return Err(err.into());
                    }
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
                    live.resize(cols, rows);
                }
            }
        }
    }
    stdin_pump.stop();
    tracing::debug!(
        elapsed_ms = elapsed_ms(bridge_started),
        "bridge stdin pump joined"
    );

    Ok(exit_code.unwrap_or(1))
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

#[cfg(test)]
mod tests {
    use super::write_bridge_cleanup_sequence;

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
}
