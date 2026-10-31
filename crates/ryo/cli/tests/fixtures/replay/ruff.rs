//! The copy of ruff a run is edited in.

/// Copies ruff to a directory of its own, where a store can be kept and a file can be edited.
///
/// # Panics
///
/// When the copy cannot be made.
#[must_use]
pub(crate) fn ruff_copy() -> tempfile::TempDir {
    copy_of(&super::base::snapshot::ruff())
}

/// Copies a directory to one of its own, where a store can be kept.
///
/// # Panics
///
/// When the copy cannot be made.
#[must_use]
pub(crate) fn copy_of(source: &std::path::Path) -> tempfile::TempDir {
    let copy = tempfile::tempdir().expect("a directory for the copy");
    copy_into(source, copy.path());
    copy
}

/// Copies the tree at `source` into the directory `into`, links kept as links, on every platform (no `cp` on Windows).
fn copy_into(source: &std::path::Path, into: &std::path::Path) {
    for entry in std::fs::read_dir(source).expect("a directory of the copy is read") {
        let entry = entry.expect("an entry of the copy is read");
        let (from, to) = (entry.path(), into.join(entry.file_name()));
        let kind = entry.file_type().expect("an entry's kind is read");
        if kind.is_symlink() {
            let target = std::fs::read_link(&from).expect("a link is read");
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &to).expect("a link is copied");
            #[cfg(windows)]
            if from.is_dir() {
                std::os::windows::fs::symlink_dir(&target, &to).expect("a link is copied");
            } else {
                std::os::windows::fs::symlink_file(&target, &to).expect("a link is copied");
            }
        } else if kind.is_dir() {
            std::fs::create_dir_all(&to).expect("a directory is copied");
            copy_into(&from, &to);
        } else {
            let _bytes = std::fs::copy(&from, &to).expect("a file is copied");
        }
    }
}
