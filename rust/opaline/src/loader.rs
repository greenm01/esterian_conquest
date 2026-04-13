use std::path::Path;

use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

use crate::error::OpalineError;
use crate::resolver;
use crate::schema::{StyleDef, ThemeFile, ThemeMeta, ThemeVariant};
use crate::theme::Theme;

/// Load a theme from a KDL string.
///
/// The optional `path` is stored for error diagnostics only.
pub fn load_from_str(kdl_str: &str, path: Option<&Path>) -> Result<Theme, OpalineError> {
    let document: KdlDocument = kdl_str.parse().map_err(|source: kdl::KdlError| OpalineError::Parse {
        path: path.map(Path::to_path_buf),
        message: source.to_string(),
    })?;
    let theme_file = parse_theme_file(&document, path)?;

    let resolved = resolver::resolve(&theme_file)?;
    Ok(Theme::from_resolved(theme_file.meta, resolved))
}

/// Load a theme from a KDL file on disk.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<Theme, OpalineError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path).map_err(|source| OpalineError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    load_from_str(&contents, Some(path))
}

fn parse_theme_file(document: &KdlDocument, path: Option<&Path>) -> Result<ThemeFile, OpalineError> {
    let mut meta = None;
    let mut palette = std::collections::HashMap::new();
    let mut tokens = std::collections::HashMap::new();
    let mut styles = std::collections::HashMap::new();
    let mut gradients = std::collections::HashMap::new();

    for node in document.nodes() {
        match node.name().value() {
            "meta" => {
                if meta.is_some() {
                    return Err(parse_error(path, "duplicate meta node"));
                }
                meta = Some(parse_meta(node, path)?);
            }
            "palette" => {
                let (name, value) = parse_pair_node(node, path, "palette")?;
                palette.insert(name, value);
            }
            "token" => {
                let (name, value) = parse_pair_node(node, path, "token")?;
                tokens.insert(name, value);
            }
            "style" => {
                let (name, style) = parse_style(node, path)?;
                styles.insert(name, style);
            }
            "gradient" => {
                let (name, stops) = parse_gradient(node, path)?;
                gradients.insert(name, stops);
            }
            other => {
                return Err(parse_error(
                    path,
                    &format!("unknown top-level node '{other}'"),
                ));
            }
        }
    }

    let Some(meta) = meta else {
        return Err(OpalineError::MissingSection {
            section: "meta".to_string(),
        });
    };

    Ok(ThemeFile {
        meta,
        palette,
        tokens,
        styles,
        gradients,
    })
}

fn parse_meta(node: &KdlNode, path: Option<&Path>) -> Result<ThemeMeta, OpalineError> {
    reject_positional_entries(node, path, "meta")?;
    reject_unknown_properties(
        node,
        path,
        &["name", "author", "variant", "version", "description"],
    )?;

    let Some(name) = prop_string(node, "name") else {
        return Err(parse_error(path, "meta node missing string property 'name'"));
    };

    let variant = match prop_string(node, "variant").as_deref() {
        None | Some("dark") => ThemeVariant::Dark,
        Some("light") => ThemeVariant::Light,
        Some(other) => {
            return Err(parse_error(
                path,
                &format!("meta.variant must be 'dark' or 'light', got '{other}'"),
            ));
        }
    };

    Ok(ThemeMeta {
        name,
        author: prop_string(node, "author"),
        variant,
        version: prop_string(node, "version"),
        description: prop_string(node, "description"),
    })
}

fn parse_pair_node(
    node: &KdlNode,
    path: Option<&Path>,
    node_type: &str,
) -> Result<(String, String), OpalineError> {
    reject_unknown_properties(node, path, &[])?;
    let values = positional_strings(node);
    if values.len() != 2 {
        return Err(parse_error(
            path,
            &format!("{node_type} node must have exactly two string arguments"),
        ));
    }
    Ok((values[0].clone(), values[1].clone()))
}

fn parse_style(node: &KdlNode, path: Option<&Path>) -> Result<(String, StyleDef), OpalineError> {
    reject_unknown_properties(
        node,
        path,
        &[
            "fg",
            "bg",
            "bold",
            "dim",
            "italic",
            "underline",
            "slow_blink",
            "rapid_blink",
            "reversed",
            "hidden",
            "crossed_out",
        ],
    )?;

    let values = positional_strings(node);
    if values.len() != 1 {
        return Err(parse_error(
            path,
            "style node must have exactly one string argument",
        ));
    }

    Ok((
        values[0].clone(),
        StyleDef {
            fg: prop_string(node, "fg"),
            bg: prop_string(node, "bg"),
            bold: prop_bool(node, "bold", path)?,
            dim: prop_bool(node, "dim", path)?,
            italic: prop_bool(node, "italic", path)?,
            underline: prop_bool(node, "underline", path)?,
            slow_blink: prop_bool(node, "slow_blink", path)?,
            rapid_blink: prop_bool(node, "rapid_blink", path)?,
            reversed: prop_bool(node, "reversed", path)?,
            hidden: prop_bool(node, "hidden", path)?,
            crossed_out: prop_bool(node, "crossed_out", path)?,
        },
    ))
}

fn parse_gradient(
    node: &KdlNode,
    path: Option<&Path>,
) -> Result<(String, Vec<String>), OpalineError> {
    reject_unknown_properties(node, path, &[])?;
    let values = positional_strings(node);
    if values.is_empty() {
        return Err(parse_error(
            path,
            "gradient node must have at least one string argument",
        ));
    }
    Ok((values[0].clone(), values[1..].to_vec()))
}

fn reject_positional_entries(
    node: &KdlNode,
    path: Option<&Path>,
    node_type: &str,
) -> Result<(), OpalineError> {
    if node.entries().iter().any(|entry| entry.name().is_none()) {
        return Err(parse_error(
            path,
            &format!("{node_type} node does not accept positional arguments"),
        ));
    }
    Ok(())
}

fn reject_unknown_properties(
    node: &KdlNode,
    path: Option<&Path>,
    allowed: &[&str],
) -> Result<(), OpalineError> {
    for entry in node.entries() {
        let Some(name) = entry.name() else {
            continue;
        };
        if !allowed.iter().any(|allowed_name| *allowed_name == name.value()) {
            return Err(parse_error(
                path,
                &format!(
                    "{} node has unknown property '{}'",
                    node.name().value(),
                    name.value()
                ),
            ));
        }
    }
    Ok(())
}

fn positional_strings(node: &KdlNode) -> Vec<String> {
    node.entries()
        .iter()
        .filter(|entry| entry.name().is_none())
        .filter_map(string_entry)
        .collect()
}

fn string_entry(entry: &KdlEntry) -> Option<String> {
    entry.value().as_string().map(str::to_string)
}

fn prop_string(node: &KdlNode, name: &str) -> Option<String> {
    node.get(name)
        .and_then(KdlValue::as_string)
        .map(str::to_string)
}

fn prop_bool(node: &KdlNode, name: &str, path: Option<&Path>) -> Result<bool, OpalineError> {
    match node.get(name) {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or_else(|| {
            parse_error(
                path,
                &format!("{}.{} must be a boolean", node.name().value(), name),
            )
        }),
    }
}

fn parse_error(path: Option<&Path>, message: &str) -> OpalineError {
    OpalineError::Parse {
        path: path.map(Path::to_path_buf),
        message: message.to_string(),
    }
}
