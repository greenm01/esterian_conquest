/// Motivational quotes loaded from embedded KDL config.

const QUOTES_KDL: &str = include_str!("../config/quotes.kdl");

pub struct Quote {
    pub text: String,
    pub author: String,
}

/// Parse embedded KDL quotes file into a list of Quote values.
/// Panics at startup if the embedded KDL is malformed (compile-time asset).
pub fn load_quotes() -> Vec<Quote> {
    let document: kdl::KdlDocument = QUOTES_KDL
        .parse()
        .expect("embedded quotes.kdl should be valid KDL");

    let mut quotes = Vec::new();
    for node in document.nodes() {
        if node.name().value() != "quote" {
            continue;
        }
        let text = node
            .get(0)
            .and_then(|v| v.as_string())
            .expect("quote node must have a text argument")
            .to_string();

        let author = node
            .children()
            .and_then(|children| {
                children.nodes().iter().find_map(|child| {
                    if child.name().value() == "author" {
                        child.get(0).and_then(|v| v.as_string()).map(str::to_string)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "Unknown".to_string());

        quotes.push(Quote { text, author });
    }

    quotes
}
