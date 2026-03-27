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
use std::sync::Arc;
use std::time::Duration;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size};
use russh::keys::ssh_key::{
    HashAlg,
    private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData},
};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, Disconnect, client};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::connect::handshake::SessionReadyPayload;
use crate::connect::ssh_key::EphemeralKeypair;

// ── Public entry point ────────────────────────────────────────────────────────

/// Error type for bridge operations.
pub type BridgeError = Box<dyn std::error::Error + Send + Sync>;

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
            username,
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

    session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await?;

    exit_code
}

// ── I/O loop ─────────────────────────────────────────────────────────────────

/// Drive stdin → channel and channel → stdout until the remote side closes.
async fn io_loop(channel: &mut russh::Channel<client::Msg>) -> Result<u32, BridgeError> {
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut buf = vec![0u8; 4096];
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
            r = stdin.read(&mut buf), if !stdin_closed => {
                match r {
                    Ok(0) => {
                        stdin_closed = true;
                        channel.eof().await?;
                    }
                    Ok(n) => {
                        channel.data(std::io::Cursor::new(&buf[..n])).await?;
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            }
            // Handle events from the SSH channel.
            Some(msg) = channel.wait() => {
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
                    ChannelMsg::WindowAdjusted { .. }
                    | ChannelMsg::Success
                    | ChannelMsg::Failure
                    | ChannelMsg::Close => {}
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

    // Drain any remaining output from the channel.
    while let Some(msg) = channel.wait().await {
        if let ChannelMsg::Data { ref data } = msg {
            let _ = stdout.write_all(data).await;
            let _ = stdout.flush().await;
        }
    }

    Ok(exit_code.unwrap_or(1))
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
