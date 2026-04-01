/// Find the first tag with the given name and return its content (index 1).
pub fn tag_content<'a>(tags: &'a nostr_sdk::event::Tags, name: &str) -> Option<&'a str> {
    tags.iter().find_map(|tag| {
        if tag.kind().as_str() == name {
            tag.content()
        } else {
            None
        }
    })
}
