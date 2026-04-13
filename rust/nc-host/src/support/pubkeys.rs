pub fn short_pubkey(value: &str) -> String {
    const PREFIX_LEN: usize = 12;

    if value.len() <= PREFIX_LEN {
        return value.to_string();
    }

    format!("{}...", &value[..PREFIX_LEN])
}
