use std::collections::HashMap;
use std::fmt;

/// Top-level structure of a theme file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThemeFile {
    pub meta: ThemeMeta,

    /// Raw hex color definitions.
    pub palette: HashMap<String, String>,

    /// Semantic token assignments referencing palette names, other tokens, or hex.
    pub tokens: HashMap<String, String>,

    /// Composed styles with fg/bg references and modifiers.
    pub styles: HashMap<String, StyleDef>,

    /// Named gradients as ordered color references.
    pub gradients: HashMap<String, Vec<String>>,
}

/// Theme metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThemeMeta {
    pub name: String,

    pub author: Option<String>,

    pub variant: ThemeVariant,

    pub version: Option<String>,

    pub description: Option<String>,
}

impl ThemeMeta {
    /// Create metadata with just a name — everything else defaults.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            author: None,
            variant: ThemeVariant::default(),
            version: None,
            description: None,
        }
    }
}

/// Whether a theme is designed for dark or light backgrounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub enum ThemeVariant {
    #[default]
    Dark,
    Light,
}

impl fmt::Display for ThemeVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dark => write!(f, "Dark"),
            Self::Light => write!(f, "Light"),
        }
    }
}

/// Style definition as it appears in a theme file.
///
/// Color references (`fg`, `bg`) are resolved against the token and palette maps
/// during theme loading.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StyleDef {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub slow_blink: bool,
    pub rapid_blink: bool,
    pub reversed: bool,
    pub hidden: bool,
    pub crossed_out: bool,
}
