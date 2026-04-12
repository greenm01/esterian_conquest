use std::path::Path;

pub fn keychain_exists(path: &Path) -> bool {
    path.is_file()
}

pub fn validate_new_password(password: &str, confirm: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    if password != confirm {
        return Err("passwords do not match".to_string());
    }
    Ok(())
}
