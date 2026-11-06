//! A file of the ignore corpus adapter's test support: the files its reads are run over, on disk and given as text.

use super::walking::{known_entry, walking};
use crate::outbound::corpus::adapters::ignore::adapter::CorpusIgnoreAdapter;
use crate::outbound::corpus::errors::CorpusError;
use crate::outbound::corpus::interface::CorpusPort as _;
use crate::outbound::corpus::types::SkippedDirectories;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusFile, CorpusListing, FileBytes, FileSize, Nanoseconds,
};

/// Reads a sparse file of one byte more than the engine can number.
pub(crate) fn too_large() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    let file = std::fs::File::create(root.path().join("big.bin")).expect("a file");
    file.set_len(u64::from(u32::MAX) + 1)
        .expect("a sparse file");
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("big.bin".into()),
        size: FileSize(u64::from(u32::MAX) + 1),
        modified: Nanoseconds(0),
    };
    walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
        .read(&entry, &mut FileBytes::default())
        .map(|()| 0)
}

/// Reads a file given as text where the disk has a file of the same path.
pub(crate) fn given_over_disk() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    let adapter = CorpusIgnoreAdapter::new(
        RootPath(root.path().to_str().expect("a utf-8 path").into()),
        CorpusListing::Walked,
        vec![CorpusFile {
            executable: Default::default(),
            path: FilePath("a.rs".into()),
            bytes: FileBytes(b"fn given() {}".to_vec()),
        }],
        SkippedDirectories::default(),
    );
    let entries = adapter.inventory()?.0;
    let mut bytes = FileBytes::default();
    adapter.read(known_entry(&entries[0]), &mut bytes)?;
    Ok(usize::from(
        entries.len() == 1 && bytes.0 == b"fn given() {}",
    ))
}

/// Reads a file that is not on disk: how many times the refusal names its path.
pub(crate) fn missing_file() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("gone.rs".into()),
        size: FileSize(9),
        modified: Nanoseconds(0),
    };
    let adapter = walking(RootPath(root.path().to_str().expect("a utf-8 path").into()));
    match adapter.read(&entry, &mut FileBytes::default()) {
        Err(error @ CorpusError::Unreadable { .. }) => {
            Ok(error.to_string().matches("gone.rs").count())
        }
        other => other.map(|()| 0),
    }
}

/// Reads a file of nine bytes: how many bytes come in.
pub(crate) fn whole() -> Result<usize, CorpusError> {
    let root = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(root.path().join("a.rs"), "fn a() {}").expect("a file");
    let entry = CorpusEntry {
        executable: Default::default(),
        path: FilePath("a.rs".into()),
        size: FileSize(9),
        modified: Nanoseconds(0),
    };
    let mut bytes = FileBytes::default();
    walking(RootPath(root.path().to_str().expect("a utf-8 path").into()))
        .read(&entry, &mut bytes)?;
    Ok(bytes.0.len())
}
