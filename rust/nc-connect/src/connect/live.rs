use std::sync::Arc;
use std::time::Duration;

use russh::keys::ssh_key::{
    HashAlg,
    private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData},
};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use russh::{ChannelMsg, client};
use tokio::sync::mpsc;

use crate::connect::handshake::SessionReadyPayload;
use crate::connect::ssh_key::EphemeralKeypair;

pub type LiveError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSpec {
    pub term: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug)]
pub enum LiveEvent {
    Output(Vec<u8>),
    Exit(u32),
    Error(String),
}

enum LiveCommand {
    Input(Vec<u8>),
    Resize { cols: u16, rows: u16 },
    Eof,
    Close,
}

pub struct LiveSession {
    tx: mpsc::UnboundedSender<LiveCommand>,
    rx: mpsc::UnboundedReceiver<LiveEvent>,
}

impl LiveSession {
    pub fn start(
        payload: SessionReadyPayload,
        keypair: EphemeralKeypair,
        username: String,
        terminal: TerminalSpec,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(err) => {
                    let _ =
                        event_tx.send(LiveEvent::Error(format!("unable to start runtime: {err}")));
                    return;
                }
            };
            if let Err(err) = rt.block_on(run_live_session(
                payload,
                keypair,
                username,
                terminal,
                command_rx,
                event_tx.clone(),
            )) {
                let _ = event_tx.send(LiveEvent::Error(err.to_string()));
            }
        });

        Self {
            tx: command_tx,
            rx: event_rx,
        }
    }

    pub fn send_input(&self, data: Vec<u8>) {
        let _ = self.tx.send(LiveCommand::Input(data));
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.tx.send(LiveCommand::Resize { cols, rows });
    }

    pub fn send_eof(&self) {
        let _ = self.tx.send(LiveCommand::Eof);
    }

    pub fn close(&self) {
        let _ = self.tx.send(LiveCommand::Close);
    }

    pub fn try_recv(&mut self) -> Result<LiveEvent, mpsc::error::TryRecvError> {
        self.rx.try_recv()
    }

    pub async fn recv(&mut self) -> Option<LiveEvent> {
        self.rx.recv().await
    }
}

async fn run_live_session(
    payload: SessionReadyPayload,
    keypair: EphemeralKeypair,
    username: String,
    terminal: TerminalSpec,
    mut command_rx: mpsc::UnboundedReceiver<LiveCommand>,
    event_tx: mpsc::UnboundedSender<LiveEvent>,
) -> Result<(), LiveError> {
    let addr = (payload.ssh_host.as_str(), payload.ssh_port);
    let ssh_username = if payload.ssh_user.is_empty() {
        username.as_str()
    } else {
        payload.ssh_user.as_str()
    };

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(300)),
        ..<_>::default()
    });

    let mut session = client::connect(
        config,
        addr,
        FingerprintHandler {
            expected_fingerprint: payload.host_fingerprint.clone(),
        },
    )
    .await?;

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
    channel
        .request_pty(
            false,
            &terminal.term,
            terminal.cols as u32,
            terminal.rows as u32,
            0,
            0,
            &[],
        )
        .await?;
    channel.exec(true, "").await?;

    loop {
        tokio::select! {
            Some(command) = command_rx.recv() => match command {
                LiveCommand::Input(data) => {
                    channel.data(std::io::Cursor::new(&data)).await?;
                }
                LiveCommand::Resize { cols, rows } => {
                    let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                }
                LiveCommand::Eof => {
                    let _ = channel.eof().await;
                }
                LiveCommand::Close => {
                    let _ = channel.eof().await;
                    break;
                }
            },
            message = channel.wait() => {
                let Some(message) = message else {
                    let _ = event_tx.send(LiveEvent::Exit(0));
                    break;
                };
                match message {
                    ChannelMsg::Data { data } => {
                        let _ = event_tx.send(LiveEvent::Output(data.to_vec()));
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        let _ = event_tx.send(LiveEvent::Exit(exit_status));
                        break;
                    }
                    ChannelMsg::Eof | ChannelMsg::Close => {
                        let _ = event_tx.send(LiveEvent::Exit(0));
                        break;
                    }
                    ChannelMsg::ExtendedData { data, .. } => {
                        let _ = event_tx.send(LiveEvent::Output(data.to_vec()));
                    }
                    ChannelMsg::WindowAdjusted { .. }
                    | ChannelMsg::Success
                    | ChannelMsg::Failure => {}
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

struct FingerprintHandler {
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

impl EphemeralKeypair {
    pub fn to_russh_private_key(&self) -> Result<PrivateKey, LiveError> {
        let seed = self.signing_key_bytes();
        let ed_priv = Ed25519PrivateKey::from_bytes(&seed);
        let ed_kp = Ed25519Keypair::from(ed_priv);
        let key_data = KeypairData::from(ed_kp);
        PrivateKey::new(key_data, "")
            .map_err(|err| format!("failed to construct PrivateKey: {err}").into())
    }
}
