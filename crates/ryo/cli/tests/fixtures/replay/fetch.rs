//! A pinned revision of a repository, fetched once into a shared cache, however many processes ask for it at once.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The checkout of `revision` of `repository` in `cache`, fetched there once. Every process and thread that fetches into the cache
/// (the distribution tests too) holds its one lock while it looks and fetches, so two never write the same checkout at once: the
/// second finds the first's.
///
/// # Panics
///
/// When the cache cannot be written or the revision cannot be fetched.
pub(crate) fn fetched(cache: &Path, name: &str, repository: &str, revision: &str) -> PathBuf {
    std::fs::create_dir_all(cache).expect("a cache directory");
    let lock = std::fs::File::create(cache.join("lock")).expect("the cache's lock file");
    lock.lock().expect("the cache's lock");
    let directory = cache.join(format!("{name}-{revision}"));
    if !directory.exists() {
        // A partial checkout left by a fetch that did not finish is started again.
        let partial = cache.join(format!("{name}-{revision}.partial"));
        if partial.exists() {
            std::fs::remove_dir_all(&partial).expect("the unfinished checkout is removed");
        }
        std::fs::create_dir_all(&partial).expect("a cache directory");
        let git = |arguments: &[&str]| {
            // Never wait on a prompt for credentials: a repository or revision that cannot be fetched fails at once.
            let status = Command::new("git")
                .args(arguments)
                .current_dir(&partial)
                .env("GIT_TERMINAL_PROMPT", "0")
                .status()
                .expect("git runs");
            assert!(
                status.success(),
                "git {arguments:?} failed for the target `{name}`: repository {repository}, revision {revision}"
            );
        };
        git(&["init", "--quiet"]);
        // The checkout is the revision byte for byte, links as links, whatever the platform's git would convert.
        git(&["config", "core.autocrlf", "false"]);
        git(&["config", "core.symlinks", "true"]);
        // Windows' paths are short of the longest of ruff's files unless git asks for long ones.
        git(&["config", "core.longpaths", "true"]);
        git(&["fetch", "--quiet", "--depth", "1", repository, revision]);
        git(&["checkout", "--quiet", "--detach", revision]);
        std::fs::rename(&partial, &directory).expect("the snapshot is kept");
    }
    directory
}
