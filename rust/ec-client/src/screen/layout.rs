use crate::terminal::Terminal;

pub const PLAYFIELD_WIDTH: usize = 80;
pub const PLAYFIELD_HEIGHT: usize = 20;

pub fn write_prompt(
    terminal: &mut dyn Terminal,
    current_lines: usize,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    terminal.write_line(prompt)?;
    terminal.set_cursor(visible_width(prompt) as u16, current_lines as u16)?;
    Ok(())
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
