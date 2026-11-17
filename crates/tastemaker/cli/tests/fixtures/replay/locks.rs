//! Every workspace lock of the repository, copied into a scene that reproduces it: a scene carries its own `.git`, so tm
//! reads the scene as the repository root and refuses, as unread, a version that only a lock outside the copied trees reads.

use std::path::Path;

/// The directories a walk for locks leaves out: build trees, caches and version control, which hold no workspace's lock.
const LEFT_OUT: [&str; 5] = [
    "target",
    ".tastemaker_cache",
    ".ryo_cache",
    ".git",
    "node_modules",
];

/// Copies every `tastemaker.lock` under `directory` to its place under `scene`, relative to the repository's `root`.
///
/// # Errors
///
/// When a directory cannot be read or a lock cannot be copied.
pub(crate) fn copy_locks(root: &Path, directory: &Path, scene: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if LEFT_OUT.iter().any(|left_out| name == *left_out) {
            continue;
        }
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            copy_locks(root, &path, scene)?;
        } else if name == "tastemaker.lock" {
            let relative = path.strip_prefix(root).map_err(std::io::Error::other)?;
            let target = scene.join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _: u64 = std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
