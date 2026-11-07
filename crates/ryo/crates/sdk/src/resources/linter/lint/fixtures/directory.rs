//! The test data of `linter lint` that fixes directory.

/// A directory holding the one file `case.txt`.
pub(crate) fn case_directory() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("corpus");
    std::fs::write(root.path().join("case.txt"), "x").expect("source");
    root
}
