/// Extract a quoted string field value by key from a compact JSON object.
///
/// Handles the JSON escape sequences `\\`, `\"`, `\n`, `\r`, `\t`.
/// Whitespace around the `:` separator is allowed.
///
/// Returns `Err` if the key is missing, the value is not a string, or the
/// string is unterminated.
pub fn extract_str(json: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{key}\"");
    let key_pos = json
        .find(&needle)
        .ok_or_else(|| format!("missing field '{key}'"))?;
    let after_key = &json[key_pos + needle.len()..];
    let colon_pos = after_key
        .find(':')
        .ok_or_else(|| format!("malformed field '{key}'"))?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return Err(format!("field '{key}' is not a string"));
    }
    let inner = &after_colon[1..];
    let mut value = String::new();
    let mut chars = inner.chars();
    loop {
        match chars.next() {
            None => return Err(format!("unterminated string for field '{key}'")),
            Some('"') => break,
            Some('\\') => match chars.next() {
                Some('"') => value.push('"'),
                Some('\\') => value.push('\\'),
                Some('n') => value.push('\n'),
                Some('r') => value.push('\r'),
                Some('t') => value.push('\t'),
                Some(c) => {
                    value.push('\\');
                    value.push(c);
                }
                None => return Err(format!("truncated escape in field '{key}'")),
            },
            Some(c) => value.push(c),
        }
    }
    Ok(value)
}

/// Extract an unsigned integer field value by key from a compact JSON object.
///
/// Returns `None` if the key is missing or the value is not a valid `u32`.
pub fn extract_u32(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{key}\"");
    let key_pos = json.find(&needle)?;
    let after_key = &json[key_pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let end = after_colon
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon[..end].parse().ok()
}

/// Escape a string for safe embedding inside a JSON double-quoted value.
///
/// Handles `\`, `"`, newline, carriage return, and tab.
pub fn escape_json_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
