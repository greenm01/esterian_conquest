pub(super) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0F) as usize] as char);
    }
    out
}

pub(super) fn decode_hex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("hex text length must be even".to_string());
    }
    let mut out = Vec::with_capacity(text.len() / 2);
    let bytes = text.as_bytes();
    for idx in (0..bytes.len()).step_by(2) {
        let hi = decode_hex_nibble(bytes[idx])
            .ok_or_else(|| format!("invalid hex character at offset {idx}"))?;
        let lo = decode_hex_nibble(bytes[idx + 1])
            .ok_or_else(|| format!("invalid hex character at offset {}", idx + 1))?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn decode_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
