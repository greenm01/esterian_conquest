use std::io::{self, Write};
use std::path::Path;

pub const WALLET_WARNING_LINES: [&str; 3] = [
    "This password encrypts your wallet.",
    "If you lose it, you will be locked out.",
    "No IT support.",
];

pub fn wallet_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn prompt_password(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    rpassword::prompt_password(prompt).map_err(|e| e.into())
}

pub fn write_wallet_warning<W: Write>(writer: &mut W) -> io::Result<()> {
    for line in WALLET_WARNING_LINES {
        writeln!(writer, "{line}")?;
    }
    Ok(())
}

pub fn prompt_new_password_with_warning() -> Result<String, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    write_wallet_warning(&mut stdout)?;
    stdout.flush()?;

    let password = prompt_password("New password: ")?;
    if password.is_empty() {
        return Err("password cannot be empty".into());
    }

    let confirm = prompt_password("Confirm password: ")?;
    if confirm != password {
        return Err("passwords do not match".into());
    }

    Ok(password)
}
