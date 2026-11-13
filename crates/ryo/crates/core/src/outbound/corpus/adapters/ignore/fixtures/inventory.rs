//! A file of the ignore corpus adapter's test support: the repositories its walk is run over and what the walk lists of them.

use super::walking::{known_entry, walking};
use crate::outbound::corpus::adapters::ignore::adapter::CorpusIgnoreAdapter;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort as _;
use crate::outbound::corpus::types::SkippedDirectories;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::CorpusListing;

/// The walk lists the files under the root, sized and dated, and leaves out the repository's own directory.
pub(crate) fn inventory() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir_all(root.path().join("sub")).expect("a subdirectory");
    std::fs::create_dir_all(root.path().join(".git")).expect("a git directory");
    std::fs::write(root.path().join("b.rs"), "fn b() {}").expect("a file");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    std::fs::write(root.path().join("sub/c.txt"), "c").expect("a file");
    std::fs::write(root.path().join(".git/config"), "x").expect("a file");
    let mut entries = walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
        .inventory()?
        .0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<(&str, u64)> = entries
        .iter()
        .map(|entry| (&*known_entry(entry).path.0, known_entry(entry).size.0))
        .collect();
    // A file the system dates carries the date, so an edit shows without a read.
    let dated = entries
        .iter()
        .all(|entry| known_entry(entry).modified.0 > 0);
    Ok(usize::from(
        found == [("a.rs", 9), ("b.rs", 9), ("sub/c.txt", 1)] && dated,
    ))
}

/// What the walk lists when only `./target` is skipped.
pub(crate) const WITHOUT_ROOT_TARGET: &[&str] = &[
    ".venv/lib/y.py",
    "node_modules/x.js",
    "pkg/node_modules/v.js",
    "pkg/target/w.rs",
    "src/a.ts",
];

/// A walk of a repository with no `.gitignore`, holding `node_modules/x.js`, `pkg/node_modules/v.js`, `.venv/lib/y.py`,
/// `target/z.rs`, `pkg/target/w.rs` and `src/a.ts`, lists the files `expected` names when the directories named `skipped` are
/// skipped.
pub(crate) fn skipping(skipped: &[&str], expected: &[&str]) -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    for file in [
        "node_modules/x.js",
        "pkg/node_modules/v.js",
        ".venv/lib/y.py",
        "target/z.rs",
        "pkg/target/w.rs",
        "src/a.ts",
    ] {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("directories");
        std::fs::write(path, "x").expect("a file");
    }
    let adapter = CorpusIgnoreAdapter::new(
        RootPath(root.path().to_str().expect("a utf-8 path").into()),
        CorpusListing::Walked,
        Vec::new(),
        SkippedDirectories(skipped.iter().map(|name| (*name).into()).collect()),
    );
    let mut entries = adapter.inventory()?.0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<&str> = entries
        .iter()
        .map(|entry| &*ryo_values::domain::files::service::input_path(entry).0)
        .collect();
    Ok(usize::from(found == expected))
}
