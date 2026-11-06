//! A file of the ignore corpus adapter's test support: a listing that names files, one of them not on disk, beside a file given as text.

use crate::outbound::corpus::adapters::ignore::adapter::CorpusIgnoreAdapter;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort as _;
use crate::outbound::corpus::types::SkippedDirectories;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::{CorpusFile, CorpusListing, FileBytes};

/// A listing lists only the named files that are there, and the given ones.
pub(crate) fn listed() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    std::fs::write(root.path().join("unlisted.rs"), "fn u() {}").expect("a file");
    let adapter = CorpusIgnoreAdapter::new(
        RootPath(root.path().to_str().expect("a utf-8 path").into()),
        CorpusListing::Listed(vec![FilePath("a.rs".into()), FilePath("gone.rs".into())]),
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("given/b.rs".into()),
            bytes: FileBytes(b"fn b() {}".to_vec()),
        }],
        SkippedDirectories::default(),
    );
    let mut entries = adapter.inventory()?.0;
    entries.sort_by(|left, right| {
        ryo_values::domain::files::service::input_path(left)
            .cmp(ryo_values::domain::files::service::input_path(right))
    });
    let found: Vec<(&str, u64)> = entries
        .iter()
        .filter_map(|input| match input {
            ryo_values::domain::files::types::CorpusInput::Available(entry) => {
                Some((&*entry.path.0, entry.size.0))
            }
            _ => None,
        })
        .collect();
    let missing = entries
        .iter()
        .filter(|input| {
            matches!(input,
        ryo_values::domain::files::types::CorpusInput::Unreadable { path, reason }
            if path.0.as_ref() == "gone.rs" && !reason.0.is_empty())
        })
        .count();
    Ok(usize::from(
        found == [("a.rs", 9), ("given/b.rs", 9)] && missing == 1 && entries.len() == 3,
    ))
}
