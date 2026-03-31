use std::path::{Component, Path, PathBuf};

pub(crate) fn default_fixture_dir() -> PathBuf {
    repo_root().join("original/v1.5")
}

pub(crate) fn init_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecutil-init/v1.5")
}

pub(crate) fn post_maint_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecmaint-post/v1.5")
}

pub(crate) fn pre_maint_replay_context_fixture_dir() -> PathBuf {
    repo_root().join("fixtures/ecmaint-fleet-pre/v1.5")
}

pub(crate) fn repo_root() -> PathBuf {
    normalize_path(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

pub(crate) fn display_repo_path(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        normalize_path(repo_root().join(path))
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}
