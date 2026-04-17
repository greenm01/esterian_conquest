use crate::buffer::PlayfieldBuffer;

pub struct UiScene {
    playfield: PlayfieldBuffer,
}

impl UiScene {
    pub fn from_playfield(playfield: PlayfieldBuffer) -> Self {
        Self { playfield }
    }

    pub fn playfield(&self) -> &PlayfieldBuffer {
        &self.playfield
    }

    pub fn into_playfield(self) -> PlayfieldBuffer {
        self.playfield
    }
}

impl From<PlayfieldBuffer> for UiScene {
    fn from(playfield: PlayfieldBuffer) -> Self {
        Self::from_playfield(playfield)
    }
}
