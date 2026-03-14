use std::fs;
use std::path::{Path, PathBuf};

const QUEUE_FILE_NAME: &str = "RUSTMAIL.QUE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedPlayerMail {
    pub sender_empire_id: u8,
    pub recipient_empire_id: u8,
    pub year: u16,
    pub subject: String,
    pub body: String,
}

pub fn queue_path(dir: &Path) -> PathBuf {
    dir.join(QUEUE_FILE_NAME)
}

pub fn load_mail_queue(dir: &Path) -> Result<Vec<QueuedPlayerMail>, Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let parts = line.split('\t').collect::<Vec<_>>();
        let (sender, recipient, year, subject, body) = match parts.as_slice() {
            [sender, recipient, year, body] => (*sender, *recipient, *year, "", *body),
            [sender, recipient, year, subject, body] => {
                (*sender, *recipient, *year, *subject, *body)
            }
            _ => continue,
        };
        out.push(QueuedPlayerMail {
            sender_empire_id: sender.parse()?,
            recipient_empire_id: recipient.parse()?,
            year: year.parse()?,
            subject: unescape_field(subject),
            body: unescape_field(body),
        });
    }
    Ok(out)
}

pub fn append_mail_queue(
    dir: &Path,
    mail: &QueuedPlayerMail,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut queue = load_mail_queue(dir)?;
    queue.push(mail.clone());
    save_mail_queue(dir, &queue)
}

pub fn save_mail_queue(
    dir: &Path,
    queue: &[QueuedPlayerMail],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if queue.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    let mut text = String::new();
    for mail in queue {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            mail.sender_empire_id,
            mail.recipient_empire_id,
            mail.year,
            escape_field(&mail.subject),
            escape_field(&mail.body)
        ));
    }
    fs::write(path, text)?;
    Ok(())
}

pub fn clear_mail_queue(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = queue_path(dir);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn escape_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}
