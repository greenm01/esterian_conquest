pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
    override_text: Option<String>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
            override_text: None,
        }
    }

    pub fn get_text(&mut self) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(text) = self.override_text.clone() {
            return Ok(Some(text));
        }
        let Some(inner) = self.inner.as_mut() else {
            return Ok(None);
        };
        match inner.get_text() {
            Ok(text) => Ok(Some(text)),
            Err(err) => Err(err.into()),
        }
    }

    pub fn replace_fallback(&mut self, text: String) {
        self.override_text = Some(text);
    }
}
