pub trait LobbyTransport {
    fn status_label(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopLobbyTransport;

impl LobbyTransport for NoopLobbyTransport {
    fn status_label(&self) -> &'static str {
        "transport: stub"
    }
}
