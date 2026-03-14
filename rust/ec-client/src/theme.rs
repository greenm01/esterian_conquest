pub mod classic {
    const RESET: &str = "\x1b[0;38;2;192;192;192;48;2;0;0;0m";
    const FG_WHITE_BG_BLUE: &str = "\x1b[0;38;2;224;224;224;48;2;0;0;170m";
    const FG_BLACK_BG_WHITE: &str = "\x1b[0;38;2;0;0;0;48;2;224;224;224m";
    const FG_WHITE_BG_BLACK: &str = "\x1b[0;38;2;192;192;192;48;2;0;0;0m";
    const FG_YELLOW_BG_BLUE: &str = "\x1b[1;38;2;255;255;85;48;2;0;0;170m";
    const FG_YELLOW_BG_BLACK: &str = "\x1b[1;38;2;255;255;85;48;2;0;0;0m";
    const FG_BRIGHT_WHITE_BG_BLACK: &str = "\x1b[1;38;2;255;255;255;48;2;0;0;0m";
    const FG_GREY_BG_BLACK: &str = "\x1b[0;38;2;192;192;192;48;2;0;0;0m";

    #[derive(Clone, Copy)]
    pub struct MenuEntry<'a> {
        pub hotkey: &'a str,
        pub label: &'a str,
        pub width: usize,
    }

    impl<'a> MenuEntry<'a> {
        pub const fn new(hotkey: &'a str, label: &'a str, width: usize) -> Self {
            Self {
                hotkey,
                label,
                width,
            }
        }
    }

    pub fn title_bar(title: &str, width: usize) -> String {
        let padding = width.saturating_sub(title.len());
        format!(
            "{FG_BLACK_BG_WHITE}{title}{FG_WHITE_BG_BLUE}{:padding$}{RESET}",
            "",
            padding = padding
        )
    }

    pub fn menu_row(entries: &[MenuEntry<'_>]) -> String {
        let mut line = String::from(FG_WHITE_BG_BLUE);
        line.push_str("  ");
        for entry in entries {
            line.push_str(&format_menu_entry(*entry));
        }
        let used_width = 2 + entries
            .iter()
            .map(|entry| entry.width)
            .sum::<usize>();
        let padding = 78usize.saturating_sub(used_width);
        line.push_str(&" ".repeat(padding));
        line.push_str(RESET);
        line
    }

    pub fn command_prompt(label: &str, keys: &str) -> String {
        format!(
            "{FG_BLACK_BG_WHITE}{label}{FG_WHITE_BG_BLACK} <-{FG_YELLOW_BG_BLACK}{keys}{FG_WHITE_BG_BLACK}-> {RESET}"
        )
    }

    pub fn status_line(label: &str, value: &str) -> String {
        format!("{FG_GREY_BG_BLACK}{label}{FG_BRIGHT_WHITE_BG_BLACK}{value}{RESET}")
    }

    pub fn splash_logo_lines() -> Vec<String> {
        vec![
            format!("{FG_BRIGHT_WHITE_BG_BLACK}ESTERIAN{RESET}"),
            format!("{FG_BRIGHT_WHITE_BG_BLACK}CONQUEST{RESET}"),
        ]
    }

    pub fn centered_text(text: &str, width: usize) -> String {
        let padding = width.saturating_sub(visible_width(text)) / 2;
        format!("{}{text}", " ".repeat(padding))
    }

    fn format_menu_entry(entry: MenuEntry<'_>) -> String {
        if entry.width == 0 || (entry.hotkey.is_empty() && entry.label.is_empty()) {
            return String::new();
        }
        let body = format!(">{}", entry.label);
        let used = entry.hotkey.len() + body.len();
        let padding = entry.width.saturating_sub(used);
        format!(
            "{FG_YELLOW_BG_BLUE}{hotkey}{FG_WHITE_BG_BLUE}{body}{:padding$}",
            "",
            hotkey = entry.hotkey,
            body = body,
            padding = padding
        )
    }

    fn visible_width(text: &str) -> usize {
        let bytes = text.as_bytes();
        let mut idx = 0;
        let mut width = 0;
        while idx < bytes.len() {
            if bytes[idx] == 0x1b {
                idx += 1;
                if idx < bytes.len() && bytes[idx] == b'[' {
                    idx += 1;
                    while idx < bytes.len() {
                        let byte = bytes[idx];
                        idx += 1;
                        if byte.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                continue;
            }
            width += 1;
            idx += 1;
        }
        width
    }
}
