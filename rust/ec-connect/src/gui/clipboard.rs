pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }

    pub fn get_text(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(None);
        };
        match inner.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(err) => Err(err.into()),
        }
    }

    pub fn set_text(&mut self, text: String) -> Result<(), Box<dyn std::error::Error>> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(());
        };
        inner.set_text(text)?;
        Ok(())
    }
}
