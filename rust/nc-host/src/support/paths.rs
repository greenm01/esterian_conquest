use std::path::{Path, PathBuf};

pub fn hosted_db_path(dir: &Path) -> PathBuf {
    dir.join("hosted.db")
}
