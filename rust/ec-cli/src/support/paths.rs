use std::path::PathBuf;

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn resolve_repo_path(arg: &str) -> PathBuf {
    let path = PathBuf::from(arg);
    if path.is_absolute() {
        path
    } else if path.exists() {
        path
    } else {
        repo_root().join(path)
    }
}
